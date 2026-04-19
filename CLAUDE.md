# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a personal website written in Rust using Axum as the web framework. It's described as the "Grand Rust Rewrite" of the original site. The application serves HTML pages generated with the Maud templating library and uses structured logging with tracing.

## Commands

### Building and Running
```bash
# Build the project
cargo build

# Run the development server (default port 3000)
cargo run

# Build for release
cargo build --release

# Run tests
cargo test

# Check code without building
cargo check

# Format code
cargo fmt

# Run clipper linter
cargo clippy
```

### Configuration
The server looks for an optional `config.json` file with:
```json
{
  "port": 3000
}
```
If not present, defaults to port 3000.

### Changelog Generation
```bash
# Generate changelog using git-cliff
git cliff
```

## Architecture

### Application Structure

**src/main.rs**: Entry point that:
- Initializes tracing subscriber for structured logging
- Loads server configuration from optional `config.json` (defaults to port 3000)
- Sets up Axum router with routes: `/`, `/about`, `/blag`, `/blag/{slug}`
- Binds to `0.0.0.0:{port}` and serves the application
- All route handlers are instrumented with tracing

**src/templates.rs**: HTML generation using Maud templates:
- All rendering functions use Maud's `html!` macro for type-safe HTML
- Three main page types: `root()`, `page()`, and `post(slug)`
- Common components: `head()`, `header()`, `footer()`
- Build metadata embedded via `build_info` crate (commit hash, build date/time)
- Helper functions for links: `page_link()` and `me_link()` (with rel="me")

**src/config.rs**: Configuration management:
- `ServerConfig` struct with single field: `port: u16`
- Loads from optional `config.json` file using the `config` crate
- Provides `Default` implementation (port 3000)

**src/flair/mod.rs**: Site branding/personality:
- `get_word()`: Returns the site name modifier (currently "deliriums")
- `get_tagline()`: Returns the site tagline (currently "a little rusty")

**build.rs**: Build-time code generation:
- Calls `build_info_build::build_script()` to generate build metadata
- This enables embedding git commit info and timestamps in the binary

### Key Dependencies

- **axum**: Web framework (v0.8.3) with JSON and multipart support
- **maud**: Type-safe HTML templating (v0.27.0) with Axum integration
- **tokio**: Async runtime (v1.44.1) with full features
- **tracing/tracing-subscriber**: Structured logging
- **config**: Configuration file management (v0.14)
- **build-info**: Embeds build metadata in the binary
- **markdown**: Markdown parsing (v1.0.0-alpha.23, currently unused)

### Content Directory

`content/`: Currently empty (just a .keep file), intended for blog posts and page content. The post rendering is stubbed out with placeholder text.

### Routing Pattern

The application uses a simple, flat routing structure:
- `/` -> `root()` handler
- `/about` -> `page()` handler
- `/blag` -> `page()` handler (blog listing)
- `/blag/{slug}` -> `post(slug)` handler (individual blog posts)

Note: The `page()` and `post()` handlers currently return placeholder content.

### Build Metadata

The application embeds build information that appears in the footer:
- Commit short hash via `COMMIT_SHORT` and `COMMIT_HASH` constants
- Build timestamp (date and time)
- Uses the `build-info` crate's format macro to extract git metadata
- Build script in `build.rs` generates this metadata at compile time

### Logging

All handlers and most template functions are instrumented with `#[instrument]`:
- Automatically adds span information to logs
- Use `info!()`, `debug!()`, etc. macros for logging
- Structured fields can be added (e.g., `info!(slug, "message")`)

## Commit Message Style

This project uses conventional commits with git-cliff for changelog generation:
- `feat:` for new features
- `fix:` for bug fixes
- `refactor:` for code refactoring
- `docs:` for documentation
- `test:` for tests
- `chore:` for miscellaneous tasks
