use std::collections::HashMap;
use rayon::prelude::*;

use crate::types::{FileEntry, DuplicateGroup};
use super::NearDuplicateDetector;

const NUM_HASHES: usize = 48;
const BAND_SIZE: usize = 6;
const NUM_BANDS: usize = NUM_HASHES / BAND_SIZE;

const HASH_SEEDS: [u64; NUM_HASHES] = [
    0x9E3779B97F4A7C15, 0xBF58476D1CE4E5B9, 0x3C6EF372FE94F82B, 0x5A6D7B8C9F0E1D2C,
    0xDEADBEEFCAFEBABE, 0x0123456789ABCDEF, 0xFEDCBA9876543210, 0xABABABABABABABAB,
    0x1234567890ABCDEF, 0x0F1E2D3C4B5A6978, 0xAABBCCDDEEFF0011, 0x9988776655443322,
    0x1122334455667788, 0x99AABBCCDDEEFF00, 0x00FFEEDDCCBBAA99, 0x8877665544332211,
    0xDEAD10CCBEEF00DA, 0xBA5EBA11C0FFEE42, 0xFEEDFACEABACABBA, 0xCAFEB0BAFACADE01,
    0x1C6F3A8B7D9E5F2C, 0xA4F3C8D1B6E7A0F9, 0x238B4D6C9F1A2E5F, 0x7A9B8C7D6E5F4A3B,
    0x3A2B1C4D5E6F7A8B, 0xC9D8E7F6A5B4C3D2, 0x1F2E3D4C5B6A7F8E, 0x9A8B7C6D5E4F3A2B,
    0x0A1B2C3D4E5F6A7B, 0x8C9DAEBFC0D1E2F3, 0x4B5A6F7E8D9C0A1B, 0x2C3D4E5F6A7B8C9D,
    0xE1F2A3B4C5D6E7F8, 0xA0B1C2D3E4F5A6B7, 0x8F7E6D5C4B3A2F1, 0x0F1E2D3C4B5A6F7E,
    0x6D5C4B3A2F1E0D9C, 0x8B7A6F5E4D3C2B1A, 0x9F0E1D2C3B4A5F6E, 0x7D6C5B4A3F2E1D0C,
    0xB9A8F7E6D5C4B3A2, 0x1D2C3B4A5F6E7D8C, 0x4F5E6D7C8B9A0F1E, 0x2D3C4B5A6F7E8D9C,
    0xA0F1E2D3C4B5A6F7, 0x8E9D0C1B2A3F4E5D, 0x6C7B8A9F0E1D2C3B, 0x4A5F6E7D8C9B0A1F,
];

pub struct NearDuplicateDetectorImpl;

impl NearDuplicateDetector for NearDuplicateDetectorImpl {
    fn find_near_duplicates(&self, files: &[FileEntry]) -> Vec<DuplicateGroup> {
        if files.len() < 2 {
            return Vec::new();
        }

        let printable: Vec<&FileEntry> = files.iter()
            .filter(|f| !f.is_binary && f.content.is_some())
            .filter(|f| f.content.as_ref().is_some_and(|c| c.lines().count() >= 2))
            .collect();

        if printable.len() < 2 {
            return Vec::new();
        }

        // Compute MinHash signatures in parallel
        let sigs: Vec<(usize, Vec<u64>)> = printable.par_iter()
            .enumerate()
            .map(|(i, file)| {
                let content = file.content.as_ref().unwrap();
                let sig = minhash_signature(content);
                (i, sig)
            })
            .collect();

        // LSH: bucket by bands — files in same bucket are candidates
        let mut buckets: HashMap<u64, Vec<usize>> = HashMap::new();
        for (idx, (_, sig)) in sigs.iter().enumerate() {
            for band in 0..NUM_BANDS {
                let start = band * BAND_SIZE;
                let key = hash_band(&sig[start..start + BAND_SIZE]);
                buckets.entry(key).or_default().push(idx);
            }
        }

        // Only compare candidates within buckets (avoids O(n²))
        let mut visited = vec![false; sigs.len()];
        let mut groups = Vec::new();

        for candidates in buckets.values() {
            if candidates.len() < 2 { continue; }
            for i in 0..candidates.len() {
                let ci = candidates[i];
                if visited[ci] { continue; }

                let mut group = vec![ci];
                for &cj in candidates.iter().skip(i + 1) {
                    if visited[cj] { continue; }

                    let size_i = printable[sigs[ci].0].size_bytes;
                    let size_j = printable[sigs[cj].0].size_bytes;
                    let max_size = size_i.max(size_j) as f64;
                    let min_size = size_i.min(size_j) as f64;
                    if min_size == 0.0 || max_size / min_size > 4.0 {
                        continue;
                    }

                    if estimate_jaccard(&sigs[ci].1, &sigs[cj].1) > 0.7 {
                        group.push(cj);
                        visited[cj] = true;
                    }
                }

                if group.len() > 1 {
                    visited[ci] = true;
                    groups.push(DuplicateGroup {
                        reason: "Near-duplicate (MinHash Jaccard > 0.7)".to_string(),
                        files: group.iter().map(|&idx| printable[sigs[idx].0].path.clone()).collect(),
                    });
                }
            }
        }

        groups
    }
}

fn minhash_signature(content: &str) -> Vec<u64> {
    let lines: Vec<&str> = content.lines().collect();
    let mut shingles = Vec::new();

    let mut add_shingles = |range: &[&str]| {
        for window in range.windows(2) {
            let shingle = format!("{} {}", window[0].trim(), window[1].trim());
            shingles.push(fxhash(&shingle));
        }
    };

    let n = lines.len();
    if n <= 10 {
        add_shingles(&lines);
    } else {
        add_shingles(&lines[..5]);
        add_shingles(&lines[n - 5..]);
        let mid = n / 2;
        add_shingles(&lines[mid.saturating_sub(2)..(mid + 3).min(n)]);
    }

    if shingles.is_empty() {
        return Vec::new();
    }

    let mut sig = vec![u64::MAX; NUM_HASHES];
    for &sh in &shingles {
        for (i, s) in sig.iter_mut().enumerate() {
            let h = sh.wrapping_mul(HASH_SEEDS[i]).rotate_left(7);
            if h < *s {
                *s = h;
            }
        }
    }

    sig
}

fn hash_band(values: &[u64]) -> u64 {
    let mut h: u64 = 0;
    for &v in values {
        h = h.wrapping_mul(0x9E3779B97F4A7C15);
        h ^= v;
        h = h.rotate_left(5);
    }
    h
}

fn estimate_jaccard(a: &[u64], b: &[u64]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let equal = a.iter().zip(b.iter()).filter(|(x, y)| x == y).count();
    equal as f64 / a.len().max(b.len()) as f64
}

fn fxhash(s: &str) -> u64 {
    let bytes = s.as_bytes();
    let mut hash: u64 = 0;
    for chunk in bytes.chunks(8) {
        let mut val: u64 = 0;
        for (j, &b) in chunk.iter().enumerate() {
            val |= (b as u64) << (j * 8);
        }
        hash = hash.wrapping_mul(0x9E3779B97F4A7C15);
        hash ^= val;
        hash = hash.rotate_left(5);
    }
    hash
}

impl NearDuplicateDetectorImpl {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NearDuplicateDetectorImpl {
    fn default() -> Self {
        Self::new()
    }
}
