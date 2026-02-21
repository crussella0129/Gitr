use std::path::PathBuf;

/// Central error type for the Gitr system.
#[derive(Debug, thiserror::Error)]
pub enum GitrError {
    #[error("host not found: {label}")]
    HostNotFound { label: String },

    #[error("host already exists: {label}")]
    HostAlreadyExists { label: String },

    #[error("repo not found: {name}")]
    RepoNotFound { name: String },

    #[error("authentication failed for host {host}: {message}")]
    AuthFailed { host: String, message: String },

    #[error("API error ({status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("rate limited by {host} — retry after {retry_after_secs}s")]
    RateLimited { host: String, retry_after_secs: u64 },

    #[error("git error: {message}")]
    GitError { message: String },

    #[error("merge conflict on branch {branch}: {message}")]
    MergeConflict { branch: String, message: String },

    #[error("fast-forward failed on branch {branch}: {message}")]
    FastForwardFailed { branch: String, message: String },

    #[error("sync link not found: {id}")]
    SyncLinkNotFound { id: String },

    #[error("collection not found: {name}")]
    CollectionNotFound { name: String },

    #[error("provider not implemented: {kind}")]
    ProviderNotImplemented { kind: String },

    #[error("path not found: {path}")]
    PathNotFound { path: PathBuf },

    #[error("config error: {message}")]
    Config { message: String },

    #[error("database error: {0}")]
    Database(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("credential error: {message}")]
    CredentialError { message: String },

    #[error("{0}")]
    Other(String),
}
