# Changelog

## Unreleased

- Add certificate-bound selection that derives AIA only through exact
  critical-extension and role-policy evidence, then retains URI components
  borrowed from that certificate's encoded bytes.
- Add payload-free missing-extension, structural-AIA, and locator-policy
  failures for the stronger composition.

## 0.1.0

- Add allocation-free, data-driven validation and unique selection for strict
  DNS-hosted OCSP locators discovered through X.509 Authority Information
  Access.
