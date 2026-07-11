use typst::visualize::Paint;

use crate::collect::Collecter;

pub type Rgba = (f32, f32, f32, f32);

impl Collecter<'_> {
    pub fn extract_rgb(&mut self, paint: &Paint) -> Rgba {
        match paint {
            Paint::Solid(color) => color.to_rgb().into_components(),
            Paint::Gradient(_) => {
                self.add_warning(super::ExportWarning::GradientNotSupprted);
                (1.0, 1.0, 1.0, 1.0)
            }
            Paint::Tiling(_) => {
                self.add_warning(super::ExportWarning::TilingNotSupported);
                (1.0, 1.0, 1.0, 1.0)
            }
        }
    }
}
