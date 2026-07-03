# coding-adventures-presentationml

Read a **`.pptx`** (PowerPoint / PresentationML) file and get its **per-slide
text** — slides in show order, each slide's shapes, each shape's text.

This is milestone **PML01** of the OOXML effort. It is the presentation-side
sibling of [`coding-adventures-spreadsheetml`](../spreadsheetml) and sits on the
exact same two lower layers:

```text
bytes → zip → xml-parser → opc → presentationml (this crate)
```

* [`coding-adventures-opc`](../opc) opens the ZIP, exposes named parts, and
  resolves relationship ids (`r:id="rId1"`) to part names.
* [`coding-adventures-xml-parser`](../xml-parser) parses a part's XML into a
  namespace-aware element tree.

## Why it exists

A `.pptx` is a ZIP of XML parts, normalized so that two things trip up everyone
reading one for the first time. This crate resolves both:

1. **`r:id` → part.** `ppt/presentation.xml` lists slides by a *relationship
   id*, not a filename: `<p:sldId id="256" r:id="rId1"/>`. `rId1` is dereferenced
   through a separate `.rels` file to a real part like `ppt/slides/slide1.xml`.
   We read the `<p:sldId>`s in document order, so slide order is preserved.

2. **Text lives in the DrawingML namespace, not PresentationML.** A slide's
   *structure* (`<p:sp>` shapes) is PresentationML (prefix `p:`), but the actual
   text runs (`<a:p>`, `<a:r>`, `<a:t>`) are **DrawingML** (prefix `a:`) — a
   different namespace URI. Look for text under `p:` and you find nothing.

See [`code/specs/PML01-presentationml.md`](../../../specs/PML01-presentationml.md)
for the full walk-through.

## Usage

```rust
use coding_adventures_presentationml::open_pptx;

let bytes: &[u8] = /* the raw .pptx file */;
let pres = open_pptx(bytes)?;

println!("{} slides", pres.slide_count());
for (i, slide) in pres.slides().iter().enumerate() {
    println!("--- slide {} ---", i + 1);
    println!("{}", slide.text());        // all shape text, joined by '\n'
    for shape in slide.shapes() {
        // shape.text is one shape's runs, concatenated
    }
}
# Ok::<(), coding_adventures_presentationml::PptxError>(())
```

## API

| Item | Meaning |
| ---- | ------- |
| `open_pptx(&[u8]) -> Result<Presentation, PptxError>` | parse a `.pptx` from bytes |
| `Presentation::slides() -> &[Slide]` | slides in show order |
| `Presentation::slide_count() -> usize` | number of slides |
| `Slide::shapes() -> &[Shape]` | shapes in document order |
| `Slide::text() -> String` | all shape text, joined by `\n` (empty shapes skipped) |
| `Shape::text` | one shape's runs concatenated |
| `PptxError` | `Opc` / `MissingPresentation` / `NotUtf8` / `MalformedXml` / `MissingSlidePart` |

## Scope

**In scope:** slide text from shape text bodies, in show order.

**Out of scope (for now):** speaker notes, tables (`<a:tbl>`), grouped shapes,
placeholders inherited from layouts/masters, run formatting, images, animation.
The shape-walk is structured so these can be added later.

## Testing

```sh
cargo test -p coding-adventures-presentationml -- --nocapture
```

The suite includes an end-to-end test over a real DEFLATE-compressed two-slide
`.pptx` fixture plus unit tests for each error, the DrawingML namespace switch,
and the text-joining logic. Coverage well exceeds 80%.
