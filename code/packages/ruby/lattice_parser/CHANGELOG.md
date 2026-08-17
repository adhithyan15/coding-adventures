# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `parse`/`create_parser` now load the pre-compiled `_grammar.rb` (via `GrammarTools::CompiledLoader`) instead of reading and parsing `lattice.grammar` from `code/grammars/` on every call. The old code walked out of the installed gem's own directory to a monorepo-relative path that a published gem does not ship, so a real `gem install` + first use would raise `Errno::ENOENT`.

## [0.1.0] - 2026-03-23

### Added

- `CodingAdventures::LatticeParser.parse(source)` — lexes and parses
  Lattice source text, returning the root `ASTNode` (rule_name `"stylesheet"`).
- `CodingAdventures::LatticeParser.create_parser(source)` — returns the
  `GrammarDrivenParser` instance for inspecting the grammar state.
- Full Lattice grammar support: variables, mixins (`@mixin`/`@include`),
  control flow (`@if`/`@else`/`@for`/`@each`), functions (`@function`/
  `@return`), modules (`@use`), and all CSS3 constructs.
- Grammar update: `mixin_definition` accepts both `FUNCTION` form
  (`@mixin button($bg) { ... }`) and `IDENT` form (`@mixin centered { ... }`).
- Grammar update: `function_definition` accepts both FUNCTION and IDENT forms.
- Grammar update: `lattice_control` is now valid at the top level of a
  stylesheet (matching Sass/SCSS behaviour for top-level `@if`/`@for`).
- Grammar path resolved via 6-level relative path to `lattice.grammar`.
