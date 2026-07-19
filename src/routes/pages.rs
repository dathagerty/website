use topcoat::{
    Result,
    context::Cx,
    router::{StatusCode, page, uri},
    view::view,
};

use crate::{
    error::{AppError, AppResult},
    routes::{install_branding, repository, set_metadata},
    views::{PageMetadata, markdown_page, not_found as not_found_view},
};

async fn load_page(cx: &Cx, slug: &str) -> AppResult<crate::content::Page> {
    install_branding(cx).await?;
    let page = repository(cx)
        .page(slug)
        .await?
        .ok_or_else(|| AppError::not_found(uri(cx).path()))?;
    let description = if slug == "root" {
        PageMetadata::default().description
    } else {
        format!("{} on dathagerty.com", page.title)
    };
    set_metadata(cx, PageMetadata::new(page_title(&page.title), description));
    Ok(page)
}

fn page_title(title: &str) -> String {
    if title == "dathagerty.com" {
        title.to_owned()
    } else {
        format!("{title} | dathagerty.com")
    }
}

#[page("/")]
pub async fn root(cx: &Cx) -> Result {
    let page = load_page(cx, "root").await?;
    view! { markdown_page(page: page) }
}

#[page("/about")]
pub async fn about(cx: &Cx) -> Result {
    let page = load_page(cx, "about").await?;
    view! { markdown_page(page: page) }
}

#[page("/reading")]
pub async fn reading(cx: &Cx) -> Result {
    let page = load_page(cx, "reading").await?;
    view! { markdown_page(page: page) }
}

#[page("/{*path}")]
pub async fn fallback(cx: &Cx) -> Result {
    install_branding(cx).await?;
    set_metadata(
        cx,
        PageMetadata::new(
            "Not Found | dathagerty.com",
            "The requested page does not exist.",
        ),
    );
    view! {
        (StatusCode::NOT_FOUND)
        not_found_view(path: uri(cx).path())
    }
}
