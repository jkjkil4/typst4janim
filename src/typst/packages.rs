use std::path::PathBuf;

use typst_kit::packages::{FsPackages, SystemPackages, UniversePackages};
use typst_kit::downloader::{Downloader, SystemDownloader};

/// Returns a new package storage for the given args.
pub fn system_packages(
    package_path: Option<PathBuf>,
    package_cache_path: Option<PathBuf>,
) -> SystemPackages {
    SystemPackages::from_parts(
        package_path
            .clone()
            .map(FsPackages::new)
            .or_else(FsPackages::system_data),
        package_cache_path
            .clone()
            .map(FsPackages::new)
            .or_else(FsPackages::system_cache),
        UniversePackages::new(downloader()),
    )
}


/// Returns a new downloader.
fn downloader() -> impl Downloader {
    let user_agent = concat!("typst4janim/", env!("CARGO_PKG_VERSION"));
    SystemDownloader::new(user_agent)
}
