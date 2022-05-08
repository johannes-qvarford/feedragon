# Introduction

Feedragon is a personal utility for aggregating feeds in a way that suits me.

# Build instructions

## Docker

Just use the provided Dockerfile.

## Ubuntu

The project has the following dependencies:

> build-essential
> pkg-config
> libssl-dev

You also need to install rustfmt to format the source code.

> rustup component add rustfmt
>
> cargo fmt

Then you can run it as any other rust binary e.g.

> cargo run