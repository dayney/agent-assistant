# agent-assistant Desktop task runner

set shell := ["bash", "-euo", "pipefail", "-c"]
set dotenv-load := false

default:
    @just --list

# Compile the React frontend and check the embedded Rust core.
build:
    cd desktop && npm run build
    cargo check --manifest-path desktop/src-tauri/Cargo.toml

# Run the complete Desktop test suite.
test: test-desktop

# Compatibility alias for existing developer workflows.
test-release: test-desktop

# Fast native-core feedback.
test-fast:
    cargo test --manifest-path desktop/src-tauri/Cargo.toml

# Test and type-check the React frontend plus the Tauri native layer.
test-desktop:
    cd desktop && npm test && npm run build
    cargo test --manifest-path desktop/src-tauri/Cargo.toml
    cargo check --manifest-path desktop/src-tauri/Cargo.toml

# Build the local Tauri application bundle for the current supported target.
desktop-build:
    cd desktop && npm run tauri:build

# Check Rust formatting and fail on every Clippy warning.
lint:
    cargo fmt --manifest-path desktop/src-tauri/Cargo.toml --check
    cargo clippy --manifest-path desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
    cd desktop && npm run typecheck

# Local pre-release validation. Signed packaging runs in GitHub Actions.
ci: lint test-desktop

# Cut a stable Desktop release tag. Usage: just release v0.1.0
release version:
    scripts/release-tag.sh "{{ version }}"

# Remove generated local build artifacts.
clean:
    rm -rf dist desktop/dist desktop/src-tauri/target
