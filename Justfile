set dotenv-load

ollama-start:
  killall Ollama || true
  ollama serve

config-map:
  promptfoo eval -c promptfoo/map-promptfooconfig.yaml

spider:
  docker compose up spider --remove-orphans

proxy:
  docker compose up -d proxy0 --remove-orphans
  docker compose up -d proxy1 --remove-orphans
  docker compose up -d proxy2 --remove-orphans

scraper:
  docker compose up scraper --remove-orphans

run: build
  docker compose up --remove-orphans

build:
  # docker build -f Dockerfile.binstall . -t binstall --build-arg="GITHUB_TOKEN=$GITHUB_TOKEN"
  docker compose build

cli:
  RUST_LOG=info cargo run --features spider/regex  --bin glob -- \
  --proxies socks5://127.0.0.1:9050 socks5://127.0.0.1:8050 socks5://127.0.0.1:7050 \
  --url "https://advrider.com/f/forums/racing.25" \
  --controllers 127.0.0.1:9051 127.0.0.1:8051 127.0.0.1:7051 \
  --rotate-proxy-every 30 \
  --page-limit 400 \
  --verbose
