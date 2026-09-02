# pdf — a PDF writer built from scratch

A zero-dependency PDF writer. This is **PDF-1**: the object model and the file
structure. No pages helper, text, fonts, or graphics API yet — those sit on top
(#13957, #13958), and this layer has to be right first, because everything above
it is expressed in these object types and *located by the byte offsets computed
here*.

Part of the from-scratch PDF effort (#13944).

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
