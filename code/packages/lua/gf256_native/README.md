# gf256_native (Lua)

Lua C extension providing GF(256) Galois Field arithmetic, backed by the
Rust `gf256` crate via the zero-dependency `lua-bridge`.

## Usage

```lua
package.cpath = "target/release/lib?.so;" .. package.cpath
local gf = require("gf256_native")

gf.add(83, 202)        -- 153 (XOR)
gf.multiply(2, 16)     -- 32
gf.divide(4, 2)        -- 2
gf.power(2, 8)         -- 29  (reduced mod 0x11D)
gf.inverse(83)         -- multiplicative inverse of 83
```

## Building

```bash
cargo build --release
```
