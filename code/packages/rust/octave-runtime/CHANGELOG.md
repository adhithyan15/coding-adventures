# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-06-16

### Added

- Initial release — **MA-3e**, GNU Octave as a source-compatibility shim over
  `matlab-runtime` (spec [`MA01`](../../../specs/MA01-matlab-language.md) §5).
- `octavify(source)`: rewrites `#` comments → `%`, the `endX` block terminators
  (`endif`/`endfor`/`endwhile`/`endfunction`/`endswitch`/`endparfor`/
  `end_try_catch`) → `end`, and `!`/`!=` → `~`/`~=`, leaving string and comment
  contents untouched (transpose-vs-quote aware).
- `Interpreter` + `eval` that `octavify` then delegate to `matlab-runtime`, so
  the matrix engine (incl. `*` lowering to `array-runtime::execute`), indexing,
  builtins, and control flow are inherited unchanged.
- Deferred: `++`/`--`, `do…until`.
