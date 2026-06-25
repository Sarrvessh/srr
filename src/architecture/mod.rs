pub mod detector;
pub mod graph;

use crate::types::{FileEntry, Architecture};
use crate::error::SrrResult;

pub trait ArchitectureAnalyzer: Send + Sync {
    fn analyze(&self, files: &[FileEntry]) -> SrrResult<Architecture>;
}
