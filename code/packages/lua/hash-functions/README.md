# hash-functions (Lua)

Pure Lua 5.4 implementations of non-cryptographic hash primitives used by the
hash-map and Bloom-filter packages.

## API

- `fnv1a_32(data)` and `fnv1a_64(data)`
- `djb2(data)`
- `polynomial_rolling(data, base?, modulus?)`
- `murmur3_32(data, seed?)`
- `avalanche_score(hash_fn, output_bits, sample_size?)`
- `distribution_test(hash_fn, inputs, num_buckets)`
- `uint64_hex(value)` for viewing a signed Lua integer as an unsigned word

Lua strings are byte sequences, so every function is binary-safe. UTF-8 text
is hashed as its encoded bytes. Lua 5.4 represents integers as signed 64-bit
values; `fnv1a_64` and `djb2` return the exact two's-complement word and may
therefore be negative. `uint64_hex` exposes the corresponding unsigned bits.

These functions are educational, deterministic, and dependency-free. They are
not cryptographic hashes and must not be used for passwords or signatures.

## Development

```bash
luarocks make --local --deps-mode=none coding-adventures-hash-functions-0.1.0-1.rockspec
cd tests
LUA_PATH="../src/?.lua;../src/?/init.lua;;" busted . --verbose --pattern=test_
```
