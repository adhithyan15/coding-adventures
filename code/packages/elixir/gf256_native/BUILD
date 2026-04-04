# Build the Rust NIF shared library and run Mix tests.
#
# IMPORTANT: The build tool runs each non-comment line as a SEPARATE subprocess
# with the working directory reset to the package root each time. Multi-step
# operations that require a directory change MUST be chained with && on a
# single line so they run in the same subprocess.
cd native/gf256_native && cargo build --release
mkdir -p priv && { cp native/gf256_native/target/release/libgf256_native.dylib priv/gf256_native.so 2>/dev/null || cp native/gf256_native/target/release/libgf256_native.so priv/gf256_native.so; }
mix deps.get --quiet && mix compile && mix test
