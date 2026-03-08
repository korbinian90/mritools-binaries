# Build stage
FROM rust:1.75-slim AS builder

WORKDIR /build

# Cache dependency layer
COPY Cargo.toml Cargo.lock ./
COPY crates/common/Cargo.toml crates/common/
COPY crates/romeo/Cargo.toml crates/romeo/
COPY crates/clearswi/Cargo.toml crates/clearswi/
COPY crates/mcpc3ds/Cargo.toml crates/mcpc3ds/
COPY crates/makehomogeneous/Cargo.toml crates/makehomogeneous/
COPY crates/romeo_mask/Cargo.toml crates/romeo_mask/

# Create stub source files so cargo can resolve the workspace
RUN mkdir -p crates/common/src \
    && echo "pub fn stub() {}" > crates/common/src/lib.rs \
    && for crate in romeo clearswi mcpc3ds makehomogeneous romeo_mask; do \
         mkdir -p crates/$crate/src && echo "fn main() {}" > crates/$crate/src/main.rs; \
       done

# Fetch and compile dependencies (cached layer)
RUN cargo build --release 2>/dev/null || true

# Copy real source code and build
COPY crates/ crates/
RUN touch crates/*/src/*.rs \
    && cargo build --release

# Runtime stage
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder \
    /build/target/release/romeo \
    /build/target/release/clearswi \
    /build/target/release/mcpc3ds \
    /build/target/release/makehomogeneous \
    /build/target/release/romeo_mask \
    /usr/local/bin/

WORKDIR /data

LABEL org.opencontainers.image.title="mritools-binaries" \
      org.opencontainers.image.description="ROMEO, CLEAR-SWI, MCPC-3D-S and related MRI processing CLI tools" \
      org.opencontainers.image.source="https://github.com/korbinian90/mritools-binaries" \
      org.opencontainers.image.licenses="MIT"

ENTRYPOINT ["/bin/sh"]
