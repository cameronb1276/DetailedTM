use std::path::Path;
use std::process::Command;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ShellActionError {
    #[error("The executable path is no longer available: {path}")]
    MissingPath { path: String },
    #[error("Windows could not open File Explorer for {path}: {source}")]
    ExplorerLaunch {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

pub fn open_file_location(path: &Path) -> Result<(), ShellActionError> {
    if !path.exists() {
        return Err(ShellActionError::MissingPath {
            path: path.display().to_string(),
        });
    }

    Command::new("explorer.exe")
        .arg(format!("/select,{}", path.display()))
        .spawn()
        .map(|_| ())
        .map_err(|source| ShellActionError::ExplorerLaunch {
            path: path.display().to_string(),
            source,
        })
}
