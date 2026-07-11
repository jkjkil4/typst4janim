use numpy::{IntoPyArray, PyArray2};
use pyo3::{IntoPyObjectExt, prelude::*};

use ttf_parser::OutlineBuilder;
use typst::{
    layout::Transform,
    visualize::{CurveItem, Geometry, Shape},
};

use crate::{
    ConvertError,
    collect::{Collecter, path::PathBuilder, utils::Rgba},
};

impl Collecter<'_> {
    pub fn collect_shape(&mut self, ts: Transform, shape: &Shape) -> PyResult<()> {
        let points = convert_geometry_to_points(self.py, &shape.geometry)?.unbind();

        let fill = shape.fill.as_ref().map(|fill| self.extract_rgb(fill));
        let stroke = shape
            .stroke
            .as_ref()
            .map(|stroke| (self.extract_rgb(&stroke.paint), stroke.thickness.to_pt()));

        let info = ShapeInfo {
            points,
            fill_rgba: fill,
            stroke_rgba: stroke.map(|s| s.0),
            stroke_thickness: stroke.map(|s| s.1),
        }
        .into_bound_py_any(self.py)?;

        self.insert_element("Shape".into(), ts, info)
    }
}

fn convert_geometry_to_points<'py>(
    py: Python<'py>,
    geometry: &Geometry,
) -> PyResult<Bound<'py, PyArray2<f32>>> {
    let mut builder = PathBuilder::new(1.0);
    match geometry {
        &Geometry::Line(t) => {
            builder.move_to(0.0, 0.0);
            builder.line_to_point(t);
        }
        &Geometry::Rect(size) => {
            let [w, h] = [size.x.to_pt() as f32, size.y.to_pt() as f32];
            builder.move_to(0.0, 0.0);
            builder.line_to(0.0, h);
            builder.line_to(w, h);
            builder.line_to(w, 0.0);
            builder.close();
        }
        Geometry::Curve(curve) => {
            for item in curve.0.iter() {
                match *item {
                    CurveItem::Move(pos) => builder.move_to_point(pos),
                    CurveItem::Line(pos) => builder.line_to_point(pos),
                    CurveItem::Cubic(p1, p2, p3) => builder.curve_to_point(p1, p2, p3),
                    CurveItem::Close => builder.close(),
                }
            }
        }
    }
    builder
        .build_array2()
        .map(|points| points.into_pyarray(py))
        .map_err(|err| ConvertError::new_err(err.to_string()))
}

#[pyclass(module = "typst4janim", frozen, skip_from_py_object)]
pub struct ShapeInfo {
    /// Bézier points
    #[pyo3(get)]
    points: Py<PyArray2<f32>>,
    /// Fill RGBA, each component value is in the range `0.0~1.0`
    #[pyo3(get)]
    fill_rgba: Option<Rgba>,
    /// Stroke RGBA, each component value is in the range `0.0~1.0`
    #[pyo3(get)]
    stroke_rgba: Option<Rgba>,
    /// Stroke thickness, divide by 2 to get JAnim's `stroke_radius`
    #[pyo3(get)]
    stroke_thickness: Option<f64>,
}
