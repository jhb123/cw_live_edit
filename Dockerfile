ARG RUST_VERSION=1.76.0
ARG APP_NAME=cw_grid_server
FROM rust:${RUST_VERSION}-slim-bullseye AS build
ARG APP_NAME
WORKDIR /app

RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=bind,source=rust-toolchain,target=rust-toolchain \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    <<EOF
set -e
cargo build --release --locked
cp ./target/release/$APP_NAME /bin/server
EOF

FROM debian:bullseye-slim AS final

ARG UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser

ENV PUZZLE_PATH=/puzzles
RUN mkdir /puzzles && chown appuser:appuser /puzzles && chmod 700 /puzzles
VOLUME /puzzles

USER appuser

COPY --from=build /bin/server /app/bin/
COPY /templates /templates
COPY /static /static

ENV PUZZLE_PORT=5051
EXPOSE 5051

ENV RUST_LOG=INFO

CMD ["/app/bin/server"]
