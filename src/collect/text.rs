use numpy::IntoPyArray;
use pyo3::IntoPyObjectExt;
use pyo3::prelude::*;
use ttf_parser::GlyphId;
use typst::{
    layout::{Abs, Ratio, Transform},
    text::{TextItem, color::should_outline},
};

use crate::collect::utils::Rgba;
use crate::{
    ConvertError,
    collect::{Collecter, path::PathBuilder},
};

impl Collecter<'_> {
    pub fn collect_text(&mut self, ts: Transform, text: &TextItem) -> PyResult<()> {
        let ts = ts.pre_concat(Transform::scale(Ratio::one(), -Ratio::one()));

        let mut x = Abs::pt(0.0);
        let mut y = Abs::pt(0.0);
        for glyph in &text.glyphs {
            let id = GlyphId(glyph.id);
            let x_offset = x + glyph.x_offset.at(text.size);
            let y_offset = y + glyph.y_offset.at(text.size);

            let ts = ts.pre_concat(Transform::translate(x_offset, y_offset));
            self.collect_glyph(ts, text, id)?;

            x += glyph.x_advance.at(text.size);
            y += glyph.y_advance.at(text.size);
        }

        Ok(())
    }

    fn collect_glyph(&mut self, ts: Transform, text: &TextItem, glyph_id: GlyphId) -> PyResult<()> {
        if should_outline(&text.font, glyph_id) {
            // Pre-scale outlined glyphs, so strokes and fill patterns don't
            // need to consider text size glyph scaling.
            let scale = text.size.to_pt() / text.font.units_per_em();
            let key = (&text.font, glyph_id, Ratio::new(scale));

            let extract_points = |py| {
                let mut builder = PathBuilder::new(scale as f32);
                let points = match text.font.ttf().outline_glyph(glyph_id, &mut builder) {
                    Some(_) => builder
                        .build_array2()
                        .map_err(|err| ConvertError::new_err(err.to_string()))?
                        .into_pyarray(py)
                        .into_bound_py_any(py)?,
                    None => ().into_bound_py_any(py)?,
                };
                Ok(points)
            };

            let points_id = self.insert_shared_with(key, extract_points)?;

            let stroke = text
                .stroke
                .as_ref()
                .map(|stroke| (self.extract_rgb(&stroke.paint), stroke.thickness.to_pt()));

            let info = TextGlyphInfo {
                points_id,
                fill_rgba: self.extract_rgb(&text.fill),
                stroke_rgba: stroke.map(|s| s.0),
                stroke_thickness: stroke.map(|s| s.1),
            }
            .into_bound_py_any(self.py)?;

            self.insert_element("TextGlyph".into(), ts, info)?;
        } else {
            self.add_warning(super::ExportWarning::ImageGlyphNotSupported);
        }
        Ok(())
    }
}

#[pyclass(module = "typst4janim", frozen, skip_from_py_object)]
pub struct TextGlyphInfo {
    /// id, reference to the Bézier points in `shared`
    #[pyo3(get)]
    points_id: u128,
    /// Fill RGBA, each component value is in the range `0.0~1.0`
    #[pyo3(get)]
    fill_rgba: Rgba,
    /// Stroke RGBA, each component value is in the range `0.0~1.0`
    #[pyo3(get)]
    stroke_rgba: Option<Rgba>,
    /// Stroke thickness, divide by 2 to get JAnim's `stroke_radius`
    #[pyo3(get)]
    stroke_thickness: Option<f64>,
}
