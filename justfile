set shell := ["pwsh", "-NoLogo", "-NoProfile", "-Command"]

default:
    @just --list

ci: fmt clippy test smoke

fmt:
    cargo fmt --all -- --check

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace

smoke:
    cargo run -p wincraft-mcp --bin wincraft -- --smoke
    cargo run -p wincraft-app -- --smoke

app:
    cargo run -p wincraft-app --release

mcp:
    cargo run -p wincraft-mcp --bin wincraft --release -- mcp
