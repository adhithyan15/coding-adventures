#!/usr/bin/env bash
# Build and test the WASM Markov Chain bindings.
#
# Native tests (cargo test) validate the wasm-bindgen glue on the host
# target.  The actual .wasm artefact is produced by wasm-pack, but that
# requires the wasm32-unknown-unknown target and is handled separately.
set -euo pipefail
cd "$(dirname "$0")"
mise exec -- cargo test -p markov-chain-wasm
