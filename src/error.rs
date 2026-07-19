use crate::content::ContentError;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("content was not found at {path}")]
    NotFound { path: String },
    #[error("content repository failed")]
    Content(#[source] ContentError),
}

impl AppError {
    #[must_use]
    pub fn not_found(path: impl Into<String>) -> Self {
        Self::NotFound { path: path.into() }
    }
}

impl From<ContentError> for AppError {
    fn from(error: ContentError) -> Self {
        Self::Content(error)
    }
}

pub type AppResult<T> = Result<T, AppError>;
