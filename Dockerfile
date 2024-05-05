FROM rustlang/rust:nightly-bookworm AS builder

ENV CARGO_TARGET_DIR=/cache \
    PATH=/root/.cargo/bin:$PATH \
    USER=root

WORKDIR /rust

RUN git clone https://github.com/trouchet/rust-hello.git .
COPY Cargo.* .
RUN cargo build && rm -rf /rust

WORKDIR /app
COPY . .
RUN cargo build

CMD ["cargo", "run", "--bin", "scraper"]
