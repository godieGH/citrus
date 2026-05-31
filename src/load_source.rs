// src/load_source.rs
use crate::error::CitrusError;
use std::fs;
use std::path::Path;

/// The result of loading a source file —
/// canonical path and raw source text together
pub struct SourceFile {
    pub path: String,    // absolute, canonical, symlinks resolved
    pub content: String, // raw source text
}

pub fn load(raw_path: &str) -> Result<SourceFile, CitrusError> {
    // 1. check extension before touching the filesystem
    if !raw_path.ends_with(".citrus") {
        return Err(CitrusError::InvalidExtension(raw_path.to_string()));
    }

    let path = Path::new(raw_path);

    // 2. check it exists
    if !path.exists() {
        return Err(CitrusError::FileNotFound(raw_path.to_string()));
    }

    // 3. check it is actually a file, not a directory
    if !path.is_file() {
        return Err(CitrusError::NotAFile(raw_path.to_string()));
    }

    // 4. canonicalize — resolves relative paths, ../, ./, symlinks
    //    this gives us the true absolute path on disk
    let canonical = fs::canonicalize(path).map_err(|e| CitrusError::IoError(e.to_string()))?;

    // 5. read the content
    let content =
        fs::read_to_string(&canonical).map_err(|e| CitrusError::IoError(e.to_string()))?;

    Ok(SourceFile {
        path: canonical.to_string_lossy().to_string(),
        content,
    })
}
