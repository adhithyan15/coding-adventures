### M0: pin the rust lane (3,566 `cargo` lines now `--locked`)

- Every `cargo test`/`run`/`build`/`clippy`/`doc`/`check`/`rustdoc` invocation in
  the 1,538 `code/packages/rust/*/BUILD` files and their `BUILD_windows` now
  passes `--locked` — 3,566 lines across 2,134 files — plus the 49 packages
  that build through `code/packages/starlark/library-rules/rust_library.star`.
  A lockfile that no longer matches its manifest now fails the build instead of
  silently re-resolving and downloading different third-party code.
- `tarpaulin` and `fmt` are left unlocked on purpose: they measure and format
  rather than resolve a build, and the same packages' `test` and `build` lines
  already fail on a stale lockfile. Locking them would risk a flag the tool does
  not accept for no soundness gain.
- `pinned.json` gains `rust` (now `["rust", "wasm"]`). Verified current:
  the workspace `Cargo.lock`, `os-kernel`'s own lockfile (excluded from the
  workspace, different target), and `programs/rust/vos-link`, which a BUILD runs
  by manifest path — all satisfy `cargo metadata --locked`.
- Measured on the real graph (5,779 packages): 1,698 packages change class or
  reasons, and the only difference in every one is that `unpinned:rust` is gone;
  no other reason appears and no key-level input changed shape. Rust is the lane
  the wasm closures were waiting on, so `unpinned` drops from 5,775 to 4,139.
- Not claimed: still no `cacheable` package. `classify` cannot return it until a
  KILN04 traced manifest exists, and the 1,636 packages that leave `unpinned`
  land in `uncacheable` because that is where the current code puts "pinned, no
  trace yet" — the bucket's name is then wrong about them. Recorded in KILN03
  §7.1 as an open naming item rather than changed inside an M0 PR.
