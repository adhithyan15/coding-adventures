# Changelog

## 0.1.0 — 2026-09-19

- Added zero-external-dependency, allocation-free DER identifier and
  definite-length TLV framing.
- Added strict rejection of BER indefinite lengths, reserved and non-minimal
  lengths, non-minimal and overflowing high-tag numbers, and universal EOC.
- Added configurable input, value, tag-number, and sibling-element limits with
  checked offsets and redacted errors.
- Added exact single-element and non-advancing bounded cursor APIs plus
  adversarial boundary coverage.
