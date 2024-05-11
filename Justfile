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

proxy:
  docker run -it -p 8118:8118 -p 9050:9050 -e TORUSER=root dperson/torproxy -b 200

scrape:
  docker compose build
  docker compose up
