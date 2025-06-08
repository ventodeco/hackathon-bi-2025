FROM rustlang/rust:nightly as builder

WORKDIR /usr/src/app
COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/backend-project /usr/local/bin/backend-project

EXPOSE 8080

CMD ["backend-project"] 