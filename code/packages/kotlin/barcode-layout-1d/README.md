# barcode-layout-1d (Kotlin)

Pure integer layout for one-dimensional barcodes. `BarcodeLayout1DV1` expands
binary or narrow/wide patterns into runs, computes quiet zones and symbol spans,
and projects bars into `paint-instructions` rectangles. It does not encode a
symbology, draw text, or invoke a native renderer.

The API follows [barcode-layout-1d v1](../../../specs/barcode-layout-1d-v1.md).
The native JUnit suite executes every checked-in neutral fixture case plus
mutation and text-precedence examples. Run `gradle --no-daemon --no-build-cache
--max-workers=1 test` from this directory.

The production library requires no filesystem, network, process, environment,
font, or backend authority; only the test adapter reads the fixture corpus.
