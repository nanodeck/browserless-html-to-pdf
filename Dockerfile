FROM rust:1-alpine AS builder
WORKDIR /app
RUN apk add --no-cache build-base musl-dev
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked --bin browserless-html-to-pdf \
    && cp target/release/browserless-html-to-pdf /app/server

FROM debian:bookworm-slim AS msfonts
RUN echo "deb http://deb.debian.org/debian bookworm contrib non-free" > /etc/apt/sources.list.d/contrib.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates ttf-mscorefonts-installer \
    && rm -rf /var/lib/apt/lists/*

FROM alpine:3.23 AS runtime
RUN apk add --no-cache font-liberation font-opensans \
    && addgroup -S app && adduser -S -G app -H -s /sbin/nologin app
COPY --from=msfonts /usr/share/fonts/truetype/msttcorefonts /usr/share/fonts/truetype/msttcorefonts
COPY --from=builder /app/server /usr/local/bin/server
RUN install -d -o app -g app /data
ENV PORT=3000 \
    STORAGE_ENABLED=false \
    OPENDAL_ROOT=/data
WORKDIR /data
USER app
EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget -qO- "http://127.0.0.1:${PORT}/health" >/dev/null 2>&1 || exit 1
ENTRYPOINT ["/usr/local/bin/server"]
