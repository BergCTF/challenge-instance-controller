# chef
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

# planning stage
FROM chef AS planner
COPY Cargo.toml Cargo.lock .
RUN cargo chef prepare --recipe-path recipe.json

# building stage
FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo.toml Cargo.lock .
COPY src/ src/
RUN cargo build --release --bin berg-controller

# runtime env
FROM debian:trixie-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/berg-controller /usr/local/bin/berg-controller
ENTRYPOINT ["/usr/local/bin/berg-controller"]
