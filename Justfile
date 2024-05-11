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
