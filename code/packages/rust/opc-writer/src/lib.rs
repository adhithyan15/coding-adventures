//! # Open Packaging Conventions (OPC) — package *writer*
//!
//! This is the **write side** of milestone **C1** and the mirror image of the
//! read-side [`opc`](https://docs.rs/coding-adventures-opc) crate. Where `opc`
//! *opens* the bytes of an Office file into a bag of named parts, `opc-writer`
//! *assembles* a bag of named parts back into the bytes of a valid Office file.
//! See `code/specs/OPCW01-opc-writer.md` for the full write-up; this module
//! summarizes the model inline so the source reads on its own.
//!
//! ## The one-paragraph mental model
//!
//! Physically, an `.xlsx` / `.docx` / `.pptx` is a ZIP archive (built here by
//! the [`zip`] crate) whose members are XML files. OPC adds three conventions on
//! top of that raw ZIP, and this writer produces all three:
//!
//! 1. **Part names.** Members are addressed by a *logical* absolute name that
//!    starts with `/` (e.g. `/xl/workbook.xml`). Inside the ZIP the leading
//!    slash is dropped (`xl/workbook.xml`). We accept either spelling and
//!    normalize to the ZIP form.
//! 2. **Content types.** `/[Content_Types].xml` states each part's media type,
//!    either by file **extension** (`<Default>`) or by exact **part name**
//!    (`<Override>`, which wins). We synthesize this file from the defaults and
//!    overrides the caller registers.
//! 3. **Relationships.** `.rels` files map short ids (`rId1`) to targets so
//!    parts point at one another *by id*, never by hard-coded path. A `.rels`
//!    file is just another XML part; [`RelationshipsBuilder`] serializes one and
//!    the caller adds it as a part.
//!
//! Crucially, `opc-writer` knows **nothing** about spreadsheets or documents —
//! it is purely the packaging layer. Turning a `Workbook` model into the right
//! parts is the job of a format-specific crate like `xlsx-writer`.
//!
//! ```
//! use coding_adventures_opc_writer::{PackageWriter, RelationshipsBuilder};
//!
//! let mut pkg = PackageWriter::new();
//! // Two Defaults every OPC package needs:
//! pkg.add_default("rels", "application/vnd.openxmlformats-package.relationships+xml");
//! pkg.add_default("xml", "application/xml");
//!
//! // The package-root relationship: package → main document.
//! let mut root_rels = RelationshipsBuilder::new();
//! root_rels.add(
//!     "rId1",
//!     "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument",
//!     "xl/workbook.xml",
//! );
//! pkg.add_part_defaulted("/_rels/.rels", &root_rels.build());
//!
//! // A typed part (gets an <Override>).
//! pkg.add_part(
//!     "/xl/workbook.xml",
//!     "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml",
//!     b"<workbook/>",
//! );
//!
//! let bytes = pkg.finish(); // valid ZIP / OPC package bytes
//! assert_eq!(&bytes[..2], b"PK");
//! ```

#![forbid(unsafe_code)]

use zip::ZipWriter;

// ===========================================================================
// Namespaces (ECMA-376 Part 2) — these mirror the constants the reader checks.
// ===========================================================================

/// Namespace of `[Content_Types].xml` (`<Types>`, `<Default>`, `<Override>`).
const CONTENT_TYPES_NS: &str = "http://schemas.openxmlformats.org/package/2006/content-types";

/// Namespace of every `.rels` file (`<Relationships>`, `<Relationship>`).
const RELATIONSHIPS_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

/// The XML declaration OOXML producers put at the head of every part. Office and
/// our own parser both tolerate its absence, but real `.xlsx` files carry it, so
/// we emit it for fidelity.
const XML_DECL: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n";

// ===========================================================================
// XML escaping
// ===========================================================================
//
// Every scrap of caller-supplied text (content types, part names, relationship
// ids / types / targets, and — in xlsx-writer — sheet names and cell text) is
// placed inside an XML **attribute** or **text node**. Five characters are
// special in XML and must be replaced by entity references, or the output is not
// well-formed and no reader can open it:
//
//     &  →  &amp;      (must be first, or we'd double-escape the others)
//     <  →  &lt;
//     >  →  &gt;
//     "  →  &quot;     (only strictly needed in double-quoted attributes)
//     '  →  &apos;     (only strictly needed in single-quoted attributes)
//
// The function is TOTAL: it never panics and passes every other character —
// including arbitrary Unicode — through unchanged. We escape the full set for
// both attributes and text so a single helper is safe everywhere.

/// XML-escape `s` for safe inclusion in an attribute value or text node.
///
/// ```
/// use coding_adventures_opc_writer::xml_escape;
/// assert_eq!(xml_escape("a & b < c"), "a &amp; b &lt; c");
/// assert_eq!(xml_escape("say \"hi\""), "say &quot;hi&quot;");
/// assert_eq!(xml_escape("plain"), "plain");
/// ```
pub fn xml_escape(s: &str) -> String {
    // Pre-size for the common case (no escapes) plus a little slack.
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}

// ===========================================================================
// Part-name normalization
// ===========================================================================

/// Turn an OPC logical part name into its ZIP member name.
///
/// OPC part names are absolute (`/xl/workbook.xml`); ZIP member names have no
/// leading slash (`xl/workbook.xml`). We accept either spelling from the caller
/// and always store the ZIP form. Any accidental leading slashes are stripped.
fn zip_member_name(part_name: &str) -> String {
    part_name.trim_start_matches('/').to_string()
}

/// Turn an OPC logical part name into the canonical `/`-prefixed form used in
/// `[Content_Types].xml` `<Override PartName="…">`.
fn override_part_name(part_name: &str) -> String {
    format!("/{}", part_name.trim_start_matches('/'))
}

// ===========================================================================
// Relationships builder
// ===========================================================================

/// One `<Relationship>` we will serialize.
struct RelEntry {
    id: String,
    rel_type: String,
    target: String,
}

/// Builds a `.rels` XML part.
///
/// A `.rels` file wires parts together by id. Targets are written **relative to
/// the `.rels` file's own directory**, per OPC: the package-root `_rels/.rels`
/// targets `xl/workbook.xml`, while `xl/_rels/workbook.xml.rels` targets
/// `worksheets/sheet1.xml` (both relative to `xl/`). The caller is responsible
/// for supplying already-correct relative targets — `opc-writer` stays free of
/// any assumption about which relationships a given format needs.
///
/// ```
/// use coding_adventures_opc_writer::RelationshipsBuilder;
/// let mut r = RelationshipsBuilder::new();
/// r.add("rId1", "http://example/type", "target.xml");
/// let bytes = r.build();
/// let text = String::from_utf8(bytes).unwrap();
/// assert!(text.contains("Id=\"rId1\""));
/// assert!(text.contains("Target=\"target.xml\""));
/// ```
#[derive(Default)]
pub struct RelationshipsBuilder {
    entries: Vec<RelEntry>,
}

impl RelationshipsBuilder {
    /// A new, empty relationships part.
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Add a `<Relationship>` with the given id, type URI, and (relative) target.
    pub fn add(&mut self, id: &str, rel_type: &str, target: &str) {
        self.entries.push(RelEntry {
            id: id.to_string(),
            rel_type: rel_type.to_string(),
            target: target.to_string(),
        });
    }

    /// Serialize to the bytes of a `.rels` XML part.
    pub fn build(&self) -> Vec<u8> {
        let mut xml = String::new();
        xml.push_str(XML_DECL);
        xml.push_str("<Relationships xmlns=\"");
        xml.push_str(RELATIONSHIPS_NS);
        xml.push_str("\">");
        for e in &self.entries {
            xml.push_str("<Relationship Id=\"");
            xml.push_str(&xml_escape(&e.id));
            xml.push_str("\" Type=\"");
            xml.push_str(&xml_escape(&e.rel_type));
            xml.push_str("\" Target=\"");
            xml.push_str(&xml_escape(&e.target));
            xml.push_str("\"/>");
        }
        xml.push_str("</Relationships>");
        xml.into_bytes()
    }
}

// ===========================================================================
// Package writer
// ===========================================================================

/// A registered `<Default Extension="…" ContentType="…"/>`.
struct DefaultType {
    extension: String,
    content_type: String,
}

/// A part to be written into the ZIP, plus whether it needs an `<Override>`.
struct PartEntry {
    /// ZIP member name (no leading slash), e.g. `xl/workbook.xml`.
    member: String,
    /// The part's bytes.
    data: Vec<u8>,
    /// The content type for the `<Override>`, or `None` if this part is typed by
    /// a `<Default>` (e.g. a `.rels` file) and needs no per-part override.
    content_type: Option<String>,
}

/// Assembles OOXML parts, content types, and relationships into a ZIP-based OPC
/// package. Format-agnostic: it knows ZIP + content types + relationships, but
/// nothing about spreadsheets or documents.
///
/// Usage: register the `<Default>` content types, add each part (with or without
/// an `<Override>`), then call [`finish`](PackageWriter::finish). On finish, the
/// writer synthesizes `[Content_Types].xml` and emits every part as a
/// DEFLATE-compressed ZIP member.
#[derive(Default)]
pub struct PackageWriter {
    defaults: Vec<DefaultType>,
    parts: Vec<PartEntry>,
}

impl PackageWriter {
    /// A new, empty package writer.
    pub fn new() -> Self {
        Self {
            defaults: Vec::new(),
            parts: Vec::new(),
        }
    }

    /// Register a `<Default>` content type for a file **extension** (without the
    /// dot), e.g. `("rels", "application/…relationships+xml")` or `("xml",
    /// "application/xml")`. If the same extension is registered twice, the last
    /// registration wins (we de-duplicate on emit, keeping the latest).
    pub fn add_default(&mut self, extension: &str, content_type: &str) {
        self.defaults.push(DefaultType {
            extension: extension.to_string(),
            content_type: content_type.to_string(),
        });
    }

    /// Add a part with an explicit content type. The type is recorded as an
    /// `<Override>` (keyed by exact part name), which wins over any matching
    /// `<Default>`.
    ///
    /// `part_name` may be given with or without a leading slash
    /// (`/xl/workbook.xml` or `xl/workbook.xml`); both are normalized.
    pub fn add_part(&mut self, part_name: &str, content_type: &str, data: &[u8]) {
        self.parts.push(PartEntry {
            member: zip_member_name(part_name),
            data: data.to_vec(),
            content_type: Some(content_type.to_string()),
        });
    }

    /// Add a part **without** an `<Override>` — its content type comes from a
    /// matching `<Default>` (by extension). Use this for `.rels` parts, which
    /// are typed by the `rels` Default rather than per-part.
    pub fn add_part_defaulted(&mut self, part_name: &str, data: &[u8]) {
        self.parts.push(PartEntry {
            member: zip_member_name(part_name),
            data: data.to_vec(),
            content_type: None,
        });
    }

    /// Serialize `[Content_Types].xml` from the registered defaults and the
    /// per-part overrides.
    ///
    /// De-duplication rules:
    /// * Defaults are keyed by (lowercased) extension; a later registration for
    ///   the same extension replaces an earlier one.
    /// * Overrides are keyed by part name; the last one wins.
    fn content_types_xml(&self) -> Vec<u8> {
        let mut xml = String::new();
        xml.push_str(XML_DECL);
        xml.push_str("<Types xmlns=\"");
        xml.push_str(CONTENT_TYPES_NS);
        xml.push_str("\">");

        // --- Defaults (de-duplicated, last-wins, stable first-seen order) ---
        let mut seen_ext: Vec<String> = Vec::new();
        for d in &self.defaults {
            let key = d.extension.to_ascii_lowercase();
            // Resolve to the LAST content type registered for this extension.
            let content_type = self
                .defaults
                .iter()
                .rev()
                .find(|x| x.extension.to_ascii_lowercase() == key)
                .map(|x| x.content_type.as_str())
                .unwrap_or(d.content_type.as_str());
            if seen_ext.contains(&key) {
                continue; // already emitted this extension
            }
            seen_ext.push(key);
            xml.push_str("<Default Extension=\"");
            xml.push_str(&xml_escape(&d.extension));
            xml.push_str("\" ContentType=\"");
            xml.push_str(&xml_escape(content_type));
            xml.push_str("\"/>");
        }

        // --- Overrides (de-duplicated by part name, last-wins) ---
        let mut seen_part: Vec<String> = Vec::new();
        for p in &self.parts {
            let Some(ct) = &p.content_type else {
                continue; // defaulted part → no override
            };
            let part_name = override_part_name(&p.member);
            // Resolve to the LAST content type registered for this part name.
            let content_type = self
                .parts
                .iter()
                .rev()
                .find(|x| {
                    x.content_type.is_some() && override_part_name(&x.member) == part_name
                })
                .and_then(|x| x.content_type.as_deref())
                .unwrap_or(ct.as_str());
            if seen_part.contains(&part_name) {
                continue;
            }
            seen_part.push(part_name.clone());
            xml.push_str("<Override PartName=\"");
            xml.push_str(&xml_escape(&part_name));
            xml.push_str("\" ContentType=\"");
            xml.push_str(&xml_escape(content_type));
            xml.push_str("\"/>");
        }

        xml.push_str("</Types>");
        xml.into_bytes()
    }

    /// Finish: synthesize `[Content_Types].xml`, then write it plus every part
    /// into a ZIP archive and return the bytes.
    ///
    /// `[Content_Types].xml` is written **first**, which is what real producers
    /// do; the ZIP format does not require an order, but keeping it first matches
    /// convention and aids reader locality.
    pub fn finish(self) -> Vec<u8> {
        let content_types = self.content_types_xml();

        let mut zip = ZipWriter::new();
        // The content-types part has no leading slash inside the ZIP.
        zip.add_file("[Content_Types].xml", &content_types, true);
        for p in &self.parts {
            zip.add_file(&p.member, &p.data, true);
        }
        zip.finish()
    }
}

#[cfg(test)]
mod tests;
