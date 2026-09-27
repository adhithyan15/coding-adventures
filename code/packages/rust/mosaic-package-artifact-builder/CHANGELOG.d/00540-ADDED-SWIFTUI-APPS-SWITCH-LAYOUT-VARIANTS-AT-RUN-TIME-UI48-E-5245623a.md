### Added — SwiftUI apps switch layout variants at run time (UI48 ENV2/ENV3)

- A SwiftUI build emits each named layout variant with
  `from_pipeline_variant` and compiles the root component's selected variants
  into `Sources/App`, so one app carries them all.
- The shell's selector uses the package's `[[app.layouts]]` rules, or the
  conventional ones (`compact`, `expanded`, `touch`) for the variants the root
  has. A rule for a variant with no `.mll` is an error. Engram's `touch`
  layout is now selected on iOS; verified by building for macOS and the iOS
  simulator.

