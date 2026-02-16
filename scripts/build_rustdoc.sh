#!/usr/bin/env bash
set -euo pipefail

cargo doc --workspace --no-deps
echo "Rustdoc generated: target/doc/rs_hyperliquid/index.html"
