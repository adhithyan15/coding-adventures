# Changelog

All notable changes to the `hyperloglog` (C) package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

### Added

- **Initial package — approximate cardinality estimator** (CCPP02 port campaign,
  bucket A / pure-ISO, port #3). The C port of the Rust `hyperloglog` crate
  (DT21): HyperLogLog distinct-count estimation in a tiny fixed footprint. A
  pure-ISO crate (no OS), so it rides the `iso-harness` (links nothing,
  `-pedantic-errors` / `/permissive-`).
  - **API.** `hll_create` (precision `[4,16]`, else `HLL_ERR_INVALID_PRECISION`) /
    `hll_create_default` (14) / `hll_destroy`; `hll_add_bytes` / `hll_add_str`;
    `hll_count` / `hll_is_empty`; `hll_merge` (register-wise max of equal-precision
    sketches, else `HLL_ERR_PRECISION_MISMATCH`); `hll_precision` /
    `hll_num_registers`; and the free functions `hll_error_rate[_for_precision]`,
    `hll_memory_bytes`, `hll_optimal_precision`.
  - **Composes two pure-ISO packages.** The 64-bit FNV-1a hash comes from
    `c/hash-functions` (`hf_fnv1a_64`) and the elementary math (`ln` / `sqrt` /
    `log2` / `ceil` / `round`) from `c/float-math` (`fm_*`) — both compiled in by
    `run.sh`, so the estimator needs no libm and links nothing (the lane's
    no-libm rule). `BUILD` declares
    `deps=c/iso-harness c/hash-functions c/float-math`.
  - **Faithfulness.** Same pipeline: `fmix64(fnv1a_64(bytes))` → top-`p` bits pick
    the bucket, leading-zero run + 1 is `rho`, register max-update; `count` uses
    the harmonic mean with linear-counting and large-range corrections. `2^(-r)`
    is the exact `1 / 2^r` (integer shift, no `pow`). `Result`/`Option` →
    `hll_status`. A portable `clz64` (no compiler builtin) keeps it MSVC-clean.
  - **Hardening (from adversarial security review).** The `double`→`size_t` cast
    in `count` saturates via `!(estimate < (double)SIZE_MAX)` — defined even at
    the `2^64` boundary and on a 32-bit `size_t` (Rust's `as usize` saturates).
    The free functions clamp an out-of-range precision into `[4,16]` rather than
    shifting past the width of `size_t` (UB). Register indexing is provably
    in-bounds (`bucket < 2^precision = nregisters`); OOM paths in `create`/`merge`
    unwind cleanly.
  - **Test (`tests/hyperloglog_test.c`).** The Rust tests (empty→0, duplicates
    stay tiny, 1000 distinct items estimate in `[900,1100]`, merge unions
    registers, precision-mismatch error, the public helpers, invalid precision
    rejected) plus binary/empty inputs and the NULL-argument paths. 50 checks,
    verified under gcc + clang with `-pedantic-errors`, clean under ASan+UBSan,
    0 leaks.
