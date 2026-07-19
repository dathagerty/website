# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a personal website written in Rust using Topcoat. The application serves server-rendered HTML, embeds its content and assets in release artifacts, and uses structured logging with tracing.

## Commands

### Building and Running
```bash
# Build the project
cargo build

# Run the development server with asset rebuilding (default port 3000)
topcoat dev --bin dathagerty

# Build the release binary and matching asset bundle
cargo build --locked --release --bin dathagerty
topcoat asset bundle --release --bin dathagerty

# Run tests
cargo test

# Check code without building
cargo check

# Format code
cargo fmt

# Run Clippy linter
cargo clippy
```

### Configuration
The server reads `PORT` (default `3000`), `INCLUDE_DRAFTS` (default `false`), and `RUST_LOG` (default `info`) from the environment. Invalid values fail startup.

### Changelog Generation
```bash
# Generate changelog using git-cliff
git cliff
```

## Architecture

### Application Structure

**src/main.rs**: Entry point that initializes JSON tracing, validates embedded content, loads and validates the required stylesheet in the Topcoat asset bundle, binds to `0.0.0.0:$PORT`, and serves until graceful shutdown. In-flight connections receive a 10-second drain window before process exit.

**src/content/**: Typed Markdown/JSON parsing and the asynchronous `ContentRepository` boundary. `EmbeddedContentRepository` validates all embedded content at startup and builds deterministic indexes.

**src/routes/**: Explicit Topcoat route registration and thin handlers for pages, posts, tags, Go modules, errors, and `/healthz`.

**src/views/**: Topcoat layouts and components. The root layout owns the document shell, route metadata, navigation, footer, and content-hashed stylesheet reference.

**build.rs**: Build-time code generation:
- Calls `build_info_build::build_script()` to generate local Git metadata
- Tracks `RAILWAY_GIT_COMMIT_SHA` so Docker/Railway revision injection rebuilds metadata consumers

### Key Dependencies

- **topcoat**: Web framework and type-safe view system (pinned to v0.1.3)
- **tokio**: Async runtime
- **tracing/tracing-subscriber**: Structured logging
- **build-info**: Embeds build metadata in the binary
- **comrak**: Markdown parsing and syntax highlighting
- **rust-embed**: Embeds pages, posts, and site data in the binary

### Content Directory

`content/` contains Markdown pages and posts plus JSON branding and Go-module metadata. All content is embedded and validated at startup.

### Routing Pattern

Routes are registered explicitly for `/`, `/about`, `/reading`, `/blag`, `/blag/{slug}`, `/tags`, `/tags/{tag}`, `/go`, `/go/{module}`, and `/healthz`. Missing content returns HTML 404 responses, and unsupported methods retain HTTP 405 behavior.

### Build Metadata

The footer prefers a plausible hexadecimal `RAILWAY_GIT_COMMIT_SHA` embedded at compile time, then falls back to the short Git commit ID from `build_info::build_info!`. It links valid revisions to the repository commit and renders plain `unknown` when neither source is usable. Docker builds should pass `--build-arg RAILWAY_GIT_COMMIT_SHA="$(git rev-parse HEAD)"`; Railway provides that built-in variable automatically after the Dockerfile declares it.

### Logging

JSON tracing respects `RUST_LOG`. A router layer records request method, path, response status, and elapsed time, and request-time repository failures are logged through tracing. Startup failures return contextual errors to Rust's termination handling, which writes them to stderr; they are not guaranteed to use the structured JSON subscriber.

## Commit Message Style

This project uses conventional commits with git-cliff for changelog generation:
- `feat:` for new features
- `fix:` for bug fixes
- `refactor:` for code refactoring
- `docs:` for documentation
- `test:` for tests
- `chore:` for miscellaneous tasks
