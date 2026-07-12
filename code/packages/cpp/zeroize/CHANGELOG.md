# Changelog

All notable changes to the C++ `zeroize` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `zeroize` crate (namespace
  `ca::zeroize`) — secure in-memory wiping the compiler may not elide.
- `zeroize_bytes` primitive: volatile byte stores plus
  `std::atomic_signal_fence(seq_cst)` — the direct equivalent of Rust's
  `compiler_fence(SeqCst)`.
- `zeroize(T&)` overloads for integers, `std::array<uint8_t, N>`,
  `std::vector<uint8_t>`, `std::string`, and `std::optional<T>` (extensible via
  ADL), and the `Zeroizing<T>` RAII wrapper (wipe on destruction, move-only,
  `into_inner` opt-out) — the C++ analogue of Rust's `Drop`.
- Faithful divergences: the `std::vector`/`std::string` overloads scrub the live
  `size()` bytes rather than the full capacity (capacity bytes are not live
  objects in C++ and touching them is UB / sanitizer-flagged; use a
  `std::array` or the C `ZrBytes` for capacity scrubbing).
- 16 checks mirroring the Rust crate's tests (array/int/vector/string/optional
  wipes, `Zeroizing` deref, drop-wipes-an-observable-buffer, `into_inner`),
  run under every available C++ compiler via the shared `iso-harness`; the
  suite also passes under ASan + UBSan.
