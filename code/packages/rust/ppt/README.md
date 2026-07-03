# ppt

A from-scratch, **zero-third-party-dependency** Rust reader for legacy
**`.ppt`** files — PowerPoint 97–2003 binary presentations ([MS-PPT]) — that
extracts each slide's text into a typed `Presentation → Slide` model.

## Where it sits in the stack

A `.ppt` is a **Compound File** (OLE2 container, [MS-CFB]) whose
`"PowerPoint Document"` stream holds a tree of binary records. This crate layers
on the [`cfb`](../cfb) crate for the container and adds only the [MS-PPT] record
walk:

```
                         cfb  →  ppt  (this crate)
      (opens the OLE2 container)   (walks the record tree, harvests text)
```

The only external dependency is the sibling `cfb` crate; everything else is
`std`.

## Usage

```rust
use ppt::open_ppt;

fn main() -> Result<(), ppt::PptError> {
    let bytes = std::fs::read("deck.ppt").unwrap();
    let deck = open_ppt(&bytes)?;

    println!("{} slides", deck.slide_count());
    for (i, slide) in deck.slides().iter().enumerate() {
        println!("--- slide {} ---", i + 1);
        println!("{}", slide.text());        // all runs joined by '\n'
        // or inspect individual runs:
        for run in slide.text_runs() {
            println!("  run: {run}");
        }
    }
    Ok(())
}
```

### API

| Item | Meaning |
| ---- | ------- |
| `open_ppt(&[u8]) -> Result<Presentation, PptError>` | Open `.ppt` bytes and extract text. |
| `parse_document_stream(&[u8]) -> Result<Presentation, PptError>` | Parse a raw "PowerPoint Document" record stream directly (useful for tests/tooling). |
| `Presentation::slides() -> &[Slide]` / `slide_count()` | The slides in document order. |
| `Slide::text() -> String` | All text on the slide, runs joined by `'\n'`. |
| `Slide::text_runs() -> &[String]` | Each TextChars/TextBytes atom as one run. |
| `PptError` | `Cfb(CfbError)` · `NoDocumentStream` · `Truncated`. Implements `Display`, `Error`, `From<CfbError>`. |

## What it reads

Each `.ppt` slide is a `0x03EE` **Slide** container. The reader walks the record
tree and, for every Slide container, collects the text atoms found anywhere
inside it (recursing through arbitrary intermediate containers), in record
order:

- **TextBytesAtom** (`0x0FA8`) — one byte per char (Latin-1 / low byte of each
  UTF-16 unit).
- **TextCharsAtom** (`0x0FA0`) — UTF-16LE text.

This "scan the tree for text atoms" approach is exactly what real-world `.ppt`
tooling (Apache POI, `catppt`, Tika) uses.

## Safety

The input is an untrusted, attacker-controlled binary, so the parser is
hardened: `#![forbid(unsafe_code)]`, no `unwrap`/`expect`/`panic!`, every offset
bounds-checked, arithmetic via `checked_add`, a recursion-depth cap (64) against
stack-overflow DoS, slide/text size caps against unbounded allocation, and a
clean stop on the CFB's trailing zero padding. Malformed input yields whatever
well-formed text was readable — never a panic or hang.

## Testing

```
cargo test -p ppt
```

See [`code/specs/PPT01-binary-reader.md`](../../../specs/PPT01-binary-reader.md)
for the full literate specification (CFB layering, RecordHeader bit-layout,
container-vs-atom rule, text-atom decoding, and the robustness rationale).

## License

MIT.
