use std::path::PathBuf;

/// Typed engine errors. Nothing here panics; the batch loop turns these into per-file
/// `Failed`/`Unreachable` outcomes so one bad file never aborts a job.
#[derive(thiserror::Error, Debug)]
pub enum EngineError {
    #[error("I/O error for {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("could not decode image: {0}")]
    Decode(String),

    #[error("could not encode image: {0}")]
    Encode(String),

    #[error("resize failed: {0}")]
    Resize(String),
}
