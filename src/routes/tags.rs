use std::str::FromStr;

use topcoat::{
    Result,
    context::Cx,
    router::{page, path_param, uri},
    view::view,
};

use crate::{
    error::AppError,
    routes::{InvalidPathParam, install_branding, repository, set_metadata},
    views::{PageMetadata, tag_page, tags_index},
};

#[derive(Debug)]
struct TagSegment(String);

impl TagSegment {
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for TagSegment {
    type Err = InvalidPathParam;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || value.chars().any(char::is_control)
            || value.contains(char::REPLACEMENT_CHARACTER)
        {
            return Err(InvalidPathParam);
        }
        Ok(Self(value.to_owned()))
    }
}

#[path_param(error = not_found)]
struct Tag(TagSegment);

#[page("/tags")]
pub async fn index(cx: &Cx) -> Result {
    install_branding(cx).await?;
    let tags = repository(cx).tags().await.map_err(AppError::from)?;
    set_metadata(
        cx,
        PageMetadata::new("Tags | dathagerty.com", "Tags used on dathagerty.com."),
    );
    view! { tags_index(tags: tags) }
}

#[page("/tags/{tag}")]
pub async fn tag(cx: &Cx) -> Result {
    let tag = path_param::<Tag>(cx)?.as_str();
    install_branding(cx).await?;
    let posts = repository(cx)
        .posts_for_tag(tag)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found(uri(cx).path()))?;
    set_metadata(
        cx,
        PageMetadata::new(
            format!("{tag} | dathagerty.com"),
            format!("Posts tagged {tag} on dathagerty.com."),
        ),
    );
    view! { tag_page(tag: tag.to_owned(), posts: posts) }
}
