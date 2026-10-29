#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."

cargo test -p tls13-supported-groups --test tls13_supported_groups -- --nocapture
cargo test -p tls13-supported-groups --doc
