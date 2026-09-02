# Changelog — pdf

## Unreleased

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
