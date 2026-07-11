use pyo3::create_exception;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

mod collect;
mod typst;

create_exception!(typst4janim, TypstError, PyRuntimeError);
create_exception!(typst4janim, ConvertError, PyRuntimeError);

#[pymodule]
pub mod typst4janim {
    use std::{collections::HashMap, path::PathBuf};

    use pyo3::prelude::*;

    use typst::comemo;
    use typst::diag::{SourceDiagnostic, Warned};
    use typst::ecow::EcoVec;
    use typst_kit::diagnostics::DiagnosticWorld;
    use typst_layout::PagedDocument;

    use crate::typst::world::{PathArgs, SystemWorld};
    use crate::typst::{fonts, terminal};
    use crate::{TypstError, collect};

    #[pymodule_export]
    use collect::{Collected, Element, ShapeInfo, TextGlyphInfo};

    #[pyfunction]
    #[pyo3(signature = (
        input: "bytes",
        sys_inputs = HashMap::new(),
        root = None,
        package_path = None,
    ))]
    fn compile<'py>(
        py: Python<'py>,
        input: Vec<u8>,
        sys_inputs: HashMap<String, String>,
        root: Option<PathBuf>,
        package_path: Option<PathBuf>,
    ) -> PyResult<Bound<'py, collect::Collected>> {
        fonts::with_fonts(|fonts| {
            let path_args = PathArgs {
                root,
                package_path,
                package_cache_path: None,
            };

            let world = SystemWorld::new(input, sys_inputs, path_args, fonts)
                .map_err(TypstError::new_err)?;

            let Warned { output, warnings } = typst::compile::<PagedDocument>(&world);
            print_diagnostics(&world, &output, &warnings);

            comemo::evict(10);

            let document = output.map_err(|errs| {
                TypstError::new_err(match errs.first() {
                    Some(err) => err.message.to_string(),
                    None => "Unknown error".into(),
                })
            })?;
            collect::export_to_python(py, document)
        })
    }

    fn print_diagnostics(
        world: &dyn DiagnosticWorld,
        output: &Result<PagedDocument, EcoVec<SourceDiagnostic>>,
        warnings: &[SourceDiagnostic],
    ) {
        let errors = match output {
            Ok(_) => &[].into(),
            Err(errors) => errors,
        };
        if let Err(err) = typst_kit::diagnostics::emit(
            &mut terminal::out(),
            world,
            errors.iter().chain(warnings),
            typst_kit::diagnostics::DiagnosticFormat::Human,
        ) {
            println!("{}", err)
        }
    }

    #[pyfunction]
    fn reset_fonts<'py>(_py: Python<'py>) {
        fonts::reset_fonts();
    }
}
