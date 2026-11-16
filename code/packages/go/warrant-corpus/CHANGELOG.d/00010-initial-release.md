## 0.1.0

- Initial Go lane runner for the `warrant-corpus-v1` evidence corpus (VER04i,
  WD-103), mirroring the Python (WD-101) and TypeScript (WD-102) lanes check for
  check: strict corpus loader with its own JSON reader (duplicate keys, invalid
  UTF-8, unpaired surrogate escapes, non-safe-integer numbers and over-deep
  nesting refused), in-lane domain enumeration with `warrant-stream-v1` digests
  cross-checked against the corpus, a model-IR interpreter that compiles to
  closures (no `eval`, no plugin loading), the binding table
  (`evidence.bindings`), the ten portable fault wrappers, example, exhaustive and
  negative-control suites, the `warrant-lane-evidence-v1` manifest writer and
  drift check, the `domain-labels.json` sidecar (backlog LP-012), the
  `sabotage` command and a structural manifest validator.
- Go cannot import a package by name at run time, so the entry program passes a
  compiled registry of adapters. Each export's runtime symbol name and source
  file are checked against the binding's `import` and `source`
  (`source-mismatch`); errors map by sentinel identity, not by class or message.
- `U64` is a native `uint64`; `Bytes` results must be exactly `[]byte`; a `fresh`
  result is checked by mutation for sharing with an argument.
- Corpus, binding and package paths must lie strictly inside `--repo-root`, using
  a cross-drive-safe containment check that treats `filepath.Rel`'s error as a
  refusal and tests `..` or `..<sep>`, never a bare `..` prefix. The label digest
  binds to the manifest's own `toolchain.name`.
- `runner.src_sha256` digests the runner's own source as embedded at compile time.
- Standard library only; no dependency on any library it tests.
