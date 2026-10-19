# Changelog

## 0.1.0 — 2026-10-18

- Added allocation-free recognition of the five RFC 5280 `DirectoryString`
  alternatives, including non-empty opaque retention for TeletexString.
- Added exact Unicode-scalar iteration for UTF8String, PrintableString,
  UniversalString, and BMPString while retaining TeletexString as opaque.
- Added fail-closed, representation-independent scalar equality that refuses
  TeletexString rather than inventing a T.61 mapping.
