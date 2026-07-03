//! End-to-end tests over the real DEFLATE-compressed `.xlsx` fixture
//! (`MINIMAL_XLSX`, 6 OPC parts). These exercise the whole stack: the `zip`
//! crate inflating dynamic-Huffman DEFLATE, the `xml-parser` crate parsing the
//! parts, and this crate resolving content types and relationships.
//!
//! Fixture parts:
//!   /[Content_Types].xml
//!   /_rels/.rels
//!   /xl/workbook.xml
//!   /xl/_rels/workbook.xml.rels
//!   /xl/sharedStrings.xml
//!   /xl/worksheets/sheet1.xml

use coding_adventures_opc::{OpcError, Package, TargetMode};

const XLSX: &[u8] = coding_adventures_opc::fixture::MINIMAL_XLSX;

/// The fixture inflates and opens cleanly — verifying the DEFLATE + XML stack
/// end to end.
#[test]
fn opens_the_fixture_and_lists_all_six_parts() {
    let pkg = Package::open(XLSX).unwrap();
    let names = pkg.part_names();
    assert_eq!(
        names,
        vec![
            "/[Content_Types].xml".to_string(),
            "/_rels/.rels".to_string(),
            "/xl/_rels/workbook.xml.rels".to_string(),
            "/xl/sharedStrings.xml".to_string(),
            "/xl/workbook.xml".to_string(),
            "/xl/worksheets/sheet1.xml".to_string(),
        ]
    );
}

#[test]
fn has_part_and_read_part_accept_either_slash_form() {
    let pkg = Package::open(XLSX).unwrap();

    assert!(pkg.has_part("/xl/workbook.xml"));
    assert!(pkg.has_part("xl/workbook.xml")); // no leading slash also works
    assert!(!pkg.has_part("/xl/nope.xml"));

    let with = pkg.read_part("/xl/workbook.xml").unwrap();
    let without = pkg.read_part("xl/workbook.xml").unwrap();
    assert_eq!(with, without);
    // It really is the workbook XML.
    let text = std::str::from_utf8(with).unwrap();
    assert!(text.contains("<workbook") || text.contains(":workbook"));

    assert!(pkg.read_part("/does/not/exist.xml").is_none());
}

/// Override beats Default: the workbook part has an explicit `<Override>`.
#[test]
fn content_type_override_for_workbook() {
    let pkg = Package::open(XLSX).unwrap();
    let ct = pkg.content_type("/xl/workbook.xml").unwrap();
    assert_eq!(
        ct,
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"
    );
    // Slash-insensitive at the boundary.
    assert_eq!(pkg.content_type("xl/workbook.xml"), Some(ct));
}

/// Default-by-extension: no `<Override>` names a `.rels` file, so it resolves
/// through the `<Default Extension="rels" …>` rule.
#[test]
fn content_type_default_by_extension_for_rels() {
    let pkg = Package::open(XLSX).unwrap();
    assert_eq!(
        pkg.content_type("/_rels/.rels").as_deref(),
        Some("application/vnd.openxmlformats-package.relationships+xml")
    );
    // The content-types part itself is `.xml` with no override ⇒ Default xml.
    assert_eq!(
        pkg.content_type("/[Content_Types].xml").as_deref(),
        Some("application/xml")
    );
}

#[test]
fn content_type_none_for_unknown_extension() {
    let pkg = Package::open(XLSX).unwrap();
    // A hypothetical part name with no matching Default/Override.
    assert_eq!(pkg.content_type("/media/logo.png"), None);
    assert_eq!(pkg.content_type("/noextension"), None);
}

/// Package-level bootstrap: `/_rels/.rels` names the officeDocument.
#[test]
fn package_relationships_find_office_document() {
    let pkg = Package::open(XLSX).unwrap();
    let rels = pkg.relationships("/").unwrap();
    assert_eq!(rels.len(), 1);
    let r = &rels[0];
    assert_eq!(r.id, "rId1");
    assert!(r.rel_type.ends_with("/officeDocument"));
    assert_eq!(r.mode, TargetMode::Internal);
    // Relative target "xl/workbook.xml" resolves against the root directory.
    assert_eq!(r.resolved_target.as_deref(), Some("/xl/workbook.xml"));
    assert_eq!(r.target, "xl/workbook.xml"); // raw value preserved

    // The empty string is an accepted alias for the package root.
    assert_eq!(pkg.relationships("").unwrap(), rels);
}

#[test]
fn main_document_part_is_the_workbook() {
    let pkg = Package::open(XLSX).unwrap();
    assert_eq!(
        pkg.main_document_part().as_deref(),
        Some("/xl/workbook.xml")
    );
}

/// Relative-target join: from `/xl/workbook.xml`, targets resolve against
/// `/xl/`.
#[test]
fn workbook_relationships_resolve_relative_targets() {
    let pkg = Package::open(XLSX).unwrap();
    let rels = pkg.relationships("/xl/workbook.xml").unwrap();
    assert_eq!(rels.len(), 2);

    // Look them up by id (order in file is rId1, rId2).
    let sheet = rels.iter().find(|r| r.id == "rId1").unwrap();
    assert!(sheet.rel_type.ends_with("/worksheet"));
    assert_eq!(
        sheet.resolved_target.as_deref(),
        Some("/xl/worksheets/sheet1.xml")
    );

    let ss = rels.iter().find(|r| r.id == "rId2").unwrap();
    assert!(ss.rel_type.ends_with("/sharedStrings"));
    assert_eq!(ss.resolved_target.as_deref(), Some("/xl/sharedStrings.xml"));
}

#[test]
fn resolve_dereferences_ids() {
    let pkg = Package::open(XLSX).unwrap();
    assert_eq!(
        pkg.resolve("/xl/workbook.xml", "rId1").as_deref(),
        Some("/xl/worksheets/sheet1.xml")
    );
    assert_eq!(
        pkg.resolve("/xl/workbook.xml", "rId2").as_deref(),
        Some("/xl/sharedStrings.xml")
    );
    // Unknown id ⇒ None.
    assert_eq!(pkg.resolve("/xl/workbook.xml", "rId999"), None);
    // A part with no .rels file ⇒ no relationships ⇒ None.
    assert_eq!(pkg.resolve("/xl/sharedStrings.xml", "rId1"), None);
}

/// A part that has no `.rels` file yields an empty (non-error) list.
#[test]
fn part_without_rels_yields_empty() {
    let pkg = Package::open(XLSX).unwrap();
    assert!(pkg
        .relationships("/xl/worksheets/sheet1.xml")
        .unwrap()
        .is_empty());
}

// --- error cases -----------------------------------------------------------

#[test]
fn not_a_zip_errors() {
    let err = Package::open(b"this is definitely not a zip file").unwrap_err();
    assert!(matches!(err, OpcError::NotAZip(_)));
}

#[test]
fn missing_content_types_errors() {
    // Build a valid ZIP that simply lacks [Content_Types].xml.
    let bytes = zip::zip(&[("hello.txt", b"hi" as &[u8])]);
    let err = Package::open(&bytes).unwrap_err();
    assert_eq!(err, OpcError::MissingContentTypes);
}

#[test]
fn malformed_content_types_errors() {
    // A [Content_Types].xml that is not well-formed XML.
    let bytes = zip::zip(&[("[Content_Types].xml", b"<Types><oops" as &[u8])]);
    let err = Package::open(&bytes).unwrap_err();
    assert!(matches!(err, OpcError::MalformedXml(_)));
}

#[test]
fn content_types_not_utf8_errors() {
    // Invalid UTF-8 bytes for the content-types part.
    let bad: &[u8] = &[0xff, 0xfe, 0x00];
    let bytes = zip::zip(&[("[Content_Types].xml", bad)]);
    let err = Package::open(&bytes).unwrap_err();
    assert!(matches!(err, OpcError::NotUtf8(_)));
}

// --- security: external targets and traversal ------------------------------

/// A synthetic package exercising External targets and a `../` traversal in a
/// relationship, asserting neither escapes the package as an internal part.
#[test]
fn external_and_traversal_targets_are_safe() {
    let ct = r#"<?xml version="1.0"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
</Types>"#;

    // Relationships FOR /xl/workbook.xml live in /xl/_rels/workbook.xml.rels.
    let wb_rels = r#"<?xml version="1.0"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rExt" Type="http://x/hyperlink" Target="https://example.com/" TargetMode="External"/>
  <Relationship Id="rEsc" Type="http://x/thing" Target="../../../../etc/passwd"/>
</Relationships>"#;

    let root_rels = r#"<?xml version="1.0"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#;

    let bytes = zip::zip(&[
        ("[Content_Types].xml", ct.as_bytes()),
        ("_rels/.rels", root_rels.as_bytes()),
        ("xl/workbook.xml", b"<workbook/>" as &[u8]),
        ("xl/_rels/workbook.xml.rels", wb_rels.as_bytes()),
    ]);

    let pkg = Package::open(&bytes).unwrap();
    let rels = pkg.relationships("/xl/workbook.xml").unwrap();

    let ext = rels.iter().find(|r| r.id == "rExt").unwrap();
    assert_eq!(ext.mode, TargetMode::External);
    // External targets are NOT resolved to a part name.
    assert_eq!(ext.resolved_target, None);
    assert_eq!(ext.target, "https://example.com/");

    let esc = rels.iter().find(|r| r.id == "rEsc").unwrap();
    // The traversal is clamped: it stays a "/"-rooted logical name and does not
    // contain "..".
    let resolved = esc.resolved_target.as_deref().unwrap();
    assert!(resolved.starts_with('/'), "must stay package-rooted");
    assert!(!resolved.contains(".."), "must not contain traversal segments");
    assert_eq!(resolved, "/etc/passwd");
    // And of course it does not name a real part in this package.
    assert!(!pkg.has_part(resolved));

    // resolve() on an external id yields None (no internal target).
    assert_eq!(pkg.resolve("/xl/workbook.xml", "rExt"), None);
}

/// Relationship caching returns identical results on repeated calls.
#[test]
fn relationships_are_cached_consistently() {
    let pkg = Package::open(XLSX).unwrap();
    let a = pkg.relationships("/xl/workbook.xml").unwrap();
    let b = pkg.relationships("/xl/workbook.xml").unwrap();
    assert_eq!(a, b);
}
