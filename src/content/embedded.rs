use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use chrono::Datelike;
use rand::prelude::IndexedRandom;
use rust_embed::RustEmbed;
use serde::Deserialize;
use url::Url;

use super::{
    Branding, ContentError, ContentRepository, GoModule, Page, Post, TagSummary, parse_page,
    parse_post,
};

#[derive(RustEmbed)]
#[folder = "content/"]
struct EmbeddedContent;

#[derive(Debug)]
pub struct EmbeddedContentRepository {
    pages: BTreeMap<String, Page>,
    posts: Vec<Post>,
    posts_by_slug: BTreeMap<String, usize>,
    posts_by_year: Vec<(i32, Vec<usize>)>,
    tags: Vec<TagSummary>,
    posts_by_tag: BTreeMap<String, Vec<usize>>,
    modules: Vec<GoModule>,
    modules_by_name: BTreeMap<String, GoModule>,
    words: Vec<String>,
    slogans: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteData {
    #[serde(rename = "reading")]
    _reading: ReadingData,
    words: Vec<String>,
    slogans: Vec<String>,
    modules: Vec<ModuleData>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadingData {
    #[serde(rename = "current")]
    _current: Vec<String>,
    #[serde(rename = "tbr")]
    _tbr: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ModuleData {
    repository_url: String,
    module_path: String,
    name: String,
    description: String,
    license: String,
}

impl EmbeddedContentRepository {
    pub fn load(include_drafts: bool) -> Result<Self, ContentError> {
        let mut files = Vec::new();
        for path in EmbeddedContent::iter() {
            let asset = EmbeddedContent::get(path.as_ref()).ok_or_else(|| {
                ContentError::MissingEmbeddedAsset {
                    path: PathBuf::from(path.as_ref()),
                }
            })?;
            files.push((path.into_owned(), asset.data.into_owned()));
        }

        Self::from_files(include_drafts, files)
    }

    fn from_files(
        include_drafts: bool,
        files: impl IntoIterator<Item = (String, Vec<u8>)>,
    ) -> Result<Self, ContentError> {
        let mut pages = BTreeMap::new();
        let mut page_paths: BTreeMap<String, PathBuf> = BTreeMap::new();
        let mut all_posts = BTreeMap::new();
        let mut post_paths: BTreeMap<String, PathBuf> = BTreeMap::new();
        let mut data = None;

        for (path, bytes) in files {
            let path = PathBuf::from(path);
            let source = std::str::from_utf8(&bytes).map_err(|source| ContentError::Utf8 {
                path: path.clone(),
                source,
            })?;

            if path == Path::new("data.json") {
                if data.is_some() {
                    return Err(ContentError::DuplicateData);
                }
                data = Some(
                    serde_json::from_str(source).map_err(|source| ContentError::Json {
                        path: path.clone(),
                        source,
                    })?,
                );
            } else if path.starts_with("pages") {
                let page = parse_page(&path, source)?;
                let slug = page.slug.clone();
                if let Some(first_path) = page_paths.get(&slug) {
                    return Err(ContentError::DuplicateSlug {
                        kind: "page",
                        slug,
                        first_path: first_path.clone(),
                        second_path: path,
                    });
                }
                page_paths.insert(slug.clone(), path);
                pages.insert(slug, page);
            } else if path.starts_with("posts") {
                let post = parse_post(&path, source)?;
                let slug = post.slug.clone();
                if let Some(first_path) = post_paths.get(&slug) {
                    return Err(ContentError::DuplicateSlug {
                        kind: "post",
                        slug,
                        first_path: first_path.clone(),
                        second_path: path,
                    });
                }
                post_paths.insert(slug.clone(), path);
                all_posts.insert(slug, post);
            } else {
                return Err(ContentError::UnexpectedFile { path });
            }
        }

        for slug in ["root", "about", "reading"] {
            if !pages.contains_key(slug) {
                return Err(ContentError::MissingRequiredPage {
                    slug: slug.to_owned(),
                });
            }
        }

        let SiteData {
            _reading: _,
            words,
            slogans,
            modules: module_data,
        } = data.ok_or(ContentError::MissingData)?;
        if words.is_empty() {
            return Err(ContentError::EmptyBranding {
                collection: "words",
            });
        }
        if slogans.is_empty() {
            return Err(ContentError::EmptyBranding {
                collection: "slogans",
            });
        }

        let mut modules = Vec::with_capacity(module_data.len());
        let mut module_names = BTreeSet::new();
        let mut module_paths = BTreeSet::new();
        for module in &module_data {
            if !module_paths.insert(module.module_path.clone()) {
                return Err(ContentError::DuplicateModulePath {
                    module_path: module.module_path.clone(),
                });
            }
        }
        for (index, module) in module_data.into_iter().enumerate() {
            validate_module(index, &module)?;
            if !module_names.insert(module.name.clone()) {
                return Err(ContentError::DuplicateModule { name: module.name });
            }
            modules.push(GoModule {
                repository_url: module.repository_url,
                module_path: module.module_path,
                name: module.name,
                description: module.description,
                license: module.license,
            });
        }
        modules.sort_by(|left, right| left.name.cmp(&right.name));
        let modules_by_name = modules
            .iter()
            .cloned()
            .map(|module| (module.name.clone(), module))
            .collect();

        let mut posts: Vec<_> = all_posts
            .into_values()
            .filter(|post| include_drafts || !post.draft)
            .collect();
        posts.sort_by(|left, right| {
            right
                .publish_date
                .cmp(&left.publish_date)
                .then_with(|| left.slug.cmp(&right.slug))
        });
        let posts_by_slug = posts
            .iter()
            .enumerate()
            .map(|(index, post)| (post.slug.clone(), index))
            .collect();

        let mut year_index: BTreeMap<Reverse<i32>, Vec<usize>> = BTreeMap::new();
        let mut tag_index: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (index, post) in posts.iter().enumerate() {
            year_index
                .entry(Reverse(post.publish_date.year()))
                .or_default()
                .push(index);
            for tag in post.tags.iter().collect::<BTreeSet<_>>() {
                tag_index.entry(tag.clone()).or_default().push(index);
            }
        }
        let posts_by_year = year_index
            .into_iter()
            .map(|(Reverse(year), posts)| (year, posts))
            .collect();
        let tags = tag_index
            .iter()
            .map(|(name, posts)| TagSummary {
                name: name.clone(),
                post_count: posts.len(),
            })
            .collect();

        Ok(Self {
            pages,
            posts,
            posts_by_slug,
            posts_by_year,
            tags,
            posts_by_tag: tag_index,
            modules,
            modules_by_name,
            words,
            slogans,
        })
    }
}

fn validate_module(index: usize, module: &ModuleData) -> Result<(), ContentError> {
    for (field, value) in [
        ("repositoryUrl", module.repository_url.as_str()),
        ("modulePath", module.module_path.as_str()),
        ("name", module.name.as_str()),
        ("description", module.description.as_str()),
        ("license", module.license.as_str()),
    ] {
        if value.trim().is_empty() || value.trim() != value {
            return Err(ContentError::InvalidModule { index, field });
        }
    }
    let repository_url =
        Url::parse(&module.repository_url).map_err(|_| ContentError::InvalidModule {
            index,
            field: "repositoryUrl",
        })?;
    if repository_url.scheme() != "https" || repository_url.host_str().is_none() {
        return Err(ContentError::InvalidModule {
            index,
            field: "repositoryUrl",
        });
    }
    if !valid_module_name(&module.name)
        || module.module_path != format!("dathagerty.com/go/{}", module.name)
    {
        return Err(ContentError::InvalidModule {
            index,
            field: "modulePath",
        });
    }
    Ok(())
}

pub(crate) fn valid_module_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && !name.ends_with('.')
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~'))
}

#[async_trait]
impl ContentRepository for EmbeddedContentRepository {
    async fn page(&self, slug: &str) -> Result<Option<Page>, ContentError> {
        Ok(self.pages.get(slug).cloned())
    }

    async fn post(&self, slug: &str) -> Result<Option<Post>, ContentError> {
        Ok(self
            .posts_by_slug
            .get(slug)
            .map(|index| self.posts[*index].clone()))
    }

    async fn posts(&self) -> Result<Vec<Post>, ContentError> {
        Ok(self.posts.clone())
    }

    async fn posts_by_year(&self) -> Result<Vec<(i32, Vec<Post>)>, ContentError> {
        Ok(self
            .posts_by_year
            .iter()
            .map(|(year, indexes)| {
                (
                    *year,
                    indexes
                        .iter()
                        .map(|index| self.posts[*index].clone())
                        .collect(),
                )
            })
            .collect())
    }

    async fn tags(&self) -> Result<Vec<TagSummary>, ContentError> {
        Ok(self.tags.clone())
    }

    async fn posts_for_tag(&self, tag: &str) -> Result<Option<Vec<Post>>, ContentError> {
        Ok(self.posts_by_tag.get(tag).map(|indexes| {
            indexes
                .iter()
                .map(|index| self.posts[*index].clone())
                .collect()
        }))
    }

    async fn modules(&self) -> Result<Vec<GoModule>, ContentError> {
        Ok(self.modules.clone())
    }

    async fn module(&self, name: &str) -> Result<Option<GoModule>, ContentError> {
        Ok(self.modules_by_name.get(name).cloned())
    }

    async fn branding(&self) -> Result<Branding, ContentError> {
        let mut rng = rand::rng();
        Ok(Branding {
            word: self
                .words
                .choose(&mut rng)
                .expect("words validated")
                .clone(),
            slogan: self
                .slogans
                .choose(&mut rng)
                .expect("slogans validated")
                .clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::Path, sync::Arc};

    use super::EmbeddedContentRepository;
    use crate::content::{ContentError, ContentRepository};

    const PAGE: &str = "---\ntitle: Fixture\n---\nFixture page";

    fn post(title: &str, date: &str, draft: bool, tags: &[&str]) -> String {
        format!(
            "---\ntitle: {title}\npublishDate: {date}\ndraft: {draft}\ntags: [{}]\n---\nFixture post",
            tags.join(", ")
        )
    }

    fn data_json() -> String {
        serde_json::json!({
            "reading": {
                "current": ["Current book"],
                "tbr": ["Future book"]
            },
            "words": ["deliriums", "directory"],
            "slogans": ["a little rusty", "value-sized"],
            "modules": [
                {
                    "repositoryUrl": "https://example.com/zulu",
                    "modulePath": "dathagerty.com/go/zulu",
                    "name": "zulu",
                    "description": "The last module.",
                    "license": "MIT"
                },
                {
                    "repositoryUrl": "https://example.com/alpha",
                    "modulePath": "dathagerty.com/go/alpha",
                    "name": "alpha",
                    "description": "The first module.",
                    "license": "WTFPL"
                }
            ]
        })
        .to_string()
    }

    fn fixture_files() -> Vec<(String, Vec<u8>)> {
        vec![
            ("pages/root.md".into(), PAGE.as_bytes().to_vec()),
            ("pages/about.md".into(), PAGE.as_bytes().to_vec()),
            ("pages/reading.md".into(), PAGE.as_bytes().to_vec()),
            (
                "posts/zeta.md".into(),
                post("Zeta", "2025-01-02", false, &["rust", "meta"]).into_bytes(),
            ),
            (
                "posts/alpha.md".into(),
                post("Alpha", "2025-01-02", false, &["rust"]).into_bytes(),
            ),
            (
                "posts/older.md".into(),
                post("Older", "2024-04-03", false, &["meta"]).into_bytes(),
            ),
            (
                "posts/draft.md".into(),
                post("Draft", "2026-06-01", true, &["hidden"]).into_bytes(),
            ),
            ("data.json".into(), data_json().into_bytes()),
        ]
    }

    fn fixture_repository(include_drafts: bool) -> EmbeddedContentRepository {
        EmbeddedContentRepository::from_files(include_drafts, fixture_files()).unwrap()
    }

    #[tokio::test]
    async fn post_indexes_store_only_canonical_positions() {
        fn assert_slug_index(_: &BTreeMap<String, usize>) {}
        fn assert_group_index(_: &[(i32, Vec<usize>)]) {}
        fn assert_tag_index(_: &BTreeMap<String, Vec<usize>>) {}

        let repository = fixture_repository(false);
        let canonical = repository.posts.clone();

        assert_slug_index(&repository.posts_by_slug);
        assert_group_index(&repository.posts_by_year);
        assert_tag_index(&repository.posts_by_tag);
        assert_eq!(repository.posts().await.unwrap(), canonical);
        assert_eq!(
            repository.post("alpha").await.unwrap(),
            Some(canonical[0].clone())
        );
        assert_eq!(
            repository.posts_by_year().await.unwrap(),
            vec![
                (2025, vec![canonical[0].clone(), canonical[1].clone()]),
                (2024, vec![canonical[2].clone()]),
            ]
        );
        assert_eq!(
            repository.posts_for_tag("meta").await.unwrap(),
            Some(vec![canonical[1].clone(), canonical[2].clone()])
        );
    }

    #[test]
    fn rejects_a_missing_required_page() {
        let files = fixture_files()
            .into_iter()
            .filter(|(path, _)| path != "pages/reading.md");

        let error = EmbeddedContentRepository::from_files(false, files).unwrap_err();

        assert!(matches!(error, ContentError::MissingRequiredPage { slug } if slug == "reading"));
    }

    #[test]
    fn rejects_duplicate_slugs() {
        let mut files = fixture_files();
        files.push((
            "posts/archive/alpha.md".into(),
            post("Duplicate", "2023-01-01", false, &[]).into_bytes(),
        ));

        let error = EmbeddedContentRepository::from_files(false, files).unwrap_err();

        let rendered = error.to_string();
        match error {
            ContentError::DuplicateSlug {
                kind,
                slug,
                first_path,
                second_path,
            } => {
                assert_eq!(kind, "post");
                assert_eq!(slug, "alpha");
                assert_eq!(first_path, Path::new("posts/alpha.md"));
                assert_eq!(second_path, Path::new("posts/archive/alpha.md"));
                assert!(rendered.contains("posts/alpha.md"));
                assert!(rendered.contains("posts/archive/alpha.md"));
            }
            other => panic!("expected duplicate slug error, got {other:?}"),
        }
    }

    #[test]
    fn rejects_malformed_json() {
        let mut files = fixture_files();
        files.retain(|(path, _)| path != "data.json");
        files.push(("data.json".into(), b"{not json}".to_vec()));

        let error = EmbeddedContentRepository::from_files(false, files).unwrap_err();

        assert!(matches!(error, ContentError::Json { .. }));
    }

    #[test]
    fn rejects_incomplete_module_metadata() {
        let mut files = fixture_files();
        files.retain(|(path, _)| path != "data.json");
        let data = data_json().replace("https://example.com/alpha", "");
        files.push(("data.json".into(), data.into_bytes()));

        let error = EmbeddedContentRepository::from_files(false, files).unwrap_err();

        assert!(matches!(
            error,
            ContentError::InvalidModule {
                index: 1,
                field: "repositoryUrl"
            }
        ));
    }

    #[test]
    fn rejects_module_metadata_with_surrounding_whitespace() {
        let mut files = fixture_files();
        files.retain(|(path, _)| path != "data.json");
        let data = data_json().replace("\"name\":\"alpha\"", "\"name\":\" alpha\"");
        files.push(("data.json".into(), data.into_bytes()));

        let error = EmbeddedContentRepository::from_files(false, files).unwrap_err();

        assert!(matches!(
            error,
            ContentError::InvalidModule {
                index: 1,
                field: "name"
            }
        ));
    }

    #[test]
    fn rejects_duplicate_module_paths() {
        let mut files = fixture_files();
        files.retain(|(path, _)| path != "data.json");
        let data = data_json().replace("dathagerty.com/go/zulu", "dathagerty.com/go/alpha");
        files.push(("data.json".into(), data.into_bytes()));

        let error = EmbeddedContentRepository::from_files(false, files).unwrap_err();

        assert!(matches!(
            error,
            ContentError::DuplicateModulePath { module_path }
                if module_path == "dathagerty.com/go/alpha"
        ));
    }

    #[test]
    fn rejects_repository_urls_that_are_not_valid_https_urls() {
        for invalid_url in ["not a url", "http://example.com/alpha", "https://"] {
            let mut files = fixture_files();
            files.retain(|(path, _)| path != "data.json");
            let data = data_json().replace("https://example.com/alpha", invalid_url);
            files.push(("data.json".into(), data.into_bytes()));

            let error = EmbeddedContentRepository::from_files(false, files).unwrap_err();

            assert!(error.to_string().contains("invalid repositoryUrl"));
            assert!(matches!(
                error,
                ContentError::InvalidModule {
                    index: 1,
                    field: "repositoryUrl"
                }
            ));
        }
    }

    #[test]
    fn rejects_invalid_or_mismatched_vanity_module_paths() {
        let wrong_namespace =
            data_json().replace("dathagerty.com/go/alpha", "example.com/go/alpha");
        let wrong_name = data_json().replace("dathagerty.com/go/alpha", "dathagerty.com/go/beta");
        let invalid_segment = data_json()
            .replace("dathagerty.com/go/alpha", "dathagerty.com/go/bad name")
            .replace("\"name\":\"alpha\"", "\"name\":\"bad name\"");

        for data in [wrong_namespace, wrong_name, invalid_segment] {
            let mut files = fixture_files();
            files.retain(|(path, _)| path != "data.json");
            files.push(("data.json".into(), data.into_bytes()));

            let error = EmbeddedContentRepository::from_files(false, files).unwrap_err();

            assert!(matches!(
                error,
                ContentError::InvalidModule {
                    index: 1,
                    field: "modulePath"
                }
            ));
        }
    }

    #[test]
    fn rejects_invalid_utf8() {
        let mut files = fixture_files();
        files.push(("pages/broken.md".into(), vec![0xff]));

        let error = EmbeddedContentRepository::from_files(false, files).unwrap_err();

        assert!(matches!(error, ContentError::Utf8 { .. }));
    }

    #[tokio::test]
    async fn production_excludes_drafts() {
        let repository = fixture_repository(false);

        assert_eq!(repository.post("draft").await.unwrap(), None);
        assert!(
            repository
                .posts()
                .await
                .unwrap()
                .iter()
                .all(|post| !post.draft)
        );
    }

    #[tokio::test]
    async fn development_includes_drafts() {
        let repository = fixture_repository(true);

        assert_eq!(
            repository.post("draft").await.unwrap().unwrap().title,
            "Draft"
        );
        assert_eq!(repository.posts().await.unwrap()[0].slug, "draft");
    }

    #[tokio::test]
    async fn posts_are_newest_first_with_a_slug_tie_break() {
        let repository = fixture_repository(false);

        let slugs: Vec<_> = repository
            .posts()
            .await
            .unwrap()
            .into_iter()
            .map(|post| post.slug)
            .collect();

        assert_eq!(slugs, ["alpha", "zeta", "older"]);
    }

    #[tokio::test]
    async fn year_groups_are_descending_and_keep_post_order() {
        let repository = fixture_repository(false);

        let groups = repository.posts_by_year().await.unwrap();

        assert_eq!(
            groups.iter().map(|(year, _)| *year).collect::<Vec<_>>(),
            [2025, 2024]
        );
        assert_eq!(
            groups[0]
                .1
                .iter()
                .map(|post| post.slug.as_str())
                .collect::<Vec<_>>(),
            ["alpha", "zeta"]
        );
    }

    #[tokio::test]
    async fn tags_are_alphabetical_with_post_counts() {
        let repository = fixture_repository(false);

        let tags = repository.tags().await.unwrap();

        assert_eq!(
            tags.iter()
                .map(|tag| (tag.name.as_str(), tag.post_count))
                .collect::<Vec<_>>(),
            [("meta", 2), ("rust", 2)]
        );
    }

    #[tokio::test]
    async fn tag_lookup_returns_ordered_posts_and_none_for_unknown_tags() {
        let repository = fixture_repository(false);

        let posts = repository.posts_for_tag("meta").await.unwrap().unwrap();

        assert_eq!(
            posts
                .iter()
                .map(|post| post.slug.as_str())
                .collect::<Vec<_>>(),
            ["zeta", "older"]
        );
        assert_eq!(repository.posts_for_tag("missing").await.unwrap(), None);
    }

    #[tokio::test]
    async fn modules_are_alphabetical_and_support_lookup() {
        let repository = fixture_repository(false);

        let modules = repository.modules().await.unwrap();

        assert_eq!(
            modules
                .iter()
                .map(|module| module.name.as_str())
                .collect::<Vec<_>>(),
            ["alpha", "zulu"]
        );
        let alpha = repository.module("alpha").await.unwrap().unwrap();
        assert_eq!(alpha.module_path, "dathagerty.com/go/alpha");
        assert_eq!(alpha.repository_url, "https://example.com/alpha");
        assert_eq!(alpha.description, "The first module.");
        assert_eq!(alpha.license, "WTFPL");
        assert_eq!(repository.module("missing").await.unwrap(), None);
    }

    #[tokio::test]
    async fn branding_is_nonempty_and_selected_from_its_collections() {
        let repository = fixture_repository(false);

        let branding = repository.branding().await.unwrap();

        assert!(!branding.word.is_empty());
        assert!(!branding.slogan.is_empty());
        assert!(["deliriums", "directory"].contains(&branding.word.as_str()));
        assert!(["a little rusty", "value-sized"].contains(&branding.slogan.as_str()));
    }

    #[tokio::test]
    async fn unknown_page_and_post_lookups_return_none() {
        let repository = fixture_repository(false);

        assert_eq!(repository.page("missing").await.unwrap(), None);
        assert_eq!(repository.post("missing").await.unwrap(), None);
    }

    #[tokio::test]
    async fn repository_trait_is_object_safe() {
        let repository: Arc<dyn ContentRepository> = Arc::new(fixture_repository(false));

        assert_eq!(repository.page("root").await.unwrap().unwrap().slug, "root");
    }

    #[tokio::test]
    async fn loads_the_real_embedded_content() {
        let repository = EmbeddedContentRepository::load(false).unwrap();

        let root = repository.page("root").await.unwrap().unwrap();
        assert_eq!(root.html.matches("<br />").count(), 22);
        assert_eq!(repository.posts().await.unwrap().len(), 1);
        assert_eq!(repository.post("how-this-works").await.unwrap(), None);
        assert_eq!(repository.modules().await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn embedded_homepage_preserves_one_non_nested_mastodon_identity_link() {
        let repository = EmbeddedContentRepository::load(false).unwrap();

        let html = repository.page("root").await.unwrap().unwrap().html;
        let identity_link = concat!(
            "<a href=\"https://hachyderm.io/@dathagerty\" rel=\"me\">",
            "@dathagerty<!-- -->@hachyderm.io</a>",
        );

        assert_eq!(html.matches(identity_link).count(), 1, "{html}");
        assert_eq!(
            identity_link.replace("<!-- -->", ""),
            concat!(
                "<a href=\"https://hachyderm.io/@dathagerty\" rel=\"me\">",
                "@dathagerty@hachyderm.io</a>",
            )
        );
        assert!(!html.contains("<a href=\"mailto:"), "{html}");
        assert!(!html.contains("rel=\"me\"><a"), "{html}");
    }

    #[tokio::test]
    async fn embedded_about_page_has_corrected_public_copy_and_preserved_edit_date() {
        let repository = EmbeddedContentRepository::load(false).unwrap();

        let about = repository.page("about").await.unwrap().unwrap();

        assert!(about.html.contains("Currently employed"));
        assert!(about.html.contains("color palette"));
        assert!(!about.html.contains("Curently"));
        assert!(!about.html.contains("pallate"));
        assert_eq!(
            about.last_edit,
            Some(chrono::NaiveDate::from_ymd_opt(2026, 7, 18).unwrap())
        );
    }
}
