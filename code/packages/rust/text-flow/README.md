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

Version 0.2 keeps that API while replacing handwritten classifiers with
generated Unicode data:

- ICU4X's compiled UAX #29 grapheme state machine,
- the UAX #9 resolver driven by ICU4X's generated `Bidi_Class` map, including
  isolate, embedding, override, and pop controls, and
- ICU4X's Unicode 17 UAX #14 state machine and full line-break pair data, with
  dictionary segmentation for Thai, Lao, Khmer, and Myanmar.

`CONFORMANCE_PROFILE` exposes the active data and algorithm profile for host
diagnostics. Generated tables remain owned by the Unicode data dependencies;
layout and paint only consume stable byte ranges and break opportunities.
