//! VER04j section 5 (WX-E) and section 14.3: the language half.
//!
//! * `vectors` is a v1 declaration (W0101 in a v0 document) with five contextual
//!   row words in a fixed order;
//! * the syntax negatives of section 5.7 (missing or malformed `sha256`, bad
//!   paths, bad `rows`, no column, no `holds`);
//! * the checker's rules: namespace, row budgets, vacuous tables, unused
//!   columns, undecodable columns, the typing of `holds`.
//!
//! The loader and the evidence half are `tests/wxe_loader.rs` and
//! `tests/wxe_evidence.rs`.

use coding_adventures_warrant_grammar::checker::{
    check_warrant, CheckedWarrantModule, CheckerError, CheckerErrorKind, DiagnosticCode,
    MAX_DOCUMENT_ROWS, MAX_TABLE_ROWS,
};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{
    parse_document, valid_table_path, Declaration, ParserError, ParserErrorKind, ParserLimits,
    WarrantFile, MAX_PATH_BYTES,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

fn parse<R>(src: &str, f: impl FnOnce(Result<&WarrantFile, ParserError>) -> R) -> R {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let parsed = parse_document(&layout, &doc, ParserLimits::default());
    f(parsed.as_ref().map_err(Clone::clone))
}

fn parse_err(src: &str) -> ParserError {
    parse(src, |p| p.expect_err("must not parse"))
}

fn check(src: &str) -> Result<CheckedWarrantModule, CheckerError> {
    parse(src, |p| check_warrant(p.expect("parse")))
}

const PIN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// A v1 document: one operation `digest` (so a table has something to call) and a
/// table built from the given pieces.
struct T {
    header: &'static str,
    name: &'static str,
    source: String,
    sha: String,
    rows: String,
    columns: Vec<&'static str>,
    holds: &'static str,
}

impl T {
    fn new() -> T {
        T {
            header: " v1",
            name: "t1",
            source: "\"dir/file.tfv\"".into(),
            sha: format!("b\"{PIN}\""),
            rows: "3".into(),
            columns: vec!["column msg: Bytes", "column md: Bytes<4>"],
            holds: "holds digest(msg) == md",
        }
    }
    fn text(&self) -> String {
        let mut s = format!(
            "warrant t{}\n  op digest(m: Bytes) -> Bytes<4>\n  labels digest:\n    m public\n  vectors {}:\n    source {}\n    sha256 {}\n    rows {}\n",
            self.header, self.name, self.source, self.sha, self.rows
        );
        for c in &self.columns {
            s.push_str(&format!("    {c}\n"));
        }
        s.push_str(&format!("    {}\n", self.holds));
        s
    }
    fn parse_err(&self) -> ParserError {
        parse_err(&self.text())
    }
    fn check(&self) -> Result<CheckedWarrantModule, CheckerError> {
        check(&self.text())
    }
    fn check_err(&self) -> CheckerError {
        self.check().expect_err("must be rejected")
    }
}

// ---------------------------------------------------------------------------
// Syntax
// ---------------------------------------------------------------------------

#[test]
fn a_valid_table_parses_and_type_checks() {
    let t = T::new();
    parse(&t.text(), |p| {
        let file = p.expect("parses");
        let Some(Declaration::Vectors(v)) = file
            .declarations
            .iter()
            .find(|d| matches!(d, Declaration::Vectors(_)))
        else {
            panic!("a table")
        };
        assert_eq!(v.name, "t1");
        assert_eq!(v.source, "dir/file.tfv");
        assert_eq!(v.sha256, PIN);
        assert_eq!(v.rows, 3);
        assert_eq!(v.columns.len(), 2);
        assert_eq!(v.columns[1].name, "md");
    });
    t.check().expect("type checks");
}

#[test]
fn vectors_is_w0101_in_a_v0_document_and_a_name_elsewhere() {
    let mut t = T::new();
    t.header = "";
    let e = t.parse_err();
    assert_eq!(e.kind(), ParserErrorKind::V1ConstructInV0);
    assert_eq!(e.code(), "W0101");
    // `vectors` and the row words are contextual: legal names in either version.
    for header in ["", " v1"] {
        let src = format!(
            "warrant t{header}\n  model vectors(a: Nat) -> Int = a + 1\n  model source(a: Nat) -> Int = a + 2\n  model rows(a: Nat) -> Int = a + 3\n  model holds(a: Nat) -> Int = a + 4\n  model column(a: Nat) -> Int = a + 5\n  model sha256(a: Nat) -> Int = a + 6\n  example e:\n    vectors(1) == 2 and source(1) == 3 and rows(1) == 4 and holds(1) == 5 and column(1) == 6 and sha256(1) == 7\n"
        );
        check(&src).unwrap_or_else(|e| panic!("header {header:?}: {e:?}"));
    }
}

#[test]
fn the_section_5_7_syntax_rejects() {
    // A missing sha256 line: the next word is `rows`.
    let src = T::new().text().replace(&format!("    sha256 b\"{PIN}\"\n"), "");
    assert_eq!(parse_err(&src).code(), "W0001");
    // sha256 that is not 32 bytes: one digit short, one pair too long, odd, empty.
    for sha in [
        &PIN[..62],
        &PIN[..63],
        &format!("{PIN}ab"),
        &PIN[..30],
        "",
    ] {
        let mut t = T::new();
        t.sha = format!("b\"{sha}\"");
        let e = t.parse_err();
        assert_eq!(e.kind(), ParserErrorKind::TableIntegrity, "{sha:?}");
        assert_eq!(e.code(), "W0708");
    }
    // Uppercase or non-hex digits in the pin.
    for sha in [PIN.to_uppercase(), format!("{}g", &PIN[..63]), PIN.replace('a', "A")] {
        let mut t = T::new();
        t.sha = format!("b\"{sha}\"");
        assert_eq!(t.parse_err().code(), "W0708", "{sha}");
    }
    // The pin must be a bytes literal.
    for sha in ["12", "\"abcd\"", "x"] {
        let mut t = T::new();
        t.sha = sha.into();
        assert_eq!(t.parse_err().code(), "W0001", "{sha}");
    }
    // Source paths: `..`, a leading slash, an empty or `.` segment, a backslash,
    // a space, a non-ASCII byte, the empty string.
    for path in [
        "../x.tfv",
        "a/../x.tfv",
        "/etc/passwd",
        "a//b.tfv",
        "./x.tfv",
        "a/.",
        "a\\\\b.tfv",
        "a b.tfv",
        "caf\u{e9}.tfv",
        "",
        "a/",
    ] {
        let mut t = T::new();
        t.source = format!("\"{path}\"");
        let e = t.parse_err();
        assert_eq!(e.kind(), ParserErrorKind::TableIntegrity, "{path:?}");
        assert_eq!(e.code(), "W0708");
    }
    // Over-long, and not a quoted text at all, and an unterminated quote.
    let mut t = T::new();
    t.source = format!("\"{}\"", "a".repeat(MAX_PATH_BYTES + 1));
    assert_eq!(t.parse_err().code(), "W0708");
    t.source = format!("\"{}\"", "a".repeat(MAX_PATH_BYTES));
    t.parse(); // exactly at the cap is fine
    for source in ["plain", "7", "b\"00\"", "\"unterminated"] {
        let mut t = T::new();
        t.source = source.into();
        assert_eq!(t.parse_err().code(), "W0001", "{source}");
    }
    // `rows`: not an integer, with a suffix, signed, with a leading zero, huge.
    for rows in ["x", "1.5", "5u8", "-1", "007", "99999999999999999999999", "0x10"] {
        let mut t = T::new();
        t.rows = rows.into();
        assert_eq!(t.parse_err().code(), "W0001", "{rows}");
    }
    // No column; no holds; the row words out of order.
    let mut t = T::new();
    t.columns.clear();
    assert_eq!(t.parse_err().code(), "W0001");
    let src = T::new().text().replace("    holds digest(msg) == md\n", "");
    assert_eq!(parse_err(&src).code(), "W0001");
    let src = T::new()
        .text()
        .replace("    rows 3\n", "")
        .replace("    column msg: Bytes\n", "    column msg: Bytes\n    rows 3\n");
    assert_eq!(parse_err(&src).code(), "W0001");
    // A table that is not indented, or has no colon.
    let src = T::new().text().replace("vectors t1:", "vectors t1");
    assert_eq!(parse_err(&src).code(), "W0001");
}

impl T {
    fn parse(&self) {
        parse(&self.text(), |p| {
            p.expect("parses");
        });
    }
}

#[test]
fn a_duplicate_column_is_w0201_and_too_many_columns_are_a_syntax_error() {
    let mut t = T::new();
    t.columns = vec!["column msg: Bytes", "column msg: Bytes<4>"];
    let e = t.parse_err();
    assert_eq!(e.kind(), ParserErrorKind::DuplicateName);
    assert_eq!(e.code(), "W0201");
    // 32 columns are the cap (as for a record), 33 are not.
    let cols = |n: usize| -> String {
        (0..n).map(|i| format!("    column c{i}: Bool\n")).collect()
    };
    let src = |n: usize| {
        format!(
            "warrant t v1\n  op digest(m: Bytes) -> Bytes<4>\n  labels digest:\n    m public\n  vectors t1:\n    source \"a.tfv\"\n    sha256 b\"{PIN}\"\n    rows 1\n{}    holds c0\n",
            cols(n)
        )
    };
    parse(&src(32), |p| {
        p.expect("32 columns");
    });
    assert_eq!(parse_err(&src(33)).code(), "W0001");
}

#[test]
fn the_path_rules_as_a_predicate() {
    for ok in ["a", "a.tfv", "dir/sub/file-1_2.tfv", "A/B", "sha256-cavs-v1/short.tfv", "a..b"] {
        assert!(valid_table_path(ok), "{ok}");
    }
    for bad in ["", "/", "/a", "a/", "a//b", ".", "..", "a/./b", "a/../b", "a/.../b", "...", "a\\b", "a b", "a\0b", "é"] {
        assert!(!valid_table_path(bad), "{bad:?}");
    }
    assert!(valid_table_path(&"a".repeat(MAX_PATH_BYTES)));
    assert!(!valid_table_path(&"a".repeat(MAX_PATH_BYTES + 1)));
}

// ---------------------------------------------------------------------------
// The checker
// ---------------------------------------------------------------------------

#[test]
fn a_table_name_shares_the_test_namespace() {
    for other in [
        "  example t1:\n    digest(b\"\") == b\"00000000\"\n",
        "  claim t1:\n    forall m: Bytes. digest(m) == digest(m)\n",
        "  vectors t1:\n    source \"b.tfv\"\n    sha256 b\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\"\n    rows 1\n    column m: Bytes\n    holds digest(m) == digest(m)\n",
    ] {
        let src = format!("{}{other}", T::new().text());
        let e = check(&src).expect_err("clash");
        assert_eq!(e.code, DiagnosticCode::W0201, "{other}");
        assert!(matches!(e.kind, CheckerErrorKind::DuplicateTestName(_)));
    }
    // A table named like a model or an operation is a different namespace.
    let mut t = T::new();
    t.name = "digest";
    t.check().expect("tests and symbols do not clash");
}

#[test]
fn row_budgets_are_w0602() {
    let mut t = T::new();
    t.rows = MAX_TABLE_ROWS.to_string();
    t.check().expect("exactly the cap");
    t.rows = (MAX_TABLE_ROWS + 1).to_string();
    let e = t.check_err();
    assert_eq!(e.code, DiagnosticCode::W0602);
    assert!(matches!(
        e.kind,
        CheckerErrorKind::TableRows { rows, limit } if rows == MAX_TABLE_ROWS + 1 && limit == MAX_TABLE_ROWS
    ));
    // 250,000 over the whole document: three tables of 100,000 are one too many.
    let table = |n: &str, rows: u64| {
        format!(
            "  vectors {n}:\n    source \"{n}.tfv\"\n    sha256 b\"{PIN}\"\n    rows {rows}\n    column msg: Bytes\n    holds digest(msg) == digest(msg)\n"
        )
    };
    let head = "warrant t v1\n  op digest(m: Bytes) -> Bytes<4>\n  labels digest:\n    m public\n";
    let ok = format!("{head}{}{}{}", table("a", 100_000), table("b", 100_000), table("c", 50_000));
    check(&ok).expect("250,000 rows");
    let over = format!("{head}{}{}{}", table("a", 100_000), table("b", 100_000), table("c", 50_001));
    let e = check(&over).expect_err("250,001 rows");
    assert_eq!(e.code, DiagnosticCode::W0602);
    assert!(matches!(e.kind, CheckerErrorKind::TableRows { limit, .. } if limit == MAX_DOCUMENT_ROWS));
    // A sum that would overflow a u64 saturates and is refused, not wrapped.
    let huge = format!("{head}{}{}", table("a", u64::MAX), table("b", u64::MAX));
    assert_eq!(check(&huge).expect_err("saturates").code, DiagnosticCode::W0602);
}

#[test]
fn an_empty_or_vacuous_table_is_w0601() {
    let mut t = T::new();
    t.rows = "0".into();
    assert_eq!(t.check_err().code, DiagnosticCode::W0601);
    // `holds` that never touches the subject.
    let mut t = T::new();
    t.holds = "holds len(msg) == len(md)";
    let e = t.check_err();
    assert_eq!(e.code, DiagnosticCode::W0601);
    assert!(matches!(e.kind, CheckerErrorKind::VacuousTable(_)));
    // ... a model that calls no operation does not help either.
    let src = T::new()
        .text()
        .replace("  vectors", "  model same(a: Bytes) -> Bytes = a\n  vectors")
        .replace("digest(msg) == md", "same(msg) == same(msg) and len(md) == 4");
    assert_eq!(check(&src).expect_err("vacuous").code, DiagnosticCode::W0601);
}

#[test]
fn holds_is_a_bool_over_exactly_the_columns() {
    // Not a Bool.
    let mut t = T::new();
    t.holds = "holds digest(msg)";
    assert_eq!(t.check_err().code, DiagnosticCode::W0301);
    // A name that is not a column.
    t.holds = "holds digest(other) == md";
    assert_eq!(t.check_err().code, DiagnosticCode::W0202);
    // A column the expression never mentions.
    t.holds = "holds digest(msg) == digest(msg)";
    let e = t.check_err();
    assert_eq!(e.code, DiagnosticCode::W0710);
    assert!(matches!(&e.kind, CheckerErrorKind::UnusedColumn(c) if c == "md"));
    // Type errors inside `holds` are ordinary type errors.
    t.holds = "holds digest(true) == md";
    assert_eq!(t.check_err().code, DiagnosticCode::W0301);
    // Columns shadow nothing: a column named like the operation, a built-in...
    for name in ["digest", "len", "Some", "t1x"] {
        let mut t = T::new();
        t.columns = vec![
            Box::leak(format!("column {name}: Bytes").into_boxed_str()),
            "column md: Bytes<4>",
        ];
        t.holds = Box::leak(format!("holds digest({name}) == md").into_boxed_str());
        if name == "t1x" {
            t.check().expect("a fresh name is fine");
        } else {
            assert_eq!(t.check_err().code, DiagnosticCode::W0203, "{name}");
        }
    }
}

#[test]
fn a_column_with_a_symbolic_width_has_no_decoding() {
    let mut t = T::new();
    t.columns = vec!["column msg: Bytes", "column md: Bytes<n>"];
    let e = t.check_err();
    assert_eq!(e.code, DiagnosticCode::W0710);
    assert!(matches!(&e.kind, CheckerErrorKind::UndecodableColumn(c) if c == "md"));
    // ... also nested.
    t.columns = vec!["column msg: Bytes", "column md: List<Option<Bytes<n>>>"];
    assert_eq!(t.check_err().code, DiagnosticCode::W0710);
    t.columns = vec!["column msg: Bytes", "column md: (Bool, Result<Bytes<n>, E>)"];
    assert_eq!(t.check_err().code, DiagnosticCode::W0710);
}

#[test]
fn columns_may_be_any_decodable_type() {
    let src = "warrant t v1\n  error E = BAD | WORSE\n  record R(a: U8, b: Bytes)\n  variant S = A(x: U16) | B\n  op f(a: U8, r: R) -> (Bool, Result<Bytes, E>)\n  labels f:\n    a public\n    r public\n  vectors t1:\n    source \"x.tfv\"\n    sha256 b\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\"\n    rows 2\n    column a: U8\n    column r: R\n    column s: S\n    column o: Option<I16>\n    column l: List<Bool>\n    column e: (Bool, Result<Bytes, E>)\n    column w: Bytes<4>\n    holds f(a, r) == e and s == s and o == o and l == l and w == w\n";
    check(src).expect("every shape of 4.9 is a column type");
}
