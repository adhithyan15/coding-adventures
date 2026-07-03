# OPC01 — Open Packaging Conventions package reader

## Overview

This is milestone **M2** of the OOXML effort. It builds a Rust crate,
`coding-adventures-opc`, that opens the bytes of an OOXML file
(`.xlsx` / `.docx` / `.pptx`) and exposes it as a **package of parts**.

Every modern Office file is, physically, a ZIP archive (milestone **M0**, the
`zip` crate) whose members are XML documents (milestone **M1**, the
`xml-parser` crate). On top of that raw ZIP, ECMA-376 Part 2 defines the **Open
Packaging Conventions** (OPC): a small, *format-agnostic* convention for

1. naming the members ("**parts**"),
2. declaring each part's media type ("**content type**"), and
3. wiring parts together with typed, ID-addressed links ("**relationships**").

OPC knows nothing about spreadsheets, documents, or slides. It only knows
"here is a bag of named parts, here is each part's type, and here is how they
point at one another." That is exactly what this crate models. The *meaning* of
`xl/workbook.xml` — that it is a workbook with sheets — is deliberately **out of
scope**; that is milestone M3 (SpreadsheetML), which is built directly on this
crate.

```text
raw bytes (.xlsx)
      |
      v
zip crate (M0)     → members: name → bytes        ("xl/workbook.xml" → <bytes>)
      |
      v
opc crate (M2)     → Package                        (THIS crate)
      |                - part names (logical, "/"-rooted)
      |                - content_type(part)
      |                - relationships(part) + resolved targets
      v
spreadsheetml (M3) → Workbook / Sheet / Cell        (later)
```

## Why an OPC layer at all?

A naive reader could just `unzip` and read `xl/workbook.xml` by its literal ZIP
name. That works until it doesn't:

- **Names are not fixed.** The main document part is *discovered*, not
  hard-coded. A `.docx` uses `word/document.xml`; an `.xlsx` uses
  `xl/workbook.xml`. You find it by following a *relationship*, not by guessing
  a path. OPC makes "the main part" a lookup, not a constant.
- **Links are indirect.** A worksheet is not referenced by path inside
  `workbook.xml`; it is referenced by a relationship **id** (`r:id="rId1"`).
  To turn `rId1` into a part name you must read a *separate* `.rels` file and
  resolve a possibly-relative target. OPC centralizes that dereference.
- **Types are declared, not sniffed.** Whether a part is XML, PNG, or a
  relationships file is stated in `[Content_Types].xml`, not inferred from
  bytes. OPC answers "what *is* this part?" authoritatively.

Doing this once, correctly, in a reusable layer means every OOXML format above
it (SpreadsheetML, WordprocessingML, PresentationML) shares the same, tested
plumbing.

## Part names — the logical namespace

A **part name** is a logical, absolute path within the package. Logically it
always begins with a forward slash:

```text
/[Content_Types].xml
/_rels/.rels
/xl/workbook.xml
/xl/worksheets/sheet1.xml
```

But **inside the ZIP**, the same members are stored *without* the leading slash
(`xl/workbook.xml`). This crate resolves the mismatch by picking **one internal
representation**: logical names are stored **with** the leading `/`. Every
public entry point accepts a name *with or without* the leading slash and
normalizes it, so callers never have to remember which form to pass.

```text
caller passes  "xl/workbook.xml"   ─┐
caller passes  "/xl/workbook.xml"  ─┼─►  internal "/xl/workbook.xml"
                                    ─┘
```

The special name `"/"` (or the empty string) denotes the **package itself** —
used to ask for the package-level relationships in `/_rels/.rels`.

## Content types — `[Content_Types].xml`

The very first part parsed is `/[Content_Types].xml` (it must exist; a package
without it is not a valid OPC package). Its root is `<Types>` in the namespace
`http://schemas.openxmlformats.org/package/2006/content-types`. It has two kinds
of child, and the content type of any part is resolved by combining them:

- `<Default Extension="xml" ContentType="application/xml"/>` — a **fallback by
  file extension**. Every part whose name ends in `.xml` and has no more
  specific rule gets `application/xml`. Extensions are matched
  **case-insensitively** (`.XML` and `.xml` are the same rule).
- `<Override PartName="/xl/workbook.xml" ContentType="…sheet.main+xml"/>` — a
  rule for **one specific part**, keyed by its exact (`/`-rooted) part name.

**Resolution rule (Override beats Default):**

```text
content_type(P):
    if there is an <Override> whose PartName == P   → that ContentType
    else if there is a <Default> for P's extension  → that ContentType
    else                                            → None
```

### Truth table for the fixture

| Part                       | Ext    | Override? | Result                                   |
|----------------------------|--------|-----------|------------------------------------------|
| `/xl/workbook.xml`         | `xml`  | yes       | `…spreadsheetml.sheet.main+xml`          |
| `/xl/sharedStrings.xml`    | `xml`  | yes       | `…spreadsheetml.sharedStrings+xml`       |
| `/xl/worksheets/sheet1.xml`| `xml`  | yes       | `…spreadsheetml.worksheet+xml`           |
| `/_rels/.rels`             | `rels` | no        | `…package.relationships+xml` (Default)   |
| some `/foo.png` (no rule)  | `png`  | no        | `None`                                   |

The `.rels` row is the important one: it exercises the **Default-by-extension**
path, since no `<Override>` names a `.rels` file.

## Relationships — `.rels` files

Relationships are how parts point at one another *without hard-coding paths*.
They live in the namespace
`http://schemas.openxmlformats.org/package/2006/relationships`.

### Where a part's relationships live

For a part at `/dir/name.ext`, its relationships are in a sibling `_rels`
folder, with `.rels` appended to the file name:

```text
part      /xl/workbook.xml
its rels  /xl/_rels/workbook.xml.rels
```

The **package-level** relationships (the bootstrap entry point) are a special
case at `/_rels/.rels`.

```text
rels_part_for("/xl/workbook.xml")  =  "/xl/_rels/workbook.xml.rels"
rels_part_for("/")                 =  "/_rels/.rels"
```

If the computed `.rels` part does not exist, the part simply has **no**
relationships (an empty list — not an error).

### What a relationship contains

```xml
<Relationship Id="rId1"
              Type="http://…/relationships/worksheet"
              Target="worksheets/sheet1.xml"
              TargetMode="Internal"/>   <!-- TargetMode optional; default Internal -->
```

- **Id** — unique *within that one `.rels` file* (`rId1`, `rId2`, …). This is
  the token that appears as `r:id="rId1"` inside the source part.
- **Type** — a URI describing *what kind of link* this is (worksheet,
  sharedStrings, officeDocument, image, hyperlink, …).
- **Target** — where the link points. Resolved (for internal targets) to a
  part name; see below.
- **TargetMode** — `Internal` (default) means the target is another part in the
  same package. `External` means the target is *outside* the package (e.g. an
  `http://` hyperlink or a linked file on disk) and is **not** resolved to a
  part name.

### Target resolution — relative to the SOURCE part's directory

This is the subtle rule. A `Target` is resolved relative to the **directory of
the source part**, *not* the directory of the `.rels` file.

```text
source part   /xl/workbook.xml        →  its directory is  /xl/
Target        worksheets/sheet1.xml
resolved      /xl/worksheets/sheet1.xml
```

A `Target` that begins with `/` is **package-root-relative** and used as-is
(after normalization). `.` and `..` segments are honored via a small URI-join.

```text
join("/xl/", "worksheets/sheet1.xml")  = /xl/worksheets/sheet1.xml
join("/xl/", "../docProps/core.xml")   = /docProps/core.xml
join("/xl/", "/media/logo.png")        = /media/logo.png        (root-relative)
```

`External` targets are returned verbatim in the `target` field with
`resolved_target = None` — the crate never tries to turn a URL into a part.

## Bootstrap — finding the main document part

The one thing a caller needs before it knows *anything* format-specific is
"where does the document proper start?" OPC answers this via the package-level
relationships:

```text
1. read /_rels/.rels
2. find the relationship whose Type ends with ".../officeDocument"
3. its resolved Target is the main document part
```

For the `.xlsx` fixture that yields `/xl/workbook.xml`; for a `.docx` it would
yield `/word/document.xml`. The **crate does not care which** — that is the
whole point of the indirection.

## Security — directory-traversal safety

The crate reads **untrusted** bytes. A hostile package could contain a
relationship like:

```xml
<Relationship Id="evil" Type="…" Target="../../../../etc/passwd"/>
```

If target resolution naively concatenated paths, a downstream consumer that
mapped part names to filesystem paths could be tricked into reading (or, in a
writer, clobbering) files **outside** the package. Even though this crate only
reads *from memory*, it must never hand back a "part name" that escapes the
package root, because consumers trust part names to be package-internal.

The URI-join therefore **clamps at the root**: a `..` that would rise above `/`
is dropped, so the result is always a well-formed, `/`-rooted logical name that
stays inside the package. A resolved target that does not correspond to an
actual part simply won't be readable (`read_part` returns `None`); it can never
name something outside the package. A dedicated test feeds a `../../..`-style
target and asserts the result stays under `/`.

(Decompression-bomb protection — a small ZIP that inflates to gigabytes — is
handled *upstream* by the `zip` crate, which caps output size. This crate does
not re-implement that.)

## Public API

```rust
pub struct Package { /* … */ }

pub enum OpcError {
    NotAZip(String),          // bytes are not a readable ZIP
    MissingContentTypes,      // no /[Content_Types].xml
    MalformedXml(String),     // a part that must be XML did not parse
    NotUtf8(String),          // a part that must be XML was not UTF-8
}

pub struct Relationship {
    pub id: String,
    pub rel_type: String,
    pub target: String,                   // raw Target attribute value
    pub mode: TargetMode,                 // Internal | External
    pub resolved_target: Option<String>,  // Some(part name) iff Internal
}

pub enum TargetMode { Internal, External }

impl Package {
    pub fn open(bytes: &[u8]) -> Result<Package, OpcError>;
    pub fn part_names(&self) -> Vec<String>;                 // logical, "/"-rooted
    pub fn has_part(&self, name: &str) -> bool;
    pub fn read_part(&self, name: &str) -> Option<&[u8]>;
    pub fn content_type(&self, part_name: &str) -> Option<String>;
    pub fn relationships(&self, source_part: &str)
        -> Result<Vec<Relationship>, OpcError>;              // source "/" ⇒ /_rels/.rels
    pub fn resolve(&self, source_part: &str, rel_id: &str) -> Option<String>;
    pub fn main_document_part(&self) -> Option<String>;
}
```

`open` eagerly reads every ZIP member into memory and eagerly parses
`[Content_Types].xml` (failing fast if it is absent or malformed).
Relationships are parsed **lazily**, on the first `relationships`/`resolve`
call for a given source part, and cached — most parts have no relationships, so
paying to parse them all up front would be wasteful.

## Design decisions

- **Logical names carry the leading `/` internally.** Matches the ECMA-376
  wording and makes `<Override PartName="/…">` a direct key lookup with no
  massaging.
- **Boundary normalization.** Every public method funnels its name argument
  through one `normalize_part_name` helper, so `"x"`, `"/x"`, and `"//x"` all
  behave identically. Callers never think about slashes.
- **Eager parts, eager content-types, lazy rels.** Fail fast on the two things
  a package cannot be valid without; defer the rest.
- **No format semantics.** The crate exposes `main_document_part()` (pure OPC:
  "follow the officeDocument relationship") but never interprets what that part
  contains. Anything sheet- or paragraph-shaped belongs one layer up.

## Relationship to other milestones

- **M0 `zip`** — provides `ZipReader` / `unzip`; OPC reads members through it.
- **M1 `xml-parser`** — provides `parse_xml`; OPC parses `[Content_Types].xml`
  and every `.rels` file with it.
- **M3 SpreadsheetML** (next) — consumes `Package`: calls
  `main_document_part()` to find `xl/workbook.xml`, then `relationships` /
  `resolve` to walk to each sheet and to `sharedStrings.xml`.
