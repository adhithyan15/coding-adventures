# gf256-native BUILD
#
# This package requires a two-step build: first compile the Rust static library,
# then copy it into place for the Swift package to link against.
#
# Full build + test sequence:
#
#   Step 1: Build the Rust static library
#   cd ../../rust/gf256-c && cargo build --release
#
#   Step 2: Copy the compiled library to the CGF256 C target directory
#   cp ../../rust/gf256-c/target/release/libgf256_c.a \
#      Sources/CGF256/
#
#   Step 3: Run Swift tests
#   swift test

swift build
