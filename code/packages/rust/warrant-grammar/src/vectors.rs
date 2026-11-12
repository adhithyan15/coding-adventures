//! # Pinned external known-answer tables (VER04j WX-E)
//!
//! A `vectors` declaration keeps the *data* of a known-answer table in a TFV
//! fixture and pins the file's identity in the document:
//!
//! ```text
//! vectors sha256_short:
//!   source "warrant-vectors-synthetic-v1/sha256-short.tfv"
//!   sha256 b"<64 hex digits: SHA-256 of the file>"
//!   rows 6
//!   column msg: Bytes
//!   column md: Bytes<32>
//!   holds sha256_digest(msg) == md
//! ```
//!
//! This module is the *tool half*: it turns a declaration into verified bytes.
//! It is deliberately paranoid, because the file it reads is data from outside
//! the document and everything downstream (the checked-in copy, the generated
//! test, the manifest) trusts what it returns.
//!
//! ## The order of operations is the security argument
//!
//! ```text
//!   path       valid_table_path()       alphabet, length, no `..`, no leading `/`
//!     |
//!   resolve    lstat every component    a symlink anywhere below the root: W0708
//!     |
//!   open       fstat the handle         regular file; (dev, ino) equals the lstat;
//!     |                                 canonical path equals root + path
//!   read       take(MAX + 1)            at most 16 MiB (+1 byte to see overflow)
//!     |
//!   digest     SHA-256 == the pin       BEFORE any parsing: mismatch is W0708
//!     |
//!   scan       lines, map headers       row count; no map with more than one entry
//!     |
//!   parse      tfv::parse_file          the lane's own TF01 parser (no new reader)
//!     |
//!   check      every row, every column  exactly the columns; each value decodes
//! ```
//!
//! A file is never parsed before its digest is checked, never read past the size
//! cap, never followed through a symlink, and never fetched from anywhere but the
//! fixtures root: **nothing in this module touches the network**.
//!
//! ## What a column may hold (VER04j section 4.9, VER04i)
//!
//! ```text
//! Bool          T | F                       Nat, U8 ... I64    I<decimal> (range-checked)
//! Bytes         X<hex>                      Bytes<N>           X<hex> of exactly N bytes
//! Option<T>     L0 | L1 x                   List<T>            L<n> x1 ... xn
//! (A, B), rec   L<n> x1 ... xn              variant            L2 S<ctor> L<k> f1 ... fk
//! Result<T, E>  M1 S"ok" v | M1 S"err" S<CASE>
//! ```
//!
//! A TFV map therefore appears only as a one-entry `Result`; a map header with
//! more entries is refused up front (W0709).  That is not only the type rule: the
//! TFV parser checks a map's keys pairwise, which is quadratic in the number of
//! entries, so a line of hostile map headers would otherwise be a denial of
//! service that the pin cannot prevent (the pin only says *which* file).

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use coding_adventures_sha256::sha256_hex;
use coding_adventures_test_bootstrap::tfv::{self, Record, Value};

use crate::codegen::{CodegenError, CodegenErrorKind, DiagnosticCode};
use crate::fixed::FixedTy;
use crate::parser::{valid_table_path, Expression, VectorsDecl};

/// The fixtures root, relative to the repository root (VER04j section 5.2: a
/// tool-profile constant; a document never names a host path).
pub const FIXTURES_ROOT: &str = "code/specs/fixtures";

/// Largest fixture, in bytes (VER04j section 5.2).
pub const MAX_TABLE_FILE_BYTES: u64 = 16 * 1024 * 1024;

/// Most tables one document may load (each keeps its bytes and rows alive).
pub const MAX_DOCUMENT_TABLES: usize = 16;

/// Most fixture bytes all the tables of one document may hold together.
pub const MAX_DOCUMENT_BYTES: u64 = 64 * 1024 * 1024;

/// Most space-separated tokens one record line may have: a line of N tokens
/// expands to hundreds of bytes per token once parsed, so an 8M-token line
/// would be hundreds of MB.
pub const MAX_ROW_TOKENS: usize = 65_536;

/// Deepest nesting of containers a row may have (TFV's own limit).
pub const MAX_NESTING: usize = tfv::MAX_DEPTH;

/// What a column value must look like, as the loader checks it.  The generator
/// derives one `Shape` per column from the declared type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shape {
    Bool,
    /// An integer in `min..=max`: a fixed-width type, a `Nat`, an `Int`.
    Int { min: i128, max: i128 },
    Bytes,
    BytesN(u64),
    Opt(Box<Shape>),
    List(Box<Shape>),
    /// A tuple or a record: one component per entry.
    Tuple(Vec<Shape>),
    /// A variant: each constructor's name and field shapes, in declaration order.
    Variant(Vec<(String, Vec<Shape>)>),
    /// `Result<T, E>`: the payload and the cases of `E`.
    Result(Box<Shape>, Vec<String>),
}

impl Shape {
    /// The shape of a fixed-width integer type.
    pub fn fixed(t: FixedTy) -> Shape {
        Shape::Int {
            min: t.min(),
            max: t.max(),
        }
    }

    /// The shape of a `Nat` (zero up to the largest model integer).
    pub fn nat() -> Shape {
        Shape::Int {
            min: 0,
            max: i128::MAX,
        }
    }

    /// The shape of an `Int`.
    pub fn int() -> Shape {
        Shape::Int {
            min: i128::MIN,
            max: i128::MAX,
        }
    }
}

/// A table whose file has passed every check of the module documentation.
#[derive(Clone, Debug)]
pub struct LoadedTable {
    pub name: String,
    /// The path as written in the document (relative to the fixtures root).
    pub source: String,
    /// SHA-256 of `bytes`, as 64 lowercase hex digits (equal to the pin).
    pub sha256: String,
    /// The file's exact bytes: the generator copies them byte-identically.
    pub bytes: Vec<u8>,
    /// The parsed records, in file order.
    pub rows: Vec<Record>,
    /// The declared columns, in declaration order.
    pub columns: Vec<String>,
}

fn err(code: DiagnosticCode, msg: impl Into<String>, span: (usize, usize)) -> CodegenError {
    CodegenError {
        code,
        kind: CodegenErrorKind::UnsupportedTarget(msg.into()),
        span,
    }
}

fn integrity(msg: impl Into<String>, span: (usize, usize)) -> CodegenError {
    err(DiagnosticCode::W0708, msg, span)
}

// ---------------------------------------------------------------------------
// Reading the file
// ---------------------------------------------------------------------------

/// Read the fixture `rel` below `root`, refusing symlinks and oversized files.
///
/// `root` is the fixtures root itself (the tool profile's constant, or a test's
/// temporary directory); symlinks *above* it are the operator's business, those
/// *below* it are data.  On any failure the message says what rule was broken,
/// never what the file contained.
pub fn read_fixture(root: &Path, rel: &str, max_bytes: u64) -> Result<Vec<u8>, String> {
    if !valid_table_path(rel) {
        return Err(format!("source path {rel:?} breaks the path rules"));
    }
    let canonical_root = root
        .canonicalize()
        .map_err(|_| "the fixtures root does not exist".to_string())?;
    // Step 1: every component below the root is a plain directory, and the last
    // one a plain file.  `symlink_metadata` does not follow the link it inspects.
    let segments: Vec<&str> = rel.split('/').collect();
    let mut path = canonical_root.clone();
    let mut last = None;
    for (i, seg) in segments.iter().enumerate() {
        path.push(seg);
        let md = fs::symlink_metadata(&path)
            .map_err(|_| format!("fixture {rel:?} does not exist"))?;
        if md.file_type().is_symlink() {
            return Err(format!("fixture {rel:?} passes through a symlink"));
        }
        let leaf = i + 1 == segments.len();
        if leaf && !md.is_file() {
            return Err(format!("fixture {rel:?} is not a regular file"));
        }
        if !leaf && !md.is_dir() {
            return Err(format!("fixture {rel:?}: {seg:?} is not a directory"));
        }
        last = Some(md);
    }
    let lstat = last.expect("a path has at least one segment");
    if lstat.len() > max_bytes {
        return Err(format!("fixture {rel:?} is larger than {max_bytes} bytes"));
    }
    // Step 2: open it, and check that the handle is the file we inspected.
    // Non-blocking: a FIFO swapped in after the lstat must not block the
    // open; `fstat` below then refuses it as not a regular file.
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        #[cfg(target_os = "linux")]
        options.custom_flags(0o4000); // O_NONBLOCK
        #[cfg(any(target_os = "macos", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
        options.custom_flags(0x4); // O_NONBLOCK
    }
    let file = options
        .open(&path)
        .map_err(|_| format!("fixture {rel:?} cannot be opened"))?;
    let md = file
        .metadata()
        .map_err(|_| format!("fixture {rel:?} cannot be inspected"))?;
    if !md.is_file() {
        return Err(format!("fixture {rel:?} is not a regular file"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if md.dev() != lstat.dev() || md.ino() != lstat.ino() {
            return Err(format!("fixture {rel:?} changed while it was opened"));
        }
    }
    // A directory component swapped for a symlink after step 1 would still be
    // followed by `open`; the canonical path of what was opened rules that out.
    let canonical = path
        .canonicalize()
        .map_err(|_| format!("fixture {rel:?} cannot be resolved"))?;
    if canonical != path {
        return Err(format!("fixture {rel:?} resolves outside its own path"));
    }
    // Step 3: read at most `max + 1` bytes; the extra byte is how an
    // over-long file is told from one that is exactly at the cap.
    let mut bytes = Vec::new();
    file.take(max_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| format!("fixture {rel:?} cannot be read"))?;
    if bytes.len() as u64 > max_bytes {
        return Err(format!("fixture {rel:?} is larger than {max_bytes} bytes"));
    }
    Ok(bytes)
}

// ---------------------------------------------------------------------------
// Pre-parse scans
// ---------------------------------------------------------------------------

/// The number of record lines (neither blank, nor a `#` comment, nor the header).
fn record_lines(text: &str) -> u64 {
    text.split('\n')
        .skip(1)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .count() as u64
}

/// Does any record line carry a map header with more than one entry (`M2`, `M10`,
/// ...)?  The first token of a record line is its id (which may start with `M`);
/// every later token that is `M` followed only by digits is a map header, since
/// every other value token starts with a different tag.
fn has_big_map(text: &str) -> bool {
    text.split('\n')
        .filter(|l| !l.starts_with('#'))
        .flat_map(|l| l.split(' ').skip(1))
        .filter_map(|t| t.strip_prefix('M'))
        .any(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) && n != "0" && n != "1")
}

/// The most tokens on any record line.
fn max_line_tokens(text: &str) -> usize {
    text.split('\n')
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.bytes().filter(|b| *b == b' ').count() + 1)
        .max()
        .unwrap_or(0)
}

/// Refuse `rel` (below `root`) if it, or any directory above it that exists,
/// is a symlink: used before the generator writes a file.
pub fn ensure_no_symlinks(root: &Path, rel: &str) -> Result<(), String> {
    if !valid_table_path(rel) {
        return Err(format!("path {rel:?} breaks the path rules"));
    }
    let mut path = root.to_path_buf();
    for seg in rel.split('/') {
        path.push(seg);
        if let Ok(md) = fs::symlink_metadata(&path) {
            if md.file_type().is_symlink() {
                return Err(format!("{rel:?} passes through a symlink"));
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Checking values against shapes
// ---------------------------------------------------------------------------

/// Check one value against its column's shape.  The message names the problem
/// (and never the value).
pub fn check_value(v: &Value, shape: &Shape, depth: usize) -> Result<(), String> {
    if depth > MAX_NESTING {
        return Err(format!("nests deeper than {MAX_NESTING} containers"));
    }
    let wrong = |want: &str| Err(format!("expected {want}"));
    match (shape, v) {
        (Shape::Bool, Value::Bool(_)) => Ok(()),
        (Shape::Bool, _) => wrong("a Bool (T or F)"),
        (Shape::Int { min, max }, Value::Int(text)) => match text.parse::<i128>() {
            Ok(n) if (*min..=*max).contains(&n) => Ok(()),
            _ => Err("an integer outside the column type's range".into()),
        },
        (Shape::Int { .. }, _) => wrong("an integer (I<decimal>)"),
        (Shape::Bytes, Value::Bytes(_)) => Ok(()),
        (Shape::Bytes, _) => wrong("bytes (X<hex>)"),
        (Shape::BytesN(n), Value::Bytes(b)) => {
            if b.len() as u64 == *n {
                Ok(())
            } else {
                Err(format!("expected exactly {n} bytes"))
            }
        }
        (Shape::BytesN(n), _) => wrong(&format!("{n} bytes (X<hex>)")),
        (Shape::Opt(inner), Value::List(items)) => match items.as_slice() {
            [] => Ok(()),
            [x] => check_value(x, inner, depth + 1),
            _ => Err("an Option is L0 or L1 x".into()),
        },
        (Shape::Opt(_), _) => wrong("an Option (L0 | L1 x)"),
        (Shape::List(inner), Value::List(items)) => items
            .iter()
            .try_for_each(|x| check_value(x, inner, depth + 1)),
        (Shape::List(_), _) => wrong("a list (L<n> ...)"),
        (Shape::Tuple(parts), Value::List(items)) => {
            if items.len() != parts.len() {
                return Err(format!("expected {} components, found {}", parts.len(), items.len()));
            }
            parts
                .iter()
                .zip(items)
                .try_for_each(|(s, x)| check_value(x, s, depth + 1))
        }
        (Shape::Tuple(_), _) => wrong("a tuple or record (L<n> ...)"),
        (Shape::Variant(ctors), Value::List(items)) => {
            let [Value::Text(name), Value::List(fields)] = items.as_slice() else {
                return Err("a variant is L2 S<ctor> L<k> ...".into());
            };
            let Some((_, shapes)) = ctors.iter().find(|(c, _)| c == name) else {
                return Err("an unknown constructor name".into());
            };
            if fields.len() != shapes.len() {
                return Err(format!(
                    "a constructor takes {} fields, found {}",
                    shapes.len(),
                    fields.len()
                ));
            }
            shapes
                .iter()
                .zip(fields)
                .try_for_each(|(s, x)| check_value(x, s, depth + 2))
        }
        (Shape::Variant(_), _) => wrong("a variant (L2 S<ctor> L<k> ...)"),
        (Shape::Result(ok, cases), Value::Map(pairs)) => match pairs.as_slice() {
            [(Value::Text(k), payload)] if k == "ok" => check_value(payload, ok, depth + 1),
            [(Value::Text(k), Value::Text(case))] if k == "err" => {
                if cases.iter().any(|c| c == case) {
                    Ok(())
                } else {
                    Err("an unknown error case".into())
                }
            }
            _ => Err("a Result is M1 S\"ok\" v or M1 S\"err\" S<CASE>".into()),
        },
        (Shape::Result(..), _) => wrong("a Result (M1 ...)"),
    }
}

/// Check one record against the declared columns: exactly those fields (no
/// missing, no extra), each decoding as its column's shape.
fn check_row(row: &Record, columns: &[(String, Shape)]) -> Result<(), String> {
    if row.fields.len() != columns.len() {
        return Err(format!(
            "row {:?} has {} fields, the table declares {}",
            row.id,
            row.fields.len(),
            columns.len()
        ));
    }
    for (name, shape) in columns {
        let Some(value) = row.field(name) else {
            return Err(format!("row {:?} is missing column {name:?}", row.id));
        };
        check_value(value, shape, 0)
            .map_err(|m| format!("row {:?}, column {name:?}: {m}", row.id))?;
    }
    // The count matched and every column was found, so no field is extra.
    Ok(())
}

// ---------------------------------------------------------------------------
// The loader
// ---------------------------------------------------------------------------

/// Load and verify the table `decl` (see the module documentation for the
/// order of the checks).  `shapes` gives each column's shape, in declaration
/// order; `root` is the fixtures root.
pub fn load_table(
    decl: &VectorsDecl<'_>,
    shapes: &[(String, Shape)],
    root: &Path,
) -> Result<LoadedTable, CodegenError> {
    let span = decl.span;
    if shapes.len() != decl.columns.len() {
        return Err(err(
            DiagnosticCode::W0710,
            format!("table {}: {} column shapes for {} columns", decl.name, shapes.len(), decl.columns.len()),
            span,
        ));
    }
    // The parser already enforces these; a hand-built declaration must not
    // be able to skip them.
    if !valid_table_path(decl.source) {
        return Err(integrity(format!("table {}: source path breaks the path rules", decl.name), span));
    }
    if decl.sha256.len() != 64 || !decl.sha256.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return Err(integrity(format!("table {}: sha256 is not 64 lowercase hex digits", decl.name), span));
    }
    let bytes = read_fixture(root, decl.source, MAX_TABLE_FILE_BYTES)
        .map_err(|m| integrity(format!("table {}: {m}", decl.name), span))?;
    // The pin comes first: nothing in the file is interpreted until it matches.
    let digest = sha256_hex(&bytes);
    if digest != decl.sha256 {
        return Err(integrity(
            format!("table {}: the file's SHA-256 does not match the pin", decl.name),
            span,
        ));
    }
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| integrity(format!("table {}: the file is not UTF-8", decl.name), span))?;
    let lines = record_lines(text);
    if lines != decl.rows {
        return Err(integrity(
            format!("table {}: the file has {lines} records, the document pins {}", decl.name, decl.rows),
            span,
        ));
    }
    if max_line_tokens(text) > MAX_ROW_TOKENS {
        return Err(err(
            DiagnosticCode::W0602,
            format!("table {}: a record has more than {MAX_ROW_TOKENS} tokens", decl.name),
            span,
        ));
    }
    if has_big_map(text) {
        return Err(err(
            DiagnosticCode::W0709,
            format!("table {}: a map with more than one entry has no Warrant type", decl.name),
            span,
        ));
    }
    let rows = tfv::parse_file(text).map_err(|e| {
        // The parser's own nesting limit is the spec's example for W0709.
        let code = if e.message.contains("nest at most") {
            DiagnosticCode::W0709
        } else {
            DiagnosticCode::W0708
        };
        err(code, format!("table {}: malformed TFV, {e}", decl.name), span)
    })?;
    // `record_lines` counted exactly the lines `parse_file` turns into records,
    // so this cannot differ; it is checked rather than assumed because a panic
    // (or a silently wrong count) in the tool would be worse than dead code.
    if rows.len() as u64 != decl.rows {
        return Err(integrity(
            format!("table {}: the file has {} records, the document pins {}", decl.name, rows.len(), decl.rows),
            span,
        ));
    }
    for row in &rows {
        check_row(row, shapes)
            .map_err(|m| err(DiagnosticCode::W0710, format!("table {}: {m}", decl.name), span))?;
    }
    Ok(LoadedTable {
        name: decl.name.to_string(),
        source: decl.source.to_string(),
        sha256: digest,
        bytes,
        rows,
        columns: decl.columns.iter().map(|c| c.name.to_string()).collect(),
    })
}

/// The path of the fixtures root below a repository root.
pub fn fixtures_root(repo_root: &Path) -> PathBuf {
    repo_root.join(FIXTURES_ROOT)
}

// ---------------------------------------------------------------------------
// The discrimination check (VER04j section 5.3 item 4)
// ---------------------------------------------------------------------------

/// The column a `holds` expression compares the subject against: for
/// `<expr> == <column>` (or the mirror) the column's name.
pub fn expected_column<'a>(holds: &'a Expression<'_>, columns: &[String]) -> Option<&'a str> {
    fn bare<'b>(e: &'b Expression<'_>, columns: &[String]) -> Option<&'b str> {
        match e {
            Expression::Grouped { inner, .. } => bare(inner, columns),
            Expression::Reference { name, args: None, .. } if name.segments.len() == 1 => {
                let n = name.segments[0];
                columns.iter().any(|c| c == n).then_some(n)
            }
            _ => None,
        }
    }
    match holds {
        Expression::Grouped { inner, .. } => expected_column(inner, columns),
        Expression::Binary {
            op: crate::parser::BinaryOp::Eq,
            left,
            right,
            ..
        } => match (bare(left, columns), bare(right, columns)) {
            (Some(_), Some(_)) => None, // two columns: no subject on either side
            (Some(c), None) => Some(c),
            (None, Some(c)) => Some(c),
            (None, None) => None,
        },
        _ => None,
    }
}

/// Is the table one that a constant function passes?  True when `holds` is
/// `<expr> == <column>` and `<column>` has the same value in every one of at
/// least two rows: whatever the inputs are, the same answer satisfies every row,
/// so no fault of VER04h section 6 could fail the table (W0601, "not
/// discriminating").
pub fn is_constant_output(table: &LoadedTable, holds: &Expression<'_>) -> bool {
    let Some(col) = expected_column(holds, &table.columns) else {
        return false;
    };
    if table.rows.len() < 2 {
        return false;
    }
    let first = table.rows[0].field(col);
    table.rows.iter().skip(1).all(|r| match (first, r.field(col)) {
        (Some(a), Some(b)) => tfv::values_equal(a, b),
        _ => false,
    })
}
