# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial release — Stream A rollout item 5
  ([`HML01`](../../../specs/HML01-math-to-semantic-ir.md) §5): a thin
  wrapper reusing `octave-runtime`'s `octavify` source-compatibility shim
  and `matlab-to-semantic-ir::compile_source` wholesale. No new grammar, no
  new SIR node kinds — Octave's departure from MATLAB is a small, local set
  of surface forms (`#` comments, `endif`/`endfor`/`endwhile`/
  `endfunction`/`endswitch`/`end_try_catch`, `!=`/`!`), not a different
  language.
- Single public entry point, `compile_source(source, module_name) ->
  Result<semantic_ir::Module, MatlabLowerError>` — deliberately no
  `compile(tree, ...)` variant, since there is no Octave-specific CST to
  hand in (the shim normalizes text, not a tree).
- 9 tests: the shim-then-delegate wiring for every normalized construct
  (`#` comments, all six `endX` terminators, `!=`/`!`), a string/comment-
  awareness regression (`#`/`!` inside a string literal must not be
  rewritten), a plain-MATLAB-passthrough sanity check, error propagation
  for both an out-of-scope MATLAB construct and an Octave-only construct
  `octavify` does not normalize (`do...until`), and one full end-to-end
  test exercising every shim rule at once through `semantic_ir::validate`.
