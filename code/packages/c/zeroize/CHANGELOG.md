# Changelog

All notable changes to the C `zeroize` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `zeroize` crate — secure in-memory
  wiping that the compiler may not optimize away.
- `zeroize_bytes` primitive (volatile byte stores) plus `zeroize_object` and
  typed integer wipes (`zeroize_u8` … `zeroize_i64`, `zeroize_size`).
- `ZrBytes`, a growable byte buffer whose `zr_bytes_zeroize` scrubs the full
  allocated capacity (mirroring the Rust `Vec<u8>` impl) before clearing the
  length; allocation guarded against `size_t` overflow.
- Faithful divergences: relies on volatile stores alone (no `compiler_fence`,
  for MSVC portability); `Zeroizing<T>` / `Option` are language features left to
  the C++ port; 128-bit integers omitted (not in pure ISO C).
- Verified the guarantee holds at `-O3` (a dead `memset` is eliminated; the
  volatile loop's stores survive).
- 28 checks (byte/array/object/integer wipes and the capacity-scrubbing
  buffer), run under every available C compiler via the shared `iso-harness`;
  the suite also passes under ASan + UBSan.
