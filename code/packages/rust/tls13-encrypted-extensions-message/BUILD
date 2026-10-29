#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."

cargo test -p tls13-encrypted-extensions-message --test tls13_encrypted_extensions_message -- --nocapture
cargo test -p tls13-encrypted-extensions-message --doc
