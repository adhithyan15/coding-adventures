#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."

cargo test -p tls13-signature-scheme-list --test tls13_signature_scheme_list -- --nocapture
cargo test -p tls13-signature-scheme-list --doc
