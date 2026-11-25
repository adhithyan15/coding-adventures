#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."

cargo test --locked -p tls13-supported-groups --test tls13_supported_groups -- --nocapture
cargo test --locked -p tls13-supported-groups --doc
