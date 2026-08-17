# Changelog

All notable changes to `coding_adventures_starlark_parser` will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `parse` now loads the pre-compiled `_grammar.rb` (via `GrammarTools::CompiledLoader`) instead of reading and parsing `starlark.grammar` from `code/grammars/` on every call. The old code walked out of the installed gem's own directory to a monorepo-relative path that a published gem does not ship, so a real `gem install` + first use would raise `Errno::ENOENT`.

## [0.1.0] - 2026-03-19

### Added
- Initial release
- `CodingAdventures::StarlarkParser.parse(source)` method that parses Starlark source code into ASTs
- Loads `starlark.grammar` and delegates to `GrammarDrivenParser`
- Supports assignments: `x = 1 + 2`
- Supports augmented assignments: `x += 1`, `x -= 1`, `x *= 2`
- Supports operator precedence via 15-level grammar rule hierarchy
- Supports compound statements: `if`/`elif`/`else`, `for`, `def`
- Supports simple statements: `return`, `break`, `continue`, `pass`, `load`
- Supports parenthesized expressions: `(1 + 2) * 3`
- Supports multiple statements separated by newlines
- Supports string literals, numeric literals, and variable references
- Supports function definitions with parameters (positional, default, *args, **kwargs)
- Full test suite with SimpleCov coverage >= 80%
