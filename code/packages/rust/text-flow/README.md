# text-flow

Device-independent analysis for text that participates in inline layout.

The package keeps three related decisions behind one reusable boundary:

- extended grapheme clusters used by caret and selection geometry,
- uniform-direction bidi runs supplied to platform text shapers, and
- Unicode line-break opportunities consumed by layout and measurement.

All offsets are UTF-8 byte offsets, matching `text-interfaces` glyph clusters.
The analyzer never resolves fonts or measures glyphs. Font fallback remains a
`TextShaper` responsibility, while hosts consume the same analyzed layout.
`TextFlow::selection_spans` accepts a caller-owned cluster measurer and projects
logical ranges into grapheme-safe visual spans split at bidi run boundaries.

Version 0.1 implements the browser-oriented UAX #9, #14, and #29 profile used
by Venture. Its conformance surface covers combining sequences, emoji ZWJ and
regional-indicator clusters, Arabic/Hebrew runs, CJK boundaries, punctuation,
non-breaking spaces, soft hyphens, and mandatory breaks. Additional generated
Unicode tables can extend the classifiers without changing the public API.
