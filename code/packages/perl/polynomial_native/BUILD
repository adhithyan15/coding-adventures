#!/usr/bin/env bash
# Build the Perl polynomial_native XS extension.
#
# This produces PolynomialNative.so which Perl's DynaLoader loads with
#   use CodingAdventures::PolynomialNative;
#
# Testing requires the .so to be installed in the correct DynaLoader
# path: auto/CodingAdventures/PolynomialNative/PolynomialNative.so
# See README.md for manual testing instructions.
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release 2>&1
echo "Built: target/release/libPolynomialNative.so (or .dylib)"
