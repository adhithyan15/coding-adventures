# Changelog

All notable changes to this package will be documented in this file.

## [0.6.0] - 2026-04-28

### Added

- finite `sorto` relation for sorting proper lists and removing duplicates
- finite `msorto` relation for sorting proper lists while preserving duplicates
- pytest coverage for sorted list output and improper-list rejection

## [0.5.0] - 2026-04-28

### Added

- finite `lengtho` relation for counting proper lists and generating known-size
  list skeletons
- pytest coverage for counting, validation, skeleton generation, and rejection
  of improper lists or negative lengths

## [0.4.0] - 2026-04-18

### Added

- sequence relations `lasto` and `subsequenceo`
- pytest coverage for final-element reasoning and ordered-subsequence search

## [0.3.0] - 2026-04-18

### Added

- structural list relations `listo` and `reverseo`
- pytest coverage for proper-list validation, improper dotted-pair rejection,
  and list reversal

## [0.2.0] - 2026-04-18

### Added

- combinatorial relations `selecto` and `permuteo`
- pytest coverage for element selection and permutation enumeration

## [0.1.0] - 2026-04-18

### Added

- the first `logic-stdlib` package on top of `logic-engine`
- relational list helpers: `emptyo`, `conso`, `heado`, `tailo`, `membero`, and `appendo`
- pytest coverage for list construction, deconstruction, membership, and concatenation
