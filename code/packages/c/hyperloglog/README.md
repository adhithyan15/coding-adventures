# hyperloglog (C)

**CCPP02 port campaign — bucket A (pure-ISO), port #3.** HyperLogLog answers
"roughly how many *distinct* items have I seen?" in a tiny, fixed amount of
memory — no matter how many items stream past, and without ever storing them.
The C port of the Rust `hyperloglog` crate (DT21), a pure-ISO crate that needs no
OS, so it rides the `iso-harness` (links nothing, `-pedantic-errors` /
`/permissive-`).

Each item is hashed; the length of the leading run of zero bits in the hash is a
cheap proxy for rarity, and the maximum such run per bucket, combined across
buckets with a harmonic mean, estimates the cardinality. The accuracy/memory
trade-off is set once by the **precision** `p ∈ [4, 16]`: `2^p` one-byte
registers, relative error ≈ `1.04 / sqrt(2^p)`.

```c
hll *h;
hll_create_default(&h);                 /* precision 14 → ~0.8% error, 16 KiB */

hll_add_str(h, "alice");
hll_add_str(h, "bob");
hll_add_str(h, "alice");                /* duplicate — no effect on the estimate */

size_t distinct = hll_count(h);         /* ≈ 2 */

hll *other; hll_create_default(&other);
/* … feed `other` from another shard … */
hll *both;
hll_merge(h, other, &both);             /* union: register-wise max */

hll_destroy(both);
hll_destroy(other);
hll_destroy(h);
```

| Function | Purpose |
|----------|---------|
| `hll_create` / `hll_create_default` | make a sketch (precision `[4,16]` / 14) |
| `hll_destroy` | free a sketch |
| `hll_add_bytes` / `hll_add_str` | observe one element (byte range / C string) |
| `hll_count` / `hll_is_empty` | estimate distinct count / whether it's zero |
| `hll_merge` | union two equal-precision sketches into a fresh one |
| `hll_precision` / `hll_num_registers` | configured precision / `2^precision` |
| `hll_error_rate` / `hll_error_rate_for_precision` | expected relative error |
| `hll_memory_bytes` | register memory for a precision |
| `hll_optimal_precision` | smallest precision meeting a target error |

## Pure, but not alone

This crate is pure-ISO yet **composes** two other pure-ISO packages rather than
re-deriving them, and still links nothing:

- the 64-bit **FNV-1a** hash (`hf_fnv1a_64`) from [`c/hash-functions`](../hash-functions); and
- the from-scratch **elementary math** (`fm_log` / `fm_sqrt` / `fm_log2` /
  `fm_ceil` / `fm_round`) from [`c/float-math`](../float-math).

The estimator needs `ln`, `sqrt`, and `log2` — but the lane forbids libm, so the
math comes from `float-math`'s from-scratch implementations. `run.sh` compiles all
three packages' sources into the test; no math library is ever linked.

## Faithfulness notes

- **Same estimation pipeline.** FNV-1a → MurmurHash3 `fmix64` avalanche → top-`p`
  bits pick the bucket, leading-zero run + 1 is `rho`; `count` uses the harmonic
  mean with linear-counting (small cardinalities) and the large-range correction
  near `2^32`, exactly as the Rust.
- **`2^(-register)` without `pow`.** Register values are ≤ 61, so `2^(-r)` is the
  exact `1 / 2^r` via an integer shift.
- **`Result` / `Option` → status codes.** Invalid precision and precision
  mismatch are `hll_status` values; `count` is infallible.
- **Saturating cast.** `count` clamps the estimate before the `double`→`size_t`
  cast (Rust's `as usize` saturates), so it is defined even where `size_t` is
  32-bit and the estimate approaches `2^32`.
- **Total helpers.** The free functions clamp an out-of-range precision into
  `[4, 16]` rather than performing an out-of-range shift (the Rust would panic).

## Build & test

Pure ISO, no OS, no link libraries.

```sh
cd code/packages/c/hyperloglog
sh tools/run.sh        # macOS / Linux (Windows: tools\run.ps1 via BUILD_windows)
```

Locally (macOS): 50 checks / 0 failed under gcc + clang with `-pedantic-errors`;
clean under ASan+UBSan; 0 leaks.

## Layout

```
hyperloglog/
├── include/hyperloglog/hyperloglog.h   # public API
├── src/hyperloglog.c                     # the estimator — one pure-ISO source
├── tests/hyperloglog_test.c              # the Rust tests + edge/NULL paths
├── tools/run.sh  · run.ps1                 # build via iso-harness (+ hash-functions, float-math)
├── BUILD  · BUILD_windows                  # deps: c/iso-harness c/hash-functions c/float-math
└── .gitignore
```
