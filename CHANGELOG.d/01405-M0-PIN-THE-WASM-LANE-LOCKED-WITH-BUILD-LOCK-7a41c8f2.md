### M0: pin the wasm lane (committed lockfiles, `--locked` builds, pinned.json)

- Every `cargo` invocation in the 75 `code/packages/wasm/*/BUILD` and
  `BUILD_windows` files now runs with `--locked` (147 lines), so a lockfile that
  no longer matches its `Cargo.toml` fails the build instead of silently
  re-resolving and downloading different third-party code.
- `gf256` and `polynomial` had **no** `Cargo.lock` at all; both now commit one.
  Their only dependency is a path crate, so the files are deterministic and
  offline-generated.
- `code/kiln/legacy/pinned.json` gains `wasm` as its first pinned ecosystem,
  which removes `unpinned:wasm` from those packages' reasons.
- Verified: all 75 crates satisfy `cargo metadata --locked`; the Kiln test suite
  passes. A real `build-tool -emit-kiln-graph` (5779 packages) followed by
  `kiln legacy-plan` with `pinned.json` empty and then with `wasm` pinned shows
  exactly 75 packages changed — every one a `wasm/` package — and the only
  difference in every case is the removal of `unpinned:wasm`; no key and no
  other package's plan changed. `wasm/tcp-client`, the one wasm crate with no
  Rust dependency in its closure, moves from `unpinned` to `uncacheable`, which
  is the documented fall-through while `cacheable` is unreachable.
- Not claimed: no package becomes `cacheable` from this. Pinning is decided per
  closure, and each wasm crate depends on a `rust` crate, so they still report
  `unpinned:rust`; and `cacheable` is unreachable until a KILN04 traced manifest
  exists. Pinning `rust` is the next M0 step.
