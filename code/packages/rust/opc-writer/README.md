# coding-adventures-opc-writer

A **generic Open Packaging Conventions (OPC) package writer** — the write-side
mirror of the [`opc`](../opc) reader. It assembles OOXML parts, content types,
and relationships into a ZIP-based OPC package (`.xlsx` / `.docx` / `.pptx`).

This is milestone **C1** of the OOXML effort. See
[`code/specs/OPCW01-opc-writer.md`](../../../specs/OPCW01-opc-writer.md) for the
full literate write-up.

## Where it fits

```text
   your parts  ─►  opc-writer  ─►  ZIP (zip crate)  ─►  .xlsx / .docx / .pptx bytes
                       │
                       ├─ [Content_Types].xml  (synthesized from defaults + overrides)
                       └─ .rels parts          (RelationshipsBuilder, caller-supplied)
```

`opc-writer` is **format-agnostic**: it knows ZIP + content types + relationships
and nothing about spreadsheets or documents. A format-specific crate such as
[`xlsx-writer`](../xlsx-writer) generates the meaningful parts and packages them
here.

## Usage

```rust
use coding_adventures_opc_writer::{PackageWriter, RelationshipsBuilder};

let mut pkg = PackageWriter::new();
pkg.add_default("rels", "application/vnd.openxmlformats-package.relationships+xml");
pkg.add_default("xml", "application/xml");

// Package-root relationship: package → main document.
let mut root_rels = RelationshipsBuilder::new();
root_rels.add(
    "rId1",
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument",
    "xl/workbook.xml",
);
pkg.add_part_defaulted("/_rels/.rels", &root_rels.build());

// A typed part (gets an <Override> in [Content_Types].xml).
pkg.add_part(
    "/xl/workbook.xml",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml",
    b"<workbook/>",
);

let bytes: Vec<u8> = pkg.finish(); // valid OPC package bytes
```

## API

| Item | Purpose |
|------|---------|
| `PackageWriter::new` | Start an empty package. |
| `add_default(ext, ct)` | Register a `<Default>` content type by file extension. |
| `add_part(name, ct, data)` | Add a part typed by an `<Override>`. |
| `add_part_defaulted(name, data)` | Add a part typed by a Default (e.g. `.rels`). |
| `finish()` | Synthesize `[Content_Types].xml` and emit the ZIP bytes. |
| `RelationshipsBuilder` | Build a `.rels` XML part from `(id, type, target)` entries. |
| `xml_escape(s)` | Total XML escaper for attribute/text values. |

## Testing

```sh
cargo test -p coding-adventures-opc-writer
```

Includes a **round-trip test** that re-opens the writer's output with the
read-side `opc` crate and asserts content-type resolution and relationship
dereferencing both work — proof the bytes are a genuine OPC package.

## Guarantees

* `#![forbid(unsafe_code)]`.
* No `unwrap`/`expect`/`panic!` on any input path; XML escaping is total.
* No filesystem, network, process, or environment access (see
  `required_capabilities.json`).
