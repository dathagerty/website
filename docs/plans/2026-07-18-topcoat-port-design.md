# Topcoat Site Port Design

## Goal

Replace the existing Axum and Maud prototype with a Topcoat-native personal site that reproduces the style and public features of `dathagerty.old/improvements`. Keep content embedded for the initial deployment while creating a narrow storage boundary that can later support Railway PostgreSQL.

## Architecture

The application will use Topcoat for routing, layouts, views, components, application context, assets, and serving. The Axum and Maud implementations will be removed rather than maintained as a compatibility layer.

A `ContentRepository` trait will expose only the queries required by the public site:

- Fetch a page by slug.
- Fetch a post by slug.
- List published posts grouped by year.
- List tags and posts associated with a tag.
- List Go vanity modules and fetch one by module path.
- Select branding words and slogans.

The trait will be asynchronous so a future PostgreSQL implementation can perform I/O without changing handlers. The initial `EmbeddedContentRepository` will parse Markdown and JSON included in the binary, validate them at startup, and construct immutable indexes. It will be installed in Topcoat's application context. Route handlers and views will not depend on the embedded representation.

The boundary will not include speculative CRUD operations, database IDs, pagination, caching, or an admin interface.

Startup will:

1. Load environment configuration.
2. Parse and validate embedded content.
3. Construct the content repository.
4. Load the Topcoat asset bundle.
5. Build an explicit router.
6. Bind to `0.0.0.0:$PORT`, using port `3000` locally.
7. Serve until graceful shutdown.

The existing `config.json` mechanism will be removed. A later PostgreSQL implementation may use `DATABASE_URL`, but the initial application will not add an unused storage-selection mechanism.

## Routes

The first release will provide:

| Route | Behavior |
| --- | --- |
| `/` | Render the root Markdown page. |
| `/about` | Render the about Markdown page. |
| `/reading` | Render the reading Markdown page. |
| `/blag` | List published posts, newest first and grouped by year. |
| `/blag/{slug}` | Render a published post. |
| `/tags` | List tags and post counts. |
| `/tags/{tag}` | List matching posts, newest first. |
| `/go` | List Go vanity modules. |
| `/go/{module}` | Render module instructions and Go discovery metadata. |
| `/healthz` | Return a lightweight health response for Railway. |

Missing pages, posts, tags, and modules will return HTTP 404. Unsupported methods will retain Topcoat's HTTP 405 behavior.

Routes will be registered explicitly. This is easier to audit than link-time discovery and avoids unnecessary reliance on inventory ordering in an experimental framework.

## Rendering

A root Topcoat layout will own the complete HTML document, including metadata, the stylesheet, header, navigation, and footer. Reusable components will render identity links, post metadata, tag links, post lists, Go-module metadata, and common page structures.

Handlers will be thin. Each handler will query the repository, map missing content to a 404, and pass a typed view model to components. Views will not parse content or perform storage operations.

Markdown will support:

- GitHub-Flavored Markdown.
- YAML frontmatter.
- Filename-based slugs.
- Automatic heading IDs.
- Fenced-code syntax highlighting.
- Raw HTML for trusted, author-controlled content.
- Draft filtering controlled by an explicit development setting.

Posts will sort deterministically by publication date and then slug. Derived year and tag indexes will not depend on map iteration order.

## Visual Design

The port will preserve the old site's intentional minimal and brutalist style:

- Catppuccin Latte and Mocha palettes selected by system color preference.
- A centered, narrow reading column.
- System sans-serif body text and monospace header/footer text.
- Centered navigation and a double-rule separator with a `§` ornament.
- Sapphire links, mauve visited links, italic hover treatment, and subdued metadata.
- Bordered code blocks and server-rendered syntax highlighting.
- A skull SVG data-URI favicon.
- A randomly selected site word and slogan for each request.

The old CSS will be corrected rather than copied literally. The new stylesheet will use responsive horizontal padding, broadly supported width rules, flat selectors, valid dark-mode borders, and accessible focus styles. Documents will include `<html lang="en">`, valid metadata, and route-specific titles and descriptions.

The stylesheet will be managed through Topcoat's content-hashed asset bundle. Railway's deployment image must contain the asset bundle generated from the same build as the executable.

## Errors And Observability

Content validation will finish before the server accepts traffic. Startup will fail with a contextual error for malformed frontmatter, invalid dates, duplicate slugs, missing required pages, or invalid module data. Content will never be silently skipped.

A small application error type will map missing content to 404 and unexpected repository or rendering failures to 500. Internal errors will be logged without exposing details in public responses.

Structured tracing will cover startup, request method, path, status, duration, and repository failures. Graceful shutdown will respond to platform termination signals and stop accepting new requests before exiting.

## Testing

Tests will cover stable behavior rather than snapshotting entire documents:

- Markdown parsing and startup validation.
- Draft filtering and deterministic ordering.
- Year grouping and tag indexes.
- Embedded repository contract behavior.
- Route status codes and content types.
- Expected titles, navigation, post metadata, and Go module discovery tags.
- Correct 404 and 405 behavior.
- Stylesheet resolution and required light/dark palette declarations.

Router tests will invoke Topcoat directly without opening a TCP listener.

## Railway Deployment

The project will pin Rust 1.95 and Topcoat library and CLI version 0.1.3 together because Topcoat is early-stage and changing rapidly.

Railway will build a multi-stage Docker image. The build stage will compile the release binary and generate the Topcoat asset bundle. The runtime stage will include the binary and its matching bundle, bind to `0.0.0.0:$PORT`, require no writable filesystem, and expose `/healthz` for Railway health checks.

Embedded content keeps deployments atomic: code, content, and assets are released together.

## Future PostgreSQL Storage

A later `PostgresContentRepository` will implement the same content queries using Railway PostgreSQL. It will add migrations, a one-time importer or seed process for the Markdown and JSON content, and repository contract tests shared with the embedded implementation.

Adding PostgreSQL must not require changes to route definitions, rendering components, or content-facing view models. Database selection and `DATABASE_URL` handling will be introduced only when the PostgreSQL implementation exists.
