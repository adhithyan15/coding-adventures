# Changelog

## Unreleased

- Added a tag-strict RFC 5280 GeneralizedTime syntax entry point for RFC 6960
  OCSP timestamps, including present-day years.
- Preserved the existing certificate-validity decoder's 2050 cutoff and
  UTCTime/GeneralizedTime selection policy.

## 0.1.0 — 2026-09-19

- Added allocation-free RFC 5280 UTCTime and GeneralizedTime decoding.
- Added fixed century and tag-selection policy plus Gregorian calendar
  validation.
- Added redacted local-offset errors, explicit field comparison, and
  adversarial boundary tests.
