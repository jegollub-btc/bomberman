# Bomberman bot arena task runner.

default:
    @just --list

# Run the server (serves the visualizer build on http://127.0.0.1:8080).
server *ARGS:
    cargo run -p bomber-server -- {{ARGS}}

# Run the server, restarting on source changes.
watch:
    watchexec -r -e rs -- cargo run -p bomber-server

# Visualizer dev server with hot reload (proxies /ws to the game server).
viz:
    cd visualizer && pnpm install && pnpm dev

# Run the example Rust bot.
bot *ARGS:
    cargo run -p bomber-bot --bin wanderer -- {{ARGS}}

# Run the example Python bot.
pybot *ARGS:
    python3 clients/python/examples/wanderer.py {{ARGS}}

# Server + visualizer together.
dev:
    #!/usr/bin/env bash
    trap 'kill 0' EXIT
    cargo run -p bomber-server &
    (cd visualizer && pnpm install && pnpm dev) &
    wait

test:
    cargo test --workspace

lint:
    cargo clippy --workspace --all-targets -- -D warnings

fmt:
    nix fmt
