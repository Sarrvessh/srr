pub mod log;
pub mod doc;
pub mod code;

use crate::types::{FileEntry, CompressedSection};
use crate::error::SrrResult;

pub trait FileCompressor: Send + Sync {
    fn can_handle(&self, file: &FileEntry) -> bool;
    fn compress(&self, files: &[FileEntry]) -> SrrResult<CompressedSection>;
}
