# build-tool: deps=rust/conduit-capi
# Build the reusable Conduit C ABI (conduit-capi), stage its static library
# where the CConduit systemLibrary target links it, then run the Swift tests.
# The capi is always built (validates it on every runner); swift test runs only
# where the Swift toolchain is present.
( cd ../../rust/conduit-capi && cargo build --release ) && if command -v swift >/dev/null 2>&1; then cp ../../rust/target/release/libconduit_capi.a Sources/CConduit/ && swift test; else echo "swift not available on this runner — conduit-capi built, skipping swift test"; fi