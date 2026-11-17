#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."

cargo test -p tls13-cookie --test tls13_cookie -- --nocapture
cargo test -p tls13-cookie --doc
