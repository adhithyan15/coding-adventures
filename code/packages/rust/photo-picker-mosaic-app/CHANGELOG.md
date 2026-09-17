# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- **The "Pick a Photo" button did nothing, on every shipped backend.**
  `dispatch` checked `event.name != "pickPhoto"`, but every backend's
  generated client sends the RAW, unstripped emit name from
  `PhotoPickerApp.mil`'s `emit onPickPhoto ;` — `"onPickPhoto"` — as
  the wire event name. The "on" prefix is stripped only for the
  generated class/case identifier (`PhotoPickerAppEvent.PickPhoto`),
  never the wire value; confirmed directly against
  `mosaic-emit-compose`'s own codegen and its own test asserting
  `mosaicName: String = "onCommit"`, and against `mosaic-emit-xaml`'s
  `event_name = escape_csharp_string(&emit.name)` (the raw name, not
  `strip_on_prefix`'s output).

  Found by actually clicking the built XAML app for the first time —
  every "real build" verification across all four backend PRs (#15218,
  #15252, #15329, #15340) compiled and linked correctly, and every
  unit test passed, because the tests dispatched the same wrong
  `"pickPhoto"` name the app itself checked for — self-consistently
  wrong, invisible to unit tests. The interactive-dialog gap stated
  explicitly in every one of those PRs ("not something this session
  can automate") is exactly where this bug was hiding; it surfaced the
  moment a human actually pressed the button.

  A live re-test after the fix (a real click on the rebuilt XAML app,
  a real photo picked from a real file dialog) confirmed the full
  round trip: dialog opens, file read, base64-encoded, status renders
  `Picked "<name>" (<mimeType>, <N> bytes).`

  `dispatch` now checks for `"onPickPhoto"`. A new regression test,
  `the_pre_fix_wire_name_is_still_just_an_unknown_event`, locks in
  that the *old*, wrong name (`"pickPhoto"`) must keep behaving as an
  unrecognised event forever, not become a quietly-supported alias —
  a future "helpful" synonym would silently mask this exact class of
  bug reappearing under a different event name. All six existing
  tests' event names were updated to the real wire convention to
  match.

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
