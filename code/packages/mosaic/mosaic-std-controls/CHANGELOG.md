# Changelog

## Unreleased

- Tests read the backend list from `mosaic_package_artifact_builder::Backend` (`Backend::NATIVE` / `Backend::ALL`) instead of a hand-kept copy, so a backend added to Mosaic is gated here automatically.
- Depends on `mosaic-std-foundation` 0.2.0 (`Surface`'s `items` slot).

- Fixed two stale acceptance pins that kept main's shard 15 red: the Flutter
  Slider readout is now matched open-ended (`Text(displayValue` — it gained a
  themed `style:`), and Compose is exempt from the number-field placeholder
  check, because its `BasicTextField` text is never empty and the emitter
  deliberately stopped writing a placeholder in #15995.

## [0.3.0] - Unreleased

- Added a labelled `Slider` facade over the all-five-native `HostSlider`
  contract, including range, step, disabled, live-change, and commit behavior.
- Required a human-readable label at the MIL boundary and attached it directly
  to the native adjustable control on every backend.
- Added an optional formatted visible value that is excluded from accessibility
  output to avoid duplicating the native range-value announcement.
- Expanded direct-package and consuming-app `native-complete` acceptance across
  SwiftUI, Qt/QML, XAML, Flutter, and Compose.

## [0.2.0] - Unreleased

- Added a portable two-state `Checkbox` facade that keeps native role, focus,
  keyboard, label, and toggle semantics on every backend.
- Added a native `NumberInput` facade with Foundation-owned light/dark styling.
- Expanded direct-package and consuming-app `native-complete` acceptance across
  SwiftUI, Qt/QML, XAML, Flutter, and Compose.

## [0.1.0] - Unreleased

- Added accessible `Button` and single-line `Input` facades over the proven
  toolkit controls.
- Added default slot values and Foundation-owned light/dark styling.
- Added direct-package and consuming-app `native-complete` acceptance across
  SwiftUI, Qt/QML, XAML, Flutter, and Compose.
