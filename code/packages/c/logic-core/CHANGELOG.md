# Changelog

All notable changes to the C `logic-core` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-13

### Added

- Initial pure-ISO C17 port of the Rust `logic-core` crate — terms,
  substitutions, and first-order unification (the data layer of a logic
  programming engine).
- Owned term tree with constructors `lc_atom`, `lc_int`, `lc_float`,
  `lc_string`, `lc_var_fresh` / `lc_term_var`, `lc_compound`, `lc_logic_list`
  (Prolog `'.'/2` cons cells); `lc_term_clone`, `lc_term_equal`,
  `lc_term_to_string`, `lc_term_free`.
- Substitutions: `lc_subst_empty`, `lc_subst_extend` (copy-on-extend, never
  mutates), `lc_subst_walk` / `lc_subst_walk_var`, `lc_subst_len`,
  `lc_subst_equal`, `lc_subst_free`.
- `lc_unify(a, b, s)` — first-order unification with the occurs-check; returns a
  new substitution or `NULL`. Numbers do not cross variants (`1` ≠ `1.0`).
- Faithful divergences: `static` variable-id counter (vs Rust `AtomicU64`),
  fixed inline variable-name buffer, `%g` float display.
- 23 checks mirroring the Rust crate's own unit tests, run under every available
  C compiler via the shared `iso-harness`; the suite also passes clean under
  AddressSanitizer + UndefinedBehaviorSanitizer.
