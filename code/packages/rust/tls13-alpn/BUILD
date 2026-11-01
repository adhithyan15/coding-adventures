#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."

cargo test -p tls13-alpn --test tls13_alpn -- --nocapture
cargo test -p tls13-alpn --doc
