# Build stage
FROM rust:1.91-bookworm as builder

WORKDIR /build

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build release binary
RUN cargo build --release --bin berg-operator

# Runtime stage
FROM gcr.io/distroless/cc-debian12

WORKDIR /

# Copy the binary from builder
COPY --from=builder /build/target/release/berg-operator /usr/local/bin/berg-operator

# Use non-root user
USER nonroot:nonroot

ENTRYPOINT ["/usr/local/bin/berg-operator"]
