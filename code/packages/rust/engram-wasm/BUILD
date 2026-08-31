cargo test -p engram-wasm -- --nocapture
# Run the smoke test against the checked-in pkg/engram_engine.wasm.
#
# This is the drift guard. The Rust tests above compile the source; they say
# nothing about the committed artifact, which is what a web build actually
# ships. That artifact went two months stale without a single failure — it still
# refused Anki import in the browser long after the source had stopped doing so,
# and the smoke test agreed with it because the assertions were written against
# the old behaviour too. Two stale things matching each other reads exactly like
# a passing test.
#
# The assertions now pin current behaviour, so an artifact that was not
# regenerated after a source change fails here instead of shipping.
node js/smoke.mjs
