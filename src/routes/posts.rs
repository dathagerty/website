use topcoat::{
    Result,
    context::Cx,
    router::{page, path_param, uri},
    view::view,
};

use crate::{
    error::AppError,
    routes::{SimpleSegment, install_branding, repository, set_metadata},
    views::{PageMetadata, blog_index, post_page},
};

#[path_param(error = not_found)]
struct PostSlug(SimpleSegment);

#[page("/blag")]
pub async fn index(cx: &Cx) -> Result {
    install_branding(cx).await?;
    let years = repository(cx)
        .posts_by_year()
        .await
        .map_err(AppError::from)?;
    set_metadata(
        cx,
        PageMetadata::new("Blag | dathagerty.com", "David Hagerty's blog posts."),
    );
    view! { blog_index(years: years) }
}

#[page("/blag/{post_slug}")]
pub async fn post(cx: &Cx) -> Result {
    let slug = path_param::<PostSlug>(cx)?;
    install_branding(cx).await?;
    let post = repository(cx)
        .post(slug.as_str())
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found(uri(cx).path()))?;
    set_metadata(
        cx,
        PageMetadata::new(
            format!("{} | dathagerty.com", post.title),
            post.summary
                .clone()
                .unwrap_or_else(|| "A post on dathagerty.com".to_owned()),
        ),
    );
    view! { post_page(post: post) }
}
