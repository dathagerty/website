use std::{path::Path, sync::OnceLock};

use chrono::NaiveDate;
use comrak::{Arena, options::Plugins, plugins::syntect::SyntectAdapter};
use serde::Deserialize;

use super::{ContentError, Page, Post};

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PageFrontmatter {
    title: Option<String>,
    kind: Option<String>,
    last_edit: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PostFrontmatter {
    title: Option<String>,
    publish_date: Option<String>,
    last_edit: Option<String>,
    kind: Option<String>,
    #[serde(default)]
    draft: bool,
    summary: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

pub fn parse_page(path: impl AsRef<Path>, source: &str) -> Result<Page, ContentError> {
    let path = path.as_ref();
    let (slug, yaml, markdown) = document_parts(path, source)?;
    let frontmatter: PageFrontmatter =
        serde_yaml_ng::from_str(yaml).map_err(|source| ContentError::Yaml {
            path: path.to_path_buf(),
            source,
        })?;
    let title = frontmatter
        .title
        .ok_or_else(|| ContentError::MissingField {
            path: path.to_path_buf(),
            field: "title",
        })?;
    let last_edit = parse_optional_date(path, "lastEdit", frontmatter.last_edit)?;

    Ok(Page {
        slug: slug.to_owned(),
        title,
        kind: frontmatter.kind,
        last_edit,
        html: render_markdown(path, markdown)?,
    })
}

pub fn parse_post(path: impl AsRef<Path>, source: &str) -> Result<Post, ContentError> {
    let path = path.as_ref();
    let (slug, yaml, markdown) = document_parts(path, source)?;
    let frontmatter: PostFrontmatter =
        serde_yaml_ng::from_str(yaml).map_err(|source| ContentError::Yaml {
            path: path.to_path_buf(),
            source,
        })?;
    let title = frontmatter
        .title
        .ok_or_else(|| ContentError::MissingField {
            path: path.to_path_buf(),
            field: "title",
        })?;
    let publish_date_value =
        frontmatter
            .publish_date
            .ok_or_else(|| ContentError::MissingField {
                path: path.to_path_buf(),
                field: "publishDate",
            })?;
    let publish_date = parse_date(path, "publishDate", publish_date_value)?;
    let last_edit = parse_optional_date(path, "lastEdit", frontmatter.last_edit)?;

    Ok(Post {
        slug: slug.to_owned(),
        title,
        publish_date,
        last_edit,
        kind: frontmatter.kind,
        draft: frontmatter.draft,
        summary: frontmatter.summary,
        tags: frontmatter.tags,
        html: render_markdown(path, markdown)?,
    })
}

fn document_parts<'p, 's>(
    path: &'p Path,
    source: &'s str,
) -> Result<(&'p str, &'s str, &'s str), ContentError> {
    let slug = slug_from_path(path)?;
    let rest = source
        .strip_prefix("---\n")
        .ok_or_else(|| ContentError::MissingFrontmatter {
            path: path.to_path_buf(),
        })?;
    let (yaml, markdown) =
        rest.split_once("\n---\n")
            .ok_or_else(|| ContentError::MalformedFrontmatter {
                path: path.to_path_buf(),
            })?;

    Ok((slug, yaml, markdown))
}

fn slug_from_path(path: &Path) -> Result<&str, ContentError> {
    let invalid = || ContentError::InvalidFilename {
        path: path.to_path_buf(),
    };
    let slug = path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(".md"))
        .ok_or_else(invalid)?;
    let is_url_safe = slug
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));

    if slug.is_empty() || !is_url_safe {
        return Err(invalid());
    }

    Ok(slug)
}

fn parse_date(path: &Path, field: &'static str, value: String) -> Result<NaiveDate, ContentError> {
    NaiveDate::parse_from_str(&value, "%Y-%m-%d").map_err(|source| ContentError::InvalidDate {
        path: path.to_path_buf(),
        field,
        value,
        source,
    })
}

fn parse_optional_date(
    path: &Path,
    field: &'static str,
    value: Option<String>,
) -> Result<Option<NaiveDate>, ContentError> {
    value
        .map(|value| parse_date(path, field, value))
        .transpose()
}

fn render_markdown(path: &Path, markdown: &str) -> Result<String, ContentError> {
    let arena = Arena::new();
    let document = comrak::parse_document(&arena, markdown, markdown_options());
    let mut html = String::new();
    comrak::format_html_with_plugins(document, markdown_options(), &mut html, markdown_plugins())
        .map_err(|source| ContentError::Render {
        path: path.to_path_buf(),
        source,
    })?;

    Ok(html)
}

fn markdown_options() -> &'static comrak::Options<'static> {
    static OPTIONS: OnceLock<comrak::Options<'static>> = OnceLock::new();

    OPTIONS.get_or_init(|| {
        let mut options = comrak::Options::default();
        options.extension.strikethrough = true;
        options.extension.tagfilter = true;
        options.extension.table = true;
        options.extension.autolink = true;
        options.extension.tasklist = true;
        options.extension.header_ids = Some(String::new());
        options.render.gfm_quirks = true;
        options.render.r#unsafe = true;
        options
    })
}

fn markdown_plugins() -> &'static Plugins<'static> {
    static HIGHLIGHTER: OnceLock<SyntectAdapter> = OnceLock::new();
    static PLUGINS: OnceLock<Plugins<'static>> = OnceLock::new();

    PLUGINS.get_or_init(|| {
        let mut plugins = Plugins::default();
        plugins.render.codefence_syntax_highlighter =
            Some(HIGHLIGHTER.get_or_init(|| SyntectAdapter::new(None)));
        plugins
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{Branding, GoModule, TagSummary};
    use chrono::NaiveDate;

    #[test]
    fn parses_valid_page() {
        let source = "---\ntitle: About\nkind: page\nlastEdit: 2023-10-03\n---\n# Hello";

        let page = parse_page("pages/about.md", source).unwrap();

        assert_eq!(page.slug, "about");
        assert_eq!(page.title, "About");
        assert_eq!(page.kind.as_deref(), Some("page"));
        assert_eq!(
            page.last_edit,
            Some(NaiveDate::from_ymd_opt(2023, 10, 3).unwrap())
        );
        assert!(page.html.contains("Hello"));
    }

    #[test]
    fn parses_valid_post() {
        let source = concat!(
            "---\n",
            "title: Hello\n",
            "publishDate: 2025-07-25\n",
            "lastEdit: 2025-07-26\n",
            "kind: post\n",
            "draft: true\n",
            "summary: A greeting\n",
            "tags: [rust, meta]\n",
            "---\n",
            "Post body",
        );

        let post = parse_post("posts/hello.md", source).unwrap();

        assert_eq!(post.slug, "hello");
        assert_eq!(post.title, "Hello");
        assert_eq!(
            post.publish_date,
            NaiveDate::from_ymd_opt(2025, 7, 25).unwrap()
        );
        assert_eq!(
            post.last_edit,
            Some(NaiveDate::from_ymd_opt(2025, 7, 26).unwrap())
        );
        assert_eq!(post.kind.as_deref(), Some("post"));
        assert!(post.draft);
        assert_eq!(post.summary.as_deref(), Some("A greeting"));
        assert_eq!(post.tags, ["rust", "meta"]);
        assert!(post.html.contains("Post body"));
    }

    #[test]
    fn post_optional_fields_have_defaults() {
        let source = "---\ntitle: Hello\npublishDate: 2025-07-25\n---\nPost body";

        let post = parse_post("hello.md", source).unwrap();

        assert!(!post.draft);
        assert_eq!(post.tags, Vec::<String>::new());
        assert_eq!(post.summary, None);
        assert_eq!(post.kind, None);
        assert_eq!(post.last_edit, None);
    }

    #[test]
    fn rejects_post_without_publication_date() {
        let source = "---\ntitle: Hello\n---\nPost body";

        let error = parse_post("posts/hello.md", source).unwrap_err();

        assert!(matches!(
            error,
            ContentError::MissingField {
                ref path,
                field: "publishDate"
            } if path == Path::new("posts/hello.md")
        ));
    }

    #[test]
    fn rejects_post_without_title() {
        let source = "---\npublishDate: 2025-07-25\n---\nPost body";

        let error = parse_post("posts/hello.md", source).unwrap_err();

        assert!(matches!(
            error,
            ContentError::MissingField {
                ref path,
                field: "title"
            } if path == Path::new("posts/hello.md")
        ));
    }

    #[test]
    fn rejects_page_without_title() {
        let source = "---\nkind: page\n---\nPage body";

        let error = parse_page("pages/about.md", source).unwrap_err();

        assert!(matches!(
            error,
            ContentError::MissingField {
                ref path,
                field: "title"
            } if path == Path::new("pages/about.md")
        ));
    }

    #[test]
    fn rejects_invalid_yaml_with_path_context() {
        let source = "---\ntitle: About\nunexpected: true\n---\nPage body";

        let error = parse_page("pages/about.md", source).unwrap_err();

        assert!(matches!(
            error,
            ContentError::Yaml { ref path, .. } if path == Path::new("pages/about.md")
        ));
    }

    #[test]
    fn rejects_non_markdown_empty_and_unsafe_filenames() {
        let source = "---\ntitle: About\n---\nPage body";

        for path in [
            "about.txt",
            ".md",
            "hello world.md",
            "what?.md",
            ".hidden.md",
        ] {
            let error = parse_page(path, source).unwrap_err();

            assert!(matches!(
                error,
                ContentError::InvalidFilename { path: error_path }
                    if error_path == Path::new(path)
            ));
        }
    }

    #[test]
    fn renders_github_flavored_markdown() {
        let source = concat!(
            "---\ntitle: GFM\n---\n",
            "~~removed~~\n\n",
            "| name | value |\n| --- | --- |\n| one | two |\n\n",
            "- [x] shipped\n",
        );

        let page = parse_page("gfm.md", source).unwrap();

        assert!(page.html.contains("<del>removed</del>"));
        assert!(page.html.contains("<table>"));
        assert!(page.html.contains("type=\"checkbox\""));
    }

    #[test]
    fn renders_heading_ids() {
        let source = "---\ntitle: Heading\n---\n# Heading Name";

        let page = parse_page("heading.md", source).unwrap();

        assert!(page.html.contains("id=\"heading-name\""));
        assert!(page.html.contains("href=\"#heading-name\""));
    }

    #[test]
    fn preserves_trusted_raw_html() {
        let source = "---\ntitle: HTML\n---\n<aside data-kind=\"note\">Trusted</aside>";

        let page = parse_page("html.md", source).unwrap();

        assert!(
            page.html
                .contains("<aside data-kind=\"note\">Trusted</aside>")
        );
    }

    #[test]
    fn filters_dangerous_gfm_html_but_preserves_trusted_html() {
        let source = concat!(
            "---\ntitle: Filtered HTML\n---\n",
            "<script>alert('danger')</script>\n\n",
            "<aside data-kind=\"note\">Trusted</aside>",
        );

        let page = parse_page("filtered-html.md", source).unwrap();

        assert!(page.html.contains("&lt;script>alert('danger')&lt;/script>"));
        assert!(!page.html.contains("<script>"));
        assert!(
            page.html
                .contains("<aside data-kind=\"note\">Trusted</aside>")
        );
    }

    #[test]
    fn highlights_fenced_code_on_the_server() {
        let source = "---\ntitle: Code\n---\n```rust\nfn main() {}\n```";

        let page = parse_page("code.md", source).unwrap();

        assert!(page.html.contains("<pre class=\"syntax-highlighting\">"));
        assert!(page.html.contains("<code class=\"language-rust\">"));
        assert!(page.html.contains("<span class=\""));
    }

    #[test]
    fn auxiliary_content_models_are_owned_and_cloneable() {
        let tag = TagSummary {
            name: "rust".to_owned(),
            post_count: 2,
        };
        let module = GoModule {
            repository_url: "https://example.com/repo".to_owned(),
            module_path: "dathagerty.com/go/example".to_owned(),
            name: "example".to_owned(),
            description: "An example".to_owned(),
            license: "MIT".to_owned(),
        };
        let branding = Branding {
            word: "deliriums".to_owned(),
            slogan: "a little rusty".to_owned(),
        };

        assert_eq!(tag.clone(), tag);
        assert_eq!(module.clone(), module);
        assert_eq!(branding.clone(), branding);
        assert!(format!("{tag:?}{module:?}{branding:?}").contains("deliriums"));
    }
}
