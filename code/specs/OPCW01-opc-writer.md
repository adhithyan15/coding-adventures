# OPCW01 — `opc-writer`: a generic Open Packaging Conventions *writer*

**Milestone C1 (write side), layer: packaging.** This is the mirror image of the
read-side [`opc`](OPC01-opc-package.md) crate. Where `opc` *opens* the bytes of an
Office file into a bag of named parts, `opc-writer` *assembles* a bag of named
parts back into the bytes of a valid Office file.

It is deliberately **format-agnostic**: it knows about ZIP, content types, and
relationships — the three OPC conventions — but nothing about spreadsheets,
documents, or presentations. The same crate underlies a future `.docx` or
`.pptx` writer; only the caller (`xlsx-writer`, `docx-writer`, …) knows what the
parts *mean*.

```text
     in-memory parts                     opc-writer                    bytes
 ┌───────────────────────┐          ┌───────────────────┐        ┌──────────────┐
 │ "/xl/workbook.xml"     │          │  registers types  │        │  ZIP archive │
 │ "/xl/worksheets/…"     │  ──────► │  emits            │ ─────► │ (.xlsx/.docx │
 │ "/_rels/.rels" (caller)│          │  [Content_Types]  │        │  /.pptx)     │
 │ …                      │          │  writes ZIP (M0)  │        └──────────────┘
 └───────────────────────┘          └───────────────────┘
```

## The three OPC conventions, from the writer's side

### 1. Parts → ZIP members

Every part has a *logical* OPC name that begins with `/` (e.g. `/xl/workbook.xml`).
Inside the ZIP the leading slash is dropped (`xl/workbook.xml`). The writer
accepts either spelling from the caller and normalizes to the ZIP form. Bytes
are handed straight to the [`zip`](../packages/rust/zip) crate's `ZipWriter`,
which DEFLATE-compresses each member (method 8, CRC-32, UTF-8 filename flag) —
exactly what Office readers, and our own `zip`/`opc` readers, expect.

### 2. Content types → `[Content_Types].xml`

Every part must have a declared media type. OPC offers two mechanisms and the
writer emits both:

* **Defaults** — keyed by file extension. `<Default Extension="xml"
  ContentType="application/xml"/>` says "any part whose name ends in `.xml` is
  `application/xml` unless overridden." Every `.xlsx` needs at least `rels` and
  `xml` defaults.
* **Overrides** — keyed by exact part name, and they win over a matching
  default. `<Override PartName="/xl/workbook.xml"
  ContentType="application/vnd.…sheet.main+xml"/>` types one specific part.

`add_part` records an Override for the part; `add_default` records a Default for
an extension. On `finish`, the writer synthesizes `[Content_Types].xml`:

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  …
</Types>
```

`[Content_Types].xml` is itself **not** a typed part — it is the one member that
types everything else, so the writer never emits an Override for it.

### 3. Relationships → `.rels` parts

Relationships wire parts together by *id*, never by hard-coded path, so a part
can be moved without editing its referrers. A `.rels` file is just another XML
part, so the writer models it as a helper (`RelationshipsBuilder`) that
serializes to bytes; the caller then `add_part`s those bytes at the correct
`.rels` name (`/_rels/.rels`, `/xl/_rels/workbook.xml.rels`). Keeping `.rels`
as "just a part the caller supplies" keeps `opc-writer` free of any assumption
about *which* relationships a given format needs.

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://…/officeDocument" Target="xl/workbook.xml"/>
</Relationships>
```

Relationship targets are written **relative to the `.rels` file's own
directory**, per the OPC convention: the package-root `_rels/.rels` targets
`xl/workbook.xml`, while `xl/_rels/workbook.xml.rels` targets
`worksheets/sheet1.xml` (both relative to `xl/`).

## Public API

```rust
pub struct PackageWriter { /* defaults, overrides, parts */ }
impl PackageWriter {
    pub fn new() -> Self;
    pub fn add_default(&mut self, extension: &str, content_type: &str);
    pub fn add_part(&mut self, part_name: &str, content_type: &str, data: &[u8]);
    /// Add a part WITHOUT an Override (its type comes from a Default). Used for
    /// `.rels` parts, which are typed by the `rels` Default, not per-part.
    pub fn add_part_defaulted(&mut self, part_name: &str, data: &[u8]);
    pub fn finish(self) -> Vec<u8>;
}

pub struct RelationshipsBuilder { /* entries */ }
impl RelationshipsBuilder {
    pub fn new() -> Self;
    pub fn add(&mut self, id: &str, rel_type: &str, target: &str);
    pub fn build(&self) -> Vec<u8>;  // serialized .rels XML bytes
}

pub fn xml_escape_attr(s: &str) -> String;   // & < > " for attribute values
```

## XML escaping

All caller-supplied text (content types, part names, relationship targets/types)
flows into XML attributes and must be escaped: `&`→`&amp;`, `<`→`&lt;`,
`>`→`&gt;`, `"`→`&quot;`. The escaper is **total** — it never panics, and it
handles arbitrary Unicode by passing non-special characters through unchanged.

## Verification approach

`opc-writer` is verified two ways:

1. **Unit tests** — `[Content_Types].xml` and `.rels` serialization, XML
   escaping, part-name normalization, ZIP-member presence.
2. **Round-trip through the read-side `opc` crate** — build a package with
   `PackageWriter`, then re-open the bytes with `coding_adventures_opc::Package`
   and assert content types resolve and relationships dereference. This proves
   the writer emits exactly what our own reader (and, by construction, any OPC
   reader) expects. The full `.xlsx` round-trip lives in `xlsx-writer`
   ([XLSXW01](XLSXW01-xlsx-writer.md)).

## Security / robustness

`#![forbid(unsafe_code)]`. This is a writer over *trusted* caller input, so the
DoS surface is small, but the code still avoids `unwrap`/`expect`/`panic!` on any
path a large or odd model could reach, and XML-escaping is total.
