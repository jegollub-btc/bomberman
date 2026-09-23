# Bomberman arena task runner.

default:
    @just --list

# Run the server: bots on udp/47800, web UI on http://127.0.0.1:8080.
server *ARGS:
    cargo run -p bomber-server -- {{ARGS}}

# Run the server, restarting on source changes.
watch:
    watchexec -r -e rs -- cargo run -p bomber-server

# Run one example Python bot. Run it twice to fill a lobby.
pybot *ARGS:
    python3 clients/python/examples/wanderer.py {{ARGS}}

# Server plus two bots, ready for you to press Start in the UI.
arena:
    #!/usr/bin/env bash
    set -euo pipefail
    trap 'kill 0' EXIT
    cargo build -p bomber-server
    ./target/debug/bomber-server &
    sleep 1
    python3 clients/python/examples/wanderer.py &
    python3 clients/python/examples/wanderer.py &
    echo
    echo "  open http://127.0.0.1:8080  --  ctrl-c to stop everything"
    echo
    wait

test:
    cargo test --workspace

lint:
    cargo clippy --workspace --all-targets -- -D warnings

fmt:
    nix fmt

# Recipes for the Rust bot SDK (crates/bomber-bot) and the visualizer
# (visualizer/) are deliberately absent: neither exists yet, and a recipe that
# fails is worse than one that is missing.
