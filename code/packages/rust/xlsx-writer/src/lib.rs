//! # `coding-adventures-xlsx-writer` — emit a real `.xlsx` from a simple model
//!
//! This is the **write side** of milestone **C1** and the mirror of the
//! read-side [`spreadsheetml`](https://docs.rs/coding-adventures-spreadsheetml)
//! crate. It takes a small in-memory workbook model and produces the bytes of a
//! valid `.xlsx`, generating the SpreadsheetML XML parts and packaging them via
//! the generic [`opc_writer`](coding_adventures_opc_writer). See
//! `code/specs/XLSXW01-xlsx-writer.md` for the full literate write-up.
//!
//! ```text
//!   Rust model                xlsx-writer                    opc-writer         bytes
//!  ┌───────────┐   generate  ┌──────────────────┐  add_part ┌───────────┐  ZIP ┌────────┐
//!  │ Workbook  │  ─────────► │ workbook.xml      │ ────────►│ Package   │ ────►│ .xlsx  │
//!  │  Sheet    │             │ sheetN.xml        │           │ Writer    │      └────────┘
//!  │  cells    │             │ sharedStrings.xml │           └───────────┘
//!  └───────────┘             │ *.rels, [CT].xml  │
//!                            └──────────────────┘
//! ```
//!
//! ## The `.xlsx` part tree we generate
//!
//! ```text
//! /
//! ├── [Content_Types].xml         content-type registry (opc-writer emits this)
//! ├── _rels/.rels                 package → workbook (rId1)
//! └── xl/
//!     ├── workbook.xml            <sheets>: name + sheetId + r:id per sheet
//!     ├── sharedStrings.xml       deduplicated text table (<sst>)
//!     ├── _rels/workbook.xml.rels workbook → each sheetN.xml + sharedStrings
//!     └── worksheets/
//!         ├── sheet1.xml          <sheetData> rows/cells for sheet 1
//!         └── …
//! ```
//!
//! ## The two normalizations, produced (not consumed)
//!
//! The reader's spec calls out two indirections. The writer must *produce* both:
//!
//! 1. **Shared strings.** Text is deduplicated into a single table; a text cell
//!    stores an *index* with `t="s"`. We build the table as we walk cells — first
//!    occurrence appends and gets the next index, repeats reuse it.
//! 2. **`r:id` → part.** `workbook.xml` names each sheet by a relationship id;
//!    `xl/_rels/workbook.xml.rels` maps that id to `worksheets/sheetN.xml`.
//!
//! ## Example
//!
//! ```
//! use coding_adventures_xlsx_writer::{Workbook, write_xlsx};
//!
//! let mut wb = Workbook::new();
//! let sheet = wb.add_sheet("Revenue");
//! sheet.set_string("A1", "Q1");
//! sheet.set_number("B1", 1000.0);
//! sheet.set_string("A2", "Total");
//! sheet.set_formula("B2", "SUM(B1:B1)", 1000.0);
//!
//! let bytes = write_xlsx(&wb);
//! assert_eq!(&bytes[..2], b"PK"); // a ZIP / .xlsx
//! ```

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use coding_adventures_opc_writer::{xml_escape, PackageWriter, RelationshipsBuilder};

// ===========================================================================
// Namespaces & content types (ECMA-376) — these must match what the reader
// looks for, or a round-trip fails.
// ===========================================================================

/// The SpreadsheetML "main" namespace, on every structural element.
const SML_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

/// The relationships namespace — the `r:` prefix on `<sheet r:id="rId1">`.
const REL_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

const CT_RELS: &str = "application/vnd.openxmlformats-package.relationships+xml";
const CT_XML: &str = "application/xml";
const CT_WORKBOOK: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml";
const CT_WORKSHEET: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";
const CT_SHARED_STRINGS: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml";

const TYPE_OFFICE_DOCUMENT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
const TYPE_WORKSHEET: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";
const TYPE_SHARED_STRINGS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings";

const XML_DECL: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n";

// ===========================================================================
// A1 reference parsing / generation
// ===========================================================================

/// Parse an A1-style reference into `(col, row)`, both 1-based, using bijective
/// base-26 for the column letters (`A`=1 … `Z`=26, `AA`=27). Returns `None` for
/// a malformed ref (empty, no letters, no digits, trailing junk, overflow).
///
/// This is the write-side twin of the reader's `parse_a1_ref`; keeping our own
/// copy avoids a dependency on the reader (which is only a dev-dependency here).
///
/// ```
/// use coding_adventures_xlsx_writer::parse_a1;
/// assert_eq!(parse_a1("A1"), Some((1, 1)));
/// assert_eq!(parse_a1("AA10"), Some((27, 10)));
/// assert_eq!(parse_a1("1A"), None);
/// ```
pub fn parse_a1(a1: &str) -> Option<(u32, u32)> {
    let bytes = a1.as_bytes();
    let mut i = 0;

    let mut col: u32 = 0;
    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
        let letter = bytes[i].to_ascii_uppercase();
        let digit = (letter - b'A' + 1) as u32;
        col = col.checked_mul(26)?.checked_add(digit)?;
        i += 1;
    }
    if col == 0 {
        return None;
    }

    let mut row: u32 = 0;
    let mut saw_digit = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let digit = (bytes[i] - b'0') as u32;
        row = row.checked_mul(10)?.checked_add(digit)?;
        saw_digit = true;
        i += 1;
    }
    if !saw_digit || i != bytes.len() || row == 0 {
        return None;
    }
    Some((col, row))
}

/// Render a 1-based column number as its bijective base-26 letters (`1`→`"A"`,
/// `26`→`"Z"`, `27`→`"AA"`). Column `0` is invalid and yields `"A"` defensively
/// (it never arises from a parsed ref, which is always ≥1).
///
/// ```
/// use coding_adventures_xlsx_writer::col_to_letters;
/// assert_eq!(col_to_letters(1), "A");
/// assert_eq!(col_to_letters(26), "Z");
/// assert_eq!(col_to_letters(27), "AA");
/// assert_eq!(col_to_letters(52), "AZ");
/// assert_eq!(col_to_letters(53), "BA");
/// ```
pub fn col_to_letters(col: u32) -> String {
    let mut n = col.max(1);
    let mut letters = Vec::new();
    while n > 0 {
        // Bijective base-26: subtract 1 so 1→'A' (remainder 0), then divide.
        let rem = (n - 1) % 26;
        letters.push(b'A' + rem as u8);
        n = (n - 1) / 26;
    }
    letters.reverse();
    // Safe: all pushed bytes are ASCII 'A'..='Z'.
    String::from_utf8(letters).unwrap_or_else(|_| "A".to_string())
}

/// Build the A1 reference for a `(col, row)` pair (both 1-based).
fn a1_of(col: u32, row: u32) -> String {
    format!("{}{}", col_to_letters(col), row)
}

// ===========================================================================
// Number formatting for <v>
// ===========================================================================

/// Render an `f64` for a SpreadsheetML `<v>` element. Integers drop the trailing
/// `.0` (`1000`, not `1000.0`) to match what Excel writes and what a human
/// expects; other values use Rust's shortest round-tripping form.
///
/// `NaN`/`±∞` are not representable in `<v>`; we emit `0` for them rather than
/// invalid XML (a documented limitation — the model is trusted not to contain
/// them).
fn format_number(n: f64) -> String {
    if !n.is_finite() {
        return "0".to_string();
    }
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

// ===========================================================================
// The model
// ===========================================================================

/// A single cell's stored content in the write-side model.
#[derive(Debug, Clone)]
enum CellData {
    /// A number: `<c r="…"><v>n</v></c>`.
    Number(f64),
    /// Text (stored via the shared-string table): `<c r="…" t="s"><v>idx</v></c>`.
    Text(String),
    /// A formula plus its cached result: `<c r="…"><f>formula</f><v>cached</v></c>`.
    Formula { formula: String, cached: f64 },
}

/// One worksheet: a name plus its cells, keyed by `(row, col)` (both 1-based) so
/// iteration is naturally row-major, left-to-right — the order Excel writes.
pub struct Sheet {
    name: String,
    cells: BTreeMap<(u32, u32), CellData>,
}

impl Sheet {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            cells: BTreeMap::new(),
        }
    }

    /// Set a numeric cell. A malformed A1 ref is a silent no-op (caller bug,
    /// never a panic).
    pub fn set_number(&mut self, a1: &str, n: f64) {
        if let Some((col, row)) = parse_a1(a1) {
            self.cells.insert((row, col), CellData::Number(n));
        }
    }

    /// Set a text cell. A malformed A1 ref is a silent no-op.
    pub fn set_string(&mut self, a1: &str, s: &str) {
        if let Some((col, row)) = parse_a1(a1) {
            self.cells.insert((row, col), CellData::Text(s.to_string()));
        }
    }

    /// Set a formula cell. `formula` is the formula text **without** a leading
    /// `=`; `cached` is the result written to `<v>` for non-computing viewers.
    /// A malformed A1 ref is a silent no-op.
    pub fn set_formula(&mut self, a1: &str, formula: &str, cached: f64) {
        if let Some((col, row)) = parse_a1(a1) {
            self.cells.insert(
                (row, col),
                CellData::Formula {
                    formula: formula.to_string(),
                    cached,
                },
            );
        }
    }

    /// The sheet's display name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A whole workbook: its sheets in insertion order.
#[derive(Default)]
pub struct Workbook {
    sheets: Vec<Sheet>,
}

impl Workbook {
    /// A new, empty workbook.
    pub fn new() -> Self {
        Self { sheets: Vec::new() }
    }

    /// Add a sheet and return a mutable handle for populating its cells.
    pub fn add_sheet(&mut self, name: &str) -> &mut Sheet {
        self.sheets.push(Sheet::new(name));
        // Just pushed → last() is Some; unwrap_or_else avoids a panic path even so.
        let idx = self.sheets.len() - 1;
        &mut self.sheets[idx]
    }

    /// The sheets, in insertion order.
    pub fn sheets(&self) -> &[Sheet] {
        &self.sheets
    }
}

// ===========================================================================
// Shared-string table
// ===========================================================================

/// A deduplicating shared-string table, built as we walk the workbook.
///
/// The first time a string appears it is appended and assigned the next index;
/// repeats return the existing index. `<sst>` records two counts: `count` =
/// total string-cell references (repeats included), `unique_count` = distinct
/// strings.
#[derive(Default)]
struct SharedStrings {
    /// Distinct strings, in first-seen order (the index is the position).
    strings: Vec<String>,
    /// string → index, for O(1) dedup lookups.
    index: BTreeMap<String, usize>,
    /// Total references, including repeats.
    total_refs: usize,
}

impl SharedStrings {
    /// Intern `s`, returning its shared-string index.
    fn intern(&mut self, s: &str) -> usize {
        self.total_refs += 1;
        if let Some(&i) = self.index.get(s) {
            return i;
        }
        let i = self.strings.len();
        self.strings.push(s.to_string());
        self.index.insert(s.to_string(), i);
        i
    }

    /// Serialize the `<sst>` part.
    fn to_xml(&self) -> Vec<u8> {
        let mut xml = String::new();
        xml.push_str(XML_DECL);
        xml.push_str("<sst xmlns=\"");
        xml.push_str(SML_NS);
        xml.push_str("\" count=\"");
        xml.push_str(&self.total_refs.to_string());
        xml.push_str("\" uniqueCount=\"");
        xml.push_str(&self.strings.len().to_string());
        xml.push_str("\">");
        for s in &self.strings {
            // <t xml:space="preserve"> guards leading/trailing whitespace so a
            // string like " x " survives the round-trip intact.
            xml.push_str("<si><t xml:space=\"preserve\">");
            xml.push_str(&xml_escape(s));
            xml.push_str("</t></si>");
        }
        xml.push_str("</sst>");
        xml.into_bytes()
    }
}

// ===========================================================================
// Part generation
// ===========================================================================

/// Serialize one worksheet's `<worksheet><sheetData>…` part. Cells are grouped
/// into `<row r="N">` runs; a text cell has already been interned and carries
/// its shared-string index.
fn worksheet_xml(sheet: &Sheet, shared: &mut SharedStrings) -> Vec<u8> {
    let mut xml = String::new();
    xml.push_str(XML_DECL);
    xml.push_str("<worksheet xmlns=\"");
    xml.push_str(SML_NS);
    xml.push_str("\"><sheetData>");

    // Walk cells row-major (BTreeMap keyed by (row, col)), opening a new <row>
    // whenever the row number changes.
    let mut current_row: Option<u32> = None;
    for (&(row, col), data) in &sheet.cells {
        if current_row != Some(row) {
            if current_row.is_some() {
                xml.push_str("</row>");
            }
            xml.push_str("<row r=\"");
            xml.push_str(&row.to_string());
            xml.push_str("\">");
            current_row = Some(row);
        }
        let r = a1_of(col, row);
        match data {
            CellData::Number(n) => {
                xml.push_str("<c r=\"");
                xml.push_str(&r);
                xml.push_str("\"><v>");
                xml.push_str(&format_number(*n));
                xml.push_str("</v></c>");
            }
            CellData::Text(s) => {
                let idx = shared.intern(s);
                xml.push_str("<c r=\"");
                xml.push_str(&r);
                xml.push_str("\" t=\"s\"><v>");
                xml.push_str(&idx.to_string());
                xml.push_str("</v></c>");
            }
            CellData::Formula { formula, cached } => {
                xml.push_str("<c r=\"");
                xml.push_str(&r);
                xml.push_str("\"><f>");
                xml.push_str(&xml_escape(formula));
                xml.push_str("</f><v>");
                xml.push_str(&format_number(*cached));
                xml.push_str("</v></c>");
            }
        }
    }
    if current_row.is_some() {
        xml.push_str("</row>");
    }

    xml.push_str("</sheetData></worksheet>");
    xml.into_bytes()
}

/// Serialize `xl/workbook.xml`: one `<sheet>` per worksheet, each naming its
/// relationship id. `rId1..rIdN` map to the N sheets (see
/// `workbook_rels_xml`).
fn workbook_xml(sheets: &[Sheet]) -> Vec<u8> {
    let mut xml = String::new();
    xml.push_str(XML_DECL);
    xml.push_str("<workbook xmlns=\"");
    xml.push_str(SML_NS);
    xml.push_str("\" xmlns:r=\"");
    xml.push_str(REL_NS);
    xml.push_str("\"><sheets>");
    for (i, sheet) in sheets.iter().enumerate() {
        let sheet_id = i + 1; // sheetId is 1-based
        let rid = format!("rId{}", i + 1);
        xml.push_str("<sheet name=\"");
        xml.push_str(&xml_escape(&sheet.name));
        xml.push_str("\" sheetId=\"");
        xml.push_str(&sheet_id.to_string());
        xml.push_str("\" r:id=\"");
        xml.push_str(&rid);
        xml.push_str("\"/>");
    }
    xml.push_str("</sheets></workbook>");
    xml.into_bytes()
}

// ===========================================================================
// The top-level writer
// ===========================================================================

/// Serialize a [`Workbook`] to the bytes of a valid `.xlsx` file.
///
/// The pipeline:
/// 1. Generate each worksheet part, interning text cells into the shared-string
///    table as we go.
/// 2. Generate `workbook.xml` (naming sheets by relationship id) and, if any
///    strings were interned, `sharedStrings.xml`.
/// 3. Wire up the two `.rels` parts (package→workbook, workbook→sheets+strings).
/// 4. Register content types and hand everything to [`PackageWriter`].
pub fn write_xlsx(wb: &Workbook) -> Vec<u8> {
    let mut shared = SharedStrings::default();

    // --- Pass 1: worksheet parts (also populates the shared-string table) ---
    // Serialize into (member_name, bytes) so we can add them after we know
    // whether a sharedStrings part exists.
    let mut worksheet_parts: Vec<(String, Vec<u8>)> = Vec::new();
    for (i, sheet) in wb.sheets.iter().enumerate() {
        let member = format!("/xl/worksheets/sheet{}.xml", i + 1);
        worksheet_parts.push((member, worksheet_xml(sheet, &mut shared)));
    }

    let has_strings = !shared.strings.is_empty();

    // --- workbook.xml -----------------------------------------------------
    let workbook_bytes = workbook_xml(&wb.sheets);

    // --- workbook rels: workbook → each sheet, then → sharedStrings -------
    let mut wb_rels = RelationshipsBuilder::new();
    for i in 0..wb.sheets.len() {
        wb_rels.add(
            &format!("rId{}", i + 1),
            TYPE_WORKSHEET,
            &format!("worksheets/sheet{}.xml", i + 1),
        );
    }
    if has_strings {
        // The shared-strings relationship id follows the sheet ids.
        let rid = format!("rId{}", wb.sheets.len() + 1);
        wb_rels.add(&rid, TYPE_SHARED_STRINGS, "sharedStrings.xml");
    }

    // --- package-root rels: package → workbook ---------------------------
    let mut root_rels = RelationshipsBuilder::new();
    root_rels.add("rId1", TYPE_OFFICE_DOCUMENT, "xl/workbook.xml");

    // --- assemble the package --------------------------------------------
    let mut pkg = PackageWriter::new();
    pkg.add_default("rels", CT_RELS);
    pkg.add_default("xml", CT_XML);

    pkg.add_part_defaulted("/_rels/.rels", &root_rels.build());
    pkg.add_part("/xl/workbook.xml", CT_WORKBOOK, &workbook_bytes);
    pkg.add_part_defaulted("/xl/_rels/workbook.xml.rels", &wb_rels.build());

    for (member, bytes) in &worksheet_parts {
        pkg.add_part(member, CT_WORKSHEET, bytes);
    }

    if has_strings {
        pkg.add_part("/xl/sharedStrings.xml", CT_SHARED_STRINGS, &shared.to_xml());
    }

    pkg.finish()
}

#[cfg(test)]
mod tests;
