# Topcoat Site Port Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the Axum/Maud placeholder with a Railway-ready Topcoat site that reproduces the old site's design, embedded content, blog, tags, and Go vanity-module behavior behind a database-ready repository boundary.

**Architecture:** Topcoat owns serving, explicit routing, the root layout, components, and content-hashed assets. An asynchronous `ContentRepository` trait separates handlers from an immutable embedded implementation that validates Markdown and JSON at startup; a future PostgreSQL implementation can satisfy the same query contract. The binary reads Railway's `PORT`, binds on all interfaces, and exposes a health route.

**Tech Stack:** Rust 1.95, Topcoat 0.1.3, Tokio, Comrak, rust-embed, serde/serde_json/serde-yaml-ng, chrono, rand, async-trait, thiserror, tracing, build-info, Topcoat CLI 0.1.3, Docker, Railway.

**Note:** Commit steps are intentionally omitted because no commit was requested. Keep each task's changes isolated and reviewable; create Conventional Commits only if the user explicitly asks.

### Task 1: Pin Tooling And Establish The Library Boundary

**Files:**
- Create: `rust-toolchain.toml`
- Create: `src/lib.rs`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `mise.toml`
- Test: `src/lib.rs`

**Step 1: Write the failing configuration test**

Add a test in `src/lib.rs` that removes `PORT`, calls `server_port()`, and asserts `3000`. Add a second test that sets `PORT=8080` and asserts `8080`, serializing environment mutation with a process-local mutex.

```rust
#[test]
fn server_port_defaults_to_3000() {
    let _guard = ENV_LOCK.lock().unwrap();
    unsafe { std::env::remove_var("PORT") };
    assert_eq!(server_port().unwrap(), 3000);
}
```

**Step 2: Run the test to verify it fails**

Run: `cargo test server_port_defaults_to_3000 --lib`

Expected: FAIL because `src/lib.rs` or `server_port` does not exist.

**Step 3: Replace obsolete dependencies and add the module skeleton**

Pin Topcoat exactly and remove direct Axum, Maud, `config`, and the old `markdown` dependency. Add only dependencies used by the planned implementation:

```toml
rust-version = "1.95"

[dependencies]
async-trait = "0.1"
build-info = "0.0.40"
chrono = { version = "0.4", features = ["serde"] }
comrak = { version = "0.48", features = ["syntect"] }
rand = "0.9"
rust-embed = "8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml_ng = "0.10"
thiserror = "2"
tokio = { version = "1", features = ["macros", "net", "rt-multi-thread", "signal"] }
topcoat = "=0.1.3"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```

Create `rust-toolchain.toml` with channel `1.95.0`. Pin `cargo:topcoat-cli` to `0.1.3` in `mise.toml`. Create `src/lib.rs` declaring `content`, `error`, `routes`, and `views`, plus a `server_port() -> Result<u16, ConfigError>` that defaults to `3000` and rejects malformed values.

**Step 4: Run focused and full tests**

Run: `cargo test server_port --lib`

Expected: PASS for default, explicit, and malformed port cases.

Run: `cargo check --all-targets`

Expected: PASS after temporary empty module files exist; no Axum/Maud imports remain in library code.

### Task 2: Define Content Models And Markdown Parsing

**Files:**
- Create: `src/content/mod.rs`
- Create: `src/content/model.rs`
- Create: `src/content/markdown.rs`
- Test: `src/content/markdown.rs`

**Step 1: Write failing parser tests**

Cover a valid page, a valid post, invalid YAML, a missing post publication date, and rendered GFM features. Assert that filename-derived slugs are used and raw trusted HTML survives rendering.

```rust
#[test]
fn parses_post_frontmatter_and_markdown() {
    let source = "---\ntitle: Hello\npublishDate: 2025-07-25\ntags: [rust]\n---\n# Hi";
    let post = parse_post("hello.md", source).unwrap();
    assert_eq!(post.slug, "hello");
    assert_eq!(post.title, "Hello");
    assert!(post.html.contains("<h1"));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test content::markdown --lib`

Expected: FAIL because models and parser functions do not exist.

**Step 3: Implement typed models and parser**

Define `Page`, `Post`, `TagSummary`, `GoModule`, and `Branding` as owned, cloneable models. Use `chrono::NaiveDate` for publication/edit dates. Keep both rendered HTML and source-derived metadata in the repository models; views must not parse Markdown.

Split YAML frontmatter strictly, deserialize with `serde_yaml_ng`, and render Markdown with one shared Comrak options/plugins configuration supporting GFM, heading IDs, fenced code, syntax highlighting, and trusted raw HTML. Reject non-`.md` names, empty slugs, malformed dates, and missing required post fields with path-aware `ContentError` variants.

**Step 4: Run parser tests**

Run: `cargo test content::markdown --lib`

Expected: PASS for valid documents and exact error variants for invalid documents.

### Task 3: Build The Embedded Repository Contract

**Files:**
- Create: `src/content/repository.rs`
- Create: `src/content/embedded.rs`
- Create: `content/pages/root.md`
- Create: `content/pages/about.md`
- Create: `content/pages/reading.md`
- Create: `content/posts/hello.md`
- Create: `content/posts/how-this-works.md`
- Create: `content/data.json`
- Delete: `content/.keep`
- Test: `src/content/embedded.rs`

**Step 1: Write failing repository contract tests**

Test required-page validation, duplicate-slug rejection, malformed module metadata, production draft exclusion, development draft inclusion, newest-first post ordering, descending year groups, alphabetical tag summaries, tag lookup, module lookup, and non-empty branding selection.

```rust
#[tokio::test]
async fn published_posts_are_newest_first() {
    let repository = fixture_repository(false).unwrap();
    let posts = repository.posts().await.unwrap();
    assert!(posts.windows(2).all(|pair| pair[0].publish_date >= pair[1].publish_date));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test content::embedded --lib`

Expected: FAIL because the trait and embedded implementation do not exist.

**Step 3: Define the asynchronous query boundary**

Use `async_trait` to make the trait object-safe and register it later as `Arc<dyn ContentRepository>`:

```rust
#[async_trait]
pub trait ContentRepository: Send + Sync {
    async fn page(&self, slug: &str) -> Result<Option<Page>, ContentError>;
    async fn post(&self, slug: &str) -> Result<Option<Post>, ContentError>;
    async fn posts(&self) -> Result<Vec<Post>, ContentError>;
    async fn posts_by_year(&self) -> Result<Vec<(i32, Vec<Post>)>, ContentError>;
    async fn tags(&self) -> Result<Vec<TagSummary>, ContentError>;
    async fn posts_for_tag(&self, tag: &str) -> Result<Option<Vec<Post>>, ContentError>;
    async fn modules(&self) -> Result<Vec<GoModule>, ContentError>;
    async fn module(&self, name: &str) -> Result<Option<GoModule>, ContentError>;
    async fn branding(&self) -> Result<Branding, ContentError>;
}
```

**Step 4: Implement immutable embedded indexes**

Use `rust-embed` to include `content/pages`, `content/posts`, and `content/data.json`. Parse every file during `EmbeddedContentRepository::load(include_drafts)`. Fail the entire load on any invalid file. Validate required pages (`root`, `about`, `reading`), unique slugs, non-empty branding collections, and complete Go module metadata. Precompute deterministic post/year/tag/module indexes.

Port the old Markdown and JSON content, correcting the old `ModuleName` JSON mismatch and stale links while preserving the site's voice.

**Step 5: Run repository tests**

Run: `cargo test content --lib`

Expected: PASS with no ordering dependent on hash-map iteration.

### Task 4: Create Topcoat Layout, Components, And CSS Asset

**Files:**
- Create: `src/views/mod.rs`
- Create: `src/views/layout.rs`
- Create: `src/views/components.rs`
- Create: `src/views/pages.rs`
- Create: `assets/styles.css`
- Test: `src/views/mod.rs`

**Step 1: Write failing component and CSS tests**

Use `CxTestBuilder` and `View::render` to assert escaped dynamic text, navigation links, the `§` separator, footer build information, post dates/tags, and Go metadata. Read `assets/styles.css` in a unit test and assert both Latte and Mocha variables, responsive padding, and `prefers-color-scheme: dark` exist.

**Step 2: Run tests to verify they fail**

Run: `cargo test views --lib`

Expected: FAIL because view modules and stylesheet do not exist.

**Step 3: Implement the Topcoat view layer**

Declare the stylesheet with `asset!("./assets/styles.css")`. Implement a root `#[layout("/")]` containing a valid doctype, `<html lang="en">`, head metadata, favicon, stylesheet, header, main landmark, and footer. Build focused `#[component]` functions for identity links, post metadata, post lists, tags, and module instructions.

Use explicit Topcoat raw-HTML rendering only for HTML produced from trusted embedded Markdown. Keep all metadata and user-visible dynamic strings escaped by `view!`.

Port the Catppuccin design while flattening nested CSS, replacing unsupported `round()` sizing, adding mobile padding and `:focus-visible`, and fixing dark-mode rule borders. Do not add JavaScript or external fonts.

**Step 4: Format views with Topcoat and run tests**

Run: `topcoat fmt src`

Run: `cargo test views --lib`

Expected: PASS; generated HTML has one complete document shell and CSS tests confirm both color schemes.

### Task 5: Implement Explicit Topcoat Routes And Error Responses

**Files:**
- Create: `src/error.rs`
- Create: `src/routes/mod.rs`
- Create: `src/routes/pages.rs`
- Create: `src/routes/posts.rs`
- Create: `src/routes/tags.rs`
- Create: `src/routes/modules.rs`
- Create: `src/routes/health.rs`
- Test: `src/routes/mod.rs`

**Step 1: Write failing router tests**

Build the application router with a fixture repository and issue in-memory requests. Cover every route, HTML content type, a known post, missing post/tag/module 404s, POST-to-page 405, `/healthz`, route-specific titles, and Go discovery metadata.

```rust
#[tokio::test]
async fn missing_post_returns_404() {
    let response = test_router().handle(request("/blag/missing")).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test routes --lib`

Expected: FAIL because route registration and handlers do not exist.

**Step 3: Implement application errors and typed path parameters**

Define an `AppError` that converts not-found errors to a public 404 view and unexpected repository errors to a generic 500 view while logging the source error. Declare Topcoat `#[path_param]` types for post slugs, tags, and Go module names, with parse failures mapped to not found.

**Step 4: Implement thin handlers and explicit registration**

Each `#[page]` handler should obtain `Arc<dyn ContentRepository>` from app context, execute one repository query, and pass the result to a view component. Register pages and the root layout explicitly on `Router::builder()`; do not use discovery. Register `/healthz` as a simple GET route returning `text/plain` and `200 OK`.

Install the matching Topcoat asset bundle through the router builder. Add one request-tracing layer recording method, path, status, and elapsed time.

**Step 5: Run route and library tests**

Run: `cargo test routes --lib`

Expected: PASS for all success, 404, 405, and metadata cases.

Run: `cargo test --lib`

Expected: PASS for parser, repository, views, and router tests together.

### Task 6: Replace The Binary And Add Graceful Railway Startup

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/lib.rs`
- Replace: `src/main.rs`
- Delete: `src/config.rs`
- Delete: `src/templates.rs`
- Delete: `src/flair/mod.rs`
- Test: `src/lib.rs`

**Step 1: Write a failing application-construction test**

Add a test that calls `build_router(repository, asset_bundle)` and verifies `/healthz` without binding a socket. Add configuration tests for `INCLUDE_DRAFTS` defaulting false and accepting only an explicit true value.

**Step 2: Run the test to verify it fails**

Run: `cargo test build_router --lib`

Expected: FAIL because the composition function is not complete.

**Step 3: Implement the composition root**

Expose a library `build_router` function so tests and the binary use exactly the same registration. In `main.rs`, initialize JSON tracing with `RUST_LOG` support, load and validate embedded content, load the Topcoat asset bundle, bind `0.0.0.0:$PORT`, log startup, and call Topcoat's listener-based serve API.

Remove `autobins = false` from `Cargo.toml` so Cargo discovers the replacement `src/main.rs` binary.

Install graceful shutdown for Ctrl-C and Unix terminate signals using Tokio. Return errors from `main` rather than panicking or silently falling back from invalid configuration.

Remove all Axum, Maud, and old configuration/template modules.

**Step 4: Run all tests and a local smoke test**

Run: `cargo test --all-targets`

Expected: PASS.

Run: `topcoat asset bundle --release`

Expected: a content-addressed bundle containing `styles.css` is generated for the release binary.

Run: `PORT=3100 cargo run`

Expected: the server binds `0.0.0.0:3100`; `/healthz` returns `200`, and `/` renders with a hashed stylesheet URL. Stop the process cleanly with SIGINT.

### Task 7: Add Deterministic Railway Packaging

**Files:**
- Create: `.dockerignore`
- Create: `Dockerfile`
- Create: `railway.json`
- Modify: `README.md`

**Step 1: Write deployment acceptance criteria before the Dockerfile**

Document in `README.md` that the image must contain the release binary and the exact Topcoat asset bundle generated from it, listen on Railway's `PORT`, and serve `/healthz` without a writable filesystem.

**Step 2: Build the multi-stage image**

Use a Rust 1.95 build stage. Install `topcoat-cli` exactly at `0.1.3` with `--locked`, compile the release binary, and generate the release asset bundle. Copy only the binary, matching assets, CA certificates if needed, and required runtime files into a small non-root runtime image.

Configure Railway to build from `Dockerfile`, start the image entrypoint, and health-check `/healthz` with a reasonable timeout and restart policy.

**Step 3: Verify the image**

Run: `docker build -t dathagerty:topcoat .`

Expected: PASS with no version drift between Topcoat library and CLI.

Run: `docker run --rm -d --name dathagerty-smoke -e PORT=3000 -p 3300:3000 dathagerty:topcoat`

Run: `curl --fail http://127.0.0.1:3300/healthz`

Expected: `ok` and HTTP 200.

Run: `docker stop dathagerty-smoke`

Expected: the container exits cleanly.

### Task 8: Final Quality And Regression Verification

**Files:**
- Modify only files required by verification findings.

**Step 1: Format all source**

Run: `topcoat fmt src`

Run: `cargo fmt --all -- --check`

Expected: PASS with no changes required after the final formatting run.

**Step 2: Run compiler and linter gates**

Run: `cargo check --locked --all-targets --all-features`

Run: `cargo clippy --locked --all-targets --all-features -- -D warnings`

Expected: PASS with no warnings.

**Step 3: Run all tests**

Run: `cargo test --locked --all-targets --all-features`

Expected: PASS with non-zero parser, repository, view, and route test counts.

**Step 4: Verify production artifacts**

Run: `cargo build --locked --release`

Run: `topcoat asset bundle --release`

Run: `docker build -t dathagerty:topcoat .`

Expected: all commands pass, and the bundle includes the content-hashed stylesheet.

**Step 5: Inspect the final diff**

Run: `git status --short`

Run: `git diff --check`

Run: `git diff --stat`

Expected: no whitespace errors, no generated `target` files, no accidental secrets, and only the intended application, content, documentation, and deployment changes.

**Step 6: Request code review**

Use `superpowers:requesting-code-review` with the design and this plan as requirements. Resolve high-confidence findings, then repeat Steps 1-5 before reporting completion.
