# Changelog

## 0.1.0

Initial release: RFC 4648 base64 encoding and decoding, no dependencies.

- Four variants: `STANDARD`, `STANDARD_NO_PAD`, `URL_SAFE`, `URL_SAFE_NO_PAD`.
- `encode`, `encode_into` (appends to a caller-owned buffer, for streaming
  writers), `decode`, `encoded_len`, `decoded_len_estimate`.
- Strict decoding: characters outside the alphabet, the other alphabet's
  characters, wrong or missing padding, impossible lengths, and non-canonical
  trailing bits are all rejected with a byte offset rather than repaired.
- Verified against RFC 4648 §10's vectors, a round trip over every length
  0..=1024 in all four variants, and Python's `base64` stdlib across 300
  lengths × 4 variants.

Written to consolidate twelve scattered copies of the base64 alphabet in this
workspace and to let `MediaAssetRecord.data` stop serialising as a JSON array
of decimal numbers, which costs 4.6 wire bytes per byte against base64's 1.33.
See `code/specs/DT20-base64.md`.
