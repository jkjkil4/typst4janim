use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use typst::diag::FileResult;
use typst::foundations::{Bytes, Datetime, Dict, Duration, Value};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

use typst_kit::datetime::Time;
use typst_kit::diagnostics::DiagnosticWorld;
use typst_kit::files::{FileLoader, FileStore, FsRoot};
use typst_kit::fonts::FontStore;
use typst_kit::packages::SystemPackages;

use crate::typst::packages::system_packages;

pub struct PathArgs {
    pub root: Option<PathBuf>,
    pub package_path: Option<PathBuf>,
    pub package_cache_path: Option<PathBuf>,
}

pub struct SystemWorld<'a> {
    workdir: Option<PathBuf>,
    library: LazyHash<Library>,
    fonts: &'a FontStore,
    files: FileStore<SystemFiles>,
    now: Time,
}

impl<'a> SystemWorld<'a> {
    pub fn new(
        // User provided parameters
        input: Vec<u8>,
        sys_inputs: HashMap<String, String>,
        path_args: PathArgs,
        // Internal parameters
        fonts: &'a FontStore,
    ) -> Result<Self, String> {
        // TODO: rayon initialization?

        let library = {
            let inputs = Dict::from_iter(
                sys_inputs
                    .into_iter()
                    .map(|(k, v)| (k.into(), Value::Str(v.into()))),
            );

            Library::builder().with_inputs(inputs).build()
        };

        let files = SystemFiles::new(input, path_args)?;

        Ok(Self {
            workdir: std::env::current_dir().ok(),
            library: LazyHash::new(library),
            fonts,
            files: FileStore::new(files),
            now: Time::system(),
        })
    }

    /// The project root relative to which absolute paths are resolved.
    pub fn root(&self) -> &Path {
        self.files.loader().project.path()
    }

    /// The current working directory.
    pub fn workdir(&self) -> &Path {
        self.workdir.as_deref().unwrap_or(Path::new("."))
    }
}

impl World for SystemWorld<'_> {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.book()
    }

    fn main(&self) -> FileId {
        self.files.loader().main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.files.source(id)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files.file(id)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.font(index)
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        self.now.today(offset)
    }
}

impl DiagnosticWorld for SystemWorld<'_> {
    fn name(&self, id: FileId) -> String {
        let vpath = id.vpath();
        match id.root() {
            VirtualRoot::Project => {
                // Try to express the path relative to the working directory.
                vpath
                    .realize(self.root())
                    .ok()
                    .and_then(|rooted| pathdiff::diff_paths(rooted, self.workdir()))
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or_else(|| vpath.get_without_slash().into())
            }
            VirtualRoot::Package(package) => {
                format!("{package}{}", vpath.get_with_slash())
            }
        }
    }
}

// Same as Typst's STDIN_ID
static JANIM_SOURCE_ID: LazyLock<FileId> = LazyLock::new(|| {
    FileId::unique(RootedPath::new(
        VirtualRoot::Project,
        VirtualPath::new("<janim>").unwrap(),
    ))
});

struct SystemFiles {
    // janim source
    main: FileId,
    main_bytes: Bytes,
    // others
    project: FsRoot,
    packages: SystemPackages,
}

impl SystemFiles {
    pub fn new(input: Vec<u8>, path_args: PathArgs) -> Result<Self, String> {
        let root = {
            let path = path_args.root.as_deref().unwrap_or(Path::new("."));
            path.canonicalize().map_err(|err| err.to_string())?
        };

        Ok(Self {
            main: *JANIM_SOURCE_ID,
            main_bytes: Bytes::new(input),
            project: FsRoot::new(root),
            packages: system_packages(path_args.package_path, path_args.package_cache_path),
        })
    }

    fn root(&self, id: FileId) -> FileResult<FsRoot> {
        Ok(match id.root() {
            VirtualRoot::Project => self.project.clone(),
            VirtualRoot::Package(spec) => self.packages.obtain(spec)?,
        })
    }
}

impl FileLoader for SystemFiles {
    fn load(&self, id: FileId) -> FileResult<Bytes> {
        if id == *JANIM_SOURCE_ID {
            Ok(self.main_bytes.clone())
        } else {
            self.root(id)?.load(id.vpath())
        }
    }
}
