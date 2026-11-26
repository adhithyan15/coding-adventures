### M0: pin the rust lane in subsets (chunk 0 of 5)

- Every `cargo` invocation in the 326 `code/packages/rust/*/BUILD` and
  `BUILD_windows` files in this subset now runs with `--locked`, so a lockfile
  that no longer matches its `Cargo.toml` fails the build instead of silently
  re-resolving and downloading different third-party code.
- This is chunk 0 of 5. Whole-lane pinning is one affected set of the entire
  rust lane (~1,500 packages), and a pull-request build is unsharded, so the
  macOS job was cancelled at 6h01m (#19854). Splitting by package directory keeps
  each PR's affected set to ~350 packages, well under the cap.
- `cargo tarpaulin` and `cargo fmt` are deliberately NOT locked: they measure and
  format rather than resolve a build, and the same packages' `test` lines already
  fail on a stale lockfile.
- The four generated `vos-sched-*` BUILDs are excluded here; they are pinned at
  their generator in a separate PR, because `vos-link check-component` requires
  the generated BUILD to match byte for byte.
- `code/kiln/legacy/pinned.json` does NOT gain `rust` yet: a language is pinned
  only when every BUILD in it installs in locked mode, so it lands in the last
  subset. Until then these packages still report `unpinned:rust`.
- Verified: the real build tool reports 326 changed packages and 328 affected
  (including prerequisites); no non-rust file changes; no generated BUILD
  changes; no double-locked or tarpaulin/fmt-locked line.
