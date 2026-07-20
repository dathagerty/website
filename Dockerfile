FROM rust:1.95.0-bookworm@sha256:6258907abe69656e41cd992e0b705cdcfabcbbe3db374f92ed2d47121282d4a1 AS builder

ARG RAILWAY_GIT_COMMIT_SHA
ENV RAILWAY_GIT_COMMIT_SHA=${RAILWAY_GIT_COMMIT_SHA}

WORKDIR /build

RUN --mount=type=cache,id=s/f05a578d-b801-404e-844e-3ff09756b5b7-dathagerty-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=s/f05a578d-b801-404e-844e-3ff09756b5b7-dathagerty-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    cargo install --locked --version 0.1.3 topcoat-cli

COPY Cargo.toml Cargo.lock build.rs rust-toolchain.toml ./
COPY assets ./assets
COPY content ./content
COPY src ./src

RUN --mount=type=cache,id=s/f05a578d-b801-404e-844e-3ff09756b5b7-dathagerty-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=s/f05a578d-b801-404e-844e-3ff09756b5b7-dathagerty-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=s/f05a578d-b801-404e-844e-3ff09756b5b7-dathagerty-target,target=/build/target,sharing=locked \
    cargo build --locked --release --bin dathagerty \
    && topcoat asset bundle --release --bin dathagerty \
    && mkdir -p /out/assets \
    && cp /build/target/release/dathagerty /out/dathagerty \
    && cp -a /build/target/assets/. /out/assets/

FROM debian:bookworm-slim@sha256:7b140f374b289a7c2befc338f42ebe6441b7ea838a042bbd5acbfca6ec875818 AS runtime

RUN groupadd --system --gid 10001 app \
    && useradd --system --uid 10001 --gid app --home-dir /nonexistent --no-create-home --shell /usr/sbin/nologin app

WORKDIR /app

COPY --from=builder --chmod=0555 /out/dathagerty /app/dathagerty
COPY --from=builder /out/assets /app/assets
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt

USER 10001:10001

EXPOSE 3000

ENTRYPOINT ["/app/dathagerty"]
