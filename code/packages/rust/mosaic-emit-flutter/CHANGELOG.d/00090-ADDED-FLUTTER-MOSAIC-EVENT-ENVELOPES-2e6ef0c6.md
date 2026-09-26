### Added - Flutter Mosaic event envelopes

Generated Dart event classes now expose `mosaicName`, `mosaicPayload`, and
`mosaicEnvelope`, and the generated Flutter app shell logs the envelope. This
lets Flutter hosts forward the same event map used by HTML, Electron, SwiftUI,
XAML, and Qt shells.

