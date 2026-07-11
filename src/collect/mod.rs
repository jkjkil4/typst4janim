mod bezier;
mod image;
mod path;
mod shape;
mod text;
mod utils;
mod warnings;

pub use shape::ShapeInfo;
pub use text::TextGlyphInfo;

use std::hash::Hash;

use indexmap::IndexMap;
use ndarray::Array2;
use numpy::{IntoPyArray, PyArray2};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use rustc_hash::FxBuildHasher;

use typst::layout::{Frame, FrameItem, GroupItem, Size, Transform};
use typst::visualize::Geometry;
use typst_layout::{Page, PagedDocument};

use crate::collect::warnings::ExportWarning;
use crate::{ConvertError, TypstError};

/// Export `document`'s first page into [`Collected`] struct
///
/// Returns [`TypstError`] is the document has more than one page
pub fn export_to_python<'py>(
    py: Python<'py>,
    document: PagedDocument,
) -> PyResult<Bound<'py, Collected>> {
    let pages = document.pages();
    if pages.len() > 1 {
        return Err(TypstError::new_err(format!(
            "Cannot compile multi-page typst document (found {} pages)",
            pages.len()
        )));
    }

    let page = pages.first().unwrap();
    // `size` is used for Gradient and Tiling
    // For further implementation, consider processing it
    let (_size, ts) = page_bleed(page);

    let mut collecter = Collecter::new(py);
    collecter.collect_page(ts, page)?;
    collecter.finish()
}

fn page_bleed(page: &Page) -> (Size, Transform) {
    let bleed = page.bleed;
    let size = page.frame.size() + bleed.sum_by_axis();
    let ts = Transform::translate(bleed.left, bleed.top);
    (size, ts)
}

/// Helps collecting elements in a Typst page
///
/// The (single) page's frame tree is walked through [Collecter::collect_page],
/// and every supported item is converted into an [Element]
///
/// Finally, everything collected is returned to Python inside a [Collected] object
struct Collecter<'py> {
    /// Python instance, for binding Python objects
    py: Python<'py>,

    /// Meaning which groups the current element is inside,
    /// used for generating [`Collecter::groups`]
    active_labels: Vec<String>,

    // Final results, see `Collected` for docstrings
    elements: Vec<Element>,
    shared: IndexMap<u128, Bound<'py, PyAny>, FxBuildHasher>,
    groups: IndexMap<String, Vec<usize>>,
    warnings: Vec<ExportWarning>,
}

/// Representing one element in Typst page
#[pyclass(module = "typst4janim", frozen, skip_from_py_object)]
pub struct Element {
    /// Element type
    #[pyo3(get)]
    pub elemtype: String,
    /// Element transform as a 2x3 matrix
    /// - Left 2x2: 2D linear transformation
    /// - Right 2x1: 2D translation
    #[pyo3(get)]
    pub transform: Py<PyArray2<f32>>,
    /// Additional element infomation, differs accroding to the element type
    #[pyo3(get)]
    pub info: Py<PyAny>,
}

#[pyclass(module = "typst4janim", frozen, skip_from_py_object)]
pub struct Collected {
    /// Collected elements in a Typst page
    #[pyo3(get)]
    elements: Vec<Py<Element>>,
    /// Shared datas of elements, e.g. same text with different position
    /// - Key (`int`): id
    /// - Value (`Any`): The specific data
    #[pyo3(get)]
    shared: Py<PyDict>,
    /// Element groups infomation
    /// - Key (`str`): Label name
    /// - Value (`list[int]`): Group members, every number representing an index in `elements`
    #[pyo3(get)]
    groups: Py<PyDict>,
    /// Warnings emitted during the collecting process, store as a `list[str]`
    #[pyo3(get)]
    warnings: Py<PyList>,
}

impl<'py> Collecter<'py> {
    fn new(py: Python<'py>) -> Self {
        Self {
            py,
            active_labels: Vec::new(),
            elements: Vec::new(),
            shared: IndexMap::default(),
            groups: IndexMap::default(),
            warnings: Vec::new(),
        }
    }

    /// Automatically pushes `label` before calling `f`, and pops it afterwards
    fn with_label<F, R>(&mut self, label: Option<String>, f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        match label {
            Some(label) => {
                self.active_labels.push(label);
                let ret = f(self);
                self.active_labels.pop();
                ret
            }
            None => f(self),
        }
    }

    /// Initialize shared data if `key` is not present
    fn insert_shared_with<K, F>(
        &mut self,
        key: K, // -> id
        f: F,
    ) -> PyResult<(u128, Bound<'py, PyAny>)>
    where
        K: Hash,
        F: FnOnce(Python<'py>) -> PyResult<Bound<'py, PyAny>>,
    {
        let id = typst_utils::hash128(&key);
        let data = match self.shared.entry(id) {
            indexmap::map::Entry::Occupied(entry) => entry.get().clone(),
            indexmap::map::Entry::Vacant(entry) => {
                let data = f(self.py)?;
                entry.insert(data.clone());
                data
            }
        };
        Ok((id, data))
    }

    /// Push an element
    fn insert_element(
        &mut self,
        elemtype: String,
        transform: Transform,
        info: Bound<'py, PyAny>,
    ) -> PyResult<()> {
        let nth = self.elements.len();

        // Make element
        let elem = Element {
            elemtype,
            transform: ts_to_pyarray(self.py, transform)?,
            info: info.unbind(),
        };
        self.elements.push(elem);

        // Mark current element to each label in active-labels
        for label in self.active_labels.iter() {
            if let Some(vec) = self.groups.get_mut(label) {
                vec.push(nth);
            }
        }

        Ok(())
    }

    /// Push an warning
    fn add_warning(&mut self, warning: ExportWarning) {
        self.warnings.push(warning);
    }

    /// Finish collecting the Typst page, returns [Collected]
    fn finish(self) -> PyResult<Bound<'py, Collected>> {
        let py = self.py;

        let elements = {
            let mut result = Vec::new();
            for elem in self.elements {
                result.push(elem.into_pyobject(py)?.unbind())
            }
            result
        };
        let shared = to_pydict(py, self.shared)?.unbind();
        let groups = to_pydict(py, self.groups)?.unbind();
        let warnings = PyList::new(py, self.warnings.iter().map(|w| w.as_str()))?.unbind();

        Collected {
            elements,
            shared,
            groups,
            warnings,
        }
        .into_pyobject(py)
    }
}

impl Collecter<'_> {
    fn collect_page(&mut self, ts: Transform, page: &Page) -> PyResult<()> {
        if let Some(fill) = page.fill_or_white() {
            let shape = Geometry::Rect(page.frame.size() + page.bleed.sum_by_axis()).filled(fill);
            let ts = ts.pre_concat(Transform::translate(-page.bleed.left, -page.bleed.top));
            self.collect_shape(ts, &shape)?;
        }
        self.collect_frame(ts, &page.frame)
    }

    fn collect_frame(&mut self, ts: Transform, frame: &Frame) -> PyResult<()> {
        for (pos, item) in frame.items() {
            let ts = ts.pre_concat(Transform::translate(pos.x, pos.y));
            match item {
                FrameItem::Group(group) => self.collect_group(ts, group)?,
                FrameItem::Text(text) => self.collect_text(ts, text)?,
                FrameItem::Shape(shape, _) => self.collect_shape(ts, shape)?,
                FrameItem::Image(image, size, _) => self.collect_image(ts, image, size)?,
                FrameItem::Link(_, _) => {} // Link is just a transparent rectangle overlay, so we can ignore it
                FrameItem::Tag(_) => {}     // Maybe something related to HTML output
            };
        }
        Ok(())
    }

    fn collect_group(&mut self, ts: Transform, group: &GroupItem) -> PyResult<()> {
        let ts = ts.pre_concat(group.transform);

        if let Some(_clip_curve) = &group.clip {
            self.add_warning(ExportWarning::ClipPathNotSupported);
        }

        let label = group.label.map(|label| label.resolve().to_string());
        self.with_label(label, |s| s.collect_frame(ts, &group.frame))
    }
}

/// Converts a [Transform] object into a numpy array [PyArray2]
fn ts_to_pyarray<'py>(py: Python<'py>, ts: Transform) -> PyResult<Py<PyArray2<f32>>> {
    Array2::from_shape_vec(
        (2, 3),
        vec![
            ts.sx.get() as f32,
            ts.kx.get() as f32,
            ts.tx.to_pt() as f32,
            //
            ts.ky.get() as f32,
            ts.sy.get() as f32,
            ts.ty.to_pt() as f32,
        ],
    )
    .map(|matrix| matrix.into_pyarray(py).unbind())
    .map_err(|err| ConvertError::new_err(err.to_string()))
}

/// Converts a map (iterator) into a [PyDict] object
fn to_pydict<'py, K, V, I>(py: Python<'py>, items: I) -> PyResult<Bound<'py, PyDict>>
where
    K: IntoPyObject<'py>,
    V: IntoPyObject<'py>,
    I: IntoIterator<Item = (K, V)>,
{
    let dict = PyDict::new(py);
    for (k, v) in items {
        dict.set_item(k, v)?;
    }
    Ok(dict)
}
