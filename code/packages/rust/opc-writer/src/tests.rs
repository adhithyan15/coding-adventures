//! Unit + round-trip tests for `opc-writer`.
//!
//! The round-trip tests are the load-bearing ones: they build a package with
//! `PackageWriter` and re-open the bytes with the read-side `opc` crate, proving
//! the writer emits exactly what our own (and any conforming) OPC reader expects.

use super::*;
use coding_adventures_opc::Package;

// ── XML escaping ──────────────────────────────────────────────────────────

#[test]
fn escape_handles_all_special_chars() {
    assert_eq!(xml_escape("a & b"), "a &amp; b");
    assert_eq!(xml_escape("<tag>"), "&lt;tag&gt;");
    assert_eq!(xml_escape("\"q\""), "&quot;q&quot;");
    assert_eq!(xml_escape("it's"), "it&apos;s");
    // Ampersand must be escaped first — no double-escaping.
    assert_eq!(xml_escape("&lt;"), "&amp;lt;");
}

#[test]
fn escape_passes_unicode_through() {
    assert_eq!(xml_escape("日本語 résumé"), "日本語 résumé");
    assert_eq!(xml_escape(""), "");
    assert_eq!(xml_escape("plain text 123"), "plain text 123");
}

// ── Part-name normalization ───────────────────────────────────────────────

#[test]
fn part_names_normalize_either_spelling() {
    assert_eq!(zip_member_name("/xl/workbook.xml"), "xl/workbook.xml");
    assert_eq!(zip_member_name("xl/workbook.xml"), "xl/workbook.xml");
    assert_eq!(override_part_name("xl/workbook.xml"), "/xl/workbook.xml");
    assert_eq!(override_part_name("/xl/workbook.xml"), "/xl/workbook.xml");
}

// ── Relationships serialization ───────────────────────────────────────────

#[test]
fn rels_builder_serializes_entries() {
    let mut r = RelationshipsBuilder::new();
    r.add("rId1", "http://example/officeDocument", "xl/workbook.xml");
    r.add("rId2", "http://example/sharedStrings", "sharedStrings.xml");
    let text = String::from_utf8(r.build()).unwrap();
    assert!(text.contains(RELATIONSHIPS_NS));
    assert!(text.contains("Id=\"rId1\""));
    assert!(text.contains("Target=\"xl/workbook.xml\""));
    assert!(text.contains("Id=\"rId2\""));
    assert!(text.contains("Type=\"http://example/sharedStrings\""));
}

#[test]
fn rels_builder_escapes_targets() {
    let mut r = RelationshipsBuilder::new();
    r.add("rId1", "http://example/t", "a&b<c>.xml");
    let text = String::from_utf8(r.build()).unwrap();
    assert!(text.contains("Target=\"a&amp;b&lt;c&gt;.xml\""));
}

// ── Content-types synthesis ───────────────────────────────────────────────

#[test]
fn content_types_emits_defaults_and_overrides() {
    let mut pkg = PackageWriter::new();
    pkg.add_default("rels", "application/vnd.openxmlformats-package.relationships+xml");
    pkg.add_default("xml", "application/xml");
    pkg.add_part("/xl/workbook.xml", "application/custom+xml", b"<w/>");
    let xml = String::from_utf8(pkg.content_types_xml()).unwrap();
    assert!(xml.contains(CONTENT_TYPES_NS));
    assert!(xml.contains("<Default Extension=\"rels\""));
    assert!(xml.contains("<Default Extension=\"xml\""));
    assert!(xml.contains("<Override PartName=\"/xl/workbook.xml\" ContentType=\"application/custom+xml\""));
}

#[test]
fn content_types_dedups_defaults_last_wins() {
    let mut pkg = PackageWriter::new();
    pkg.add_default("xml", "application/first");
    pkg.add_default("xml", "application/second");
    let xml = String::from_utf8(pkg.content_types_xml()).unwrap();
    // Exactly one Default for "xml", and it carries the LAST content type.
    assert_eq!(xml.matches("Extension=\"xml\"").count(), 1);
    assert!(xml.contains("application/second"));
    assert!(!xml.contains("application/first"));
}

#[test]
fn content_types_dedups_overrides_last_wins() {
    let mut pkg = PackageWriter::new();
    pkg.add_part("/a.xml", "application/first", b"1");
    pkg.add_part("/a.xml", "application/second", b"2");
    let xml = String::from_utf8(pkg.content_types_xml()).unwrap();
    assert_eq!(xml.matches("PartName=\"/a.xml\"").count(), 1);
    assert!(xml.contains("application/second"));
}

#[test]
fn defaulted_part_gets_no_override() {
    let mut pkg = PackageWriter::new();
    pkg.add_default("rels", "application/vnd.openxmlformats-package.relationships+xml");
    pkg.add_part_defaulted("/_rels/.rels", b"<Relationships/>");
    let xml = String::from_utf8(pkg.content_types_xml()).unwrap();
    // No Override for the .rels part — it is typed by the Default.
    assert!(!xml.contains("PartName=\"/_rels/.rels\""));
}

// ── Empty package ─────────────────────────────────────────────────────────

#[test]
fn empty_package_still_produces_valid_zip() {
    let pkg = PackageWriter::new();
    let bytes = pkg.finish();
    // Starts with the ZIP local-file-header signature "PK\x03\x04" (or is the
    // empty-archive EOCD "PK\x05\x06" if no members) — here [Content_Types].xml
    // is always present, so it's a local header.
    assert_eq!(&bytes[..2], b"PK");
}

// ── Round-trip through the read-side opc crate ────────────────────────────
//
// Build a minimal package with a main-document relationship and re-open it with
// coding_adventures_opc::Package. This proves content-type resolution and
// relationship dereferencing both work against our own reader.

#[test]
fn round_trips_through_opc_reader() {
    const OFFICE_DOC_TYPE: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
    const WORKSHEET_TYPE: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";

    let mut pkg = PackageWriter::new();
    pkg.add_default("rels", "application/vnd.openxmlformats-package.relationships+xml");
    pkg.add_default("xml", "application/xml");

    // Package-root rels: package → workbook.
    let mut root_rels = RelationshipsBuilder::new();
    root_rels.add("rId1", OFFICE_DOC_TYPE, "xl/workbook.xml");
    pkg.add_part_defaulted("/_rels/.rels", &root_rels.build());

    // Workbook part (typed via Override).
    pkg.add_part(
        "/xl/workbook.xml",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml",
        b"<workbook/>",
    );

    // Workbook rels: workbook → sheet1 (target relative to xl/).
    let mut wb_rels = RelationshipsBuilder::new();
    wb_rels.add("rId1", WORKSHEET_TYPE, "worksheets/sheet1.xml");
    pkg.add_part_defaulted("/xl/_rels/workbook.xml.rels", &wb_rels.build());

    pkg.add_part(
        "/xl/worksheets/sheet1.xml",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml",
        b"<worksheet/>",
    );

    let bytes = pkg.finish();

    // Re-open with the READ-SIDE opc crate.
    let read = Package::open(&bytes).expect("opc reader should open our package");

    // The main document part is discovered via the /officeDocument relationship.
    assert_eq!(read.main_document_part().as_deref(), Some("/xl/workbook.xml"));

    // Content type resolves through the Override.
    assert_eq!(
        read.content_type("/xl/workbook.xml").as_deref(),
        Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"),
    );

    // The rels default types the .rels parts.
    assert_eq!(
        read.content_type("/_rels/.rels").as_deref(),
        Some("application/vnd.openxmlformats-package.relationships+xml"),
    );

    // The workbook's own relationship dereferences to the sheet part.
    assert_eq!(
        read.resolve("/xl/workbook.xml", "rId1").as_deref(),
        Some("/xl/worksheets/sheet1.xml"),
    );

    // Part bytes survive the round-trip.
    assert_eq!(read.read_part("/xl/worksheets/sheet1.xml"), Some(&b"<worksheet/>"[..]));
}

// --- Security regressions (from the C1 security review) --------------------

#[test]
fn xml_escape_drops_illegal_control_chars() {
    // NUL and 0x01 are illegal in XML 1.0 and cannot be entity-escaped; they
    // must be dropped so the package stays parseable.
    assert_eq!(xml_escape("ab\u{0}cd\u{1}ef"), "abcdef");
    // But the three legal control chars are preserved.
    assert_eq!(xml_escape("a\tb\nc\rd"), "a\tb\nc\rd");
    // And the ordinary specials still escape.
    assert_eq!(xml_escape("x&<>\"'y"), "x&amp;&lt;&gt;&quot;&apos;y");
}

#[test]
fn part_names_cannot_traverse_out_of_the_package() {
    // A hostile part name must not become a Zip-Slip member name.
    assert_eq!(zip_member_name("/../../evil.xml"), "evil.xml");
    assert_eq!(zip_member_name("xl\\..\\..\\evil"), "xl/evil");
    assert_eq!(zip_member_name("//a///b/./c"), "a/b/c");
    // A normal name is unchanged (minus the leading slash).
    assert_eq!(zip_member_name("/xl/workbook.xml"), "xl/workbook.xml");
    // Override form stays consistent with the member form.
    assert_eq!(override_part_name("/../../evil.xml"), "/evil.xml");
}
