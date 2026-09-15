## Unreleased — 2026-09-13 — VM-040 Dartmouth BASIC BEAM pure-string family

With all 58 COBOL-60 rows now BEAM-declared, a fresh non-ALGOL reprioritization
asked whether any other frontend still had undeclared BEAM rows the way COBOL
did. Dartmouth BASIC was the only one at zero: `iir-to-beam` has no `f64`
lowering at all, and BASIC routes every scalar numeric value — even an
integer-spelled literal — through the shared `f64` value track (BA7-1b), so
its `__basic_print_real`/`__basic_print_fixed_mag`/`__basic_print_real_e`
helpers (unconditionally emitted into every compiled module) poison whole-module
BEAM validation for nearly the entire corpus.

Exactly 18 of the 51 Dartmouth BASIC rows are purely string-valued (no numeric
`PRINT`/`LET`/`FOR`/`INPUT` anywhere in source, so no float `const` is ever
emitted) and needed zero new `iir-to-beam` lowering. All 18 now declare `Beam`
and pass on real Erlang with byte-identical stdout to every other backend
(`portable_text_stdout_dartmouth_basic_beam_strings`). The remaining ~28
numeric rows still need new `iir-to-beam` float-op lowering — a separate,
properly scoped item — and the 5 `INPUT` rows need the still-unscoped BEAM
host-input design (VM-060b), shared with FLOW-MATIC's 4 blocked `INPUT`/EOF
rows. See `code/specs/LANG-VM-NON-ALGOL-BACKLOG.md` (VM-040 Dartmouth BASIC
BEAM pure-string family / VM-D033) and `code/specs/LANG-VM-FEATURE-COVERAGE.md`
for the full investigation and updated counts (Dartmouth BASIC: 357 → 375
declared cells; non-ALGOL total: 1593 → 1611).

