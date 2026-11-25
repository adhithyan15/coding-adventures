## Unreleased — `profile algorithm` header and `realizes` binding field (VER33 ADL-001a)

- Grammar: `warrant NAME [ v1 ] [ profile NAME ]`, one new rule `profile_clause` and one
  new reserved word `profile` in `code/grammars/warrant/warrant.{tokens,grammar}`;
  `_tokens.rs`/`_parser.rs` regenerated; the WX-D pin in `tests/wxd_syntax.rs` re-pinned
  with the reason (VER33 is the specification).
- Parser: `Profile` (`Pure` default, `Algorithm`) and `WarrantFile::profile`. A profile
  needs `v1`; anything else is `W0801`. In an algorithm document the first refused word in
  source order is reported at its span: `W0802` `measure`, `W0803` unbounded loops and
  recursion, `W0804` floats, `W0805` effects and state. A pure document is never scanned,
  and the v0 pins (`tests/v0_compat.rs`) are untouched: `CheckedWarrantModule` did not grow.
- Checker: the same restriction over the AST (`DiagnosticCode::W0801..W0807`,
  `CheckerErrorKind::ProfileRefusal`/`InvalidRealization`).
- Bindings: `realizes=MODULE.MODEL embedding=...` and `panic OP TEMPLATE` lines in
  `evidence.bindings`; `EvidenceBindingFile::realizations`; closed `EMBEDDINGS`;
  `check_panic_template` (empty, wildcard, duplicate, more than 16, compiler-generated are
  `W0807`); `validate_realizations` with `ModelModule` (missing function/module, model not
  in `profile algorithm`, arity or type mismatch are `W0806`). Schema and validation only.
- Tests: `tests/profile_algorithm.rs` over `code/specs/fixtures/warrant-profile-algorithm-v1/`
  (document and binding cases) plus span, embedding-set, template and hand-built-AST
  tests; four header cases in the syntax corpus (57 to 61).
- This changelog is now sharded: the former `CHANGELOG.md` is `00010-history-before-sharding.md`.
