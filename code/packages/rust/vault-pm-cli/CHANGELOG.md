# Changelog

## Unreleased

- Added reversible authenticated `item delete ITEM` and
  `history restore ITEM REVISION` mutations with strict item-bound selectors,
  causal tombstones, and restore-as-new-revision semantics.
- Added authenticated `history list ITEM` with canonical revision selectors,
  newest-first causal metadata, and redacted record titles.
- Added revision-safe `item edit ITEM` for complete login-field replacement
  while preserving identity, metadata, notes, and causal history.
- Added strict `item add login`, `item list`, and `item show ITEM` commands.
- Added controlling-terminal item input, fresh mutation identities, durable
  application publication, escaped redacted rendering, and restart coverage.
- Added one-shot authenticated `audit verify` with aggregate-only output.
- Added opt-in full repository health verification through `doctor --unlock`.
- Added strict parser, wrong-passphrase, synchronous re-lock, and real-process
  controlling-terminal coverage for authenticated verification.

## 0.1.0

- Added the closed `init`, `status`, and `doctor` command grammar.
- Added stable exit classes and payload-free text/JSON rendering.
- Composed secure local roots, exact configuration, durable application state,
  immutable filesystem storage, fixed terminal prompts, and OS entropy.
- Added crash-resumable generation-zero activation and restart tests.
