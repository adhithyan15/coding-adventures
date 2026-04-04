# Build the Lua gf256_native extension (cdylib loaded by require()).
# cargo sets CWD to the package directory; no cd needed.
cargo build --release 2>&1
echo "Built: target/release/libgf256_native.so (or .dylib)"
