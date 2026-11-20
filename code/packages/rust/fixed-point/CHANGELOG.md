# Changelog

## Unreleased

- Add the raw signed Q16.16 API with four rounding modes, checked and saturating arithmetic, and
  deterministic shared vectors.
- Add an independent arbitrary-precision oracle, generated Warrant evaluator comparisons, seeded
  property/fuzz campaigns, and bounded DRAT-checked add/subtract SAT evidence. Source-IR proof
  dispositions remain `not-live` pending VER-P047; these finite results are not a package-wide
  source-equivalence proof.
- Check in the analyzer-generated effect certificate and verify it, along with generated capability
  metadata, in both platform build recipes.
- Generate and check the source-refusal proof manifest and report-only assurance report in both
  platform build recipes; missing Warrant step bounds remain unapproved for CWE-400 and CWE-407.
