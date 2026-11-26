## 0.1.0

- Initial Dart lane of the hkdf identity: legacy `hkdfExtract`/
  `hkdfExpand`/`hkdf` over `HashAlgorithm` (HMAC-SHA-256/512 plus
  keyed-BLAKE2b) and fixed-memory `hkdfExtractFixed`/
  `hkdfExpandFixed` over `FixedHkdfHash` with the wipe-on-drop
  `HkdfPrk` owner, the four-variant `HkdfError` type, and the
  payload-free `HkdfPrkImportError`; hygiene via the in-lane
  zeroize package on the fixed paths.
- Tests mirror the reference suite (45 unit tests incl. the RFC
  5869 vectors), add 5 extras, 3 differential tests over 60 seeded
  Node.js-OpenSSL vectors, 8 seeded property tests over the
  in-lane `pbt` package, and a 2,000-iteration differential fuzz
  campaign over the in-lane `fuzz` package; 100% line coverage.
