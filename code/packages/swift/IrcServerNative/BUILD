# build-tool: deps=rust/irc-server-capi
# Build the reusable IRC-server C ABI (irc-server-capi), stage its static library
# where the CIrcServer systemLibrary target links it, then run the Swift tests.
# The capi is always built (validates it on every runner); swift test runs only
# where the Swift toolchain is present.
( cd ../../rust/irc-server-capi && cargo build --release ) && if command -v swift >/dev/null 2>&1; then cp ../../rust/target/release/libirc_server_capi.a Sources/CIrcServer/ && swift test; else echo "swift not available on this runner — irc-server-capi built, skipping swift test"; fi
