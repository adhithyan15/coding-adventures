//! # `coding-adventures-spreadsheetml` — read an `.xlsx` as a typed cell grid
//!
//! This is milestone **M3** of the OOXML effort (see `code/specs/SML01`). It
//! takes the raw bytes of an `.xlsx` file and produces a
//! [`Workbook`] → [`Sheet`] → [`Cell`] model, where each populated cell carries
//! a decoded [`Value`] and, where present, its formula text plus cached result.
//!
//! It sits directly on two lower layers that already did the hard plumbing:
//!
//! ```text
//! bytes → zip (M0) → xml-parser (M1) → opc (M2) → spreadsheetml (M3, HERE)
//! ```
//!
//! * The **`opc`** crate opens the ZIP, exposes named parts, and — crucially —
//!   resolves relationship ids (`r:id="rId1"`) to part names.
//! * The **`xml-parser`** crate parses a part's UTF-8 XML into a namespaced
//!   element tree with entity decoding already done.
//!
//! ## The two indirections
//!
//! An `.xlsx` is normalized like a database, and two indirections trip up every
//! newcomer. This crate exists to resolve both so the caller sees a plain grid.
//!
//! ### 1. `r:id` → part (which file *is* this sheet?)
//!
//! `workbook.xml` lists sheets by a relationship **id**, not a path:
//!
//! ```xml
//! <sheet name="Revenue" sheetId="1" r:id="rId1"/>
//! ```
//!
//! `rId1` is dereferenced through a *separate* `.rels` file. The OPC layer does
//! that for us: [`Package::resolve`](coding_adventures_opc::Package::resolve)
//! turns `("/xl/workbook.xml", "rId1")` into `"/xl/worksheets/sheet1.xml"`.
//!
//! ### 2. shared string index → text (why is this cell just `0`?)
//!
//! Text is deduplicated into one **shared string table**. A text cell stores an
//! *index* into that table and flags itself `t="s"`:
//!
//! ```xml
//! <c r="A1" t="s"><v>0</v></c>   <!-- sharedStrings[0], e.g. "Q1" -->
//! ```
//!
//! So `<v>0</v>` under `t="s"` means "shared string #0", not the number zero.
//! We build the table once and dereference each `t="s"` cell into it.
//!
//! Both shared strings and inline strings surface as [`Value::Text`]; the caller
//! never sees the storage indirection.

use coding_adventures_opc::{OpcError, Package};
use coding_adventures_xml_parser::{parse_xml, XmlElement};
use std::collections::BTreeMap;

mod styles;
pub use styles::{
    builtin_format_code, classify_format_code, classify_id, serial_to_date, serial_to_datetime,
    CellRange, NumberFormat, NumberFormatKind, StyleTable, FIRST_CUSTOM_FORMAT_ID,
};

// ===========================================================================
// Namespace constants
// ===========================================================================

/// The SpreadsheetML "main" namespace. Every structural element we care about
/// (`workbook`, `sheets`, `sheet`, `sheetData`, `row`, `c`, `v`, `f`, `is`,
/// `t`, `sst`, `si`) lives here.
const SML_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

/// The relationships namespace — the `r:` prefix on `<sheet r:id="rId1">`. Note
/// the asymmetry: `name`/`sheetId` are *unprefixed* (namespace `None`), while
/// `id` is in this namespace because it is written `r:id`.
const REL_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// The relationship *type* URI that marks the sheet's actual bytes… we do not
/// actually need the worksheet type (we resolve sheets by their explicit r:id),
/// but we do need this one to *find* the shared-strings part by scanning the
/// workbook's relationships for it.
const SHARED_STRINGS_TYPE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings";

/// The relationship *type* URI of the workbook's `xl/styles.xml` part. Like the
/// shared-strings part, we find it by scanning the workbook's relationships for
/// this type (M4).
const STYLES_TYPE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles";

/// The logical part name of the workbook. `main_document_part()` yields this for
/// an `.xlsx`; we keep the constant for the (rare) fallback path.
const WORKBOOK_PART: &str = "/xl/workbook.xml";

// ===========================================================================
// Errors
// ===========================================================================

/// Everything that can go wrong opening a workbook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XlsxError {
    /// The bytes were not a readable OPC package (not a ZIP, no content types,
    /// …). Wraps the underlying [`OpcError`].
    Opc(OpcError),
    /// The package opened but declared no main document part — i.e. it is not a
    /// workbook (`/xl/workbook.xml` is neither the declared main part nor
    /// present).
    MissingWorkbook,
    /// A part that had to be XML was not valid UTF-8. Carries the part name.
    NotUtf8(String),
    /// A part failed to parse as XML. Carries a human-readable message.
    MalformedXml(String),
    /// A `<sheet r:id="…">` did not resolve to any part. Carries the r:id.
    MissingSheetPart(String),
    /// A `t="s"` cell referenced a shared-string index outside the table.
    BadSharedStringIndex(usize),
}

impl std::fmt::Display for XlsxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XlsxError::Opc(e) => write!(f, "package error: {e}"),
            XlsxError::MissingWorkbook => {
                write!(f, "not a workbook: no /xl/workbook.xml main part")
            }
            XlsxError::NotUtf8(p) => write!(f, "part {p} is not valid UTF-8"),
            XlsxError::MalformedXml(m) => write!(f, "malformed XML: {m}"),
            XlsxError::MissingSheetPart(id) => {
                write!(f, "sheet relationship {id} did not resolve to a part")
            }
            XlsxError::BadSharedStringIndex(i) => {
                write!(f, "shared-string index {i} out of range")
            }
        }
    }
}

impl std::error::Error for XlsxError {}

impl From<OpcError> for XlsxError {
    fn from(e: OpcError) -> Self {
        XlsxError::Opc(e)
    }
}

// ===========================================================================
// The typed model
// ===========================================================================

/// A single cell's decoded value. Styles and number formats are *not* applied
/// here (milestone M4) — numbers are the bare stored `f64`, so a cell that
/// *displays* as a date or currency still surfaces as [`Value::Number`].
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A numeric value (`t` absent or `t="n"`), parsed from `<v>` as `f64`.
    Number(f64),
    /// Text. Shared strings (`t="s"`), formula string results (`t="str"`), and
    /// inline strings (`t="inlineStr"`) all surface here.
    Text(String),
    /// A boolean (`t="b"`): `<v>1</v>` → `true`, `<v>0</v>` → `false`.
    Bool(bool),
    /// A worksheet error (`t="e"`), e.g. `#DIV/0!`, kept as its text.
    Error(String),
    /// A blank cell: present in the XML but with no `<v>`/`<is>`/`<f>`.
    Empty,
}

/// One populated cell.
///
/// M4 adds [`number_format`](Cell::number_format): the format applied to this
/// cell's stored value, resolved through its `s=` style index (see the
/// [`styles`] module). The raw [`value`](Cell::value) is **unchanged** from M3 —
/// a date cell still holds `Value::Number(45292.0)` — but the format lets the
/// caller *interpret* it (e.g. [`as_date`](Cell::as_date) → `"2024-01-01"`).
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// The A1 reference exactly as written, e.g. `"B2"`.
    pub reference: String,
    /// The decoded value (already dereferenced through the shared-string table).
    pub value: Value,
    /// The formula **text** if the cell had a `<f>` child, e.g.
    /// `Some("SUM(B1:B1)")`. The [`value`](Cell::value) is then the *cached*
    /// result; we never evaluate formulas at this milestone.
    pub formula: Option<String>,
    /// The number format applied to this cell, resolved through its `s=` style
    /// index. `None` for a cell with no style, an out-of-range style index, or a
    /// `General`-formatted cell (kept `None` so unstyled numbers behave as in
    /// M3). See [`NumberFormat`].
    pub number_format: Option<NumberFormat>,
}

impl Cell {
    /// The coarse [`NumberFormatKind`] applied to this cell, or
    /// [`NumberFormatKind::General`] when it has no attached format.
    pub fn format_kind(&self) -> NumberFormatKind {
        self.number_format
            .as_ref()
            .map(|f| f.kind)
            .unwrap_or(NumberFormatKind::General)
    }

    /// If this cell is a **date** (or date-time) holding a numeric serial,
    /// render it as an ISO `YYYY-MM-DD` string using the 1900 date system.
    ///
    /// Returns `None` when the cell is not a date-kinded numeric cell — so a
    /// plain number, a text cell, or a percent cell all yield `None`. A
    /// date-time cell's *date* part is returned here; use
    /// [`as_datetime`](Cell::as_datetime) for the time too.
    ///
    /// This is M4's headline: `45292` styled as a date becomes `"2024-01-01"`.
    pub fn as_date(&self) -> Option<String> {
        match (self.format_kind(), &self.value) {
            (NumberFormatKind::Date | NumberFormatKind::DateTime, Value::Number(n)) => {
                serial_to_date(*n)
            }
            _ => None,
        }
    }

    /// If this cell is a date-time (or date) numeric serial, render it as an ISO
    /// `YYYY-MM-DDTHH:MM:SS` string. `None` otherwise.
    pub fn as_datetime(&self) -> Option<String> {
        match (self.format_kind(), &self.value) {
            (NumberFormatKind::Date | NumberFormatKind::DateTime, Value::Number(n)) => {
                serial_to_datetime(*n)
            }
            _ => None,
        }
    }

    /// A human-readable rendering of the cell that applies its format where it
    /// matters. This is a *pragmatic* renderer, **not** a full Excel
    /// number-format engine (that is out of scope):
    ///
    /// * **Date / DateTime** — the ISO date (or date-time) string. This is the
    ///   part that must be exact, and it is.
    /// * **Percent** — the stored fraction ×100 with a trailing `%`
    ///   (`0.25` → `"25%"`). We do not honour the code's decimal-place count.
    /// * **Currency** — the raw number as a plain string; we deliberately do
    ///   **not** synthesize the currency symbol / grouping (documented
    ///   limitation).
    /// * **Everything else** (General, Number, Text, Bool, Error, Empty) — the
    ///   value's natural string form.
    pub fn formatted(&self) -> String {
        // Dates/date-times: exact ISO rendering.
        if let Some(dt) = self.as_datetime() {
            // For a date-only format, trim the midnight time component.
            if matches!(self.format_kind(), NumberFormatKind::Date) {
                if let Some(d) = self.as_date() {
                    return d;
                }
            }
            return dt;
        }

        match (&self.value, self.format_kind()) {
            // Percent: the fraction ×100 with a % sign.
            (Value::Number(n), NumberFormatKind::Percent) => {
                format_number(n * 100.0) + "%"
            }
            // Currency: raw number (documented — no symbol synthesis).
            (Value::Number(n), NumberFormatKind::Currency) => format_number(*n),
            (Value::Number(n), _) => format_number(*n),
            (Value::Text(s), _) => s.clone(),
            (Value::Bool(b), _) => {
                if *b {
                    "TRUE".to_string()
                } else {
                    "FALSE".to_string()
                }
            }
            (Value::Error(e), _) => e.clone(),
            (Value::Empty, _) => String::new(),
        }
    }
}

/// Render an `f64` without a trailing `.0` for integers, so `25.0` shows as
/// `"25"` while `12.5` stays `"12.5"`.
fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.is_finite() && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

/// One worksheet: a name plus its populated cells.
///
/// Cells are stored in a [`BTreeMap`] keyed by `(row, col)` (both 1-based), so
/// natural map order is **row-major, then left-to-right** — the order a human
/// reads a grid. [`cells`](Sheet::cells) iterates in that order; lookups by A1
/// are O(log n).
#[derive(Debug, Clone)]
pub struct Sheet {
    /// The sheet's display name, e.g. `"Revenue"`.
    pub name: String,
    /// Populated cells keyed by `(row, col)` (both 1-based) so natural map order
    /// is row-major, then left-to-right — the order a human reads.
    cells: BTreeMap<(u32, u32), Cell>,
    /// Merged-cell ranges (`<mergeCell ref="A1:B1"/>`), in document order (M4).
    merged: Vec<CellRange>,
}

impl Sheet {
    /// Look up a cell by its A1 reference (`"B2"`). Returns `None` for an
    /// unparseable ref or an unpopulated cell.
    pub fn cell(&self, a1: &str) -> Option<&Cell> {
        let (col, row) = parse_a1_ref(a1)?;
        self.cells.get(&(row, col))
    }

    /// Iterate the populated cells in reading order (row-major, left-to-right).
    pub fn cells(&self) -> impl Iterator<Item = &Cell> {
        self.cells.values()
    }

    /// How many populated cells this sheet has.
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// The merged-cell ranges on this sheet, in document order (M4). Each is a
    /// [`CellRange`] such as `A1:B1`. Empty when the sheet merges nothing.
    pub fn merged_ranges(&self) -> &[CellRange] {
        &self.merged
    }
}

/// A whole workbook: its sheets in workbook order.
#[derive(Debug, Clone)]
pub struct Workbook {
    sheets: Vec<Sheet>,
    /// Defined (named) ranges: `(name, reference text)`, in document order (M4).
    defined_names: Vec<(String, String)>,
}

impl Workbook {
    /// The sheet names in workbook (declaration) order.
    pub fn sheet_names(&self) -> Vec<String> {
        self.sheets.iter().map(|s| s.name.clone()).collect()
    }

    /// Find a sheet by exact name.
    pub fn sheet_by_name(&self, name: &str) -> Option<&Sheet> {
        self.sheets.iter().find(|s| s.name == name)
    }

    /// All sheets, in workbook order.
    pub fn sheets(&self) -> &[Sheet] {
        &self.sheets
    }

    /// The workbook's defined (named) ranges as `(name, reference)` pairs, in
    /// document order (M4). E.g. `("TaxRate", "Report!$B$4")`. The reference is
    /// the raw formula text of the `<definedName>` — we do **not** evaluate it.
    pub fn defined_names(&self) -> &[(String, String)] {
        &self.defined_names
    }
}

// ===========================================================================
// A1 reference parsing
// ===========================================================================

/// Parse an A1-style cell reference into `(col, row)`, **both 1-based**.
///
/// The column letters are *bijective* base-26: `A`=1 … `Z`=26, `AA`=27,
/// `AB`=28, … There is no zero digit, which is why it is bijective rather than
/// ordinary base-26 (in ordinary base-26 `AA` would be 26, not 27).
///
/// Returns `None` if the ref is malformed: empty, missing letters or digits,
/// out-of-order (digits before letters), or containing anything but ASCII
/// letters then ASCII digits. A leading `$` (absolute ref) is **not** accepted
/// here — cell refs inside `sheetData` never use `$`.
///
/// ```
/// use coding_adventures_spreadsheetml::parse_a1_ref;
/// assert_eq!(parse_a1_ref("A1"), Some((1, 1)));
/// assert_eq!(parse_a1_ref("B2"), Some((2, 2)));
/// assert_eq!(parse_a1_ref("Z1"), Some((26, 1)));
/// assert_eq!(parse_a1_ref("AA10"), Some((27, 10)));
/// assert_eq!(parse_a1_ref(""), None);
/// assert_eq!(parse_a1_ref("1A"), None);
/// ```
pub fn parse_a1_ref(a1: &str) -> Option<(u32, u32)> {
    let bytes = a1.as_bytes();
    let mut i = 0;

    // --- column letters (at least one) ---
    let mut col: u32 = 0;
    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
        // Uppercase the letter, then fold in bijective base-26.
        let letter = bytes[i].to_ascii_uppercase();
        let digit = (letter - b'A' + 1) as u32; // A→1 … Z→26
        col = col.checked_mul(26)?.checked_add(digit)?;
        i += 1;
    }
    if col == 0 {
        return None; // no leading letters
    }

    // --- row digits (at least one), and nothing must remain after them ---
    let mut row: u32 = 0;
    let mut saw_digit = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let digit = (bytes[i] - b'0') as u32;
        row = row.checked_mul(10)?.checked_add(digit)?;
        saw_digit = true;
        i += 1;
    }
    if !saw_digit || i != bytes.len() || row == 0 {
        return None; // no digits, trailing junk, or row 0
    }

    Some((col, row))
}

// ===========================================================================
// Reading the workbook
// ===========================================================================

/// Open an `.xlsx` from its bytes and read it into a [`Workbook`].
///
/// The pipeline:
/// 1. Open the OPC package and locate the workbook part.
/// 2. Load the shared-string table (if any) from the workbook's relationships.
/// 3. For each `<sheet>`, resolve its `r:id` to a part, parse it, and decode
///    every `<c>` into a [`Cell`].
pub fn open_workbook(bytes: &[u8]) -> Result<Workbook, XlsxError> {
    let package = Package::open(bytes)?;

    // --- locate the workbook part ---------------------------------------
    // main_document_part() follows the package-level /officeDocument
    // relationship. For a real .xlsx this is "/xl/workbook.xml". If a producer
    // omitted that relationship but the part exists, fall back to the constant.
    let workbook_part = match package.main_document_part() {
        Some(p) => p,
        None if package.has_part(WORKBOOK_PART) => WORKBOOK_PART.to_string(),
        None => return Err(XlsxError::MissingWorkbook),
    };

    let workbook_root = parse_part(&package, &workbook_part)?;

    // --- shared strings --------------------------------------------------
    let shared_strings = load_shared_strings(&package, &workbook_part)?;

    // --- styles (M4) -----------------------------------------------------
    // Resolve xl/styles.xml via the workbook's relationships and parse its
    // numFmts + cellXfs into a StyleTable. A workbook with no styles part is
    // legal (every cell is General) → an empty table.
    let style_table = load_styles(&package, &workbook_part)?;

    // --- defined names (M4) ---------------------------------------------
    // <workbook><definedNames><definedName name=…>ref</definedName></definedNames>
    let defined_names = read_defined_names(&workbook_root);

    // --- sheets ----------------------------------------------------------
    // <workbook><sheets><sheet name=… r:id=…/></sheets></workbook>
    let sheets_el = workbook_root.get_child(Some(SML_NS), "sheets");
    let mut sheets = Vec::new();
    if let Some(sheets_el) = sheets_el {
        for sheet_el in sheets_el.get_children(Some(SML_NS), "sheet") {
            // name is unprefixed → namespace None. Missing name is tolerated as
            // "" (a producer that omits it is degenerate, not fatal).
            let name = sheet_el.get_attr(None, "name").unwrap_or("").to_string();
            // r:id is prefixed → REL_NS. Without it we cannot find the bytes.
            let rid = sheet_el
                .get_attr(Some(REL_NS), "id")
                .ok_or_else(|| XlsxError::MissingSheetPart(String::new()))?;
            let sheet_part = package
                .resolve(&workbook_part, rid)
                .ok_or_else(|| XlsxError::MissingSheetPart(rid.to_string()))?;

            let (cells, merged) =
                read_sheet(&package, &sheet_part, &shared_strings, &style_table)?;
            sheets.push(Sheet {
                name,
                cells,
                merged,
            });
        }
    }

    Ok(Workbook {
        sheets,
        defined_names,
    })
}

/// Load and parse `xl/styles.xml` into a [`StyleTable`].
///
/// We find the styles part by scanning the workbook's relationships for the
/// [`STYLES_TYPE`]. No styles part → an empty table (every cell is `General`).
fn load_styles(package: &Package, workbook_part: &str) -> Result<StyleTable, XlsxError> {
    let rels = package.relationships(workbook_part)?;
    let styles_part = rels
        .into_iter()
        .find(|r| r.rel_type == STYLES_TYPE)
        .and_then(|r| r.resolved_target);

    let styles_part = match styles_part {
        Some(p) => p,
        None => return Ok(StyleTable::empty()), // no styles — legal
    };

    let root = parse_part(package, &styles_part)?;
    Ok(StyleTable::from_root(&root))
}

/// Read the `<definedNames>` from the workbook root into `(name, ref)` pairs.
///
/// Each `<definedName name="TaxRate">Report!$B$4</definedName>` becomes
/// `("TaxRate", "Report!$B$4")`. The reference is the element's text content;
/// we keep it verbatim and never evaluate it. A workbook with no defined names
/// yields an empty vector.
fn read_defined_names(workbook_root: &XmlElement) -> Vec<(String, String)> {
    let mut names = Vec::new();
    if let Some(dn) = workbook_root.get_child(Some(SML_NS), "definedNames") {
        for entry in dn.get_children(Some(SML_NS), "definedName") {
            if let Some(name) = entry.get_attr(None, "name") {
                names.push((name.to_string(), entry.text_content()));
            }
        }
    }
    names
}

/// Parse a package part as XML, returning its root element. Turns UTF-8 and
/// parse failures into [`XlsxError`].
fn parse_part(package: &Package, part: &str) -> Result<XmlElement, XlsxError> {
    let bytes = package
        .read_part(part)
        .ok_or_else(|| XlsxError::MissingSheetPart(part.to_string()))?;
    let text =
        std::str::from_utf8(bytes).map_err(|_| XlsxError::NotUtf8(part.to_string()))?;
    let doc = parse_xml(text).map_err(|e| XlsxError::MalformedXml(format!("{part}: {e:?}")))?;
    Ok(doc.root)
}

/// Build the shared-string table for a workbook.
///
/// We find the shared-strings part by scanning the workbook's relationships for
/// the [`SHARED_STRINGS_TYPE`]. A workbook with no text cells legitimately has
/// no such part → an empty table.
///
/// Each `<si>` (string item) is either a single `<t>` or several `<r><t>` runs;
/// its *string value* is the concatenation of all descendant `<t>` text, which
/// is precisely `text_content()` on the `<si>` element. So rich text needs no
/// special case.
fn load_shared_strings(
    package: &Package,
    workbook_part: &str,
) -> Result<Vec<String>, XlsxError> {
    let rels = package.relationships(workbook_part)?;
    let ss_part = rels
        .into_iter()
        .find(|r| r.rel_type == SHARED_STRINGS_TYPE)
        .and_then(|r| r.resolved_target);

    let ss_part = match ss_part {
        Some(p) => p,
        None => return Ok(Vec::new()), // no shared strings — legal
    };

    let root = parse_part(package, &ss_part)?;
    let mut table = Vec::new();
    // <sst><si>…</si><si>…</si></sst>
    for si in root.get_children(Some(SML_NS), "si") {
        table.push(si.text_content());
    }
    Ok(table)
}

/// Populated cells keyed by `(row, col)`, both 1-based — the sheet's grid.
type CellGrid = BTreeMap<(u32, u32), Cell>;

/// Parse one worksheet part into its populated cells **and** its merged ranges.
///
/// M4 additions over M3: each `<c>` is decoded *with* the [`StyleTable`] so its
/// `s=` style index resolves to a [`NumberFormat`], and the worksheet's
/// `<mergeCells>` are parsed into [`CellRange`]s.
fn read_sheet(
    package: &Package,
    sheet_part: &str,
    shared_strings: &[String],
    styles: &StyleTable,
) -> Result<(CellGrid, Vec<CellRange>), XlsxError> {
    let root = parse_part(package, sheet_part)?;
    let mut cells = BTreeMap::new();

    // <worksheet><sheetData><row><c>…</c></row></sheetData></worksheet>
    if let Some(sheet_data) = root.get_child(Some(SML_NS), "sheetData") {
        for row_el in sheet_data.get_children(Some(SML_NS), "row") {
            for c_el in row_el.get_children(Some(SML_NS), "c") {
                let cell = decode_cell(c_el, shared_strings, styles)?;
                if let Some((col, row)) = parse_a1_ref(&cell.reference) {
                    cells.insert((row, col), cell);
                }
                // A <c> without a usable r ref is skipped: we key cells by A1.
            }
        }
    }

    // <worksheet><mergeCells><mergeCell ref="A1:B1"/></mergeCells>
    let mut merged = Vec::new();
    if let Some(mc) = root.get_child(Some(SML_NS), "mergeCells") {
        for m in mc.get_children(Some(SML_NS), "mergeCell") {
            if let Some(range) = m.get_attr(None, "ref").and_then(CellRange::parse) {
                merged.push(range);
            }
        }
    }

    Ok((cells, merged))
}

/// Decode a single `<c>` element into a [`Cell`].
///
/// See the SML01 spec's truth table for the `t` attribute. In short:
/// * `t` absent / `n` → number from `<v>`
/// * `s` → shared string, `<v>` is the index
/// * `str` → literal string from `<v>`
/// * `inlineStr` → `<is>` text
/// * `b` → boolean from `<v>`
/// * `e` → error text from `<v>`
///
/// A `<f>` child means the cell has a formula; we keep its text and treat `<v>`
/// (or `<is>`) as the *cached* result.
///
/// M4: the cell's `s=` attribute is resolved through `styles` into an optional
/// [`NumberFormat`]. The decoded [`Value`] is **unchanged** from M3 — the format
/// is attached alongside, never applied to the stored value.
fn decode_cell(
    c_el: &XmlElement,
    shared_strings: &[String],
    styles: &StyleTable,
) -> Result<Cell, XlsxError> {
    let reference = c_el.get_attr(None, "r").unwrap_or("").to_string();
    let cell_type = c_el.get_attr(None, "t");

    // The style index (s=) → NumberFormat, if this cell carries a non-General
    // style. A missing / unparseable / out-of-range index yields None.
    let style_index = c_el.get_attr(None, "s").and_then(|s| s.parse::<u32>().ok());
    let number_format = styles.format_for(style_index);

    // Formula text, if any. Its presence does not change how we decode <v>;
    // <v> is simply the cached result.
    let formula = c_el
        .get_child(Some(SML_NS), "f")
        .map(|f| f.text_content());

    // The <v> element's text (numbers, indices, bool, error, str result).
    let v_text = c_el
        .get_child(Some(SML_NS), "v")
        .map(|v| v.text_content());

    let value = match cell_type {
        // --- shared string: <v> is an index into the table ---
        Some("s") => {
            let v = v_text.ok_or(XlsxError::BadSharedStringIndex(usize::MAX))?;
            let idx: usize = v
                .trim()
                .parse()
                .map_err(|_| XlsxError::BadSharedStringIndex(usize::MAX))?;
            let s = shared_strings
                .get(idx)
                .ok_or(XlsxError::BadSharedStringIndex(idx))?;
            Value::Text(s.clone())
        }
        // --- literal formula-result string ---
        Some("str") => Value::Text(v_text.unwrap_or_default()),
        // --- inline string: text lives in <is>, not <v> ---
        Some("inlineStr") => {
            let text = c_el
                .get_child(Some(SML_NS), "is")
                .map(|is| is.text_content())
                .unwrap_or_default();
            Value::Text(text)
        }
        // --- boolean ---
        Some("b") => match v_text.as_deref() {
            Some("1") => Value::Bool(true),
            Some("0") => Value::Bool(false),
            // A stray value — treat any non-"0" as true, absent as Empty.
            Some(other) => Value::Bool(other.trim() != "0" && !other.trim().is_empty()),
            None => Value::Empty,
        },
        // --- error ---
        Some("e") => match v_text {
            Some(t) => Value::Error(t),
            None => Value::Empty,
        },
        // --- number: t absent, t="n", or an unknown t we treat as numeric ---
        _ => match v_text {
            Some(t) => {
                let n: f64 = t
                    .trim()
                    .parse()
                    .map_err(|_| XlsxError::MalformedXml(format!("bad number {t:?}")))?;
                Value::Number(n)
            }
            None => Value::Empty,
        },
    };

    Ok(Cell {
        reference,
        value,
        formula,
        number_format,
    })
}

#[cfg(test)]
mod fixture;
#[cfg(test)]
mod tests;
