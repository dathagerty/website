use std::str::FromStr;

use topcoat::{
    Result,
    context::Cx,
    router::{page, path_param, uri},
    view::view,
};

use crate::{
    content::valid_module_name,
    error::AppError,
    routes::{InvalidPathParam, install_branding, repository, set_metadata},
    views::{PageMetadata, module_page, modules_index},
};

#[derive(Debug)]
struct ModuleSegment(String);

impl ModuleSegment {
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ModuleSegment {
    type Err = InvalidPathParam;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        valid_module_name(value)
            .then(|| Self(value.to_owned()))
            .ok_or(InvalidPathParam)
    }
}

#[path_param(error = not_found)]
struct Module(ModuleSegment);

#[page("/go")]
pub async fn index(cx: &Cx) -> Result {
    install_branding(cx).await?;
    let modules = repository(cx).modules().await.map_err(AppError::from)?;
    set_metadata(
        cx,
        PageMetadata::new(
            "Go Modules | dathagerty.com",
            "Go modules published at dathagerty.com.",
        ),
    );
    view! { modules_index(modules: modules) }
}

#[page("/go/{module}")]
pub async fn module(cx: &Cx) -> Result {
    let name = path_param::<Module>(cx)?;
    install_branding(cx).await?;
    let module = repository(cx)
        .module(name.as_str())
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found(uri(cx).path()))?;
    let metadata = PageMetadata::for_module(&module);
    set_metadata(cx, metadata);
    view! { module_page(module: module) }
}
