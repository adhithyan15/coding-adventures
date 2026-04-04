# polynomial_native (Lua)

Lua C extension providing polynomial arithmetic over `f64`, backed by the
Rust `polynomial` crate via the zero-dependency `lua-bridge`.

## Where it fits

```
polynomial_native.so  (this package — Lua C extension)
         │
         └── lua-bridge (Rust)   ──── Lua 5.4 C API declarations
         └── polynomial (Rust)   ──── core arithmetic
```

## Building

```bash
cargo build --release
# Produces: target/release/libpolynomial_native.so (Linux)
#       or: target/release/libpolynomial_native.dylib (macOS)
```

## Usage

```lua
-- Copy the .so into your Lua cpath, or set package.cpath explicitly:
package.cpath = package.cpath .. ";./target/release/lib?.so"
-- On macOS, use lib?.dylib

local poly = require("polynomial_native")

-- Polynomials are Lua tables, 1-indexed, index = degree + 1
-- {3.0, 0.0, 1.0} represents 3 + 0·x + 1·x²
local a = {1.0, 2.0}   -- 1 + 2x
local b = {3.0, 4.0}   -- 3 + 4x

print(poly.add(a, b))       -- {4.0, 6.0}
print(poly.multiply(a, b))  -- {3.0, 10.0, 8.0}
print(poly.degree({3.0, 0.0, 2.0}))  -- 2
print(poly.evaluate({3.0, 0.0, 1.0}, 2.0))  -- 7.0

local q, r = poly.divmod({5.0,1.0,3.0,2.0}, {2.0,1.0})
-- q = {3.0, -1.0, 2.0},  r = {-1.0}
```

## Testing

Automated tests require Lua 5.4 and the compiled `.so` on `package.cpath`.
Run manually:

```lua
-- test.lua
package.cpath = "target/release/lib?.so;" .. package.cpath
local poly = require("polynomial_native")
assert(poly.evaluate({3.0, 0.0, 1.0}, 2.0) == 7.0)
print("Tests passed")
```

```bash
lua test.lua
```

## Notes

- `lua_module!` from lua-bridge requires `concat_idents` (not in stable Rust),
  so the entry point `luaopen_polynomial_native` is written by hand as a
  `#[no_mangle] pub unsafe extern "C"` function.
- `divmod` returns two Lua values (Lua's multiple-return idiom).
