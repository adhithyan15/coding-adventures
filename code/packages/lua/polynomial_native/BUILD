# Build the Lua polynomial_native extension.
#
# This produces a shared library that Lua can load with:
#   local poly = require("polynomial_native")
#
# Testing requires Lua 5.4 to be installed. Tests are not automated
# here because the shared library must be placed on Lua's cpath.
# See README.md for manual testing instructions.
cargo build --release 2>&1
echo "Built: target/release/libpolynomial_native.so (or .dylib)"
