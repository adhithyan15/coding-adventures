#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."

cargo test -p tls13-client-hello-message --test tls13_client_hello_message -- --nocapture
cargo test -p tls13-client-hello-message --doc
