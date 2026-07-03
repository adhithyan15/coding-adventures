//! # Open Packaging Conventions (OPC) — package reader
//!
//! This is milestone **M2** of the OOXML effort. It turns the raw bytes of an
//! Office file (`.xlsx` / `.docx` / `.pptx`) into a [`Package`]: a bag of named
//! **parts**, each with a **content type**, wired together by typed
//! **relationships**. See `code/specs/OPC01-opc-package.md` for the full,
//! newcomer-friendly write-up; this module summarizes the model inline so the
//! source reads on its own.
//!
//! ## The one-paragraph mental model
//!
//! Physically, an `.xlsx` is a ZIP archive (handled by the [`zip`] crate) whose
//! members are XML files (handled by the [`coding_adventures_xml_parser`]
//! crate). OPC adds three conventions on top of that raw ZIP:
//!
//! 1. **Part names.** Members are addressed by a *logical* absolute name that
//!    starts with `/` (e.g. `/xl/workbook.xml`) even though the ZIP stores them
//!    without the leading slash (`xl/workbook.xml`).
//! 2. **Content types.** `/[Content_Types].xml` states each part's media type,
//!    either by file **extension** (`<Default>`) or by exact **part name**
//!    (`<Override>`, which wins).
//! 3. **Relationships.** `.rels` files map short ids (`rId1`) to targets, so
//!    parts point at one another *by id*, never by hard-coded path.
//!
//! Crucially, OPC knows **nothing** about spreadsheets or documents — it is
//! purely the packaging layer. Interpreting `xl/workbook.xml` as a workbook is
//! the next milestone (M3).
//!
//! ```
//! # // (doctest uses the same fixture the unit tests use.)
//! # use coding_adventures_opc::Package;
//! # const BYTES: &[u8] = coding_adventures_opc::fixture::MINIMAL_XLSX;
//! let pkg = Package::open(BYTES).unwrap();
//! // The main document part is discovered, not hard-coded:
//! assert_eq!(pkg.main_document_part().as_deref(), Some("/xl/workbook.xml"));
//! // Content type comes from an <Override>:
//! assert!(pkg.content_type("/xl/workbook.xml").unwrap().contains("sheet.main+xml"));
//! // A relationship id dereferences to a part name:
//! assert_eq!(
//!     pkg.resolve("/xl/workbook.xml", "rId1").as_deref(),
//!     Some("/xl/worksheets/sheet1.xml"),
//! );
//! ```

use std::cell::RefCell;
use std::collections::HashMap;

use coding_adventures_xml_parser::parse_xml;

// ===========================================================================
// Namespaces (ECMA-376 Part 2)
// ===========================================================================

/// Namespace of `[Content_Types].xml` (`<Types>`, `<Default>`, `<Override>`).
const CONTENT_TYPES_NS: &str = "http://schemas.openxmlformats.org/package/2006/content-types";

/// Namespace of every `.rels` file (`<Relationships>`, `<Relationship>`).
const RELATIONSHIPS_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

/// The relationship **Type** URI of the main document part ends with this. A
/// `.docx` and an `.xlsx` use different *targets* but the same *type* here,
/// which is exactly why bootstrap is a lookup and not a constant.
const OFFICE_DOCUMENT_TYPE_SUFFIX: &str = "/officeDocument";

/// The canonical name of the content-types part (it has no leading slash inside
/// the ZIP, and is special-cased — it is the one part *not* itself typed).
const CONTENT_TYPES_PART: &str = "/[Content_Types].xml";

// ===========================================================================
// Errors
// ===========================================================================

/// Everything that can go wrong opening or reading an OPC package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpcError {
    /// The bytes were not a readable ZIP archive (wraps the zip crate's error).
    NotAZip(String),
    /// The package had no `/[Content_Types].xml`; it is not a valid OPC package.
    MissingContentTypes,
    /// A part that must be XML (content-types or a `.rels`) failed to parse.
    MalformedXml(String),
    /// A part that must be XML was not valid UTF-8.
    NotUtf8(String),
}

impl std::fmt::Display for OpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpcError::NotAZip(m) => write!(f, "not a ZIP archive: {m}"),
            OpcError::MissingContentTypes => write!(f, "missing /[Content_Types].xml"),
            OpcError::MalformedXml(m) => write!(f, "malformed XML: {m}"),
            OpcError::NotUtf8(m) => write!(f, "part is not valid UTF-8: {m}"),
        }
    }
}

impl std::error::Error for OpcError {}

// ===========================================================================
// Relationships
// ===========================================================================

/// Whether a relationship target is inside the package or points outside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetMode {
    /// The target is another part in this package (the default).
    Internal,
    /// The target is outside the package (e.g. an `http://` hyperlink); it is
    /// **not** resolved to a part name.
    External,
}

/// One `<Relationship>` entry, as parsed from a `.rels` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relationship {
    /// The `Id`, unique within its own `.rels` file (e.g. `"rId1"`).
    pub id: String,
    /// The `Type` URI describing what kind of link this is.
    pub rel_type: String,
    /// The raw `Target` attribute value, exactly as written in the XML.
    pub target: String,
    /// `Internal` (default) or `External`, from the `TargetMode` attribute.
    pub mode: TargetMode,
    /// The target resolved to a logical part name — `Some` iff the mode is
    /// `Internal`; always `None` for `External` targets.
    pub resolved_target: Option<String>,
}

// ===========================================================================
// Package
// ===========================================================================

/// An opened OPC package: parts, content types, and (lazily parsed)
/// relationships.
///
/// [`Package::open`] reads every ZIP member into memory and parses
/// `[Content_Types].xml` eagerly (failing fast if it is missing or malformed).
/// Relationships are parsed on first use per source part and cached in a
/// [`RefCell`] — most parts have none, so parsing them all up front would be
/// wasted work.
#[derive(Debug)]
pub struct Package {
    /// Logical part name (`/`-rooted) → the part's raw bytes.
    parts: HashMap<String, Vec<u8>>,
    /// `<Default>` rules: lowercased extension → content type.
    default_types: HashMap<String, String>,
    /// `<Override>` rules: exact logical part name → content type.
    override_types: HashMap<String, String>,
    /// Memoized relationship lists, keyed by *source part* logical name (with
    /// `"/"` denoting the package-level `/_rels/.rels`).
    rels_cache: RefCell<HashMap<String, Vec<Relationship>>>,
}

impl Package {
    /// Open a package from the bytes of an OOXML file.
    ///
    /// Fails with [`OpcError::NotAZip`] if the bytes are not a ZIP, or
    /// [`OpcError::MissingContentTypes`] / [`OpcError::MalformedXml`] if the
    /// content-types part is absent or unparseable.
    pub fn open(bytes: &[u8]) -> Result<Package, OpcError> {
        // Step 1 — inflate every ZIP member into memory. `unzip` returns
        // (name, bytes) pairs; entry names have NO leading slash, so we add one
        // to get the logical part name. Directory entries (trailing `/`) are
        // dropped — they are not parts.
        let members = zip::unzip(bytes).map_err(OpcError::NotAZip)?;
        let mut parts = HashMap::new();
        for (name, data) in members {
            if name.ends_with('/') {
                continue; // a directory entry, not a part
            }
            parts.insert(to_logical(&name), data);
        }

        // Step 2 — parse [Content_Types].xml eagerly. A package without it is
        // not valid OPC.
        let ct_bytes = parts
            .get(CONTENT_TYPES_PART)
            .ok_or(OpcError::MissingContentTypes)?;
        let ct_doc = parse_part_xml(ct_bytes)?;
        let root = &ct_doc.root;

        let mut default_types = HashMap::new();
        for def in root.get_children(Some(CONTENT_TYPES_NS), "Default") {
            if let (Some(ext), Some(ct)) = (
                def.get_attr(None, "Extension"),
                def.get_attr(None, "ContentType"),
            ) {
                default_types.insert(ext.to_ascii_lowercase(), ct.to_string());
            }
        }

        let mut override_types = HashMap::new();
        for ov in root.get_children(Some(CONTENT_TYPES_NS), "Override") {
            if let (Some(part), Some(ct)) = (
                ov.get_attr(None, "PartName"),
                ov.get_attr(None, "ContentType"),
            ) {
                // PartName is already "/"-rooted per spec, but normalize to be
                // robust against sloppy producers.
                override_types.insert(normalize_part_name(part), ct.to_string());
            }
        }

        Ok(Package {
            parts,
            default_types,
            override_types,
            rels_cache: RefCell::new(HashMap::new()),
        })
    }

    /// All logical part names in the package, sorted for determinism. Includes
    /// `/[Content_Types].xml` and every `.rels` part.
    pub fn part_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.parts.keys().cloned().collect();
        names.sort();
        names
    }

    /// Whether a part exists. Accepts the name with or without a leading `/`.
    pub fn has_part(&self, name: &str) -> bool {
        self.parts.contains_key(&normalize_part_name(name))
    }

    /// The raw bytes of a part, or `None` if it does not exist. Accepts the name
    /// with or without a leading `/`.
    pub fn read_part(&self, name: &str) -> Option<&[u8]> {
        self.parts
            .get(&normalize_part_name(name))
            .map(|v| v.as_slice())
    }

    /// The content type of a part, resolving **Override before Default**
    /// (see the spec's truth table). `None` if no rule applies.
    pub fn content_type(&self, part_name: &str) -> Option<String> {
        let name = normalize_part_name(part_name);
        // Override wins: an exact part-name match.
        if let Some(ct) = self.override_types.get(&name) {
            return Some(ct.clone());
        }
        // Otherwise fall back to the Default for this part's extension.
        let ext = extension_of(&name)?;
        self.default_types.get(&ext).cloned()
    }

    /// The relationships declared *for* `source_part`. Pass `"/"` (or `""`) to
    /// get the **package-level** relationships in `/_rels/.rels`.
    ///
    /// Returns an empty list (not an error) when the source part has no `.rels`
    /// file. Errors only if a present `.rels` file is malformed.
    pub fn relationships(&self, source_part: &str) -> Result<Vec<Relationship>, OpcError> {
        let source = normalize_source(source_part);

        // Fast path: already parsed and cached.
        if let Some(cached) = self.rels_cache.borrow().get(&source) {
            return Ok(cached.clone());
        }

        let rels = self.parse_relationships(&source)?;
        self.rels_cache
            .borrow_mut()
            .insert(source.clone(), rels.clone());
        Ok(rels)
    }

    /// Dereference a relationship id on `source_part` to a resolved part name.
    /// Returns `None` for unknown ids, external targets, or malformed `.rels`.
    pub fn resolve(&self, source_part: &str, rel_id: &str) -> Option<String> {
        let rels = self.relationships(source_part).ok()?;
        rels.into_iter()
            .find(|r| r.id == rel_id)
            .and_then(|r| r.resolved_target)
    }

    /// The main document part, found by following the package-level
    /// relationship whose Type ends with `/officeDocument`. Yields
    /// `/xl/workbook.xml` for a workbook, `/word/document.xml` for a document,
    /// etc. `None` if the package declares no such relationship.
    pub fn main_document_part(&self) -> Option<String> {
        let rels = self.relationships("/").ok()?;
        rels.into_iter()
            .find(|r| r.rel_type.ends_with(OFFICE_DOCUMENT_TYPE_SUFFIX))
            .and_then(|r| r.resolved_target)
    }

    // -----------------------------------------------------------------------
    // Internals
    // -----------------------------------------------------------------------

    /// Parse (uncached) the `.rels` file for a source part. An absent `.rels`
    /// part means "no relationships", which is a legitimate empty result.
    fn parse_relationships(&self, source: &str) -> Result<Vec<Relationship>, OpcError> {
        let rels_part = rels_part_for(source);
        let bytes = match self.parts.get(&rels_part) {
            Some(b) => b,
            None => return Ok(Vec::new()), // no rels file ⇒ no relationships
        };

        let doc = parse_part_xml(bytes)?;
        // The source *directory* is the base for resolving relative targets.
        let base_dir = directory_of(source);

        let mut out = Vec::new();
        for rel in doc
            .root
            .get_children(Some(RELATIONSHIPS_NS), "Relationship")
        {
            let id = match rel.get_attr(None, "Id") {
                Some(v) => v.to_string(),
                None => continue, // a Relationship with no Id is unusable
            };
            let rel_type = rel.get_attr(None, "Type").unwrap_or("").to_string();
            let target = rel.get_attr(None, "Target").unwrap_or("").to_string();

            // TargetMode is optional and defaults to Internal.
            let mode = match rel.get_attr(None, "TargetMode") {
                Some(m) if m.eq_ignore_ascii_case("External") => TargetMode::External,
                _ => TargetMode::Internal,
            };

            // Only internal targets resolve to a part name. External targets
            // are left as opaque URIs.
            let resolved_target = match mode {
                TargetMode::Internal => Some(join_target(&base_dir, &target)),
                TargetMode::External => None,
            };

            out.push(Relationship {
                id,
                rel_type,
                target,
                mode,
                resolved_target,
            });
        }
        Ok(out)
    }
}

// ===========================================================================
// Free functions — name & URI arithmetic (unit-tested directly)
// ===========================================================================

/// Turn a ZIP entry name (no leading slash) into a logical part name.
fn to_logical(zip_name: &str) -> String {
    normalize_part_name(zip_name)
}

/// Normalize a caller-supplied part name to the internal `/`-rooted form.
/// Accepts `"x"`, `"/x"`, and even `"//x"`; collapses to a single leading `/`.
fn normalize_part_name(name: &str) -> String {
    let trimmed = name.trim_start_matches('/');
    format!("/{trimmed}")
}

/// Normalize a *source-part* argument. The package sentinel `"/"` or `""` maps
/// to `"/"`; everything else is a normal part name.
fn normalize_source(source: &str) -> String {
    if source.is_empty() || source == "/" {
        "/".to_string()
    } else {
        normalize_part_name(source)
    }
}

/// The directory of a part, as a `/`-rooted, `/`-terminated prefix.
///
/// ```text
/// /xl/workbook.xml   → /xl/
/// /_rels/.rels       → /_rels/
/// /workbook.xml      → /
/// /                  → /       (the package root)
/// ```
fn directory_of(part: &str) -> String {
    if part == "/" {
        return "/".to_string();
    }
    match part.rfind('/') {
        Some(idx) => part[..=idx].to_string(), // keep the trailing slash
        None => "/".to_string(),
    }
}

/// The lowercased file extension of a part name, or `None` if it has none.
/// The extension is the text after the last `.` in the final path segment.
fn extension_of(part: &str) -> Option<String> {
    let last_segment = part.rsplit('/').next().unwrap_or("");
    // A leading-dot name like ".rels" — for `/_rels/.rels` the segment is
    // ".rels", whose extension is "rels".
    let dot = last_segment.rfind('.')?;
    // Guard against an empty extension ("foo.").
    let ext = &last_segment[dot + 1..];
    if ext.is_empty() {
        None
    } else {
        Some(ext.to_ascii_lowercase())
    }
}

/// The `.rels` part name that holds a source part's relationships.
///
/// ```text
/// "/"                 → /_rels/.rels          (package level)
/// /xl/workbook.xml    → /xl/_rels/workbook.xml.rels
/// /word/document.xml  → /word/_rels/document.xml.rels
/// ```
fn rels_part_for(source: &str) -> String {
    if source == "/" {
        return "/_rels/.rels".to_string();
    }
    let dir = directory_of(source); // "/xl/"
    let file = source.rsplit('/').next().unwrap_or(""); // "workbook.xml"
    format!("{dir}_rels/{file}.rels")
}

/// Resolve a relationship `Target` against the source part's directory,
/// honoring `.`/`..` and **clamping at the package root** so a hostile
/// `../../..` can never escape the package. The result is always a well-formed,
/// `/`-rooted logical part name.
///
/// ```text
/// join("/xl/",   "worksheets/sheet1.xml")  = /xl/worksheets/sheet1.xml
/// join("/xl/",   "../docProps/core.xml")   = /docProps/core.xml
/// join("/xl/",   "/media/logo.png")        = /media/logo.png    (root-relative)
/// join("/xl/",   "../../../../etc/passwd") = /etc/passwd        (clamped, stays rooted)
/// ```
fn join_target(base_dir: &str, target: &str) -> String {
    // A target starting with '/' is package-root-relative; ignore the base.
    let combined = if target.starts_with('/') {
        target.to_string()
    } else {
        format!("{base_dir}{target}")
    };

    // Walk the segments, resolving "." and ".." against a stack. A ".." that
    // would pop past the root is simply dropped — this is the traversal clamp.
    let mut stack: Vec<&str> = Vec::new();
    for seg in combined.split('/') {
        match seg {
            "" | "." => {} // empty (from leading '/' or '//') and "." are no-ops
            ".." => {
                stack.pop(); // pop() on an empty stack is a harmless no-op ⇒ clamp
            }
            other => stack.push(other),
        }
    }
    format!("/{}", stack.join("/"))
}

/// Parse a part's raw bytes as UTF-8 XML, mapping both failure modes to
/// [`OpcError`].
fn parse_part_xml(
    bytes: &[u8],
) -> Result<coding_adventures_xml_parser::XmlDocument, OpcError> {
    let text = std::str::from_utf8(bytes).map_err(|e| OpcError::NotUtf8(e.to_string()))?;
    parse_xml(text).map_err(|e| OpcError::MalformedXml(format!("{e:?}")))
}

// ===========================================================================
// Test fixture — a real DEFLATE-compressed .xlsx (6 OPC parts).
//
// Exposed publicly (behind a plain module, not #[cfg(test)]) so the doctest
// above and downstream crates can reuse the same known-good bytes.
// ===========================================================================

/// A minimal but real `.xlsx` package used across the tests and the crate
/// doctest. See `tests/` for how it is exercised.
pub mod fixture {
    include!("fixture.rs");
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- pure name/URI arithmetic (no package needed) ----------------------

    #[test]
    fn normalize_adds_single_leading_slash() {
        assert_eq!(normalize_part_name("xl/workbook.xml"), "/xl/workbook.xml");
        assert_eq!(normalize_part_name("/xl/workbook.xml"), "/xl/workbook.xml");
        assert_eq!(normalize_part_name("//xl/workbook.xml"), "/xl/workbook.xml");
    }

    #[test]
    fn source_sentinel_maps_to_root() {
        assert_eq!(normalize_source(""), "/");
        assert_eq!(normalize_source("/"), "/");
        assert_eq!(normalize_source("xl/workbook.xml"), "/xl/workbook.xml");
    }

    #[test]
    fn directory_of_various() {
        assert_eq!(directory_of("/xl/workbook.xml"), "/xl/");
        assert_eq!(directory_of("/workbook.xml"), "/");
        assert_eq!(directory_of("/xl/worksheets/sheet1.xml"), "/xl/worksheets/");
        assert_eq!(directory_of("/"), "/");
    }

    #[test]
    fn extension_of_various() {
        assert_eq!(extension_of("/xl/workbook.xml").as_deref(), Some("xml"));
        assert_eq!(extension_of("/_rels/.rels").as_deref(), Some("rels"));
        assert_eq!(extension_of("/media/LOGO.PNG").as_deref(), Some("png")); // lowercased
        assert_eq!(extension_of("/noext"), None);
        assert_eq!(extension_of("/trailingdot."), None);
    }

    #[test]
    fn rels_part_for_various() {
        assert_eq!(rels_part_for("/"), "/_rels/.rels");
        assert_eq!(
            rels_part_for("/xl/workbook.xml"),
            "/xl/_rels/workbook.xml.rels"
        );
        assert_eq!(
            rels_part_for("/word/document.xml"),
            "/word/_rels/document.xml.rels"
        );
    }

    #[test]
    fn join_relative_target() {
        assert_eq!(
            join_target("/xl/", "worksheets/sheet1.xml"),
            "/xl/worksheets/sheet1.xml"
        );
        assert_eq!(join_target("/xl/", "sharedStrings.xml"), "/xl/sharedStrings.xml");
    }

    #[test]
    fn join_parent_target() {
        assert_eq!(join_target("/xl/", "../docProps/core.xml"), "/docProps/core.xml");
        assert_eq!(join_target("/xl/worksheets/", "../styles.xml"), "/xl/styles.xml");
    }

    #[test]
    fn join_root_relative_target_ignores_base() {
        assert_eq!(join_target("/xl/", "/media/logo.png"), "/media/logo.png");
    }

    #[test]
    fn join_dot_segment_is_noop() {
        assert_eq!(join_target("/xl/", "./workbook.xml"), "/xl/workbook.xml");
    }

    #[test]
    fn join_clamps_traversal_at_root() {
        // A hostile target must never escape above the package root.
        let escaped = join_target("/xl/", "../../../../etc/passwd");
        assert_eq!(escaped, "/etc/passwd");
        assert!(escaped.starts_with('/'));
        assert!(!escaped.contains(".."));
    }
}
