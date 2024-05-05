FROM rustlang/rust:nightly-bookworm AS builder

ENV CARGO_TARGET_DIR=/cache \
    PATH=/root/.cargo/bin:$PATH \
    USER=root

WORKDIR /cache

RUN git clone https://github.com/trouchet/rust-hello.git /rust \
    && cp /rust/Cargo.toml /rust/Cargo.lock /rust/ \
    && cargo build --release \
    && rm -rf /rust

WORKDIR /app
COPY . .
RUN cargo build --release

CMD ["cargo", "run", "--bin", "scraper"]
