# Changelog — TypeScript 4.0 (2020) Parser

## 0.1.2 (2026-04-05)

- Add `BUILD_windows` for Windows CI compatibility (unquoted `.[dev]`, `uv run` instead of `.venv/bin/python`)
- Fix `read_text()` to use `encoding="utf-8"` explicitly (prevents `UnicodeDecodeError` on Windows cp1252)

## 0.1.1 (2026-04-05)

### Fixed

- Moved `left_hand_side_expression assignment_operator assignment_expression`
  before `conditional_expression` in ts4.0.grammar so short-circuit assignment
  operators (`&&=`, `||=`, `??=`) are tried before collapsing to a plain expr
- Added `AND_AND_EQUALS`, `OR_OR_EQUALS`, `NULLISH_EQUALS` to `assignment_operator`
- Fixed `tuple_element` rule to handle labeled optional (`label?: type`),
  labeled rest (`...label: type`), and unlabeled rest (`...type`) elements
- Updated all function rules to use `typed_parameter_list` (TypeScript-style
  typed parameters) instead of `formal_parameters` (plain ES params)
- Fixed `variable_declaration` and `lexical_binding` to support type annotations
- Fixed `arrow_parameters` for return type annotations and generics
- Added `ambient_function_declaration` rule (bodyless `declare function`)

## 0.1.0 (2026-04-05)

### Added

- Initial implementation of the TypeScript 4.0 (2020) parser
- Thin wrapper around `GrammarParser` loading `ts4.0.grammar`
- Public API: `create_ts40_parser(source)`, `parse_ts40(source)`
- Comprehensive test suite verifying AST structure for TS 4.0 features
- Tests for variadic tuple types, labeled tuple elements, short-circuit assignment
- Tests for TS 3.0 compatibility (`unknown` type), type annotations, classes, control flow