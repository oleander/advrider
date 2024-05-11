set dotenv-load

ollama-start:
  killall Ollama || true
  ollama serve

config-map:
  promptfoo eval -c promptfoo/map-promptfooconfig.yaml

spider:
  docker compose up spider --remove-orphans

proxy:
  docker compose up proxy --remove-orphans

scraper:
  docker compose up scraper --remove-orphans

run: build
  docker compose up --remove-orphans

build:
  docker compose build
