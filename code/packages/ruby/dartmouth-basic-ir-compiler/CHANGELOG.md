# Changelog

## Unreleased

### Fixed
- `parse_line` matches the leading digit run with an anchored pattern and
  trims the remainder imperatively, avoiding the `\s*(.*)` tail that
  triggered codeql `rb/polynomial-redos` on whitespace-padded input.

## 0.1.0

- Initial Ruby Dartmouth BASIC lowering to LANG InterpreterIR.
