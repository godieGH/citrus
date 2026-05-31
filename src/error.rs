// src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CitrusError {
    #[error("expected a .citrus file — '{0}'")]
    InvalidExtension(String),

    #[error("file not found — '{0}'")]
    FileNotFound(String),

    #[error("not a file — '{0}'")]
    NotAFile(String),

    #[error("could not read file — {0}")]
    IoError(String),

    #[error("unknown token '{src}' at {file}:{line}:{col}")]
    LexError {
        file: String,
        line: usize,
        col: usize,
        src: String,
    },

    // added now — parser errors
    // expected says what the parser wanted
    // found says what was actually there
    #[error("parse error at {file}:{line}:{col} — expected {expected}, found {found}")]
    ParseError {
        expected: String,
        found: String,
        file: String,
        line: usize,
        col: usize,
        hint: Option<String>,
    },

    // when we reach end of file unexpectedly
    #[error("unexpected end of file in {file} — {message}")]
    UnexpectedEof { file: String, message: String, hint: Option<String>,},
}
