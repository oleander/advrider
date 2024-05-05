set dotenv-load

deps:
  docker pull mattes/rotating-proxy:latest

test:
  curl --proxy 127.0.0.1:5566 https://api.my-ip.io/ip

summerize:
  cargo run --bin summerize

ollama-start:
  killall Ollama || true
  ollama serve

config-map:
  promptfoo eval -c promptfoo/map-promptfooconfig.yaml

worker:
  RUST_LOG=info SPIDER_WORKER_PORT=3030 spider_worker
  SPIDER_WORKER=http://127.0.0.1:3030 cargo run --bin scraper
