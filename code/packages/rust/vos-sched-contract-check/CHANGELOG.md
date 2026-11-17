# Changelog

All notable changes to this package are documented here.

## [Unreleased]

### Added (VOS-D081)

- `safety_init` joins `PRODUCTION` (13) and `OBLIGATIONS` (16) and the count tables (2 cases).
- Two mutants (21 in all): a bitmap with initial mask 8 and the safety block only, and `-any` with
  mask 8 and oracle 0 (`dispatch` still answers `Idle`); each fails exactly `safety_init`. The
  existing `init() == Impl(1)` mutant now fails `canon_init` and `safety_init`.

### Changed (VOS-012a PR, VOS-D079)

- The any-tier count test expects `dispatch_covers_run` at 65,536 admitted cases (the `t < 8`
  refinement is gone); the four `any` mutants still fail exactly the claims they should.

### Changed (VOS-011 PR, VOS-D064)

- The lemmas are evaluated from the contract's new `warrant/lemmas.part` (joined after the
  contract block); the contract block alone must hold no claim.  `parts_follow_the_marker_discipline`
  admits `lemmas.part` and checks that it holds exactly the nine lemmas.  The lemma mutants are kept.

### Changed (VOS-033, backlog VOS-D049)

- `parts_follow_the_marker_discipline` now applies `vos_manifest::part::admit_part` (the rules
  `vos-link gen-doc` enforces) and `claims_of_block` to the five real parts, instead of
  re-implementing the rules.  New dependency: `vos-manifest`.
- The string-joining `assemble` is documented as evaluation-only; real document assembly and
  checking are `vos-link`'s.  The crate stays: its ground evaluation and mutation tables are a
  semantic suite, not a document check.

## [0.1.0] - Unreleased

### Added

- VOS-010: Warrant-level validation of `vos-sched-contract` (type check, lemma
  evaluation over whole domains, obligations against three model implementations,
  negative controls, Rust/Warrant name agreement). See `README.md`.
