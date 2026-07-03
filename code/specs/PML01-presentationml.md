# PML01 — PresentationML (`.pptx`) reader

**Status:** implemented
**Crate:** `coding-adventures-presentationml` (`code/packages/rust/presentationml`)
**Depends on:** `coding-adventures-opc` (OPC01), `coding-adventures-xml-parser` (XML01)
**Sibling:** `coding-adventures-spreadsheetml` (SML01) — same architecture, different OOXML dialect.

## 1. What this crate does

It takes the raw bytes of a `.pptx` file (a PowerPoint presentation) and produces
a plain, ordered model:

```text
Presentation → [Slide] → [Shape] → text
```

The headline output is **per-slide text**: for each slide, in the order it
appears in the show, the text of every shape on it. That is enough to power
search, summarization, or accessibility over a deck without a rendering engine.

It sits on the two lower OOXML layers that already did the hard plumbing — the
exact same stack the `spreadsheetml` crate uses:

```text
bytes → zip (M0) → xml-parser (M1) → opc (M2) → presentationml (HERE)
```

* **`opc`** opens the ZIP, exposes named parts, and — crucially — resolves
  relationship ids (`r:id="rId1"`) to part names.
* **`xml-parser`** parses a part's UTF-8 XML into a namespace-aware element tree
  with entity decoding already done.

## 2. Two gotchas a newcomer must understand

A `.pptx` is a ZIP of XML "parts". Two things trip up everyone reading one for
the first time. This crate exists to resolve both so the caller sees plain text.

### 2.1 `r:id` → part: which file *is* this slide?

`ppt/presentation.xml` lists slides by a relationship **id**, not by a path:

```xml
<p:presentation xmlns:p="…/presentationml/2006/main"
                xmlns:r="…/officeDocument/2006/relationships">
  <p:sldIdLst>
    <p:sldId id="256" r:id="rId1"/>
    <p:sldId id="257" r:id="rId2"/>
  </p:sldIdLst>
</p:presentation>
```

`rId1` is *not* a filename. It is an index into a **separate** relationships
file (`ppt/_rels/presentation.xml.rels`) that maps each id to a real part such
as `ppt/slides/slide1.xml`. The OPC layer does that dereference for us:

```rust
opc.resolve("/ppt/presentation.xml", "rId1")  // → Some("/ppt/slides/slide1.xml")
```

We read the `<p:sldId>` elements **in document order** and resolve each one, so
the slide order in our `Presentation` is exactly the order of the show. Note the
part-name convention: OPC part names are `/`-rooted (`/ppt/slides/slide1.xml`),
even though the ZIP entry is `ppt/slides/slide1.xml` without the leading slash.

Also note the *namespace asymmetry* on `<p:sldId>`: `id="256"` is unprefixed
(namespace `None`), while `r:id="rId1"` is in the **relationships** namespace
because it is written `r:id`. So we read them with different namespaces:

```rust
sld_id.get_attr(None,          "id");  // "256"  — the numeric slide id (unused)
sld_id.get_attr(Some(REL_NS),  "id");  // "rId1" — the relationship id (used)
```

### 2.2 Slide text lives in the **DrawingML** namespace, not PresentationML

This is the subtle one. A slide's *structure* — the shape tree, the shapes — is
in the **PresentationML** namespace (prefix `p:`). But the actual **text runs**
are in the **DrawingML** namespace (prefix `a:`), a completely different URI.
DrawingML is the shared "graphics" vocabulary used across Word, Excel, and
PowerPoint, and text bodies are part of it.

```xml
<p:sp>                              <!-- p: shape (PresentationML) -->
  <p:txBody>                        <!-- p: text body -->
    <a:p>                           <!-- a: PARAGRAPH  (DrawingML!) -->
      <a:r>                         <!-- a: run -->
        <a:t>Slide One Title</a:t>  <!-- a: the actual text (DrawingML!) -->
      </a:r>
    </a:p>
  </p:txBody>
</p:sp>
```

If you look for `<a:t>` under the `p:` namespace you find **nothing** and
conclude the slide is empty. The boundary is exactly at `<p:txBody>`: `txBody`
and everything above it is PresentationML; `<a:p>`, `<a:r>`, `<a:t>` and below
are DrawingML. We switch namespaces at that boundary.

The two URIs:

| prefix | namespace URI                                                    | used for                     |
| ------ | ---------------------------------------------------------------- | ---------------------------- |
| `p:`   | `http://schemas.openxmlformats.org/presentationml/2006/main`     | presentation / slide / shape |
| `a:`   | `http://schemas.openxmlformats.org/drawingml/2006/main`          | paragraphs / runs / **text** |
| `r:`   | `http://schemas.openxmlformats.org/officeDocument/2006/relationships` | the `r:id` on `<p:sldId>` |

## 3. The pipeline (step by step)

1. **Bootstrap.** `Package::open(bytes)` → `main_document_part()` yields
   `/ppt/presentation.xml`. Parse it as `<p:presentation>`.
2. **Slide list.** `<p:presentation>` → `<p:sldIdLst>` → each `<p:sldId>`, in
   order. Read each one's `r:id` via `get_attr(Some(REL_NS), "id")`.
3. **Resolve.** For each `r:id`, `opc.resolve("/ppt/presentation.xml", rid)` →
   the slide part name, e.g. `/ppt/slides/slide1.xml`.
4. **Parse the slide.** `<p:sld>` → `<p:cSld>` (common slide data) →
   `<p:spTree>` (shape tree) → each `<p:sp>` (shape).
5. **Extract text.** For each shape: `<p:txBody>` → each `<a:p>` (paragraph,
   DrawingML) → each `<a:r>` (run) → `<a:t>` (text). Join a shape's runs into
   one string. A shape with no `<p:txBody>` or no runs contributes no text.
6. The slide's full text is its shapes' texts joined by newlines.

## 4. Public API

```rust
pub fn open_pptx(bytes: &[u8]) -> Result<Presentation, PptxError>;

pub struct Presentation { /* … */ }
impl Presentation {
    pub fn slides(&self) -> &[Slide];
    pub fn slide_count(&self) -> usize;
}

pub struct Slide { /* … */ }
impl Slide {
    pub fn shapes(&self) -> &[Shape];
    pub fn text(&self) -> String;   // all shape text, joined by '\n'
}

pub struct Shape { pub text: String }

pub enum PptxError {
    Opc(OpcError),          // not a readable OPC package
    MissingPresentation,    // no /ppt/presentation.xml main part
    NotUtf8(String),        // a part that must be XML was not UTF-8
    MalformedXml(String),   // a part failed to parse as XML
    MissingSlidePart(String), // a <p:sldId r:id=…> did not resolve to a part
}
```

Slides are returned in `sldIdLst` order. `Slide::text()` joins its shapes' text
with `\n`; a shape with empty text is skipped in that join so the result has no
stray blank lines.

## 5. Scope & non-goals

* **In scope:** slide text from shapes' text bodies, in show order.
* **Out of scope (for now):** speaker notes (`notesSlide`), tables
  (`<a:tbl>`), grouped shapes (`<p:grpSp>` recursion), placeholders inherited
  from layouts/masters, run formatting, images, and animation. Text that lives
  only on a layout or master (not on the slide) is not surfaced. These are
  deliberate omissions to keep the first milestone small; the shape-walk is
  structured so they can be added later.

## 6. Testing

An end-to-end test opens a real DEFLATE-compressed two-slide `.pptx` fixture and
asserts:

* `slide_count() == 2`;
* slides are in order — slide 0 carries the slide-one strings, slide 1 the
  slide-two strings;
* `slides[0].text()` contains `"Slide One Title"` and `"First slide body"`;
* `slides[1].text()` contains `"Slide Two Title"` and `"Second slide body"`.

Plus unit tests for each error (`open_pptx` on non-`.pptx` bytes), a slide with
no text, the namespace-switch at `txBody`, and the `r:id → part` resolution.
Coverage well exceeds 80%.
