pub mod crud;

use crate::types::{FileEntry, Pattern};

pub trait PatternDetector: Send + Sync {
    fn detect_patterns(&self, files: &[FileEntry]) -> Vec<Pattern>;
}
