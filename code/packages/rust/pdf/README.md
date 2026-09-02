# pdf — a PDF writer built from scratch

A zero-dependency PDF writer, through **PDF-2**: the object model and file
structure (#13944), plus the page tree, content streams and graphics operators
(#13957). Font embedding and subsetting are next (#13958); until then, text
uses the base-14 faces, which every reader already has and none of which can
render Tamil, Devanagari or CJK.

```rust
use pdf::{ColorTarget, Content, Document, Page, Paint, StandardFont, LETTER};

let (width, height) = LETTER;

// Top-down coordinates: origin top-left, y downward, like a layout tree.
let mut content = Content::top_down(height);
content
    .gray(0.9, ColorTarget::Fill)
    .rect(40.0, 40.0, 200.0, 60.0)   // 40 points from the TOP
    .paint(Paint::Fill)
    .begin_text()
    .font("F1", 24.0)
    .text_position(52.0, 80.0)
    .show_text(b"Engram")
    .end_text();

let mut page = Page::with_content(width, height, content)?;
page.add_font("F1", StandardFont::HelveticaBold);

let mut doc = Document::new();
doc.add_page(page);
let bytes = doc.finish()?;
# Ok::<(), pdf::PdfError>(())
```

## Which way is up

**PDF's origin is bottom-left and y grows upward.** Screens, box-layout trees
and SVG all put it top-left with y growing downward, so anything rendering into
a PDF has to reconcile the two — and doing it ad hoc produces pages that are
perfectly plausible *upside down*.

`Content` is built in one space, chosen once:

| | origin | y |
|---|---|---|
| `Content::pdf_space()` | bottom-left | upward |
| `Content::top_down(h)` | top-left | downward |

The conversion lives in a single private function that every coordinate passes
through, including `rect` — which needs one extra step, since `re` takes a
rectangle's *bottom*-left corner and mirroring the corner you were given lands
on the one diagonally opposite.

It is deliberately **not** implemented as a `1 0 0 -1 0 h cm` matrix: that
mirrors everything drawn under it, so the text lands in the right place with
every glyph reversed.

## Why the foundation is mostly counting bytes

A PDF is opened from the **back**. A reader seeks to the end, reads `startxref`
to find the cross-reference table, then jumps directly to each object's recorded
byte offset. **Nothing is scanned for.** An offset wrong by one byte does not
degrade gracefully — the reader lands mid-token.

So this crate tracks its position as it appends, rather than deriving offsets
from a finished buffer. The moment an offset comes from a second traversal, it
can disagree with the first.

## Scope

- The eight object types: null, boolean, numeric, string (literal and hex), name,
  array, dictionary, stream — plus indirect references, which make it a graph
- Header with binary marker, body, cross-reference table, trailer
- `FlateDecode` streams, with `/Length` derived from the data
- Forward references via `reserve()` / `fill()`, so a page and its parent can
  name each other
- The page tree: `/Catalog` → `/Pages` → `/Page`, with `MediaBox` and resources
- Content streams: graphics state, paths, clipping, and text placement
- The base-14 standard fonts (no embedding required)

Out of scope here: reading, incremental update, encryption, object streams,
cross-reference streams.

## Usage

```rust
use pdf::{dict, Object, PdfWriter};

let mut w = PdfWriter::new();
let pages = w.reserve();                 // reserve first: page and parent
let page = w.add(Object::Dict(dict! {    // must reference each other
    "Type"     => Object::name("Page"),
    "Parent"   => Object::Ref(pages),
    "MediaBox" => Object::Array(vec![
        Object::Int(0), Object::Int(0), Object::Int(612), Object::Int(792),
    ]),
}));
w.fill(pages, Object::Dict(dict! {
    "Type"  => Object::name("Pages"),
    "Kids"  => Object::Array(vec![Object::Ref(page)]),
    "Count" => Object::Int(1),
}));
let root = w.add(Object::Dict(dict! {
    "Type" => Object::name("Catalog"), "Pages" => Object::Ref(pages),
}));
let bytes = w.finish(root)?;
# Ok::<(), pdf::PdfError>(())
```

## Verification

Correctness here **cannot** be established by reading our own output back — if
our idea of where the xref goes is wrong, our reader looks in the same wrong
place and agrees.

`tests/qpdf_gate.rs` runs **`qpdf --check`**, an independent implementation,
over every PDF produced. It **fails when `qpdf` is absent** rather than
skipping: a test that silently passes when its oracle is missing is not a test.

Two of those tests deliberately corrupt a file — a wrong xref offset, a wrong
`/Length` — and assert qpdf *complains*, so the gate is shown to be load-bearing
rather than decorative.

The oracle earned its keep immediately: the first version used
`zip::raw_deflate` for `FlateDecode`. **PDF's `FlateDecode` is RFC 1950 zlib,
not the bare deflate stream ZIP uses.** Our own reader inflated it back happily;
qpdf reported `unknown compression method`.

```
brew install qpdf          # macOS
apt-get install qpdf       # Debian/Ubuntu
cargo test -p pdf
```

## Dependencies

`zip`, for `raw_deflate` only. PDF's Flate filter is deflate, which this
repository already implements from scratch, so pulling a compression crate would
mean adding a dependency to duplicate our own code.
