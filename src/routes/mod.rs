mod health;
mod modules;
mod pages;
mod posts;
mod tags;

use std::{
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::{Cx, CxBuilder, app_context, request_context},
    router::{Body, IntoResponse, Next, Response, Router, layer, method, uri},
};

use crate::{
    content::ContentRepository,
    error::AppResult,
    views::{PageMetadata, RequestView, root_layout},
};

#[derive(Debug)]
struct InvalidPathParam;

#[derive(Debug)]
struct SimpleSegment(String);

impl SimpleSegment {
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for SimpleSegment {
    type Err = InvalidPathParam;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        if value.is_empty()
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(InvalidPathParam);
        }
        Ok(Self(value.to_owned()))
    }
}

fn repository(cx: &Cx) -> &Arc<dyn ContentRepository> {
    app_context(cx)
}

async fn install_branding(cx: &Cx) -> AppResult<()> {
    let branding = repository(cx).branding().await?;
    request_context::<RequestView>(cx).set_branding(branding);
    Ok(())
}

fn set_metadata(cx: &Cx, metadata: PageMetadata) {
    request_context::<RequestView>(cx).set_metadata(metadata);
}

#[layer("/")]
async fn request_view_context(cx: &mut CxBuilder, body: Body, next: Next<'_>) -> Result<Response> {
    cx.insert(RequestView::default());
    next.run(cx, body).await
}

#[layer("/")]
async fn request_tracing(cx: &mut CxBuilder, body: Body, next: Next<'_>) -> Result<Response> {
    let started = Instant::now();
    let request_method = method(cx).clone();
    let request_path = uri(cx).path().to_owned();
    let response = next.run(cx, body).await.into_response(cx)?;
    let elapsed = started.elapsed();
    tracing::info!(
        method = %request_method,
        path = %request_path,
        status = response.status().as_u16(),
        elapsed_ms = duration_ms(elapsed),
        "request completed"
    );
    Ok(response)
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

pub fn build_router(repository: Arc<dyn ContentRepository>, asset_bundle: AssetBundle) -> Router {
    Router::builder()
        .layout(root_layout)
        .page(pages::root)
        .page(pages::about)
        .page(pages::reading)
        .page(posts::index)
        .page(posts::post)
        .page(tags::index)
        .page(tags::tag)
        .page(modules::index)
        .page(modules::module)
        .page(pages::fallback)
        .route(health::route_fn())
        .layer(request_view_context)
        .layer(request_tracing)
        .app_context(repository)
        .assets(asset_bundle)
        .build()
}

#[cfg(test)]
mod tests {
    use std::{
        fmt::Write,
        fs,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use async_trait::async_trait;
    use chrono::NaiveDate;
    use sha2::{Digest, Sha256};
    use tempfile::TempDir;
    use topcoat::{
        asset::{AssetBundle, MANIFEST_VERSION, Manifest, ManifestEntry},
        router::{Body, Method, Request, Response, StatusCode, to_bytes},
    };

    use super::*;
    use crate::{
        content::{Branding, ContentError, ContentRepository, GoModule, Page, Post, TagSummary},
        views::STYLESHEET,
    };

    const APPLICATION_STYLESHEET: &[u8] = include_bytes!("../../assets/styles.css");

    fn sha256_hex(bytes: &[u8]) -> String {
        let digest = Sha256::digest(bytes);
        let mut hash = String::with_capacity(digest.len() * 2);
        for byte in digest {
            write!(hash, "{byte:02x}").unwrap();
        }
        hash
    }

    struct TestBundle {
        bundle: AssetBundle,
        dir: TempDir,
        asset_path: String,
    }

    impl TestBundle {
        fn new() -> Self {
            let dir = TempDir::new().unwrap();
            let hash = sha256_hex(APPLICATION_STYLESHEET);
            let file = format!("styles-{}.css", &hash[..16]);
            fs::write(dir.path().join(&file), APPLICATION_STYLESHEET).unwrap();
            Manifest {
                version: MANIFEST_VERSION,
                assets: vec![ManifestEntry {
                    id: STYLESHEET,
                    file: file.clone(),
                    hash,
                    content_type: "text/css".to_owned(),
                }],
            }
            .save(dir.path().join("manifest.toml"))
            .unwrap();

            Self {
                bundle: AssetBundle::load_dir(dir.path()).unwrap(),
                dir,
                asset_path: format!("/_topcoat/assets/{file}"),
            }
        }
    }

    #[derive(Clone)]
    struct FixtureRepository {
        fail_branding: bool,
        fail_page: bool,
        missing_pages: bool,
        branding_calls: Option<Arc<AtomicUsize>>,
        page_calls: Option<Arc<AtomicUsize>>,
    }

    impl FixtureRepository {
        fn ok() -> Self {
            Self {
                fail_branding: false,
                fail_page: false,
                missing_pages: false,
                branding_calls: None,
                page_calls: None,
            }
        }

        fn branding_failure() -> Self {
            Self {
                fail_branding: true,
                fail_page: false,
                missing_pages: false,
                branding_calls: None,
                page_calls: None,
            }
        }

        fn page_failure() -> (Self, Arc<AtomicUsize>, Arc<AtomicUsize>) {
            let branding_calls = Arc::new(AtomicUsize::new(0));
            let page_calls = Arc::new(AtomicUsize::new(0));
            (
                Self {
                    fail_branding: false,
                    fail_page: true,
                    missing_pages: false,
                    branding_calls: Some(branding_calls.clone()),
                    page_calls: Some(page_calls.clone()),
                },
                branding_calls,
                page_calls,
            )
        }

        fn content_error<T>() -> Result<T, ContentError> {
            Err(ContentError::MissingData)
        }

        fn successful<T>(value: T) -> Result<T, ContentError> {
            Ok(value)
        }

        fn without_pages() -> Self {
            Self {
                fail_branding: false,
                fail_page: false,
                missing_pages: true,
                branding_calls: None,
                page_calls: None,
            }
        }

        fn counting_branding() -> (Self, Arc<AtomicUsize>) {
            let calls = Arc::new(AtomicUsize::new(0));
            (
                Self {
                    fail_branding: false,
                    fail_page: false,
                    missing_pages: false,
                    branding_calls: Some(calls.clone()),
                    page_calls: None,
                },
                calls,
            )
        }

        fn post() -> Post {
            Post {
                slug: "hello".to_owned(),
                title: "A True Hello World".to_owned(),
                publish_date: NaiveDate::from_ymd_opt(2024, 10, 14).unwrap(),
                last_edit: None,
                kind: Some("post".to_owned()),
                draft: false,
                summary: Some("A tour of this blog".to_owned()),
                tags: vec!["rust/web".to_owned()],
                html: "<p>Hello from the fixture.</p>".to_owned(),
            }
        }

        fn module() -> GoModule {
            Self::module_named("example")
        }

        fn module_named(name: &str) -> GoModule {
            GoModule {
                repository_url: format!("https://github.com/dathagerty/{name}"),
                module_path: format!("dathagerty.com/go/{name}"),
                name: name.to_owned(),
                description: format!("The {name} module."),
                license: "MIT".to_owned(),
            }
        }
    }

    #[async_trait]
    impl ContentRepository for FixtureRepository {
        async fn page(&self, slug: &str) -> Result<Option<Page>, ContentError> {
            if let Some(calls) = &self.page_calls {
                calls.fetch_add(1, Ordering::Relaxed);
            }
            if self.fail_page {
                return Self::content_error();
            }
            let page = if self.missing_pages {
                None
            } else {
                match slug {
                    "root" => Some(("dathagerty.com", "Root fixture")),
                    "about" => Some(("About", "About fixture")),
                    "reading" => Some(("Reading", "Reading fixture")),
                    _ => None,
                }
            }
            .map(|(title, body)| Page {
                slug: slug.to_owned(),
                title: title.to_owned(),
                kind: Some("page".to_owned()),
                last_edit: None,
                html: format!("<p>{body}</p>"),
            });
            Self::successful(page)
        }

        async fn post(&self, slug: &str) -> Result<Option<Post>, ContentError> {
            Self::successful((slug == "hello").then(Self::post))
        }

        async fn posts(&self) -> Result<Vec<Post>, ContentError> {
            Self::successful(vec![Self::post()])
        }

        async fn posts_by_year(&self) -> Result<Vec<(i32, Vec<Post>)>, ContentError> {
            Self::successful(vec![(2024, vec![Self::post()])])
        }

        async fn tags(&self) -> Result<Vec<TagSummary>, ContentError> {
            Self::successful(vec![TagSummary {
                name: "rust/web".to_owned(),
                post_count: 1,
            }])
        }

        async fn posts_for_tag(&self, tag: &str) -> Result<Option<Vec<Post>>, ContentError> {
            Self::successful((tag == "rust/web").then(|| vec![Self::post()]))
        }

        async fn modules(&self) -> Result<Vec<GoModule>, ContentError> {
            Self::successful(vec![Self::module(), Self::module_named("example.v2~beta")])
        }

        async fn module(&self, name: &str) -> Result<Option<GoModule>, ContentError> {
            Self::successful(
                ["example", "example.v2~beta"]
                    .contains(&name)
                    .then(|| Self::module_named(name)),
            )
        }

        async fn branding(&self) -> Result<Branding, ContentError> {
            if let Some(calls) = &self.branding_calls {
                calls.fetch_add(1, Ordering::Relaxed);
            }
            if self.fail_branding {
                return Self::content_error();
            }
            Self::successful(Branding {
                word: "deliriums".to_owned(),
                slogan: "a little rusty".to_owned(),
            })
        }
    }

    struct TestResponse {
        response: Response,
        body: String,
    }

    async fn send(router: &topcoat::router::Router, method: Method, path: &str) -> TestResponse {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .body(Body::empty())
            .unwrap();
        let response = router.handle(request).await;
        let (parts, body) = response.into_parts();
        let body = to_bytes(body, usize::MAX).await.unwrap();
        TestResponse {
            response: Response::from_parts(parts, Body::empty()),
            body: String::from_utf8(body.to_vec()).unwrap(),
        }
    }

    fn assert_html(response: &TestResponse, status: StatusCode) {
        assert_eq!(response.response.status(), status);
        assert_eq!(
            response.response.headers()["content-type"],
            "text/html; charset=utf-8"
        );
        assert!(response.body.starts_with("<!DOCTYPE html>"));
        assert!(response.body.contains("href=\"/about\""));
        assert!(response.body.contains("david's deliriums"));
    }

    fn test_router(
        repository: impl ContentRepository + 'static,
        bundle: AssetBundle,
    ) -> topcoat::router::Router {
        build_router(Arc::new(repository), bundle)
    }

    #[tokio::test]
    async fn every_page_route_renders_html_with_expected_content() {
        let assets = TestBundle::new();
        let router = test_router(FixtureRepository::ok(), assets.bundle.clone());
        let cases = [
            ("/", "<title>dathagerty.com</title>", "Root fixture"),
            (
                "/about",
                "<title>About | dathagerty.com</title>",
                "About fixture",
            ),
            (
                "/reading",
                "<title>Reading | dathagerty.com</title>",
                "Reading fixture",
            ),
            (
                "/blag",
                "<title>Blag | dathagerty.com</title>",
                "A True Hello World",
            ),
            (
                "/blag/hello",
                "<title>A True Hello World | dathagerty.com</title>",
                "Hello from the fixture",
            ),
            (
                "/tags",
                "<title>Tags | dathagerty.com</title>",
                "rust/web (1)",
            ),
            (
                "/tags/rust%2Fweb",
                "<title>rust/web | dathagerty.com</title>",
                "Posts tagged rust/web",
            ),
            (
                "/go",
                "<title>Go Modules | dathagerty.com</title>",
                "example",
            ),
        ];

        for (path, title, content) in cases {
            let response = send(&router, Method::GET, path).await;
            assert_html(&response, StatusCode::OK);
            assert!(response.body.contains(title), "missing {title:?} at {path}");
            assert!(
                response.body.contains(content),
                "missing {content:?} at {path}"
            );
        }
    }

    #[tokio::test]
    async fn homepage_uses_a_meaningful_metadata_description() {
        let assets = TestBundle::new();
        let router = test_router(FixtureRepository::ok(), assets.bundle.clone());

        let response = send(&router, Method::GET, "/").await;

        assert!(response.body.contains(concat!(
            "<meta name=\"description\" content=\"",
            "David Hagerty's personal website and assorted ramblings.\">",
        )));
        assert!(!response.body.contains("dathagerty.com on dathagerty.com"));
    }

    #[tokio::test]
    async fn module_route_uses_name_lookup_and_emits_go_discovery_metadata() {
        let assets = TestBundle::new();
        let router = test_router(FixtureRepository::ok(), assets.bundle.clone());

        let response = send(&router, Method::GET, "/go/example").await;

        assert_html(&response, StatusCode::OK);
        assert!(
            response
                .body
                .contains("<title>example | dathagerty.com</title>")
        );
        assert!(response.body.contains(concat!(
            "name=\"go-import\" content=\"dathagerty.com/go/example git ",
            "https://github.com/dathagerty/example\""
        )));
        assert!(response.body.contains("name=\"go-source\""));
        assert!(response.body.contains("go get dathagerty.com/go/example"));
    }

    #[tokio::test]
    async fn module_names_with_dots_and_tildes_reach_repository_lookup() {
        let assets = TestBundle::new();
        let router = test_router(FixtureRepository::ok(), assets.bundle.clone());

        let index = send(&router, Method::GET, "/go").await;
        assert!(index.body.contains("href=\"/go/example.v2~beta\""));

        let response = send(&router, Method::GET, "/go/example.v2~beta").await;
        assert_html(&response, StatusCode::OK);
        assert!(
            response
                .body
                .contains("<title>example.v2~beta | dathagerty.com</title>")
        );
        assert!(
            response
                .body
                .contains("go get dathagerty.com/go/example.v2~beta")
        );
    }

    #[tokio::test]
    async fn missing_content_returns_public_html_404_pages() {
        let assets = TestBundle::new();
        let router = test_router(FixtureRepository::ok(), assets.bundle.clone());

        for path in ["/blag/missing", "/tags/missing", "/go/missing"] {
            let response = send(&router, Method::GET, path).await;
            assert_html(&response, StatusCode::NOT_FOUND);
            assert!(response.body.contains("Not all who wander are lost"));
            assert!(response.body.contains(path));
        }

        let assets = TestBundle::new();
        let router = test_router(FixtureRepository::without_pages(), assets.bundle.clone());
        let response = send(&router, Method::GET, "/about").await;
        assert_html(&response, StatusCode::NOT_FOUND);
        assert!(response.body.contains("Not all who wander are lost"));
    }

    #[tokio::test]
    async fn branding_is_selected_once_for_a_rendered_not_found_page() {
        let assets = TestBundle::new();
        let (repository, branding_calls) = FixtureRepository::counting_branding();
        let router = test_router(repository, assets.bundle.clone());

        let response = send(&router, Method::GET, "/blag/missing").await;

        assert_html(&response, StatusCode::NOT_FOUND);
        assert_eq!(branding_calls.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn malformed_typed_parameters_return_404() {
        let assets = TestBundle::new();
        let router = test_router(FixtureRepository::ok(), assets.bundle.clone());

        for path in [
            "/blag/%2F",
            "/tags/%00",
            "/go/dathagerty.com%2Fgo%2Fexample",
            "/go/.leading-dot",
            "/go/trailing-dot.",
            "/go/bad%2Bname",
        ] {
            let response = send(&router, Method::GET, path).await;
            assert_html(&response, StatusCode::NOT_FOUND);
        }
    }

    #[tokio::test]
    async fn malformed_typed_parameters_do_not_depend_on_branding_repository() {
        let assets = TestBundle::new();
        let router = test_router(FixtureRepository::branding_failure(), assets.bundle.clone());

        for path in ["/blag/%2F", "/tags/%00", "/go/.invalid"] {
            let response = send(&router, Method::GET, path).await;
            assert_html(&response, StatusCode::NOT_FOUND);
            assert!(response.body.contains("Not all who wander are lost"));
        }
    }

    #[tokio::test]
    async fn unknown_get_renders_branded_html_not_found_page() {
        let assets = TestBundle::new();
        let (repository, branding_calls) = FixtureRepository::counting_branding();
        let router = test_router(repository, assets.bundle.clone());

        let response = send(&router, Method::GET, "/missing/nested-page").await;

        assert_html(&response, StatusCode::NOT_FOUND);
        assert!(
            response
                .body
                .contains("<title>Not Found | dathagerty.com</title>")
        );
        assert!(response.body.contains("Not all who wander are lost"));
        assert!(response.body.contains("/missing/nested-page"));
        assert_eq!(branding_calls.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn page_and_unknown_post_requests_are_method_not_allowed() {
        let assets = TestBundle::new();
        let router = test_router(FixtureRepository::ok(), assets.bundle.clone());

        let response = send(&router, Method::POST, "/about").await;
        assert_eq!(response.response.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert!(
            response.response.headers()["allow"]
                .to_str()
                .unwrap()
                .contains("GET")
        );

        let response = send(&router, Method::POST, "/missing/nested-page").await;
        assert_eq!(response.response.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert!(
            response.response.headers()["allow"]
                .to_str()
                .unwrap()
                .contains("GET")
        );
    }

    #[tokio::test]
    async fn health_is_plain_text_without_the_layout() {
        let assets = TestBundle::new();
        let router = test_router(FixtureRepository::ok(), assets.bundle.clone());

        let response = send(&router, Method::GET, "/healthz").await;

        assert_eq!(response.response.status(), StatusCode::OK);
        assert_eq!(
            response.response.headers()["content-type"],
            "text/plain; charset=utf-8"
        );
        assert_eq!(response.body, "ok");
        assert!(!response.body.contains("<!DOCTYPE html>"));
    }

    #[tokio::test]
    async fn repository_errors_return_generic_html_500_without_internal_details() {
        let assets = TestBundle::new();
        let (repository, branding_calls, page_calls) = FixtureRepository::page_failure();
        let router = test_router(repository, assets.bundle.clone());

        let response = send(&router, Method::GET, "/about").await;

        assert_html(&response, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(branding_calls.load(Ordering::Relaxed), 1);
        assert_eq!(page_calls.load(Ordering::Relaxed), 1);
        assert!(response.body.contains("Something went wrong"));
        assert!(!response.body.contains("data.json"));
        assert!(!response.body.contains("MissingData"));
    }

    #[tokio::test]
    async fn application_stylesheet_resolves_through_valid_content_addressed_bundle() {
        let assets = TestBundle::new();
        let expected_hash = sha256_hex(APPLICATION_STYLESHEET);
        let manifest = Manifest::load(assets.dir.path().join("manifest.toml")).unwrap();
        assert_eq!(manifest.assets.len(), 1);
        assert_eq!(manifest.assets[0].hash, expected_hash);
        let digest_prefix = assets
            .asset_path
            .strip_prefix("/_topcoat/assets/styles-")
            .and_then(|path| path.strip_suffix(".css"))
            .expect("stylesheet URL must use Topcoat's content-addressed filename shape");
        assert_eq!(digest_prefix.len(), 16);
        assert_eq!(digest_prefix, &expected_hash[..16]);
        assert!(
            digest_prefix
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        );
        let router = test_router(FixtureRepository::ok(), assets.bundle.clone());
        let page = send(&router, Method::GET, "/").await;

        assert!(
            page.body
                .contains(&format!("href=\"{}\"", assets.asset_path))
        );

        let stylesheet = send(&router, Method::GET, &assets.asset_path).await;
        assert_eq!(stylesheet.response.status(), StatusCode::OK);
        assert_eq!(stylesheet.response.headers()["content-type"], "text/css");
        assert_eq!(stylesheet.body.as_bytes(), APPLICATION_STYLESHEET);
        assert_eq!(
            stylesheet.response.headers()["cache-control"],
            "public, max-age=31536000, immutable"
        );
    }
}
