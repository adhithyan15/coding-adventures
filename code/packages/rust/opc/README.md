# coding-adventures-opc

**Open Packaging Conventions (OPC) package reader** — milestone **M2** of the
OOXML effort.

An `.xlsx`, `.docx`, or `.pptx` file is, physically, a ZIP archive of XML
documents. On top of that raw ZIP, ECMA-376 Part 2 defines the *Open Packaging
Conventions*: a small, **format-agnostic** convention for naming the members
("parts"), declaring each part's media type ("content type"), and wiring parts
together with typed, id-addressed links ("relationships").

This crate implements exactly that packaging layer — and **nothing above it**.
It knows how to open the ZIP, resolve content types, and dereference
relationships to part names. It knows *nothing* about workbooks, sheets,
paragraphs, or slides; that document-format semantics is a later milestone
(SpreadsheetML, M3) built directly on this crate.

```text
raw bytes (.xlsx)
      │
      ▼
zip crate (M0)   → members: name → bytes
      │
      ▼
opc crate (M2)   → Package        ← THIS crate
      │             • part names (logical, "/"-rooted)
      │             • content_type(part)
      │             • relationships(part) + resolved targets
      ▼
spreadsheetml (M3) → Workbook / Sheet / Cell   (later)
```

## Where it fits in the stack

- **`zip` (M0)** provides the ZIP reader and `unzip`. OPC reads members through
  it.
- **`coding-adventures-xml-parser` (M1)** provides `parse_xml`. OPC parses
  `[Content_Types].xml` and every `.rels` file with it.
- **`coding-adventures-opc` (M2, this crate)** turns members into a `Package`.
- **SpreadsheetML (M3, next)** consumes `Package` to find `xl/workbook.xml` and
  walk to each sheet.

## Usage

```rust
use coding_adventures_opc::Package;

// `bytes` is the raw contents of an .xlsx/.docx/.pptx file.
let pkg = Package::open(bytes)?;

// List logical part names ("/"-rooted).
for name in pkg.part_names() {
    println!("{name}");
}

// Content type: <Override> wins over <Default>-by-extension.
let ct = pkg.content_type("/xl/workbook.xml"); // Some("…sheet.main+xml")

// The main document part is *discovered*, not hard-coded.
let main = pkg.main_document_part();           // Some("/xl/workbook.xml")

// Relationships map short ids to (possibly relative) targets.
let sheet = pkg.resolve("/xl/workbook.xml", "rId1");
//        → Some("/xl/worksheets/sheet1.xml")
# Ok::<(), coding_adventures_opc::OpcError>(())
```

Every method accepts a part name **with or without** a leading `/` and
normalizes it internally, so callers never have to remember which form to pass.
The sentinel `"/"` (or `""`) denotes the package itself — used to ask for the
package-level relationships in `/_rels/.rels`.

## Key semantics (ECMA-376 Part 2)

- **Part names** are logical, `/`-rooted (`/xl/workbook.xml`), even though the
  ZIP stores them without the leading slash. This crate stores the `/`-rooted
  form internally.
- **Content types** resolve *Override before Default*: an exact
  `<Override PartName="…">` wins; otherwise a `<Default Extension="…">` matches
  by (case-insensitive) file extension; otherwise `None`.
- **Relationships** for a part `/dir/name.ext` live in
  `/dir/_rels/name.ext.rels`; the package-level bootstrap is `/_rels/.rels`. A
  `Target` is resolved relative to the **source part's directory** (not the
  `.rels` file's). `TargetMode="External"` targets are left as opaque URIs and
  are never resolved to parts.
- **Bootstrap:** the main document part is whichever package-level relationship
  has a Type ending in `/officeDocument`.

## Security

The crate reads untrusted bytes. Target resolution **clamps directory
traversal at the package root**: a hostile `Target` like
`../../../../etc/passwd` still resolves to a well-formed, `/`-rooted logical
name that stays inside the package and does not correspond to any real part, so
it can never name a file outside the package. (Decompression-bomb protection is
handled upstream by the `zip` crate.)

## Testing

```sh
cargo test -p coding-adventures-opc -- --nocapture
```

Tests exercise the whole stack against a real DEFLATE-compressed `.xlsx`
fixture (`coding_adventures_opc::fixture::MINIMAL_XLSX`, 6 OPC parts): part
listing, content-type resolution (Override and Default paths), package- and
part-level relationships, id dereferencing with relative-target joins, external
and traversal-target safety, and the error cases (not a ZIP, missing/malformed
content types).

## Specification

See [`code/specs/OPC01-opc-package.md`](../../../specs/OPC01-opc-package.md) for
the full, newcomer-friendly write-up.
