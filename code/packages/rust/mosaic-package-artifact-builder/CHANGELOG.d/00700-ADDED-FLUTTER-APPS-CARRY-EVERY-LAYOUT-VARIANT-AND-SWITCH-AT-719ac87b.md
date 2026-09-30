### Added — Flutter apps carry every layout variant and switch at run time (UI48 ENV2/ENV3, §7.9)

- Flutter variants are compiled with `mosaic_emit_flutter::pipeline::from_pipeline_variant`,
  so `<C>.<variant>.dart` declares `class <C><Variant>` and imports the
  default file's event types instead of redeclaring them.
- `flutter_layout_choices` computes the root's rules with the same
  `effective_layout_rules` as SwiftUI and Compose (`[[app.layouts]]`, or the
  conventions), refuses a rule for a missing variant or one that cannot name
  a Dart widget, and keys each condition by `EnvironmentAxis::wire_name`; the
  emitter writes them into `main.dart` as the selector.
- The project shell copies every export's `<C>.<variant>.dart` into `lib/`,
  so `flutter analyze` checks every layout beside its default.
- A package without variants produces the same files as before.
- Tests: convention, declared rules under wire names, a rule for a missing
  variant refused, no variants → no selector, the native-complete shell
  observing and selecting, and every rule axis present in the Flutter
  binding's `environmentReport`.
