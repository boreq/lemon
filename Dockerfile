# syntax=docker/dockerfile:1.6
FROM rustlang/rust:nightly-slim AS builder
WORKDIR /build
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release --bin lemon \
    && mkdir -p /build/artifacts \
    && cp /build/target/release/lemon /build/artifacts/

FROM debian:trixie-slim AS runtime
RUN useradd -m -u 1000 lemon
COPY --from=builder /build/artifacts/lemon /usr/local/bin/lemon
USER lemon
ENTRYPOINT ["/usr/local/bin/lemon"]
CMD ["run", "/config.toml"]
