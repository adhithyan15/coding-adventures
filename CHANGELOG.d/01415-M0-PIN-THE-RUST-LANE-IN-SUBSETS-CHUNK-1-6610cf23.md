### M0: pin the rust lane in subsets (chunk 1 of 5)

- Every `cargo` invocation in the 344 `code/packages/rust/*/BUILD` and
  `BUILD_windows` files in this subset now runs with `--locked`.
- Chunk 1 of 5. Chunk 0 landed as #19872; this is the next 350 packages by
  dependency-closure size.
- `cargo tarpaulin` and `cargo fmt` are deliberately NOT locked: they measure
  and format rather than resolve a build.
- The four generated `vos-sched-*` BUILDs are excluded; they are pinned at
  their generator separately.
- `code/kiln/legacy/pinned.json` does not gain `rust` yet: it lands in the last
  subset.
- Verified: the real build tool reports 344 changed packages and 478 affected
  (including prerequisites); no generated BUILD changes.
