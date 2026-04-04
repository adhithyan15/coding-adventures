# polynomial-native BUILD
#
# This package requires a two-step build: first compile the Rust static library,
# then copy it into place for the Swift package to link against.
#
# Full build + test sequence:
#
#   Step 1: Build the Rust static library
#   cd ../../rust/polynomial-c && cargo build --release
#
#   Step 2: Copy the compiled library to the CPolynomial C target directory
#   cp ../../rust/polynomial-c/target/release/libpolynomial_c.a \
#      Sources/CPolynomial/
#
#   Step 3: Run Swift tests
#   swift test

swift build
