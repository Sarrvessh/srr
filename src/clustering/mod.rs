pub mod domain;

use crate::types::{FileEntry, Cluster};

pub trait FileClusterer: Send + Sync {
    fn cluster(&self, files: &[FileEntry]) -> Vec<Cluster>;
}
