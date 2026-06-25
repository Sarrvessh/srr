use std::path::Path;
use walkdir::WalkDir;
use rayon::prelude::*;
use globset::GlobSet;


use crate::error::{SrrResult, SrrError};
use crate::types::FileEntry;
use super::FileScanner;

pub struct DefaultScanner;

impl FileScanner for DefaultScanner {
    fn scan(&self, path: &Path, excludes: &[String], verbose: bool, respect_gitignore: bool) -> SrrResult<Vec<FileEntry>> {
        if !path.exists() {
            return Err(SrrError::DirectoryNotFound(path.to_path_buf()));
        }
        let scan_root = path.to_path_buf();

        let entries: Vec<_> = if respect_gitignore {
            use ignore::WalkBuilder;
            WalkBuilder::new(path)
                .follow_links(false)
                .git_ignore(true)
                .git_global(true)
                .git_exclude(true)
                .require_git(false)
                .build()
                .filter_map(|result| match result {
                    Ok(e) => Some(e),
                    Err(err) => {
                        if verbose { eprintln!("  ⚠ Walk error: {}", err); }
                        None
                    }
                })
                .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                .map(|e| e.path().to_path_buf())
                .filter(|p| !is_excluded_path(p, excludes))
                .collect()
        } else {
            WalkDir::new(path)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| !is_excluded(e, excludes))
                .filter_map(|result| match result {
                    Ok(e) => Some(e),
                    Err(err) => {
                        if verbose { eprintln!("  ⚠ Walk error: {}", err); }
                        None
                    }
                })
                .filter(|e| e.file_type().is_file())
                .map(|e| e.path().to_path_buf())
                .filter(|p| !is_excluded_path(p, excludes))
                .collect()
        };

        let files: Vec<SrrResult<FileEntry>> = entries
            .par_iter()
            .map(|path| {
                let relative_path = path.strip_prefix(&scan_root).unwrap_or(path).to_path_buf();

                let metadata = match std::fs::metadata(path) {
                    Ok(m) => m,
                    Err(e) => return Err(SrrError::Io { path: path.clone(), source: e }),
                };

                let size_bytes = metadata.len();
                let extension = path
                    .extension()
                    .map(|e| e.to_string_lossy().to_lowercase())
                    .unwrap_or_default();

                if size_bytes > 10_000_000 {
                    let content = read_truncated(path, 1_000_000)?;
                    let line_count = content.lines().count();
                    let is_binary = is_binary_content(&content);
                    let token_count = estimate_token_count(&content, size_bytes, 1_000_000);

                    return Ok(FileEntry {
                        path: path.clone(),
                        relative_path,
                        extension,
                        size_bytes,
                        line_count,
                        is_binary,
                        content: if is_binary { None } else { Some(content) },
                        token_count,
                    });
                }

                let content = read_file_content(path)?;
                let line_count = content.lines().count();
                let is_binary = is_binary_content(&content);
                let token_count = count_tokens_tiktoken(&content);

                Ok(FileEntry {
                    path: path.clone(),
                    relative_path,
                    extension,
                    size_bytes,
                    line_count,
                    is_binary,
                    content: if is_binary { None } else { Some(content) },
                    token_count,
                })
            })
            .collect();

        let mut result = Vec::new();
        for entry in files {
            match entry {
                Ok(f) => result.push(f),
                Err(SrrError::BinaryContent(_)) => {}
                Err(SrrError::InvalidUtf8(_)) => {}
                Err(e) => {
                    if verbose {
                        eprintln!("  ⚠ Skipping: {}", e);
                    }
                }
            }
        }

        Ok(result)
    }
}

pub fn filter_by_include(files: Vec<FileEntry>, glob_set: &GlobSet) -> Vec<FileEntry> {
    files.into_iter()
        .filter(|f| glob_set.is_match(&f.path))
        .collect()
}

fn is_excluded(entry: &walkdir::DirEntry, excludes: &[String]) -> bool {
    let name = entry.file_name().to_string_lossy();
    if excludes.iter().any(|e| name == *e) {
        return true;
    }
    has_matching_component(entry.path(), excludes)
}

fn is_excluded_path(path: &Path, excludes: &[String]) -> bool {
    let name = path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
    if excludes.iter().any(|e| name == *e) {
        return true;
    }
    has_matching_component(path, excludes)
}

fn has_matching_component(path: &Path, excludes: &[String]) -> bool {
    let path_str = path.to_string_lossy();
    for excl in excludes {
        for component in path_str.split(&['/', '\\'][..]) {
            if component == excl.as_str() {
                return true;
            }
        }
    }
    false
}

fn read_file_content(path: &Path) -> SrrResult<String> {
    let bytes = std::fs::read(path).map_err(|e| SrrError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    if bytes.iter().take(8192).any(|&b| b == 0) {
        return Err(SrrError::BinaryContent(path.to_path_buf()));
    }

    let content = String::from_utf8_lossy(&bytes).to_string();
    Ok(content)
}

fn read_truncated(path: &Path, max_bytes: usize) -> SrrResult<String> {
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(path).map_err(|e| SrrError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    let mut buf = vec![0u8; max_bytes];
    let n = file.read(&mut buf).map_err(|e| SrrError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    buf.truncate(n);

    let content = String::from_utf8_lossy(&buf).to_string();
    Ok(content)
}

fn is_binary_content(content: &str) -> bool {
    let bytes = content.as_bytes();
    let check_len = bytes.len().min(8192);
    let null_count = bytes[..check_len].iter().filter(|&&b| b == 0).count();
    null_count > 0
}

fn count_tokens_tiktoken(content: &str) -> usize {
    match tiktoken_rs::cl100k_base() {
        Ok(bpe) => bpe.encode_ordinary(content).len(),
        Err(_) => content.len().div_ceil(4),
    }
}

fn estimate_token_count(content: &str, total_size: u64, sampled_size: usize) -> usize {
    let sampled_tokens = count_tokens_tiktoken(content);
    if total_size as usize <= sampled_size {
        return sampled_tokens;
    }
    let ratio = total_size as f64 / sampled_size as f64;
    (sampled_tokens as f64 * ratio) as usize
}

impl DefaultScanner {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultScanner {
    fn default() -> Self {
        Self::new()
    }
}
