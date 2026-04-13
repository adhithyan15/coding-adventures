# Changelog

## 0.1.0

- Initial implementation of HKDF (RFC 5869)
- HKDF-Extract: compress input keying material into pseudorandom key
- HKDF-Expand: stretch PRK into output keying material of any length
- Combined hkdf() convenience function (extract-then-expand)
- Support for SHA-256 (32-byte output) and SHA-512 (64-byte output)
- All three RFC 5869 test vectors verified
- Input validation: throws HKDFError for length <= 0 or > 255 * HashLen
- Empty salt handling: uses HashLen zero bytes per RFC 5869 Section 2.2
