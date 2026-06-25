use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SrrError {
    #[error("Directory not found: {0}")]
    DirectoryNotFound(PathBuf),

    #[error("I/O error processing {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Cannot read path: {0}")]
    WalkDir(#[from] walkdir::Error),

    #[error("Binary file skipped: {0}")]
    BinaryContent(PathBuf),

    #[error("Invalid UTF-8 in file: {0}")]
    InvalidUtf8(PathBuf),

    #[error("Token estimation failed: {0}")]
    TokenizerError(String),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("I/O error: {0}")]
    IoSimple(#[from] std::io::Error),

    #[error("{0}")]
    Anyhow(#[from] anyhow::Error),

    #[error("File watch error: {0}")]
    Notify(#[from] notify::Error),
}

pub type SrrResult<T> = Result<T, SrrError>;
