# Changelog — pdf

## Unreleased


### Added — PDF-2: page tree, content streams, graphics operators

- `Document` / `Page` build the catalogue → `Pages` → `Page` tree, reserving
  the `Pages` object first because the links run both ways: the node lists its
  kids and every page names its parent.
- `Content` writes the operators: `q`/`Q`/`cm`/`w`, `rg`/`g`/`k` fill and
  stroke colour, `m`/`l`/`c`/`re`/`h` paths painted with `S`/`f`/`B`/`n`, `W`
  clipping, and `BT`/`ET`/`Tf`/`Td`/`Tm`/`TL`/`T*`/`Tj`/`TJ` text.
- `StandardFont` covers the base-14 faces, which need no embedding. They also
  cannot render Tamil, Devanagari or CJK — that is what PDF-3 is for.

### The coordinate convention, in one place

PDF's origin is bottom-left with y upward; box trees and SVG put it top-left
with y downward. `Content::top_down(page_height)` reconciles them, and the
conversion lives in exactly one function so a sign error fails every rendering
test at once rather than whichever call sites nobody tested.

The flip is deliberately **not** a `cm` matrix: `1 0 0 -1 0 h cm` puts text in
the right place and renders every glyph mirrored. Converting points instead
leaves the text matrix upright. `Page::with_content` also rejects a stream
mirrored about a different height than the page it is placed on, which would
otherwise offset everything silently.

### Fixed — content streams were written without `/Filter`

`flate_encode` returns the filter **name**, not a dictionary. Both new stream
paths matched it as a `Dict` and got an empty one, so the compressed bytes went
out with nothing marking them compressed. A reader parses them as operators,
recognises none, and renders a **blank page** — without erroring, because an
unparsable content stream is not a structural fault. `qpdf --check` was happy
throughout; the rendering oracle caught it on its first run.

### Testing — a second, independent renderer

`tests/render_gate.rs` rasterises with poppler (`pdftoppm`) and asserts where
the ink landed. Structural validation cannot see an upside-down page, because
nothing about it is structurally wrong. Mutation-checked: dropping the y
conversion fails three of the four rendering tests, while the PDF-space control
keeps passing — so the suite distinguishes the two coordinate spaces rather
than failing indiscriminately.

Initial release: the PDF object model and file-structure writer (PDF-1 of
#13944).

Covers the eight object types, indirect references with forward declaration via
`reserve()`/`fill()`, the header with binary marker, the body, a cross-reference
table, and the trailer. `FlateDecode` streams are supported with `/Length`
derived from the encoded bytes rather than taken from the caller — a `/Length`
that disagrees with its data yields a file some readers accept and others
reject, which is the worst available failure mode.

Verified against **`qpdf --check`**, an independent implementation, rather than
by reading our own output back. The gate fails when `qpdf` is absent instead of
skipping, and two tests corrupt a file deliberately to demonstrate the gate is
load-bearing.

That oracle paid for itself on its first run. The initial implementation used
`zip::raw_deflate` for `FlateDecode`, on the reasonable-sounding basis that
Flate is deflate. It is not quite: **PDF's `FlateDecode` is RFC 1950 zlib** — a
two-byte header, the deflate payload, and an Adler-32 trailer — while ZIP
method 8 is the bare stream with no wrapper. Our own reader round-tripped the
bare form perfectly; qpdf reported `unknown compression method`, because it read
the first deflate byte as the zlib CMF.

Output was also confirmed to open in a second independent implementation
(`pdftoppm`), which rasterised a drawn path correctly.
