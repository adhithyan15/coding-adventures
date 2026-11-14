# Changelog

## 0.1.1 — Warrant WX-G pilot evidence (VER04k section 4)

- Added `code/specs/zeroize.warrant` and `evidence.bindings`: a generator-path pilot of
  `inout` parameters and `ensures` postconditions against `zeroize_bytes` (`&mut [u8]`),
  `<[u8; 32] as Zeroize>::zeroize`, `<u64 as Zeroize>::zeroize` and
  `<Vec<u8> as Zeroize>::zeroize`: every byte becomes 0, the length is unchanged, the
  result is idempotent (the `Vec` is emptied). Domains are labelled: `Bytes<0>` and
  `Bytes<1>` full, `Bytes<2>` and `Bytes<32>` samples, `Bytes` a set of 347 plus random
  buffers of up to 64 bytes, `U64` an edge set. Sabotage (`stale-post`, `skip-last-write`,
  `cursor-off-by-one`) is caught.
- New generated test target `warrant_evidence` (dev-dependencies `test-bootstrap` and
  `sha256`); no library code changed. Not an adoption: no secret labels, report or jail,
  and the evidence cannot see dead-store elimination of a never-read buffer (CWE-14).

## 0.1.0 — 2026-04-20

- Initial release.
- `zeroize_bytes(slice: &mut [u8])` primitive: volatile stores followed
  by `compiler_fence(SeqCst)`.
- `trait Zeroize` with impls for `[u8]`, `[u8; N]`, `Vec<u8>`, `String`,
  `Option<T: Zeroize>`, and all fixed-width integers (`u8`..`u128`,
  `i8`..`i128`, `usize`, `isize`).
- `Zeroizing<T: Zeroize>` RAII wrapper that zeroizes on `Drop`. Deref
  and DerefMut to `T`. Deliberately does not implement `Debug`,
  `Display`, or `Clone`.
- `Zeroizing::into_inner()` escape hatch for cases where the secret
  must outlive the wrapper.
- `Vec<u8>` and `String` impls scrub the full `capacity()`, not just
  the live prefix, to catch stale secret material in the unused tail
  of the allocation.
- 13 unit tests: byte-slice and array wipe, integer wipe, `Option`
  wipe-and-reset, `Zeroizing` drop observed via a caller-owned buffer,
  `Zeroizing` drop during panic unwind, `Vec` capacity-tail scrub,
  `String` capacity-tail scrub, `into_inner` opt-out.
- Built for the D18 Chief-of-Staff Vault master-key store.
