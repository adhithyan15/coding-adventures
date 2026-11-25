## 0.1.0 — first registry release (VER31 tier Tested)

- First version published to crates.io. The `0.1.0 — 2026-04-20` heading in
  `00010-pre-registry-history.md` was a development label; nothing was ever
  published under it. The registry 0.1.0 contains every change listed there,
  including all the `Unreleased` entries.
- Released at VER31 release tier **Tested**, computed by
  `code/scripts/release_gate.py` from `assurance_report.json` and
  `code/assumptions/buckets-v1.json`. Claim evidence: `ct_eq`, `ct_eq_fixed`
  and `ct_select_bytes` at `bounded(128)`; `ct_eq_u64` at `checked` (its
  generator label is `proved`, but 20 of this repository's own tool
  assumptions are still open). The tier is capped at Tested because the report
  verdict is `report-only` (E2 not live; five unapproved exemptions).
- Packaging: `license = "MIT"` and a `LICENSE` file; `repository`, `readme`,
  `keywords` and `categories`; an explicit `include` list that ships
  `assurance_report.json` verbatim and the generated `ASSURANCE.md` (every
  assumption, grouped by VER31 bucket); `security-review.json` records the
  review of the shipped bytes.
- **Security fix (SEC-013).** At `-C opt-level=3`, `ct_select_bytes` compiled to
  `select i1 %choice, ptr %a, ptr %b` plus `memcpy`: it read only the chosen
  slice, so the memory addresses it touched depended on the secret `choice`.
  The optimizer could do this because it knew the mask was 0x00 or 0xFF. The
  mask now passes through `core::hint::black_box` (the proof pipeline lowers it
  to identity; a volatile read of a local would not lower), and
  `tests/optimized_ir.rs::optimized_select_reads_both_inputs_at_public_addresses`
  fails on any pointer select, `memcpy`/`memmove` or branch on `choice` in the
  optimized IR. Evidence manifests, the effect certificate, the assurance
  report and `ASSURANCE.md` are regenerated for the new source.
- The monolithic `CHANGELOG.md` became this sharded `CHANGELOG.d/`.
