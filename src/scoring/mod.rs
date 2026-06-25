pub mod ranker;

use crate::types::{FileEntry, ScoredFile, Architecture};

pub trait FileScorer: Send + Sync {
    fn score(&self, files: &[FileEntry], architecture: &Architecture) -> Vec<ScoredFile>;
}
