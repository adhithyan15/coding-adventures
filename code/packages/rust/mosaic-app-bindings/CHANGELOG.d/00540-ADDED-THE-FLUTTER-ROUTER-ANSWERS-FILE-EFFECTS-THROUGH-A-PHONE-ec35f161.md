### Added — the Flutter router answers file effects through a phone document plugin (UI89 §7.11)

- `mosaic_platform_effects_core.dart` has a phone path beside the desktop
  dialogs:
  - `MosaicPhoneDocuments` is the plugin's interface: `temporaryDirectory`,
    `copyForOpening` and `export`. It throws a
    `MosaicPhoneDocumentsException` carrying the plugin's code.
  - `mosaicRunPhoneFilesOpen` reads the plugin's copy with the desktop's
    bounded read. It accepts the copy only when it is a regular file, not a
    link, directly inside the request's own directory.
  - `mosaicRunPhoneFilesSave` checks the request exactly as the desktop
    does, stages the bytes under the suggested name with an exclusive
    create, and exports them. The answer is the reported name, made
    ordinary by `mosaicOrdinaryReportedName`.
  - Each request has its own directory under
    `<temporary>/mosaic-files/`, which is removed whatever happens. The
    router's first request sweeps out entries unchanged for over an hour,
    without following links.
  - Every plugin code maps to a fixed message
    (`mosaicPhoneFailureMessage`). Any other error gives the generic
    message, never the platform's text.
- `installMosaicPlatformRouter` takes `phoneDocuments`. When it is given,
  the standard kinds go through the plugin, and the platform counts as
  having pickers.
- The `files.save` checks are now a single function shared by the desktop
  and the phone, `_mosaicCheckSaveRequest`. Desktop behaviour is
  unchanged.
- The conformance harness drives the phone path with fake plugins: the
  copy, the limit, hostile answered paths, every code, the reported-name
  rule, staging and cleanup, the sweep, and the router's deferral and busy
  rule.
- No phone build passes a plugin yet, so nothing changes on a device until
  the Android and iOS halves land.
