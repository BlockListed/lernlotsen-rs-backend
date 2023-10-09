FROM rust:latest as BUILDER

WORKDIR /app/src

RUN apt-get update && apt-get install libssl-dev pkg-config -y

RUN cargo install bunyan

COPY . .

RUN --mount=type=cache,target=/app/src/target \
	cargo install --path .

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install openssl ca-certificates -y && rm -rf /var/lib/apt/lists/*

COPY --from=BUILDER /usr/local/cargo/bin/bunyan /app/bunyan

COPY --from=BUILDER /usr/local/cargo/bin/backend /app/backend

ENTRYPOINT [ "/bin/bash", "-c", "/app/backend | /app/bunyan" ]