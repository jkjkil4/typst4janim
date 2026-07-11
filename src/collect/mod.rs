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

struct Collecter<'py> {
    py: Python<'py>,

    // Temporary states
    active_labels: Vec<String>,

    // Final results
    elements: Vec<Element>,
    shared: IndexMap<u128, Bound<'py, PyAny>, FxBuildHasher>,
    groups: IndexMap<String, Vec<usize>>,
    warnings: Vec<ExportWarning>,
}

#[pyclass(module = "typst4janim", frozen, skip_from_py_object)]
pub struct Element {
    #[pyo3(get)]
    pub elemtype: String,
    #[pyo3(get)]
    pub transform: Py<PyArray2<f32>>,
    #[pyo3(get)]
    pub info: Py<PyAny>,
}

#[pyclass(module = "typst4janim", frozen, skip_from_py_object)]
pub struct Collected {
    #[pyo3(get)]
    elements: Vec<Py<Element>>,
    #[pyo3(get)]
    shared: Py<PyDict>,
    #[pyo3(get)]
    groups: Py<PyDict>,
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

    fn insert_shared_with<K, F>(
        &mut self,
        key: K, // -> id
        f: F,
    ) -> PyResult<u128>
    where
        K: Hash,
        F: FnOnce(Python<'py>) -> PyResult<Bound<'py, PyAny>>,
    {
        let id = typst_utils::hash128(&key);

        // Ensure the shared data
        // If the `id`'s corresponding data does not exist, `f` will be called
        match self.shared.entry(id) {
            indexmap::map::Entry::Occupied(_) => {}
            indexmap::map::Entry::Vacant(entry) => {
                entry.insert(f(self.py)?);
            }
        }

        Ok(id)
    }

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
            transform: ts_pyarray(self.py, transform)?,
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

    fn add_warning(&mut self, warning: ExportWarning) {
        self.warnings.push(warning);
    }

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

fn ts_pyarray<'py>(py: Python<'py>, t: Transform) -> PyResult<Py<PyArray2<f32>>> {
    Array2::from_shape_vec(
        (3, 3),
        vec![
            t.sx.get() as f32,
            t.kx.get() as f32,
            t.tx.to_pt() as f32,
            //
            t.ky.get() as f32,
            t.sy.get() as f32,
            t.ty.to_pt() as f32,
            //
            0.0,
            0.0,
            1.0,
        ],
    )
    .map(|matrix| matrix.into_pyarray(py).unbind())
    .map_err(|err| ConvertError::new_err(err.to_string()))
}

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
