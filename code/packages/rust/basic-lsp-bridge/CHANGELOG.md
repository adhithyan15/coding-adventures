# Changelog — `basic-lsp-bridge`

## 0.2.0 — 2026-06-01 (wires basic-formatter as the format_fn)

basic-formatter 0.1.0 (BASIC-FMT01) is now wired in as the
`textDocument/formatting` provider.  Editors driving the BASIC
language server will get "Format Document" + "Format Selection"
that produce canonical BASIC.

### Changed

- `basic_language_spec().format_fn` flipped from `None` to
  `Some(basic_format_wrapper)`.
- Added `basic_format_wrapper(&str) -> Result<String, String>` that
  adapts `basic_formatter::format`'s `FormatError` into the
  string-error shape `LanguageSpec::format_fn` expects.
- Two new tests (`format_fn_is_set`, `format_fn_uppercases_keywords`).

### Dependencies

- Added `basic-formatter` 0.1.0 as a path dependency.

## 0.1.0 — 2026-06-01 (BASIC-LSP01 — initial Dartmouth BASIC LSP bridge)

Initial release.  Sibling of `twig-lsp-bridge` 0.2.0 — first crate
in the BASIC LSP phase (task #41 part 1 of 3).

### What's wired

- Embeds `code/grammars/dartmouth_basic.tokens` and
  `code/grammars/dartmouth_basic.grammar` via `include_str!`.
- `basic_language_spec()` returns a static `LanguageSpec` with:
  - Token kind map covering KEYWORD / LINE_NUM / NUMBER / STRING /
    NAME / BUILTIN_FN / USER_FN / operators.
  - `declaration_rules: &["def_stmt"]` — user-defined functions
    (`DEF FNa = …`) become document symbols.
  - All 20 BASIC reserved keywords (LET, PRINT, INPUT, IF, THEN,
    GOTO, GOSUB, RETURN, FOR, TO, STEP, NEXT, END, STOP, REM,
    READ, DATA, RESTORE, DIM, DEF) for completion + hover.
  - `format_fn: None` — formatter integration follows in a
    separate PR (basic-formatter).
- `basic-lsp-server` binary wires the spec into
  `grammar_lsp_bridge::GrammarLanguageBridge` and runs
  `coding_adventures_ls00::server::LspServer::serve()` over stdio.

### Token kind map

| BASIC token | LSP semantic type |
|-------------|-------------------|
| `KEYWORD`   | `Keyword`         |
| `LINE_NUM`  | `Number`          |
| `NUMBER`    | `Number`          |
| `STRING`    | `String`          |
| `NAME`      | `Variable`        |
| `BUILTIN_FN`| `Function`        |
| `USER_FN`   | `Function`        |
| `EQ`/`LE`/`GE`/`NE`/`LT`/`GT`/`PLUS`/`MINUS`/`STAR`/`SLASH`/`CARET` | `Operator` |

`LPAREN` / `RPAREN` / `COMMA` / `SEMICOLON` / `COLON` are
intentionally absent — punctuation gets no colour.

### Tests

- 8 unit tests covering spec sanity, file-extension detection,
  declaration-rule presence, keyword coverage, bridge
  construction, tokenize/parse, and the format_fn default.
