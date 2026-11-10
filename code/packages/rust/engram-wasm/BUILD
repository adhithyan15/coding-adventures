cargo test -p engram-wasm -- --nocapture
# The .wasm is generated, not committed (BUILD-001): build it for the browser
# target, then run the smoke test against that fresh build.
#
# This is the drift guard. The Rust tests above compile the source for the host;
# they say nothing about the wasm32 build a web shell actually ships. A checked-in
# artifact once went two months stale without a single failure -- it still refused
# Anki import in the browser long after the source had stopped doing so, and the
# smoke test agreed with it because the assertions were written against the old
# behaviour too. Building here removes the artifact that could go stale; the
# smoke assertions pin current behaviour.
rustup target add wasm32-unknown-unknown
bash build-wasm.sh
node js/smoke.mjs
