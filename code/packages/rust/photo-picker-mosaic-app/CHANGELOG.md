# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-09-15

### Added

- Initial implementation: `PhotoPickerApp`, a `MosaicApp` implementing
  UI59's `files.open` effect contract end to end — one event
  (`pickPhoto`), one `Delivery::Await` effect (`files.open`), and a
  status line reporting the picked file's name, MIME type, and exact
  byte count (or cancellation/failure).
- `describe_picked_file` — decodes the `ok` result's base64 `bytes`
  field only to report an accurate size; never retains the decoded
  bytes, and never panics on a host that sends malformed base64 or a
  missing field (both are untrusted, host-controlled data).
- `mint_effect_id` — bounds minted effect ids by
  `mosaic_app_runtime::MAX_EFFECT_ID`, returning `None` (never
  panicking) rather than overflowing if ever exhausted.
- Exports the standard Mosaic C ABI via
  `mosaic_app_capi::export_mosaic_app!`, making this crate's `cdylib`
  the `libmosaic_app`/`mosaic_app.dll` any of the five native hosts
  can load — proven for real against XAML via
  `code/programs/mosaic/photo-picker-app`'s `--profile
  native-complete` build.
- 8 tests covering `start`, event dispatch (known and unknown), and
  all three `EffectResult` completion branches, including malformed
  and missing-field host payloads.
