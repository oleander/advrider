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
