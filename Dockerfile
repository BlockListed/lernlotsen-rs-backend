FROM rust:latest as BUILDER

WORKDIR /app/src

RUN apt-get update

RUN apt-get install libssl-dev pkg-config -y

COPY . .

RUN cargo install --path .

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install openssl ca-certificates -y && rm -rf /var/lib/apt/lists/*

COPY --from=BUILDER /usr/local/cargo/bin/backend /app/backend

ENTRYPOINT [ "/app/backend" ]