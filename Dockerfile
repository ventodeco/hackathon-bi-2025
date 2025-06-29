# syntax=docker/dockerfile:1.4

# Stage 1: Build the hackathon-bi-2025
# Use the official Rust nightly-slim image
FROM rustlang/rust:nightly-slim AS builder

WORKDIR /app

# Copy your project's files into the builder stage
COPY . .

# Build your hackathon-bi-2025
# Install musl-tools for static linking
RUN apt-get update && apt-get install -y --no-install-recommends musl-tools \
    && rustup target add x86_64-unknown-linux-musl \
    && SQLX_OFFLINE=true cargo build --release --target x86_64-unknown-linux-musl

# Stage 2: Create the final minimal image
FROM alpine:latest

# Install CA certificates
RUN apk add --no-cache ca-certificates

# Copy the statically linked binary from the builder stage
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/hackathon-bi-2025 /

# Expose the port your application listens on
EXPOSE 8080

# Add healthcheck
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost:8080/health || exit 1

# Define the command to run your application
CMD ["./hackathon-bi-2025"]