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
    pwsh -NoProfile -ExecutionPolicy Bypass -File start.ps1

mcp:
    cargo run -p wincraft-mcp --bin wincraft --release -- mcp

# Build and stage dist/ (plain exes, installer, launcher zip, SHA256SUMS). Nothing is published.
release-dry:
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/release.ps1

# Same, then tag, push the tag and upload to GitHub (clean, pushed trunk required).
release:
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/release.ps1 -Publish
