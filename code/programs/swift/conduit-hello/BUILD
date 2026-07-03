# build-tool: deps=swift/conduit rust/conduit-capi
# Build the Conduit C ABI, stage it where the Conduit package links it, then run
# the demo's smoke test (where Swift is available).
( cd ../../../packages/rust/conduit-capi && cargo build --release ) && if command -v swift >/dev/null 2>&1; then cp ../../../packages/rust/target/release/libconduit_capi.a ../../../packages/swift/conduit/Sources/CConduit/ && swift test; else echo "swift not available on this runner — conduit-capi built, skipping swift test"; fi
