# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Initial release — HML01 Stream B rollout, the item right after
  `macsyma-to-semantic-ir`: a direct `pub use` re-export of
  `macsyma_to_semantic_ir::{compile, compile_source, MacsymaLowerError}`
  under Maxima's own crate name. No new grammar, no new SIR node kinds, no
  shim function at all — Maxima and Macsyma share the exact same algebraic
  surface (`maxima-runtime`'s own doc comment: "a program written for one
  runs on the other"), a stronger guarantee than Octave's relationship to
  MATLAB, which still needs `octave-runtime`'s `octavify` source-rewriting
  shim for a handful of real surface departures. This crate is therefore
  thinner than `octave-to-semantic-ir`: *delegate*, not *shim-then-delegate*.
- Both `compile_source` (source-in) and `compile` (already-parsed
  `GrammarASTNode`-in) are re-exported, matching every other
  `-to-semantic-ir` frontend's own pair — `octave-to-semantic-ir`
  deliberately omits `compile(tree, ...)` since its shim rewrites text, not
  a tree; Maxima has no shim, so the full pair applies unchanged here.
- 6 tests: bare arithmetic, Maxima's own `;`/`$` display/suppress
  terminator pair (unchanged from Macsyma), `diff`/`integrate` builtin
  bridging, a `block`/`for`/control-flow program that also passes
  `semantic_ir::validate`, malformed-input rejection, and a symmetry check
  confirming both `compile` and `compile_source` are reachable under this
  crate's own name.
