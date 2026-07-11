use pyo3::PyResult;
use typst::{
    layout::{Abs, Axes, Transform},
    visualize::Image,
};

use crate::collect::Collecter;

impl Collecter<'_> {
    #[allow(unused_variables)]
    pub fn collect_image(
        &mut self,
        ts: Transform,
        image: &Image,
        size: &Axes<Abs>,
    ) -> PyResult<()> {
        self.add_warning(super::warnings::ExportWarning::ImageNotSupported);
        Ok(())
    }
}
