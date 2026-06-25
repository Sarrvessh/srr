use std::collections::HashMap;
use sha2::{Sha256, Digest};
use rayon::prelude::*;

use crate::types::{FileEntry, DuplicateGroup};
use super::ExactDuplicateDetector;

pub struct ExactDuplicateDetectorImpl;

impl ExactDuplicateDetector for ExactDuplicateDetectorImpl {
    fn find_exact_duplicates(&self, files: &[FileEntry]) -> Vec<DuplicateGroup> {
        let hashes: Vec<(usize, [u8; 32])> = files
            .par_iter()
            .enumerate()
            .map(|(i, file)| {
                let hash = match &file.content {
                    Some(content) => {
                        let mut hasher = Sha256::new();
                        hasher.update(content.as_bytes());
                        let result = hasher.finalize();
                        let mut arr = [0u8; 32];
                        arr.copy_from_slice(&result);
                        arr
                    }
                    None => [0u8; 32],
                };
                (i, hash)
            })
            .collect();

        let mut groups: HashMap<[u8; 32], Vec<&FileEntry>> = HashMap::new();
        for (i, hash) in &hashes {
            if *hash == [0u8; 32] {
                continue;
            }
            groups.entry(*hash).or_default().push(&files[*i]);
        }

        groups
            .into_values()
            .filter(|group| group.len() > 1)
            .map(|group| DuplicateGroup {
                reason: "Exact duplicate (identical content)".to_string(),
                files: group.iter().map(|f| f.path.clone()).collect(),
            })
            .collect()
    }

}
