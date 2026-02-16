set shell := ["bash", "-euo", "pipefail", "-c"]

default: help

help:
  @just --list

build:
  cargo build --all-targets

build-release:
  cargo build --release --all-targets

fmt:
  cargo fmt --all

fmt-check:
  cargo fmt --all --check

lint:
  cargo clippy --all-targets --all-features -- -D warnings

test:
  cargo test --all-targets --all-features

doc:
  cargo doc --workspace --no-deps

docs-no-emoji:
  bash scripts/check_docs_no_emoji.sh

check: fmt-check lint test docs-no-emoji

ci: check doc

run +args:
  cargo run -- {{args}}

clean:
  cargo clean
