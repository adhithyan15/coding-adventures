# Changelog

## Unreleased

- Use mutable limb iteration so the exact cutoff helper passes Clippy with
  warnings denied; preserve frozen multi-limb results.

## 0.1.0

- Add a finite-input sample median and exact median confidence interval.
- Validate confidence ranks against an independent bounded sign-count oracle.
