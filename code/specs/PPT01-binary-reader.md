# PPT01 — Legacy PowerPoint (`.ppt`) binary text reader

Status: implemented · Layer: B3 (above `cfb`) · Crate: `code/packages/rust/ppt`

## 1. Why this exists

A `.ppt` file — PowerPoint 97–2003 — is not a zip of XML the way a modern
`.pptx` is. It is a **[MS-CFB] Compound File** (an "OLE2" container: a tiny
FAT-style filesystem living inside one file) whose streams hold a tree of
**[MS-PPT]** binary records. To read the words on the slides we must:

1. Open the outer container (delegated wholesale to the already-merged `cfb`
   crate — PPT01 adds **zero** new container parsing).
2. Pull out the one stream that holds the presentation records — it is named
   exactly `"PowerPoint Document"`.
3. Walk the [MS-PPT] record tree in that stream and collect the text atoms.

This crate is the third rung of the OOXML/legacy-Office stack:

```
zip → deflate → xml → opc → spreadsheetml → xlsx-eval → CLI     (modern .xlsx)
                                    cfb → ppt (THIS)              (legacy .ppt)
```

## 2. Layering on `cfb`

The entire container concern is `cfb`'s job. PPT01 uses exactly two calls:

```rust
let cf = cfb::CompoundFile::open(bytes)?;              // -> Result<_, CfbError>
let stream = cf.read_stream("PowerPoint Document")     // -> Option<Vec<u8>>
    .ok_or(PptError::NoDocumentStream)?;
```

`read_stream` returns `None` when the stream is absent → we surface
`PptError::NoDocumentStream`. `CompoundFile::open` failing (non-CFB bytes,
truncated header, bad FAT) surfaces as `PptError::Cfb(CfbError)` via a
`From<CfbError>` impl and the `?` operator.

## 3. The [MS-PPT] record tree

Everything in the "PowerPoint Document" stream is a **record**. Records are laid
back-to-back, and containers hold child records inside their body — so the whole
stream is a depth-first tree.

### 3.1 RecordHeader (8 bytes, little-endian)

```
 offset  size  field       meaning
 ------  ----  ----------  ---------------------------------------------------
   0      u16  recVerInst  low 4 bits  = recVer   (0xF ⇒ this is a CONTAINER)
                           high 12 bits = recInstance (a per-type sub-variant)
   2      u16  recType     what kind of record this is (table below)
   4      u32  recLen      length of the BODY that follows, NOT incl. this header
 ------  ----  ----------
   8    recLen  body       child records (container) OR raw atom data (atom)
```

Bit layout of the first `u16` (call it `w`):

```
   15                                 4   3        0
  +-------------------------------------+-----------+
  |            recInstance (12)          |  recVer(4)|
  +-------------------------------------+-----------+

  recVer      = w & 0x000F
  recInstance = w >> 4          (unused by this reader; kept for documentation)
```

**Container vs atom is decided solely by `recVer == 0xF`.** A container's body is
more records (recurse); an atom's body is opaque data we interpret by `recType`.

### 3.2 Record types this reader acts on

| recType  | name           | kind      | what we do                                  |
| -------- | -------------- | --------- | ------------------------------------------- |
| `0x03E8` | Document       | container | recurse (top-level wrapper around slides)   |
| `0x03EE` | Slide          | container | **start a new `Slide`; recurse for its text** |
| `0x0FA0` | TextCharsAtom  | atom      | body is UTF-16LE → decode to a text run     |
| `0x0FA8` | TextBytesAtom  | atom      | body is one byte per char (U+0000..=U+00FF) |
| other    | —              | either    | if container, recurse; if atom, ignore body |

Any other container is still recursed into (a `Slide`'s text lives several
containers deep in a real file — inside a `PPDrawing` → `OfficeArtDgContainer` →
… → `TextBox`). We do not need to understand those wrappers; we only need to not
stop at them.

### 3.3 Text atom decoding

- **TextBytesAtom (`0x0FA8`)** — each body byte `b` maps to the Unicode scalar
  `U+00{b}` (Latin-1 / the low byte of the UTF-16 code unit PowerPoint would have
  stored). `char::from(b)` is exactly this and is always valid for `0..=255`.
- **TextCharsAtom (`0x0FA0`)** — body is UTF-16LE. We read `u16` code units in
  pairs. A trailing odd byte (malformed) is ignored. Unpaired surrogates are
  emitted as U+FFFD (via `char::decode_utf16`) rather than rejected — attacker
  input must never panic.

Both strip a single trailing NUL, which PowerPoint appends as a C-string
terminator (the fixture's runs end in `0x00`).

## 4. The reader model

```rust
pub fn open_ppt(bytes: &[u8]) -> Result<Presentation, PptError>;

pub struct Presentation { /* Vec<Slide>, in document order */ }
impl Presentation {
    pub fn slides(&self) -> &[Slide];
    pub fn slide_count(&self) -> usize;
}

pub struct Slide { /* Vec<String> text runs */ }
impl Slide {
    pub fn text(&self) -> String;         // runs joined by '\n', in record order
    pub fn text_runs(&self) -> &[String]; // each TextChars/TextBytes atom = one run
}
```

**Text-extraction strategy.** Real-world `.ppt` text tooling (Apache POI's
`HSLFSlideShow`, `catppt`, tika) walks the record tree and harvests
TextChars/TextBytes atoms rather than reconstructing the full drawing model. We
adopt the same faithful, well-trodden strategy: each `0x03EE` Slide container
becomes one `Slide`, and every text atom found **anywhere inside it** (recursing
through arbitrary intermediate containers) is one of its runs, in record order.

Text atoms not inside any Slide container are ignored (headers, masters, the
document summary). The fixture places all text inside Slide containers, so the
required test is independent of that choice.

## 5. Robustness — this is attacker-controlled input

The bytes come from an untrusted file. Every one of these is a hard requirement,
enforced in `src/lib.rs` and covered by tests:

- **`#![forbid(unsafe_code)]`.** No `unwrap`/`expect`/`panic!`, no panicking
  index/slice, no unchecked arithmetic on the parse path. Reads go through
  bounds-checked helpers (`read_u16`, `read_u32`, `get(..)`); lengths combine
  with `checked_add`.

- **Trailing zero padding stop.** A CFB stores streams padded up to a
  sector/mini-sector boundary, so `read_stream` returns the logical records
  *plus* a zero tail. When the walker sees a header with `recType == 0` **and**
  `recLen == 0`, or fewer than 8 bytes remain, it **stops that level cleanly** —
  it does not treat the padding as a record and never loops on it.

- **Recursion depth cap (`MAX_DEPTH = 64`).** Containers nest, and a crafted
  chain of thousands of nested containers would otherwise overflow the native
  stack — an uncatchable DoS. Past the cap we stop descending (return cleanly)
  rather than recurse.

- **Every offset is bounds-checked before slicing or recursing.** `recLen` is
  validated against the bytes actually remaining; a `recLen` that runs past the
  buffer stops the walk cleanly (no panic).

- **The cursor always advances by at least `8 + recLen`.** Children live strictly
  inside the parent's body, so offsets strictly increase; the loop can never spin
  in place, so no cyclic/among-siblings hang is possible.

- **Allocation caps.** `MAX_SLIDES = 100_000` and `MAX_TOTAL_TEXT_BYTES = 64 MiB`
  bound how much a hostile file can make us allocate. We never pre-size a `Vec`
  to a length declared in the file; growth follows bytes actually consumed.

## 6. Testing

`parse_document_stream(&[u8]) -> Result<Presentation, PptError>` is factored out
so tests can feed synthetic record streams without building a whole CFB. Tests
cover: the end-to-end fixture (2 slides, correct text, ordering guard);
TextBytes (Latin-1) and TextChars (UTF-16) decoding in isolation;
container-recursion (text atom nested inside a Slide inside a Document
container); the trailing-zero-padding stop; and the error paths (non-CFB →
`Cfb`; CFB lacking the stream → `NoDocumentStream`; a `recLen` past the buffer →
clean stop; deep nesting → no overflow). Target ≥90% coverage.
