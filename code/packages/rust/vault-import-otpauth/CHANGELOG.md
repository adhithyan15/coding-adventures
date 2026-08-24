# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-08-24

### Added

- Initial implementation of the standalone `otpauth://totp/...` URI
  adapter named by `code/specs/VLT-PM49-cli-external-import.md` §5.5,
  extending VLT-PM49 (previously "Bitwarden JSON and Browser CSV") to
  also cover a standalone-URI-file import, at the user's explicit
  request for QR-code/`otpauth://` TOTP setup.
- `OtpauthUriImporter` — implements `vault-import-export`'s `Importer`
  trait (`name()` returns `"otpauth-uri"`).
- `decode(&[u8]) -> Result<PortableRecord, ImportError>` — the free
  function the trait impl calls, exposed directly for callers that don't
  need a trait object.
- Recognizes exactly `otpauth://totp/<label>?<query>` (case-insensitive
  scheme and type); `otpauth://hotp/...` and every other type is refused
  with `ImportError::Adapter`, not silently coerced.
- Extracts and percent-decodes only the URI's label segment for the
  created record's title; the query string (`secret`, `issuer`,
  `algorithm`, `digits`, `period`) is passed through to
  `PortableRecord::totp_seed` untouched, so `vault-pm-cli`'s existing
  VLT-PM49 §5.3 `decode_external_totp_field` / `parse_otpauth_totp_uri`
  remains the single place that decodes it — reused unchanged, not
  duplicated.
- Bounded against adversarial input: whole-source byte ceiling
  (`MAX_SOURCE_BYTES`), label byte ceiling matching VLT-PM29's own
  `Label` prompt bound (`MAX_LABEL_BYTES`), no fixed-byte-offset slicing
  of untrusted `&str` (every boundary is either `str::get`-checked or a
  single-byte ASCII delimiter offset from `str::find`, so a multi-byte
  character cannot trigger a boundary panic), and a bounded,
  non-panicking percent-decoder for the label only.
