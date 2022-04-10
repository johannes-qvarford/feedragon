FROM rust:latest as builder
WORKDIR /usr/src/feedragon

# create a new empty project
RUN cargo init
COPY Cargo.toml Cargo.lock ./
# build dependencies, when my source code changes, this build can be cached, we don't need to compile dependency again.
RUN cargo build
# remove the dummy build.
RUN cargo clean -p feedragon

COPY ./src src

RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update && apt-get install -y build-essential pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/feedragon /usr/local/bin/feedragon
CMD ["feedragon"]