mod embedded;
mod markdown;
mod model;
mod repository;

use std::path::PathBuf;

pub use embedded::EmbeddedContentRepository;
pub(crate) use embedded::valid_module_name;
pub use markdown::{parse_page, parse_post};
pub use model::{Branding, GoModule, Page, Post, TagSummary};
pub use repository::ContentRepository;

#[derive(Debug, thiserror::Error)]
pub enum ContentError {
    #[error("embedded content asset disappeared while loading: {path}")]
    MissingEmbeddedAsset { path: PathBuf },
    #[error("embedded content is not valid UTF-8: {path}")]
    Utf8 {
        path: PathBuf,
        #[source]
        source: std::str::Utf8Error,
    },
    #[error("unexpected file in embedded content: {path}")]
    UnexpectedFile { path: PathBuf },
    #[error("embedded content is missing data.json")]
    MissingData,
    #[error("embedded content contains more than one data.json")]
    DuplicateData,
    #[error("could not deserialize JSON data in {path}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("invalid content filename: {path}")]
    InvalidFilename { path: PathBuf },
    #[error("content is missing YAML frontmatter: {path}")]
    MissingFrontmatter { path: PathBuf },
    #[error("content has malformed YAML frontmatter delimiters: {path}")]
    MalformedFrontmatter { path: PathBuf },
    #[error("could not deserialize YAML frontmatter in {path}")]
    Yaml {
        path: PathBuf,
        #[source]
        source: serde_yaml_ng::Error,
    },
    #[error("content is missing required field {field}: {path}")]
    MissingField { path: PathBuf, field: &'static str },
    #[error("invalid {field} date {value:?} in {path}")]
    InvalidDate {
        path: PathBuf,
        field: &'static str,
        value: String,
        #[source]
        source: chrono::ParseError,
    },
    #[error("could not render Markdown in {path}")]
    Render {
        path: PathBuf,
        #[source]
        source: std::fmt::Error,
    },
    #[error(
        "duplicate {kind} slug {slug:?} in {first_path} and {second_path}",
        first_path = first_path.display(),
        second_path = second_path.display()
    )]
    DuplicateSlug {
        kind: &'static str,
        slug: String,
        first_path: PathBuf,
        second_path: PathBuf,
    },
    #[error("missing required page: {slug}")]
    MissingRequiredPage { slug: String },
    #[error("branding collection {collection} must not be empty")]
    EmptyBranding { collection: &'static str },
    #[error("module {index} has invalid {field}")]
    InvalidModule { index: usize, field: &'static str },
    #[error("duplicate module name: {name}")]
    DuplicateModule { name: String },
    #[error("duplicate module path: {module_path}")]
    DuplicateModulePath { module_path: String },
}
