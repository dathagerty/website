mod components;
mod layout;
mod pages;

pub use components::*;
pub use layout::*;
pub use pages::*;

pub const STYLESHEET: topcoat::asset::Asset = topcoat::asset::asset!("assets/styles.css");

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use chrono::NaiveDate;
    use topcoat::{
        Result,
        asset::{AssetBundle, AssetRouteResolver, MANIFEST_VERSION, Manifest, ManifestEntry},
        context::{CxTestBuilder, request_context},
        router::LayoutFn,
        view::view,
    };

    use super::*;
    use crate::content::{Branding, GoModule, Page, Post, TagSummary};

    static BUNDLE_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

    fn render(result: Result, cx: &topcoat::context::Cx) -> String {
        result.unwrap().render(cx)
    }

    fn branding() -> Branding {
        Branding {
            word: "deliriums".to_owned(),
            slogan: "a little rusty".to_owned(),
        }
    }

    fn post() -> Post {
        Post {
            slug: "rust-and-<tea>".to_owned(),
            title: "Rust & <Tea>".to_owned(),
            publish_date: NaiveDate::from_ymd_opt(2025, 7, 25).unwrap(),
            last_edit: Some(NaiveDate::from_ymd_opt(2025, 7, 26).unwrap()),
            kind: Some("post".to_owned()),
            draft: false,
            summary: Some("A <small> summary".to_owned()),
            tags: vec!["rust".to_owned(), "web & html".to_owned()],
            html: "<p><strong>trusted markdown</strong></p>".to_owned(),
        }
    }

    fn module() -> GoModule {
        GoModule {
            repository_url: "https://git.sr.ht/~gloatingfiddle/example".to_owned(),
            module_path: "dathagerty.com/go/example".to_owned(),
            name: "example".to_owned(),
            description: "Useful <sometimes> & small".to_owned(),
            license: "MIT".to_owned(),
        }
    }

    fn layout_context(request_view: RequestView) -> (topcoat::context::Cx, PathBuf) {
        let bundle_dir = std::env::temp_dir().join(format!(
            "dathagerty-view-test-{}-{}",
            std::process::id(),
            BUNDLE_SEQUENCE.fetch_add(1, Ordering::Relaxed),
        ));
        if bundle_dir.exists() {
            fs::remove_dir_all(&bundle_dir).unwrap();
        }
        fs::create_dir(&bundle_dir).unwrap();
        fs::write(bundle_dir.join("styles-test.css"), "/* test */").unwrap();
        Manifest {
            version: MANIFEST_VERSION,
            assets: vec![ManifestEntry {
                id: STYLESHEET,
                file: "styles-test.css".to_owned(),
                hash: "test".to_owned(),
                content_type: "text/css".to_owned(),
            }],
        }
        .save(bundle_dir.join("manifest.toml"))
        .unwrap();
        let bundle = AssetBundle::load_dir(&bundle_dir).unwrap();
        let resolver = AssetRouteResolver::new(Box::new(|asset, writer| {
            write!(writer, "/test-assets/{}", asset.name().to_string_lossy())
        }));
        let cx = CxTestBuilder::new()
            .app_context(bundle)
            .app_context(resolver)
            .request_context(request_view)
            .build();

        (cx, bundle_dir)
    }

    #[tokio::test]
    async fn root_layout_renders_complete_document_and_route_metadata() {
        let module = module();
        let (cx, bundle_dir) = layout_context(RequestView::new(
            PageMetadata::for_module(&module),
            branding(),
        ));
        let __cx = &cx;
        let content = view! { <article>"module body"</article> }.unwrap();

        let html = render(view! { root_layout(slot: content) }, &cx);

        assert!(html.starts_with("<!DOCTYPE html><html lang=\"en\">"));
        assert!(html.contains("<meta charset=\"utf-8\">"));
        assert!(html.contains("name=\"viewport\""));
        assert!(html.contains(concat!(
            "<meta name=\"theme-color\" content=\"#e6e9ef\" ",
            "media=\"(prefers-color-scheme: light)\">",
        )));
        assert!(html.contains(concat!(
            "<meta name=\"theme-color\" content=\"#181825\" ",
            "media=\"(prefers-color-scheme: dark)\">",
        )));
        assert!(html.contains("name=\"description\" content=\"Useful <sometimes> &amp; small\""));
        assert!(html.contains("name=\"go-import\""));
        assert!(html.contains("name=\"go-source\""));
        assert!(html.contains("data:image/svg+xml"));
        assert!(html.contains("href=\"/test-assets/styles-test.css\""));
        assert!(html.contains("<title>example | dathagerty.com</title>"));
        assert!(html.contains("<header id=\"header\">"));
        assert!(html.contains("<main id=\"main\"><article>module body</article></main>"));
        assert!(html.contains("<footer id=\"footer\">"));

        fs::remove_dir_all(bundle_dir).unwrap();
    }

    #[tokio::test]
    async fn root_layout_uses_metadata_set_while_rendering_its_page_slot() {
        let (cx, bundle_dir) = layout_context(RequestView::default());
        let layout: LayoutFn = root_layout.into();
        let slot = Box::pin(async {
            request_context::<RequestView>(&cx).set_metadata(PageMetadata::new(
                "Dynamic post | dathagerty.com",
                "Metadata loaded by the page",
            ));
            let __cx = &cx;
            view! { <article>"dynamic body"</article> }
        });

        let html = layout.render(&cx, slot).await.unwrap().render(&cx);

        assert!(html.contains("<title>Dynamic post | dathagerty.com</title>"));
        assert!(html.contains("content=\"Metadata loaded by the page\""));
        fs::remove_dir_all(bundle_dir).unwrap();
    }

    #[tokio::test]
    async fn header_has_branding_navigation_identity_links_and_separator() {
        let cx = CxTestBuilder::new().build();
        let __cx = &cx;

        let html = render(view! { site_header(branding: branding()) }, &cx);

        for href in ["/about", "/blag", "/reading", "/tags", "/go"] {
            assert!(html.contains(&format!("href=\"{href}\"")));
        }
        assert!(html.contains("david's deliriums"));
        assert!(html.contains("a little rusty"));
        assert!(html.contains("rel=\"me\""));
        assert!(html.contains('§'));
    }

    #[tokio::test]
    async fn regular_dynamic_text_is_escaped() {
        let cx = CxTestBuilder::new().build();
        let __cx = &cx;
        let page = Page {
            slug: "about".to_owned(),
            title: "About <script>".to_owned(),
            kind: None,
            last_edit: None,
            html: "<p>trusted</p>".to_owned(),
        };

        let html = render(view! { markdown_page(page: page) }, &cx);

        assert!(html.contains("About &lt;script&gt;"));
        assert!(!html.contains("<h2>About <script>"));
    }

    #[tokio::test]
    async fn parsed_markdown_is_rendered_as_intentional_raw_html() {
        let cx = CxTestBuilder::new().build();
        let __cx = &cx;
        let page = Page {
            slug: "about".to_owned(),
            title: "About".to_owned(),
            kind: None,
            last_edit: None,
            html: "<p><em>trusted markdown</em></p>".to_owned(),
        };

        let html = render(view! { markdown_page(page: page) }, &cx);

        assert!(html.contains("<p><em>trusted markdown</em></p>"));
        assert!(!html.contains("&lt;em&gt;"));
    }

    #[tokio::test]
    async fn post_page_has_dates_tags_and_raw_markdown() {
        let cx = CxTestBuilder::new().build();
        let __cx = &cx;

        let html = render(view! { post_page(post: post()) }, &cx);

        assert!(html.contains("<time datetime=\"2025-07-25\">2025-07-25</time>"));
        assert!(html.contains(concat!(
            "last edited on ",
            "<time datetime=\"2025-07-26\">2025-07-26</time>",
        )));
        assert!(html.contains("href=\"/tags/rust\""));
        assert!(html.contains("web &amp; html"));
        assert!(html.contains("<strong>trusted markdown</strong>"));
    }

    #[tokio::test]
    async fn tag_links_percent_encode_one_path_segment_and_preserve_display_text() {
        let cx = CxTestBuilder::new().build();
        let __cx = &cx;
        let tags = [
            ("c#", "c%23"),
            ("rust/web", "rust%2Fweb"),
            ("what?", "what%3F"),
            ("two words", "two%20words"),
            ("100%", "100%25"),
            ("ordinary-ascii_123", "ordinary-ascii_123"),
        ];
        let summaries = tags
            .iter()
            .map(|(name, _)| TagSummary {
                name: (*name).to_owned(),
                post_count: 1,
            })
            .collect();
        let mut tagged_post = post();
        tagged_post.tags = tags.iter().map(|(name, _)| (*name).to_owned()).collect();

        let html = format!(
            "{}{}",
            render(view! { tags_index(tags: summaries) }, &cx),
            render(view! { post_metadata(post: tagged_post) }, &cx),
        );

        for (display, encoded) in tags {
            assert_eq!(
                html.matches(&format!("href=\"/tags/{encoded}\"")).count(),
                2,
            );
            assert!(html.contains(&format!(">{display} (1)</a>")));
            assert!(html.contains(&format!(">{display}</a>")));
        }
    }

    #[tokio::test]
    async fn lists_render_posts_tags_and_modules() {
        let cx = CxTestBuilder::new().build();
        let __cx = &cx;
        let post = post();

        let posts_html = render(
            view! { blog_index(years: vec![(2025, vec![post.clone()])]) },
            &cx,
        );
        let tags_html = render(
            view! {
                tags_index(
                    tags: vec![
                        TagSummary { name : "rust".to_owned(), post_count : 2 }
                    ]
                )
            },
            &cx,
        );
        let modules_html = render(view! { modules_index(modules: vec![module()]) }, &cx);

        assert!(posts_html.contains("<h3>2025</h3>"));
        assert!(posts_html.contains("A &lt;small&gt; summary"));
        assert!(tags_html.contains("rust (2)"));
        assert!(modules_html.contains("href=\"/go/example\""));
        assert!(modules_html.contains("Useful &lt;sometimes&gt; &amp; small"));
    }

    #[tokio::test]
    async fn blog_heading_separates_struck_writings_from_ramblings() {
        let cx = CxTestBuilder::new().build();
        let __cx = &cx;

        let html = render(view! { blog_index(years: Vec::new()) }, &cx);

        assert!(html.contains("<s>writings</s> ramblings"), "{html}");
    }

    #[tokio::test]
    async fn module_page_has_discovery_metadata_and_install_instructions() {
        let cx = CxTestBuilder::new().build();
        let __cx = &cx;
        let module = module();

        let body = render(view! { module_page(module: module.clone()) }, &cx);
        let metadata = PageMetadata::for_module(&module);
        let view_context = RequestView::new(metadata, branding());

        assert!(body.contains("go get dathagerty.com/go/example"));
        assert!(body.contains("import \"dathagerty.com/go/example\""));
        assert!(body.contains("https://pkg.go.dev/dathagerty.com/go/example"));
        assert_eq!(
            view_context
                .metadata()
                .go
                .as_ref()
                .unwrap()
                .import_content(),
            "dathagerty.com/go/example git https://git.sr.ht/~gloatingfiddle/example"
        );
    }

    #[tokio::test]
    async fn go_source_metadata_uses_repository_provider_templates() {
        let sourcehut = module();
        let github = GoModule {
            repository_url: "https://github.com/dathagerty/example".to_owned(),
            ..module()
        };
        let unknown = GoModule {
            repository_url: "https://code.example/dathagerty/example".to_owned(),
            ..module()
        };

        let (sourcehut_cx, sourcehut_dir) = layout_context(RequestView::new(
            PageMetadata::for_module(&sourcehut),
            branding(),
        ));
        let __cx = &sourcehut_cx;
        let sourcehut_html = render(
            view! { root_layout(slot: view! {}.unwrap()) },
            &sourcehut_cx,
        );
        assert!(sourcehut_html.contains(concat!(
            "content=\"dathagerty.com/go/example ",
            "https://git.sr.ht/~gloatingfiddle/example ",
            "https://git.sr.ht/~gloatingfiddle/example/tree/main/item{/dir} ",
            "https://git.sr.ht/~gloatingfiddle/example/tree/main/item{/dir}/{file}#L{line}\"",
        )));
        fs::remove_dir_all(sourcehut_dir).unwrap();

        let (github_cx, github_dir) = layout_context(RequestView::new(
            PageMetadata::for_module(&github),
            branding(),
        ));
        let __cx = &github_cx;
        let github_html = render(view! { root_layout(slot: view! {}.unwrap()) }, &github_cx);
        assert!(github_html.contains(concat!(
            "content=\"dathagerty.com/go/example ",
            "https://github.com/dathagerty/example ",
            "https://github.com/dathagerty/example/tree/main{/dir} ",
            "https://github.com/dathagerty/example/blob/main{/dir}/{file}#L{line}\"",
        )));
        fs::remove_dir_all(github_dir).unwrap();

        let (unknown_cx, unknown_dir) = layout_context(RequestView::new(
            PageMetadata::for_module(&unknown),
            branding(),
        ));
        let __cx = &unknown_cx;
        let unknown_html = render(view! { root_layout(slot: view! {}.unwrap()) }, &unknown_cx);
        assert!(unknown_html.contains("name=\"go-import\""));
        assert!(!unknown_html.contains("name=\"go-source\""));
        fs::remove_dir_all(unknown_dir).unwrap();
    }

    #[tokio::test]
    async fn footer_has_license_year_and_github_build_marker() {
        let cx = CxTestBuilder::new().build();
        let __cx = &cx;

        let year_before_render = chrono::Utc::now().format("%Y").to_string();
        let html = render(view! { site_footer() }, &cx);
        let year_after_render = chrono::Utc::now().format("%Y").to_string();

        assert!(html.contains("CC BY-SA 4.0"));
        assert!(html.contains("Build commit:"));
        assert!(html.contains("https://github.com/dathagerty/website/commit/"));
        let rendered_year = html
            .split_once(" as of ")
            .and_then(|(_, rest)| rest.split_once('.'))
            .map(|(year, _)| year)
            .expect("footer did not contain its rendered year");
        assert_eq!(rendered_year.len(), 4);
        assert!(rendered_year.bytes().all(|byte| byte.is_ascii_digit()));
        assert!(
            rendered_year == year_before_render || rendered_year == year_after_render,
            "rendered stale footer year {rendered_year}",
        );
    }

    #[tokio::test]
    async fn unknown_footer_commit_is_plain_text() {
        let cx = CxTestBuilder::new().build();
        let __cx = &cx;

        let html = render(view! { build_commit(commit_hash: None) }, &cx);

        assert!(html.contains("Build commit: unknown"));
        assert!(!html.contains("href=\"https://github.com/dathagerty/website/commit/unknown\""));
    }

    #[test]
    fn railway_revision_takes_precedence_over_local_build_info() {
        let railway = "0123456789abcdef0123456789abcdef01234567";

        assert_eq!(
            select_commit_hash(Some(railway), Some("abcdef0")),
            Some(railway)
        );
    }

    #[test]
    fn invalid_railway_revision_falls_back_to_plausible_build_info() {
        assert_eq!(
            select_commit_hash(Some("not-a-commit"), Some("abcdef0")),
            Some("abcdef0")
        );
        assert_eq!(select_commit_hash(Some("1234"), Some("also-invalid")), None);
    }

    #[tokio::test]
    async fn injected_full_revision_renders_an_exact_commit_link() {
        let cx = CxTestBuilder::new().build();
        let __cx = &cx;
        let revision = "0123456789abcdef0123456789abcdef01234567";

        let html = render(view! { build_commit(commit_hash: Some(revision)) }, &cx);

        assert_eq!(
            html,
            concat!(
                "Build commit: <a href=\"https://github.com/dathagerty/website/commit/",
                "0123456789abcdef0123456789abcdef01234567\">",
                "0123456789abcdef0123456789abcdef01234567</a>",
            )
        );
    }

    #[test]
    fn stylesheet_asset_uses_manifest_relative_path_semantics() {
        let expected = topcoat::asset::Asset::new(
            env!("CARGO_CRATE_NAME"),
            file!(),
            "assets/styles.css",
            &topcoat::asset::AssetOptions::NONE,
        );

        assert_eq!(STYLESHEET, expected);
    }

    #[test]
    fn stylesheet_has_palettes_responsive_accessibility_and_flat_css() {
        let css = include_str!("../../assets/styles.css");

        assert!(css.contains("--rosewater-latte: #dc8a78"));
        assert!(css.contains("--rosewater-mocha: #f5e0dc"));
        assert!(css.contains("@media (prefers-color-scheme: dark)"));
        assert!(css.contains("padding-inline:"));
        assert!(css.contains(":focus-visible"));
        assert!(css.contains(".syntax-highlighting"));
        assert!(!css.contains("round("));
        assert!(!css.contains("header {\n    hr {"));
        assert!(!css.contains(".tags {\n    a:"));
    }

    #[test]
    fn latte_normal_text_tokens_meet_wcag_aa_on_mantle() {
        let css = include_str!("../../assets/styles.css");
        let background = css_color(css, "--mantle-latte");
        let normal_tokens = [
            "--text-latte",
            "--link-latte",
            "--visited-latte",
            "--muted-latte",
            "--tag-link-latte",
            "--tag-visited-latte",
        ];

        for token in normal_tokens {
            let ratio = contrast_ratio(css_color(css, token), background);
            assert!(ratio >= 4.5, "{token} contrast was only {ratio:.2}:1");
        }
        for token in ["--visited-latte", "--tag-visited-latte"] {
            let ratio = contrast_ratio(css_color(css, token), background);
            assert!(ratio >= 5.0, "{token} is too close to the AA boundary");
        }
        for mapping in [
            "--link-color: var(--link-latte)",
            "--visited-color: var(--visited-latte)",
            "--muted-color: var(--muted-latte)",
            "--tag-link-color: var(--tag-link-latte)",
            "--tag-visited-color: var(--tag-visited-latte)",
        ] {
            assert!(
                css.contains(mapping),
                "missing active token mapping {mapping}"
            );
        }
    }

    fn css_color(css: &str, token: &str) -> [f64; 3] {
        let prefix = format!("{token}: #");
        let hex = css
            .lines()
            .map(str::trim)
            .find_map(|line| line.strip_prefix(&prefix))
            .and_then(|value| value.strip_suffix(';'))
            .unwrap_or_else(|| panic!("missing hex color token {token}"));
        assert_eq!(hex.len(), 6, "invalid color token {token}");

        [0, 2, 4].map(|offset| {
            f64::from(u8::from_str_radix(&hex[offset..offset + 2], 16).unwrap()) / 255.0
        })
    }

    fn contrast_ratio(foreground: [f64; 3], background: [f64; 3]) -> f64 {
        let foreground = relative_luminance(foreground);
        let background = relative_luminance(background);
        (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05)
    }

    fn relative_luminance(color: [f64; 3]) -> f64 {
        let [red, green, blue] = color.map(|channel| {
            if channel <= 0.04045 {
                channel / 12.92
            } else {
                ((channel + 0.055) / 1.055).powf(2.4)
            }
        });
        0.2126 * red + 0.7152 * green + 0.0722 * blue
    }

    #[tokio::test]
    async fn error_pages_escape_requested_path() {
        let cx = CxTestBuilder::new().build();
        let __cx = &cx;

        let missing_html = render(view! { not_found(path: "/<missing>") }, &cx);
        let internal = render(view! { internal_error() }, &cx);

        assert!(missing_html.contains("/&lt;missing&gt;"));
        assert!(internal.contains("Something went wrong"));
    }
}
