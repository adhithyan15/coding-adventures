# PPTXW01 — `pptx-writer`: writing a valid `.pptx` from a slide-deck model

*Milestone **C3** of the OOXML effort. Sibling of `xlsx-writer` (C1, SpreadsheetML)
and `wordprocessingml`; built on the format-agnostic `opc-writer` (C1).*

## 1. What this crate is

`pptx-writer` turns a tiny in-memory slide-deck model —

```rust
let mut p = Presentation::new();
let s = p.add_slide();
s.add_text("Slide One Title");
s.add_text("First slide body");
let bytes = write_pptx(&p);          // valid .pptx bytes
```

— into the bytes of a real `.pptx` file that strict consumers (PowerPoint,
LibreOffice Impress, python-pptx) will open. It knows the **PresentationML**
vocabulary (the `p:` and `a:` XML dialects) and delegates *all* of the ZIP +
content-types + relationships packaging to `opc-writer`. It contains no ZIP
code, no XML-escaping code of its own (it re-exports `opc_writer::xml_escape`),
and no filesystem/network access — it is pure `model → bytes`.

The layering, top to bottom:

```text
  Presentation / Slide  (this crate's model)
        │  write_pptx
        ▼
  PresentationML parts   (this crate: presentation.xml, slideN.xml, master, …)
        │  opc_writer::PackageWriter::add_part / add_part_defaulted
        ▼
  OPC package            (opc-writer: [Content_Types].xml, .rels, ZIP framing)
        │  zip::ZipWriter
        ▼
  .pptx bytes            ("PK\x03\x04…")
```

## 2. Why a deck is not just "slides in a zip"

A naive first attempt writes `presentation.xml` and a `slide1.xml` and stops.
PowerPoint and python-pptx **reject** that. A conformant PresentationML package
requires a whole *scaffold* of parts wired by relationships, because every slide
inherits its placeholders, colour map, and fonts from a chain of parent parts:

```text
                       ┌─────────────────────────────┐
   package root  ─rId1─▶│  ppt/presentation.xml       │
   (_rels/.rels)        │  (the deck: size, slide list│
                        │   + slide-master list)      │
                        └──────┬───────────────┬──────┘
                               │ rIdM          │ rId1…rIdN
                               ▼               ▼
                 ┌──────────────────────┐  ┌───────────────────────┐
                 │ slideMasters/        │  │ slides/slideN.xml      │
                 │   slideMaster1.xml   │  │ (the visible text)     │
                 └──┬────────────────┬──┘  └───────────┬───────────┘
                    │ theme          │ layout          │ rId1 (layout)
                    ▼                ▼                  ▼
        ┌────────────────┐  ┌────────────────────┐  (points back at
        │ theme/theme1   │  │ slideLayouts/      │◀──  slideLayout1)
        │ .xml           │  │   slideLayout1.xml │
        └────────────────┘  └─────────┬──────────┘
                                       │ rId1 (master)
                                       ▼
                              (points back at slideMaster1)
```

So the relationship graph is **cyclic-looking but well-founded**:

* `presentation.xml` → each `slideN.xml` **and** → `slideMaster1.xml`.
* each `slideN.xml` → `slideLayout1.xml`.
* `slideLayout1.xml` → `slideMaster1.xml`.
* `slideMaster1.xml` → `slideLayout1.xml` **and** → `theme1.xml`.

The master also carries a `<p:clrMap>` (bg1/tx1/…/folHlink) and a
`<p:sldLayoutIdLst>`; the presentation carries a `<p:sldMasterIdLst>` and a
`<p:sldIdLst>`. python-pptx walks exactly these relationships when it loads a
package, which is why *all* of the scaffold parts must be present and
well-formed — even for a one-line "hello world" deck.

`pptx-writer` emits one master, one layout, and one theme (shared by every
slide) plus one `slideN.xml` per slide. The master/layout/theme are constant
**boilerplate** with no user data in them; only `slideN.xml` carries text.

### The full part list (for N slides)

| Part | Content type / rel type | Notes |
|------|------------------------|-------|
| `[Content_Types].xml` | — | synthesised by `opc-writer` from registered defaults + overrides |
| `_rels/.rels` | `.../officeDocument` | rId1 → `ppt/presentation.xml` |
| `ppt/presentation.xml` | `…presentationml.presentation.main+xml` | deck: master-id-list, slide-id-list, sizes |
| `ppt/_rels/presentation.xml.rels` | — | rId1…rIdN → slides, rIdM → master |
| `ppt/slides/slideN.xml` | `…presentationml.slide+xml` | the text, in the `a:` namespace |
| `ppt/slides/_rels/slideN.xml.rels` | `.../slideLayout` | rId1 → `../slideLayouts/slideLayout1.xml` |
| `ppt/slideLayouts/slideLayout1.xml` | `…presentationml.slideLayout+xml` | `type="blank"`, empty spTree |
| `ppt/slideLayouts/_rels/slideLayout1.xml.rels` | `.../slideMaster` | rId1 → `../slideMasters/slideMaster1.xml` |
| `ppt/slideMasters/slideMaster1.xml` | `…presentationml.slideMaster+xml` | clrMap + layout-id-list |
| `ppt/slideMasters/_rels/slideMaster1.xml.rels` | `.../slideLayout`, `.../theme` | rId1 → layout, rId2 → theme |
| `ppt/theme/theme1.xml` | `…theme+xml` | clrScheme + fontScheme + fmtScheme |

## 3. The `a:` namespace gotcha (the one non-obvious thing about slide text)

A slide is `<p:sld>` in the **PresentationML** namespace
`…/presentationml/2006/main`. But the *text itself* — paragraphs, runs, the
literal characters — lives in the **DrawingML** namespace
`…/drawingml/2006/main`, conventionally bound to prefix `a:`. So inside a
`p:sp` shape's `p:txBody` you switch dialects:

```xml
<p:txBody>
  <a:bodyPr/>
  <a:p><a:r><a:t>Slide One Title</a:t></a:r></a:p>   ← a:p / a:r / a:t are DrawingML
  <a:p><a:r><a:t>First slide body</a:t></a:r></a:p>
</p:txBody>
```

Each paragraph of slide text (`Slide::add_text`) becomes one
`<a:p><a:r><a:t>…</a:t></a:r></a:p>`. Confusing `a:p` (a DrawingML *paragraph*)
with `p:` (the PresentationML namespace *prefix*) is the classic PowerPoint-XML
beginner trap, so we name the two namespaces explicitly in the source.

The text between `<a:t>` and `</a:t>` is the only place user-supplied data
enters an XML document, so every paragraph string is passed through
`opc_writer::xml_escape` before it is written. Scaffold parts hold no user data
and are emitted verbatim.

## 4. The model and public API

```rust
pub struct Presentation { /* Vec<Slide> */ }
impl Presentation {
    pub fn new() -> Self;
    pub fn add_slide(&mut self) -> &mut Slide;   // append, return handle to fill
}

pub struct Slide { /* Vec<String> paragraphs */ }
impl Slide {
    pub fn add_text(&mut self, text: &str);      // one paragraph / run
}

pub fn write_pptx(p: &Presentation) -> Vec<u8>;  // model → .pptx bytes
```

The model is deliberately minimal for C3: a deck is an ordered list of slides;
a slide is an ordered list of text paragraphs. Fonts, colours, positioning,
images, and notes are out of scope (they are the same shared scaffold for every
slide) and can be layered on in later milestones without changing this API.

### Id conventions

* Slides are numbered `1..=N`; the parts are `slideN.xml` in that order.
* In `presentation.xml`, `<p:sldId>` ids start at **256** (`256, 257, …`) — the
  conventional base PowerPoint uses — and each `r:id` is `rId1…rIdN`.
* The slide-master id is the conventional large sentinel `2147483648`; the
  layout id in the master is `2147483649`.
* In `presentation.xml.rels`, slides take `rId1…rIdN` and the master takes
  `rIdM = rId{N+1}` so no id collides.

An **empty deck** (no slides) is still valid: `presentation.xml` carries an
empty `<p:sldIdLst/>`, no `slideN.xml` parts exist, and the master/layout/theme
scaffold is still written so the package remains loadable.

## 5. Security / robustness

* `#![forbid(unsafe_code)]`.
* No `unwrap` / `expect` / `panic!` on any model-driven path. Empty deck, empty
  slide, and arbitrary Unicode / XML-special text must never panic.
* All user text flows through `opc_writer::xml_escape`, which also **drops**
  characters illegal in XML 1.0 (NUL, other C0 controls) so untrusted text can
  never make the package unparseable.
* Zip-Slip and path traversal are handled one layer down by `opc-writer`'s
  part-name normalisation; this crate only ever emits fixed, well-formed part
  names.

## 6. Verification

Because the read side of `.pptx` is not yet on this branch, tests verify the
generated bytes **structurally**:

1. Unzip the output with this repo's `zip::ZipReader` and assert every expected
   member is present (content-types, presentation, each `slideN.xml`, master,
   layout, theme, and all the `.rels`).
2. Parse `ppt/slides/slide1.xml` with `coding_adventures_xml_parser::parse_xml`
   and assert the `<a:t>` text nodes contain that slide's text; assert slide-2
   text is **not** in slide1 (an ordering guard).
3. Unit-test XML escaping of specials, multi-slide id/rels alignment, and the
   empty-deck case.

Separately, an external python-pptx cross-check confirms a real consumer opens
the file.
