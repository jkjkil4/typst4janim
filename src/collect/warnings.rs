pub enum ExportWarning {
    ClipPathNotSupported,
    ImageGlyphNotSupported,
    GradientNotSupprted,
    TilingNotSupported,
}

impl ExportWarning {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExportWarning::ClipPathNotSupported => "ClipPathNotSupported",
            ExportWarning::ImageGlyphNotSupported => "ImageGlyphNotSupported",
            ExportWarning::GradientNotSupprted => "GradientNotSupported",
            ExportWarning::TilingNotSupported => "TilingNotSupported",
        }
    }
}
