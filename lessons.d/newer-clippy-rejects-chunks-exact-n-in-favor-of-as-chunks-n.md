---
category: Rust
---

# Newer clippy rejects chunks_exact(N) in favor of as_chunks::<N>()

PR #15684 (`pdp8-simulator`) passed locally but failed `build (ubuntu-latest)`
and `build (macos-latest)` in CI (windows-2025 passed) with:

```
error: using `chunks_exact` with a constant chunk size
   --> pdp8-simulator/src/lib.rs:230:14
    |
230 |             .chunks_exact(2)
    |              ^^^^^^^^^^^^^^^ help: consider using `as_chunks` instead: `as_chunks::<2>().0.iter()`
    = note: `-D clippy::chunks-exact-to-as-chunks` implied by `-D warnings`
```

`chunks_exact(N)` called with a literal `N` now trips
`clippy::chunks_exact_to_as_chunks` starting with the clippy shipped in
rustc 1.98 (the ubuntu/macos runner images had already picked up that
toolchain; windows-2025 had not yet, which is why only two of the three
`build` jobs failed on an otherwise-identical diff). This is a moving target:
whichever runner images are on the newer toolchain fail first, so don't read
a partial pass across OSes as "OS-specific code", check the clippy version.

Fix: replace `slice.chunks_exact(N)` with `slice.as_chunks::<N>().0.iter()`
when N is a compile-time constant and exactness (no remainder) is required.
`as_chunks` is stable as of the toolchain this repo's CI runs. Each `[u8; N]`
item from `as_chunks` can be passed directly to APIs like
`u16::from_le_bytes` without re-slicing (`u16::from_le_bytes(*bytes)` instead
of `u16::from_le_bytes([bytes[0], bytes[1]])`).

Before adding a new fixed-size chunking loop in Rust, grep the crate for
`chunks_exact(` with a literal argument and prefer `as_chunks::<N>()` from
the start, rather than discovering the lint on ubuntu/macos CI after windows
already passed.
