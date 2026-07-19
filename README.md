# dathagerty.com

The current incarnation of my personal site is a server-rendered Rust application built with [Topcoat](https://github.com/tokio-rs/topcoat). Topcoat is experimental, so both the library and `topcoat-cli` are pinned to `0.1.3`; update them together.

Pages, posts, branding, and Go vanity-module metadata live under `content/` and are embedded in the binary. Startup parses and validates all embedded content before accepting traffic. Route handlers query an asynchronous repository interface rather than the embedded implementation directly, leaving a seam for a future PostgreSQL repository without requiring one today.

## Local development

Install the pinned Rust toolchain and Topcoat CLI with [mise](https://mise.jdx.dev/):

```sh
mise install
```

Start the development server with Topcoat's rebuild and asset workflow:

```sh
topcoat dev --bin dathagerty
```

The application listens on `0.0.0.0:3000` unless `PORT` is set. Topcoat reports the development URL and watches the project for changes.

Useful quality commands:

```sh
topcoat fmt src
cargo fmt --all -- --check
cargo build --locked
cargo check --locked --all-targets --all-features
cargo test --locked --all-targets --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
```

## Release build

A release deployment requires the binary and the asset bundle generated from that exact release target. The pinned CLI command builds `dathagerty` and writes the content-hashed bundle to `target/assets/`:

```sh
topcoat asset bundle --release --bin dathagerty
PORT=3000 target/release/dathagerty
```

Keep `target/release/dathagerty` and `target/assets/` together when packaging. `AssetBundle::load()` discovers `assets/manifest.toml` beside the executable (and in Topcoat's conventional Cargo target locations). Startup verifies that the manifest contains the application stylesheet and that its file is readable before binding. Hashed assets are served below `/_topcoat/assets/` with a one-year immutable cache policy.

## Configuration

The application reads only these runtime environment variables:

| Variable | Default | Purpose |
| --- | --- | --- |
| `PORT` | `3000` | TCP port; Railway injects this dynamically. |
| `INCLUDE_DRAFTS` | `false` | Include draft posts when set to `true` or `1`. Production should leave this unset. |
| `RUST_LOG` | `info` | `tracing-subscriber` filter directives for JSON logs. |

Invalid values fail startup instead of silently falling back.

## Docker and Railway

The multi-stage Docker build pins Rust `1.95.0`, installs `topcoat-cli 0.1.3` with Cargo's locked dependency graph, builds the release binary, and generates its matching Topcoat asset bundle. The runtime image contains only the binary, bundle, CA roots, and minimal Debian runtime files. It runs as a non-root user with a direct binary entrypoint and does not require a writable root filesystem.

```sh
docker build \
  --build-arg RAILWAY_GIT_COMMIT_SHA="$(git rev-parse HEAD)" \
  -t dathagerty:topcoat .
docker run --rm --read-only -e PORT=3000 -p 3000:3000 dathagerty:topcoat
```

The build argument is optional for ordinary local builds, where build-info can read the checkout's Git metadata, but it should be passed explicitly for Docker because `.git` is excluded from the image context. Railway supplies its built-in `RAILWAY_GIT_COMMIT_SHA` automatically once the Dockerfile declares the argument. The footer prefers a plausible injected hexadecimal revision, falls back to build-info Git metadata, and renders plain `unknown` only if neither is usable.

Port `3000` is exposed as documentation and is the local default. Railway supplies its own dynamic `PORT`; `railway.json` selects the Dockerfile builder, checks `/healthz`, allows graceful draining, and restarts only failed containers. Shutdown stops accepting traffic and gives in-flight connections up to 10 seconds to drain, below Railway's 15-second termination grace period. No start command overrides the image entrypoint.

The deployment contract is:

- Ship the release binary with the bundle generated from that same binary.
- Bind to Railway's `PORT` on all interfaces.
- Return `200 OK` from `/healthz`.
- Serve successfully as a non-root user with a read-only root filesystem.
