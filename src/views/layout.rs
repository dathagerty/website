use std::sync::RwLock;

use topcoat::{
    Result,
    context::{Cx, try_request_context},
    router::{NotFoundError, Slot, StatusCode, layout, uri},
    view::view,
};

use crate::{
    content::{Branding, GoModule},
    error::AppError,
};

use super::{internal_error, not_found, site_footer, site_header};

const DEFAULT_TITLE: &str = "dathagerty.com";
const DEFAULT_DESCRIPTION: &str = "David Hagerty's personal website and assorted ramblings.";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoMetadata {
    module_path: String,
    repository_url: String,
}

impl GoMetadata {
    #[must_use]
    pub fn import_content(&self) -> String {
        format!("{} git {}", self.module_path, self.repository_url)
    }

    #[must_use]
    pub fn source_content(&self) -> Option<String> {
        let host = url::Url::parse(&self.repository_url)
            .ok()?
            .host_str()?
            .to_owned();
        let repository = self.repository_url.trim_end_matches('/');
        let (directory, file) = match host.as_str() {
            "git.sr.ht" => (
                format!("{repository}/tree/main/item{{/dir}}"),
                format!("{repository}/tree/main/item{{/dir}}/{{file}}#L{{line}}"),
            ),
            "github.com" => (
                format!("{repository}/tree/main{{/dir}}"),
                format!("{repository}/blob/main{{/dir}}/{{file}}#L{{line}}"),
            ),
            _ => return None,
        };

        Some(format!(
            "{} {} {directory} {file}",
            self.module_path, self.repository_url
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageMetadata {
    pub title: String,
    pub description: String,
    pub go: Option<GoMetadata>,
}

impl PageMetadata {
    #[must_use]
    pub fn new(title: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            go: None,
        }
    }

    #[must_use]
    pub fn for_module(module: &GoModule) -> Self {
        Self {
            title: format!("{} | {DEFAULT_TITLE}", module.name),
            description: module.description.clone(),
            go: Some(GoMetadata {
                module_path: module.module_path.clone(),
                repository_url: module.repository_url.clone(),
            }),
        }
    }
}

impl Default for PageMetadata {
    fn default() -> Self {
        Self::new(DEFAULT_TITLE, DEFAULT_DESCRIPTION)
    }
}

#[derive(Debug)]
pub struct RequestView {
    inner: RwLock<RequestViewData>,
}

#[derive(Clone, Debug)]
struct RequestViewData {
    metadata: PageMetadata,
    branding: Branding,
}

impl RequestView {
    #[must_use]
    pub fn new(metadata: PageMetadata, branding: Branding) -> Self {
        Self {
            inner: RwLock::new(RequestViewData { metadata, branding }),
        }
    }

    pub fn set_metadata(&self, metadata: PageMetadata) {
        self.inner
            .write()
            .expect("request view lock was poisoned")
            .metadata = metadata;
    }

    pub fn set_branding(&self, branding: Branding) {
        self.inner
            .write()
            .expect("request view lock was poisoned")
            .branding = branding;
    }

    #[must_use]
    pub fn metadata(&self) -> PageMetadata {
        self.snapshot().metadata
    }

    fn snapshot(&self) -> RequestViewData {
        self.inner
            .read()
            .expect("request view lock was poisoned")
            .clone()
    }
}

impl Default for RequestView {
    fn default() -> Self {
        Self::new(
            PageMetadata::default(),
            Branding {
                word: "deliriums".to_owned(),
                slogan: "a little rusty".to_owned(),
            },
        )
    }
}

#[layout("/")]
pub async fn root_layout(cx: &Cx, slot: Slot<'_>) -> Result {
    let content = match slot.await {
        Ok(content) => content,
        Err(error) if error.downcast_ref::<NotFoundError>().is_some() => {
            if let Some(request_view) = try_request_context::<RequestView>(cx) {
                request_view.set_metadata(PageMetadata::new(
                    "Not Found | dathagerty.com",
                    "The requested page does not exist.",
                ));
            }
            view! {
                (StatusCode::NOT_FOUND)
                not_found(path: uri(cx).path())
            }?
        }
        Err(error) => match error.downcast_ref::<AppError>() {
            Some(AppError::NotFound { path }) => {
                if let Some(request_view) = try_request_context::<RequestView>(cx) {
                    request_view.set_metadata(PageMetadata::new(
                        "Not Found | dathagerty.com",
                        "The requested page does not exist.",
                    ));
                }
                view! {
                    (StatusCode::NOT_FOUND)
                    not_found(path: path)
                }?
            }
            Some(AppError::Content(source)) => {
                tracing::error!(error = ?source, "content repository request failed");
                if let Some(request_view) = try_request_context::<RequestView>(cx) {
                    request_view.set_metadata(PageMetadata::new(
                        "Server Error | dathagerty.com",
                        "The server could not complete this request.",
                    ));
                }
                view! {
                    (StatusCode::INTERNAL_SERVER_ERROR)
                    internal_error()
                }?
            }
            None => {
                tracing::error!(error = ?error, "page rendering failed");
                if let Some(request_view) = try_request_context::<RequestView>(cx) {
                    request_view.set_metadata(PageMetadata::new(
                        "Server Error | dathagerty.com",
                        "The server could not complete this request.",
                    ));
                }
                view! {
                    (StatusCode::INTERNAL_SERVER_ERROR)
                    internal_error()
                }?
            }
        },
    };
    let request_view = try_request_context::<RequestView>(cx)
        .map(RequestView::snapshot)
        .unwrap_or_else(|| RequestView::default().snapshot());
    let metadata = request_view.metadata;
    let branding = request_view.branding;

    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <meta
                    name="theme-color"
                    content="#e6e9ef"
                    media="(prefers-color-scheme: light)"
                >
                <meta
                    name="theme-color"
                    content="#181825"
                    media="(prefers-color-scheme: dark)"
                >
                <meta name="description" content=(&metadata.description)>
                if let Some(go) = &metadata.go {
                    <meta name="go-import" content=(go.import_content())>
                    if let Some(source) = go.source_content() {
                        <meta name="go-source" content=(source)>
                    }
                }
                <link
                    rel="icon"
                    href="data:image/svg+xml,%3Csvg xmlns=%22http://www.w3.org/2000/svg%22 viewBox=%220 0 100 100%22%3E%3Ctext y=%22.9em%22 font-size=%2290%22%3E%F0%9F%92%80%3C/text%3E%3C/svg%3E"
                >
                <link rel="stylesheet" href=(super::STYLESHEET)>
                <title>(&metadata.title)</title>
            </head>
            <body>
                site_header(branding: branding)
                <main id="main">(content)</main>
                site_footer()
            </body>
        </html>
    }
}
