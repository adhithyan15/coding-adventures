# Changelog — coding-adventures-lattice-transpiler (Lua)

## [0.1.0] — 2026-03-29

### Added

- Initial implementation of `coding_adventures.lattice_transpiler`.
- `M.transpile(source)` — parse Lattice source and compile to CSS.
  Returns `(css, nil)` on success or `(nil, error_message)` on failure.
- `M.transpile_file(path)` — read a file and transpile it.
  Returns `(css, nil)` on success or `(nil, error_message)` on failure.
- Error handling via `pcall` wrapping `lattice_parser.parse()` and
  `lattice_ast_to_css.compile()` — all errors propagated as strings.
- Busted test suite covering success path, error path, file transpilation,
  and all major Lattice features end-to-end.
