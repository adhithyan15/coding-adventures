//! # Number formats — teaching the reader that `45292` *is* `2024-01-01`
//!
//! This is milestone **M4** of the OOXML effort. M3 handed back the bare stored
//! value of every cell — a date is stored on disk as the plain number `45292`,
//! a currency amount as `1234.5`, a percentage as `0.25`. That is faithful to
//! the bytes but useless to a human: nobody wants to be told a cell "is 45292"
//! when Excel shows `2024-01-01`. M4 reads the **style** attached to each cell
//! and interprets its number so the caller can recover the human meaning.
//!
//! ## The chain of indirections (yet another one)
//!
//! A cell does not carry its format directly. It carries a *style index*:
//!
//! ```xml
//! <c r="A2" s="1"><v>45292</v></c>
//! ```
//!
//! That `s="1"` is an index into `<cellXfs>` in `xl/styles.xml` — the list of
//! "cell formats" (`xf` = *eXtended Format*). Each `<xf>` names a `numFmtId`:
//!
//! ```xml
//! <cellXfs count="4">
//!   <xf numFmtId="0"  xfId="0"/>   <!-- s=0 → General -->
//!   <xf numFmtId="14" xfId="0"/>   <!-- s=1 → built-in date m/d/yyyy -->
//!   <xf numFmtId="164" xfId="0"/>  <!-- s=2 → custom "$"#,##0.00 -->
//!   <xf numFmtId="10" xfId="0"/>   <!-- s=3 → built-in 0.00% percent -->
//! </cellXfs>
//! ```
//!
//! So `s="1"` → `cellXfs[1]` → `numFmtId=14` → the built-in format `m/d/yyyy`.
//! The full path a cell walks to learn its format is therefore:
//!
//! ```text
//! <c s="1">  →  cellXfs[1]  →  numFmtId 14  →  format code "m/d/yyyy"  →  Date
//! ```
//!
//! ## Built-in vs custom format ids
//!
//! `numFmtId`s below **164** are *built-in*: their meaning is fixed by the OOXML
//! spec (ECMA-376) and is **not** written into the file. Everyone agrees that id
//! `14` means `m/d/yyyy` and id `9` means `0%`. We hard-code that table below
//! ([`builtin_format_code`]).
//!
//! Ids **≥ 164** are *custom*: the producer defines them in `<numFmts>`, giving
//! each an explicit `formatCode` string:
//!
//! ```xml
//! <numFmts count="1">
//!   <numFmt numFmtId="164" formatCode="&quot;$&quot;#,##0.00"/>
//! </numFmts>
//! ```
//!
//! (The `&quot;` entities are decoded by the XML parser, so the code we see is
//! `"$"#,##0.00` — a currency format.)
//!
//! ## The 1900 date system and the famous leap-year bug
//!
//! Excel stores a date as the count of days since an epoch. In the (default)
//! **1900 date system**, serial `1` is `1900-01-01`. The naive reading would
//! put serial `0` at `1899-12-31`, but Excel deliberately includes a
//! **non-existent** `1900-02-29`: 1900 was *not* a leap year (divisible by 100
//! but not 400), yet Lotus 1-2-3 pretended it was, and Excel copied the bug for
//! compatibility. So for serials ≥ 60 the calendar is off by one day from
//! reality.
//!
//! The clean way to reproduce this exactly is to anchor serial **0** at the
//! fictitious `1899-12-30` and then add days on a *real* proleptic Gregorian
//! calendar. This makes:
//!
//! | serial | renders as   | note                                    |
//! |--------|--------------|-----------------------------------------|
//! | 1      | 1900-01-01   | the documented epoch                    |
//! | 59     | 1900-02-28   | last day before the phantom             |
//! | 60     | 1900-02-29   | **does not exist** — Excel's fake day   |
//! | 61     | 1900-03-01   | real calendar resumes, now +0 again     |
//! | 45292  | 2024-01-01   | the headline example                    |
//!
//! We render serial 60 as `1900-02-29` on purpose: it is what Excel shows, and
//! round-tripping fidelity beats calendar correctness for a *reader*.

use coding_adventures_xml_parser::XmlElement;

/// The SpreadsheetML "main" namespace — the same one worksheets live in.
/// `styleSheet`, `numFmts`, `numFmt`, `cellXfs`, `xf` are all here.
const SML_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

/// The first custom `numFmtId`. Ids below this are built-in (spec-defined);
/// ids at or above it are defined by the file's own `<numFmts>`.
pub const FIRST_CUSTOM_FORMAT_ID: u32 = 164;

// ===========================================================================
// NumberFormatKind — the coarse classification a caller usually wants
// ===========================================================================

/// What *kind* of thing a number format displays. This is the coarse bucket a
/// caller reaches for first: "is this cell a date?" is far more common than
/// "what is the exact format code?".
///
/// The kind is derived either from a built-in id (whose meaning is fixed) or by
/// inspecting a custom format code's tokens (see [`classify_format_code`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberFormatKind {
    /// `General` (id 0) — no specific format; the raw number, shown compactly.
    General,
    /// A plain number format like `0`, `0.00`, `#,##0` (no date/percent/currency
    /// meaning).
    Number,
    /// A date-only format (`m/d/yyyy`, `yyyy-mm-dd`, …).
    Date,
    /// A time-only format (`h:mm:ss`, `mm:ss`, …).
    Time,
    /// A combined date **and** time format (`m/d/yyyy h:mm`).
    DateTime,
    /// A percentage (`0%`, `0.00%`) — the stored number is a fraction.
    Percent,
    /// A currency / accounting format (`"$"#,##0.00`, `[$€-x]#,##0.00`, …).
    Currency,
    /// Text format (`@`) — the value is shown verbatim as text.
    Text,
    /// Anything we recognize as a format but do not bucket further (fractions,
    /// scientific notation, …).
    Other,
}

/// The number format applied to a cell: its `numFmtId`, the resolved format
/// **code** string, and the coarse [`NumberFormatKind`].
///
/// A cell with no `s=` attribute (or one pointing at `General`) has *no*
/// `NumberFormat` in the general case; we only attach one when a style resolves
/// to a real format. See [`Cell`](crate::Cell).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberFormat {
    /// The numeric format id (`numFmtId`). `< 164` is built-in; `>= 164` custom.
    pub id: u32,
    /// The format **code** string, e.g. `m/d/yyyy` or `"$"#,##0.00`. For a
    /// built-in id this is the spec-defined code; for a custom id it is the
    /// file's own `formatCode`.
    pub code: String,
    /// The coarse classification of `code`.
    pub kind: NumberFormatKind,
}

// ===========================================================================
// Built-in number format table (ECMA-376, §18.8.30)
// ===========================================================================

/// The spec-defined format code for a **built-in** `numFmtId` (`< 164`).
///
/// These are fixed by ECMA-376 and never written into the file, so any reader
/// must carry the table. Ids that the spec leaves *reserved* / locale-defined
/// (8, 23-36, 41-44, 50-58) return `None` — we simply do not know their code
/// without a locale, and they are exceedingly rare in the wild.
///
/// The commonly-seen ids and their codes:
///
/// | id | code            | id | code                 |
/// |----|-----------------|----|----------------------|
/// | 0  | `General`       | 18 | `h:mm AM/PM`         |
/// | 1  | `0`             | 19 | `h:mm:ss AM/PM`      |
/// | 2  | `0.00`          | 20 | `h:mm`               |
/// | 3  | `#,##0`         | 21 | `h:mm:ss`            |
/// | 4  | `#,##0.00`      | 22 | `m/d/yyyy h:mm`      |
/// | 9  | `0%`            | 37 | `#,##0 ;(#,##0)`     |
/// | 10 | `0.00%`         | 38 | `#,##0 ;[Red](#,##0)`|
/// | 11 | `0.00E+00`      | 39 | `#,##0.00;(#,##0.00)`|
/// | 12 | `# ?/?`         | 40 | `#,##0.00;[Red](…)`  |
/// | 13 | `# ??/??`       | 45 | `mm:ss`              |
/// | 14 | `m/d/yyyy`      | 46 | `[h]:mm:ss`          |
/// | 15 | `d-mmm-yy`      | 47 | `mmss.0`             |
/// | 16 | `d-mmm`         | 48 | `##0.0E+0`           |
/// | 17 | `mmm-yy`        | 49 | `@`                  |
pub fn builtin_format_code(id: u32) -> Option<&'static str> {
    Some(match id {
        0 => "General",
        1 => "0",
        2 => "0.00",
        3 => "#,##0",
        4 => "#,##0.00",
        // 5-8 are locale-dependent currency/accounting; not portable → None.
        9 => "0%",
        10 => "0.00%",
        11 => "0.00E+00",
        12 => "# ?/?",
        13 => "# ??/??",
        14 => "m/d/yyyy",
        15 => "d-mmm-yy",
        16 => "d-mmm",
        17 => "mmm-yy",
        18 => "h:mm AM/PM",
        19 => "h:mm:ss AM/PM",
        20 => "h:mm",
        21 => "h:mm:ss",
        22 => "m/d/yyyy h:mm",
        // 23-36 reserved / locale.
        37 => "#,##0 ;(#,##0)",
        38 => "#,##0 ;[Red](#,##0)",
        39 => "#,##0.00;(#,##0.00)",
        40 => "#,##0.00;[Red](#,##0.00)",
        // 41-44 locale accounting.
        45 => "mm:ss",
        46 => "[h]:mm:ss",
        47 => "mmss.0",
        48 => "##0.0E+0",
        49 => "@",
        _ => return None,
    })
}

// ===========================================================================
// Classification — code string → NumberFormatKind
// ===========================================================================

/// Classify a format **code** string into a [`NumberFormatKind`].
///
/// For a few ids the classification is unambiguous from the id itself
/// (`0` → General, `49`/`@` → Text) and callers should prefer
/// [`classify_id`]. But custom codes (id ≥ 164) have no id-based meaning, so we
/// must read the code. The rules, in priority order:
///
/// 1. Exactly `General` → `General`; exactly `@` → `Text`.
/// 2. Scanning the code **outside** quoted literals (`"..."`), bracket
///    directives (`[...]`), and backslash escapes (`\x`):
///    - a `y`, or a `d`/`m` that is a date field → date component present;
///    - an `h` or `s` → time component present;
///    - both date and time → `DateTime`; date only → `Date`; time only →
///      `Time`.
/// 3. Else if the code contains `%` → `Percent`.
/// 4. Else if it contains a currency signal (a `$`, `€`, `£`, `¥`, or a
///    `[$...]` currency directive) → `Currency`.
/// 5. Else if it contains an `@` anywhere → `Text`.
/// 6. Else if it contains digit placeholders (`0`, `#`, `?`) → `Number`.
/// 7. Else → `Other`.
///
/// ### The `m` ambiguity
///
/// `m` means **month** in a date context but **minute** in a time context. Excel
/// disambiguates positionally: an `m` adjacent to an `h` (hours) or `s`
/// (seconds) is minutes. We approximate that: an `m` is treated as a *time*
/// minute only when the code also contains `h` or `s` **and** no `y`/`d` date
/// field is present; otherwise `m` is a month (date). This matches every code in
/// the built-in table and the common custom codes.
pub fn classify_format_code(code: &str) -> NumberFormatKind {
    let trimmed = code.trim();
    if trimmed.eq_ignore_ascii_case("general") {
        return NumberFormatKind::General;
    }
    if trimmed == "@" {
        return NumberFormatKind::Text;
    }

    let scan = scan_code(code);

    // Date/time detection wins first: a date or time component is unambiguous.
    match (scan.has_date, scan.has_time) {
        (true, true) => return NumberFormatKind::DateTime,
        (true, false) => return NumberFormatKind::Date,
        (false, true) => return NumberFormatKind::Time,
        (false, false) => {}
    }

    if scan.has_percent {
        return NumberFormatKind::Percent;
    }
    if scan.has_currency {
        return NumberFormatKind::Currency;
    }
    if scan.has_at {
        return NumberFormatKind::Text;
    }
    if scan.has_digit_placeholder {
        return NumberFormatKind::Number;
    }
    NumberFormatKind::Other
}

/// The signals we harvest from a single left-to-right scan of a format code,
/// respecting quoting/bracket/escape context so literal text never triggers a
/// false positive (e.g. the literal `"May"` must not be read as a month field).
struct CodeScan {
    has_date: bool,
    has_time: bool,
    has_percent: bool,
    has_currency: bool,
    has_at: bool,
    has_digit_placeholder: bool,
}

/// Scan a format code once, honoring the three "literal" contexts OOXML uses:
///
/// * `"..."` — a quoted literal string (its bytes are shown verbatim);
/// * `\x` — a single escaped literal character;
/// * `[...]` — a directive (colour like `[Red]`, condition like `[>100]`, a
///   locale/currency like `[$€-407]`, or an elapsed-time like `[h]`).
///
/// Only *format tokens outside* those contexts carry meaning. Two subtleties:
/// a `[$...]` bracket is a **currency** directive (so it sets `has_currency`),
/// and a `[h]`/`[m]`/`[s]` bracket is an **elapsed-time** field (so it sets the
/// time flag). We special-case both while otherwise skipping bracket contents.
fn scan_code(code: &str) -> CodeScan {
    let mut has_date = false;
    let mut has_time = false;
    let mut has_percent = false;
    let mut has_currency = false;
    let mut has_at = false;
    let mut has_digit_placeholder = false;
    // Was a date field (y or d, or an m already resolved to month) seen? Used to
    // decide whether a lone `m` before any h/s is a month.
    let mut saw_ymd = false;
    // Was a clock field (h/s) seen anywhere? Governs how `m` is read.
    let has_clock = code_has_clock(code);

    let chars: Vec<char> = code.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        match ch {
            // --- quoted literal: skip to the closing quote ---
            //
            // The bytes are shown verbatim, so date/percent/number tokens inside
            // must NOT fire. BUT a currency *symbol* inside a quoted literal is
            // exactly how the common `"$"#,##0.00` format writes its sign, so we
            // still let a quoted currency symbol flag currency.
            '"' => {
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if is_currency_symbol(chars[i]) {
                        has_currency = true;
                    }
                    i += 1;
                }
                // step over the closing quote (if present)
                if i < chars.len() {
                    i += 1;
                }
                continue;
            }
            // --- escaped single char: a quoted-style single literal. Like a
            // quoted literal, a currency symbol here still counts (e.g. `\$`). ---
            '\\' => {
                if i + 1 < chars.len() && is_currency_symbol(chars[i + 1]) {
                    has_currency = true;
                }
                i += 2;
                continue;
            }
            // --- bracket directive: inspect, then skip to ']' ---
            '[' => {
                // Grab the directive body (up to ']').
                let start = i + 1;
                let mut j = start;
                while j < chars.len() && chars[j] != ']' {
                    j += 1;
                }
                let body: String = chars[start..j].iter().collect();
                let low = body.to_ascii_lowercase();
                if low.starts_with('$') {
                    // [$€-407], [$-409], [$USD] … a locale/currency directive.
                    has_currency = true;
                } else if low.starts_with('h') || low.starts_with('m') || low.starts_with('s') {
                    // [h], [mm], [ss] — elapsed-time fields.
                    has_time = true;
                }
                i = j + 1; // step past ']'
                continue;
            }
            // --- currency symbols as bare characters ---
            c if is_currency_symbol(c) => {
                has_currency = true;
            }
            '%' => has_percent = true,
            '@' => has_at = true,
            '0' | '#' | '?' => has_digit_placeholder = true,
            // --- date/time field letters ---
            'y' | 'Y' => {
                has_date = true;
                saw_ymd = true;
            }
            'd' | 'D' => {
                has_date = true;
                saw_ymd = true;
            }
            'h' | 'H' | 's' | 'S' => {
                has_time = true;
            }
            'm' | 'M' => {
                // month vs minute. Minute only when a clock field exists AND we
                // have not already committed to a date context via y/d.
                if has_clock && !saw_ymd {
                    has_time = true;
                } else {
                    has_date = true;
                    saw_ymd = true;
                }
            }
            _ => {}
        }
        i += 1;
    }

    CodeScan {
        has_date,
        has_time,
        has_percent,
        has_currency,
        has_at,
        has_digit_placeholder,
    }
}

/// Is `c` a currency symbol we recognize? Covers the common Western and a few
/// other majors; the `[$...]` bracket directive handles anything locale-tagged.
fn is_currency_symbol(c: char) -> bool {
    matches!(c, '$' | '€' | '£' | '¥' | '₹' | '₽' | '₩' | '¢')
}

/// Does the code contain a clock field (`h` or `s`) outside literals/brackets?
/// Used up front so a leading `m` can be read as a minute when appropriate.
/// This is a lightweight pre-pass (it ignores the `m`-ambiguity itself).
fn code_has_clock(code: &str) -> bool {
    let chars: Vec<char> = code.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '"' => {
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    i += 1;
                }
            }
            '\\' => {
                i += 1; // skip escaped char
            }
            '[' => {
                // A [h]/[s] elapsed field also counts as a clock.
                let start = i + 1;
                let mut j = start;
                while j < chars.len() && chars[j] != ']' {
                    j += 1;
                }
                let body: String = chars[start..j].iter().collect();
                let low = body.to_ascii_lowercase();
                if low.starts_with('h') || low.starts_with('s') {
                    return true;
                }
                i = j; // will be +1'd below to pass ']'
            }
            'h' | 'H' | 's' | 'S' => return true,
            _ => {}
        }
        i += 1;
    }
    false
}

/// Classify by `numFmtId`, preferring the id's fixed meaning where it has one.
///
/// For built-in ids we resolve the spec code and classify *that*; a couple of
/// ids (`0` → General, `49` → Text) are pinned directly. For an unknown built-in
/// id with no code we fall back to `General`. Custom ids (≥ 164) have no id-based
/// meaning here — the caller must classify their explicit `formatCode` — so we
/// return `Other` as a neutral default (the real answer comes from the code).
pub fn classify_id(id: u32) -> NumberFormatKind {
    match id {
        0 => NumberFormatKind::General,
        49 => NumberFormatKind::Text,
        _ => match builtin_format_code(id) {
            Some(code) => classify_format_code(code),
            None if id < FIRST_CUSTOM_FORMAT_ID => NumberFormatKind::General,
            None => NumberFormatKind::Other,
        },
    }
}

// ===========================================================================
// The parsed style table
// ===========================================================================

/// The parts of `xl/styles.xml` this reader needs: the custom-format code map
/// and the ordered `cellXfs` list (each entry is a `numFmtId`).
///
/// Kept as an owned, self-contained table so a `Cell` can be given its
/// [`NumberFormat`] at decode time without holding a reference back into the
/// styles XML.
#[derive(Debug, Clone, Default)]
pub struct StyleTable {
    /// Custom `numFmtId` (≥ 164) → its explicit `formatCode`.
    custom_codes: std::collections::BTreeMap<u32, String>,
    /// `cellXfs` in document order: index (the cell's `s=`) → `numFmtId`.
    cell_xfs: Vec<u32>,
}

impl StyleTable {
    /// An empty style table — the legal result for a workbook with no
    /// `xl/styles.xml` (every cell is then `General`).
    pub fn empty() -> Self {
        StyleTable::default()
    }

    /// Parse a `<styleSheet>` root element into a [`StyleTable`].
    ///
    /// Reads two children:
    /// * `<numFmts><numFmt numFmtId formatCode/></numFmts>` → the custom map;
    /// * `<cellXfs><xf numFmtId/></cellXfs>` → the ordered id list.
    ///
    /// Both are optional; a stylesheet may declare neither.
    pub fn from_root(root: &XmlElement) -> Self {
        let mut custom_codes = std::collections::BTreeMap::new();
        if let Some(num_fmts) = root.get_child(Some(SML_NS), "numFmts") {
            for nf in num_fmts.get_children(Some(SML_NS), "numFmt") {
                if let (Some(id), Some(code)) = (
                    nf.get_attr(None, "numFmtId").and_then(|s| s.parse::<u32>().ok()),
                    nf.get_attr(None, "formatCode"),
                ) {
                    custom_codes.insert(id, code.to_string());
                }
            }
        }

        let mut cell_xfs = Vec::new();
        if let Some(xfs) = root.get_child(Some(SML_NS), "cellXfs") {
            for xf in xfs.get_children(Some(SML_NS), "xf") {
                // A missing numFmtId defaults to 0 (General), per spec.
                let id = xf
                    .get_attr(None, "numFmtId")
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0);
                cell_xfs.push(id);
            }
        }

        StyleTable {
            custom_codes,
            cell_xfs,
        }
    }

    /// Resolve a cell's `s=` style index into a [`NumberFormat`].
    ///
    /// Returns `None` when:
    /// * `style_index` is `None` (the cell had no `s=` attribute), or
    /// * the index is out of range for `cellXfs` (a malformed style ref — we
    ///   degrade gracefully rather than erroring), or
    /// * the resolved format is `General` (id 0) — a `General` cell needs no
    ///   `NumberFormat`, keeping M3 behaviour for unstyled numeric cells.
    ///
    /// Otherwise it builds the format: resolves the `numFmtId` to a code
    /// (built-in table or the custom map) and classifies it.
    pub fn format_for(&self, style_index: Option<u32>) -> Option<NumberFormat> {
        let idx = style_index? as usize;
        let id = *self.cell_xfs.get(idx)?;

        // General (0) → no format attached (M3-compatible bare number).
        if id == 0 {
            return None;
        }

        let code = self.code_for(id)?;
        let kind = if id < FIRST_CUSTOM_FORMAT_ID {
            classify_id(id)
        } else {
            classify_format_code(&code)
        };

        Some(NumberFormat { id, code, kind })
    }

    /// The format code for a `numFmtId`: the custom map for ids ≥ 164, else the
    /// built-in table. `None` if neither has it (an unknown built-in id).
    fn code_for(&self, id: u32) -> Option<String> {
        if id >= FIRST_CUSTOM_FORMAT_ID {
            self.custom_codes.get(&id).cloned()
        } else {
            builtin_format_code(id).map(|s| s.to_string())
        }
    }
}

// ===========================================================================
// Serial → date rendering (1900 date system)
// ===========================================================================

/// Convert an Excel **1900-system** date serial into an ISO `YYYY-MM-DD` string.
///
/// Serial `0` is anchored at the fictitious `1899-12-30`; days are then added on
/// a real proleptic Gregorian calendar. This reproduces Excel's phantom
/// `1900-02-29` (serial 60) exactly — see the module docs. The integer part of
/// the serial selects the day; any fractional part (the time-of-day) is dropped
/// here (see [`serial_to_datetime`] for date+time).
///
/// Returns `None` only for a serial so large it overflows the day arithmetic,
/// which cannot happen for any real spreadsheet date.
///
/// ```
/// use coding_adventures_spreadsheetml::serial_to_date;
/// assert_eq!(serial_to_date(1.0).as_deref(), Some("1900-01-01"));
/// assert_eq!(serial_to_date(60.0).as_deref(), Some("1900-02-29")); // Excel bug
/// assert_eq!(serial_to_date(61.0).as_deref(), Some("1900-03-01"));
/// assert_eq!(serial_to_date(45292.0).as_deref(), Some("2024-01-01"));
/// ```
pub fn serial_to_date(serial: f64) -> Option<String> {
    let (y, m, d) = serial_to_ymd(serial)?;
    Some(format!("{y:04}-{m:02}-{d:02}"))
}

/// Convert a 1900-system serial into an ISO `YYYY-MM-DDTHH:MM:SS` string,
/// including the fractional time-of-day.
///
/// The integer part is the date (as in [`serial_to_date`]); the fraction is the
/// portion of a 24-hour day, so `0.5` is noon. Seconds are rounded to the
/// nearest whole second.
///
/// ```
/// use coding_adventures_spreadsheetml::serial_to_datetime;
/// // 45292.5 → 2024-01-01 at noon.
/// assert_eq!(serial_to_datetime(45292.5).as_deref(), Some("2024-01-01T12:00:00"));
/// ```
pub fn serial_to_datetime(serial: f64) -> Option<String> {
    let whole = serial.floor();
    let (y, m, d) = serial_to_ymd(whole)?;

    // Fractional day → seconds since midnight, rounded to the second.
    let frac = serial - whole;
    let mut total_secs = (frac * 86_400.0).round() as i64;
    // Rounding can push us to a full day (86400); carry into the date.
    let (y, m, d) = if total_secs >= 86_400 {
        total_secs -= 86_400;
        serial_to_ymd(whole + 1.0)?
    } else {
        (y, m, d)
    };
    let hh = total_secs / 3600;
    let mm = (total_secs % 3600) / 60;
    let ss = total_secs % 60;
    Some(format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}"))
}

/// The shared date core: a 1900-system serial's **integer** part → `(y, m, d)`.
///
/// Algorithm: serial 0 = 1899-12-30. Excel's fake `1900-02-29` occupies serial
/// 60, so for serials **≥ 60** the on-disk serial is one greater than the true
/// day count since the epoch. We therefore subtract one for serials ≥ 60 to get
/// a *real* day offset, then walk the proleptic Gregorian calendar — except we
/// map serial 60 itself to the phantom `1900-02-29` explicitly.
fn serial_to_ymd(serial: f64) -> Option<(i64, u32, u32)> {
    let s = serial.floor() as i64;

    // The phantom day: Excel shows 1900-02-29, which does not exist.
    if s == 60 {
        return Some((1900, 2, 29));
    }

    // Real day offset from 1899-12-30. For serials past the phantom, the stored
    // serial over-counts by one, so subtract it back out.
    let day_offset = if s >= 60 { s - 1 } else { s };

    // Days from the proleptic-Gregorian epoch we compute against. We pick a
    // fixed civil-date algorithm anchored at 1899-12-30.
    ymd_from_1899_12_30(day_offset)
}

/// Civil date `days` after 1899-12-30, on the proleptic Gregorian calendar.
///
/// Uses Howard Hinnant's well-known `civil_from_days` algorithm, shifted so its
/// day-0 lands on 1899-12-30. The algorithm is exact for the entire range a
/// spreadsheet could hold and needs no leap-year special-casing in our code —
/// it is baked into the closed-form below.
fn ymd_from_1899_12_30(days: i64) -> Option<(i64, u32, u32)> {
    // Hinnant's algorithm counts days from 1970-01-01 (the Unix epoch). Our
    // anchor 1899-12-30 is a fixed number of *real* days before that: 25568.
    // (The raw Excel serial for 1970-01-01 is 25569, but that count includes the
    // phantom 1900-02-29; `days` here is the phantom-adjusted real offset, so we
    // shift by 25568 — one less — to reach the Unix epoch.)
    let z = days.checked_sub(25_568)?; // now days since 1970-01-01
    let z = z.checked_add(719_468)?; // shift to era-based (0000-03-01 origin)

    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = if m <= 2 { y + 1 } else { y };

    Some((year, m as u32, d as u32))
}

// ===========================================================================
// CellRange — a merged-cell span
// ===========================================================================

/// A rectangular range of cells, e.g. the `A1:B1` of a `<mergeCell>`.
///
/// Stored as start/end `(col, row)` pairs, both **1-based**, matching
/// [`parse_a1_ref`](crate::parse_a1_ref). `start` is the top-left, `end` the
/// bottom-right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRange {
    /// Top-left `(col, row)`, both 1-based.
    pub start: (u32, u32),
    /// Bottom-right `(col, row)`, both 1-based.
    pub end: (u32, u32),
}

impl CellRange {
    /// Parse an `A1:B2`-style range into a [`CellRange`]. Returns `None` if
    /// either endpoint is not a valid A1 reference or the `:` is missing.
    pub fn parse(range: &str) -> Option<CellRange> {
        let (a, b) = range.split_once(':')?;
        let start = crate::parse_a1_ref(a.trim())?;
        let end = crate::parse_a1_ref(b.trim())?;
        Some(CellRange { start, end })
    }
}
