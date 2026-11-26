## 0.2.0 — first registry release (VER31 tier Tested)

- First version published to npm, as `@coding-adventures/ct-compare`. The earlier
  `0.1.0` and `0.2.0` entries were development labels; nothing was published
  under them. The registry 0.2.0 contains every change in this directory.
- Released at VER31 release tier **Tested**. Every claim is `tested`: this is the
  TypeScript lane of `rust/ct-compare`'s report, whose lane row is `live`, and
  lanes carry no proofs (VER31 section 6.2). The package ships that report
  verbatim as `assurance_report.json`, plus the generated `ASSURANCE.md`.
- Ships compiled JavaScript: `main`, `types` and `exports` point at `dist/`
  (built by `tsc` with declarations), and `files` lists exactly what is
  published. `src/index.ts` is no longer the entry point.
- New property suite (`npm run test:property`, VER05a `@coding-adventures/pbt`):
  agreement with an early-exit oracle on generated inputs up to 512 bytes,
  reflexivity and symmetry, `ctSelectBytes` returning a fresh copy of the chosen
  side, and `ctEqU64` agreement, 2,048 cases each with fixed seeds.
- New fuzz suite (`npm run test:fuzz`, VER06b `@coding-adventures/fuzz`): 4,096
  coverage-guided iterations over all four functions, including malformed plain
  arrays that must be refused with a `RangeError`.
- New microbenchmarks (`npm run bench`) and `evidence/bench-manifest.json`, which
  the BUILD regenerates. Timings go to stdout only, so the manifest reproduces
  byte for byte.
- **Security fix (SEC-012).** A genuine `ArrayBuffer` input was wrapped, not
  copied. Converting the other argument can run caller code (a Proxy `length`
  trap, an element getter) that detached, shrank or rewrote the buffer
  afterwards, so two different inputs compared equal. `toBytes` now copies the
  buffer at conversion. `tests/snapshot.test.ts` covers detach, resizable
  shrink, rewrite and `ctSelectBytes`; the lane evidence is regenerated.
- MIT `LICENSE`, `repository` and `publishConfig.access: public` in
  `package.json`; `security-review.json` records the review of the shipped bytes.
