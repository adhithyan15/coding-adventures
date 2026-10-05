# barcode-layout-1d (Dart)

Pure integer layout for one-dimensional barcodes. `BarcodeLayout1DV1` expands
binary or narrow/wide patterns into runs, computes quiet zones and symbol spans,
and projects bars into `paint-instructions` rectangles. It does not encode a
symbology, draw text, or invoke a native renderer.

The API follows [barcode-layout-1d v1](../../../specs/barcode-layout-1d-v1.md).
The Dart tests execute every checked-in neutral fixture case plus copy and
text-precedence examples. Run the checked-in `BUILD` front for formatting,
analysis, native examples, and coverage-gated conformance. On Windows, direct
`dart run test/...` execution avoids a test-runner SDK path-launch issue while
executing the same test files.

The production library requires no filesystem, network, process, environment,
font, or backend authority; only the test adapter reads the fixture corpus.
