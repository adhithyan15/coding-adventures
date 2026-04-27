# Changelog

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
