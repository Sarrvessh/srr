pub mod walker;

use crate::types::FileEntry;
use crate::error::SrrResult;

pub trait FileScanner: Send + Sync {
    fn scan(&self, path: &std::path::Path, excludes: &[String], verbose: bool, respect_gitignore: bool) -> SrrResult<Vec<FileEntry>>;
}
