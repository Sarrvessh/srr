pub mod exact;
pub mod near;

use crate::types::{FileEntry, DuplicateGroup};

pub trait ExactDuplicateDetector: Send + Sync {
    fn find_exact_duplicates(&self, files: &[FileEntry]) -> Vec<DuplicateGroup>;
}

pub trait NearDuplicateDetector: Send + Sync {
    fn find_near_duplicates(&self, files: &[FileEntry]) -> Vec<DuplicateGroup>;
}
