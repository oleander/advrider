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
  cargo run --bin glob -- \
  --proxies socks5://127.0.0.1:9050 socks5://127.0.0.1:8050 socks5://127.0.0.1:7050 \
  --url https://advrider.com/f/threads/the-toolkit-thread.262998/page-\[1-\10] \
  --controllers 127.0.0.1:9051 127.0.0.1:8051 127.0.0.1:7051 \
  --rotate-proxy-every 3 \
  --verbose
