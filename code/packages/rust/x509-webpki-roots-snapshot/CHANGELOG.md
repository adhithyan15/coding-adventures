# Changelog

## 0.1.0 — 2026-10-18

- Added the reviewed `webpki-roots` 1.0.9 Mozilla public-Web-PKI trust-anchor
  snapshot as repository data without compiling `webpki-roots` or its
  `rustls-pki-types` dependency.
- Added an allocation-free repository-owned parser for the canonical snapshot
  format and stable redacted failures for malformed data.
- Recorded the upstream tag and commit, crates.io archive checksum, root count,
  root-set digest, excluded platform roots, and update cadence.
- Added CI verification that fails if the checked-in root bytes, count, digest,
  provenance metadata, or generated format drift without review.
