### Changed - deterministic continuity complexity regression (#12322)

- Replace the adversarial whole-word matcher's wall-clock budget with exact
  candidate-check and skipped-run counts, preserving the nonlinear-regression
  gate without making it sensitive to parallel Windows runner load.

