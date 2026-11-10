//! VER04j section 5.2/5.7 and 14.3: the table loader, fail closed.
//!
//! Every test builds a throw-away fixtures root in the temp directory, writes the
//! files it needs, and loads a document's tables from it:
//!
//! * the pin is compared before the file is parsed (a garbage file with the wrong
//!   digest is a digest error, never a parse error);
//! * sizes, row counts and symlinks are refused, each with a regression test for
//!   the cap it enforces;
//! * every row is decoded against its column types (W0710), a map with more than
//!   one entry and a value nested beyond 64 are W0709;
//! * the generator refuses what the loader did not verify, and a table that no
//!   fault could fail (W0601).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use coding_adventures_sha256::sha256_hex;
use coding_adventures_warrant_grammar::binding::parse_evidence_bindings;
use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::codegen::{CodegenError, DiagnosticCode};
use coding_adventures_warrant_grammar::evidence::{
    compile_rust_evidence_harness_with_tables, generate_evidence_harness, load_document_tables,
    EvidenceOptions, TableSet,
};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits, WarrantFile};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};
use coding_adventures_warrant_grammar::vectors::{
    check_value, read_fixture, Shape, MAX_TABLE_FILE_BYTES,
};
use coding_adventures_test_bootstrap::tfv::{parse_file, Value};

/// A temporary fixtures root that removes itself.
struct Root(PathBuf);

impl Root {
    fn new() -> Root {
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "wxe-loader-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        Root(dir)
    }
    fn write(&self, rel: &str, bytes: &[u8]) {
        let path = self.0.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A document with one operation and one table over `bytes` (pinned correctly
/// unless `pin` overrides it).
fn doc(source: &str, pin: &str, rows: u64, columns: &str, holds: &str) -> String {
    format!(
        "warrant t v1\n  error E = X | Y\n  record R(a: U8, b: Bytes)\n  variant S = A(x: U16) | B\n  op f(m: Bytes) -> Bytes<4>\n  labels f:\n    m public\n  vectors t1:\n    source \"{source}\"\n    sha256 b\"{pin}\"\n    rows {rows}\n{columns}    holds {holds}\n"
    )
}

const COLS: &str = "    column msg: Bytes\n    column md: Bytes<4>\n";
const HOLDS: &str = "f(msg) == md";

fn parse<R>(src: &str, f: impl FnOnce(&WarrantFile) -> R) -> R {
    let d = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&d, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &d, ParserLimits::default()).expect("parse");
    f(&file)
}

fn load(root: &Root, src: &str) -> Result<usize, CodegenError> {
    parse(src, |file| {
        load_document_tables(file, root.path()).map(|t| t.iter().map(|x| x.rows.len()).sum())
    })
}

/// Write `bytes` as `t.tfv` and load the one-table document that pins it.
fn load_bytes(bytes: &[u8], rows: u64) -> Result<usize, CodegenError> {
    let root = Root::new();
    root.write("t.tfv", bytes);
    load(&root, &doc("t.tfv", &sha256_hex(bytes), rows, COLS, HOLDS))
}

fn good() -> String {
    "tfv 1\nr1 R2 Kmsg X00 Kmd X01020304\nr2 R2 Kmsg X Kmd Xffffffff\n".to_string()
}

fn code(r: Result<usize, CodegenError>) -> DiagnosticCode {
    r.expect_err("must be refused").code
}

// ---------------------------------------------------------------------------
// The happy path
// ---------------------------------------------------------------------------

#[test]
fn a_good_table_loads_with_its_bytes_digest_and_rows() {
    let root = Root::new();
    let text = good();
    root.write("dir/t.tfv", text.as_bytes());
    let src = doc("dir/t.tfv", &sha256_hex(text.as_bytes()), 2, COLS, HOLDS);
    parse(&src, |file| {
        let tables = load_document_tables(file, root.path()).expect("loads");
        assert_eq!(tables.len(), 1);
        let t = &tables[0];
        assert_eq!(t.name, "t1");
        assert_eq!(t.source, "dir/t.tfv");
        assert_eq!(t.bytes, text.as_bytes());
        assert_eq!(t.sha256, sha256_hex(text.as_bytes()));
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.columns, ["msg", "md"]);
        let set = TableSet {
            dir: "tests/generated/tables",
            tables: &tables,
        };
        assert_eq!(set.files(), vec![("tests/generated/tables/t1.tfv".to_string(), text.as_bytes())]);
    });
}

#[test]
fn comments_and_blank_lines_are_not_rows() {
    let text = "tfv 1\n# a comment\n\nr1 R2 Kmsg X00 Kmd X01020304\n\n# another\n";
    assert_eq!(load_bytes(text.as_bytes(), 1).unwrap(), 1);
}

// ---------------------------------------------------------------------------
// The pin is checked before anything is parsed
// ---------------------------------------------------------------------------

#[test]
fn a_flipped_byte_is_w0708_and_a_garbage_file_is_a_digest_error_not_a_parse_error() {
    let root = Root::new();
    let text = good();
    root.write("t.tfv", text.as_bytes());
    let mut bad = text.clone().into_bytes();
    bad[10] ^= 1;
    root.write("t.tfv", &bad);
    let e = load(&root, &doc("t.tfv", &sha256_hex(text.as_bytes()), 2, COLS, HOLDS)).unwrap_err();
    assert_eq!(e.code, DiagnosticCode::W0708);
    assert!(format!("{e}").contains("does not match the pin"), "{e}");
    // A file that is not TFV at all, with the wrong pin: still a pin error,
    // because nothing in the file is interpreted before the digest matches.
    root.write("t.tfv", b"\x00\xff not tfv at all \x01");
    let e = load(&root, &doc("t.tfv", &"0".repeat(64), 2, COLS, HOLDS)).unwrap_err();
    assert_eq!(e.code, DiagnosticCode::W0708);
    assert!(format!("{e}").contains("does not match the pin"), "{e}");
    // The same garbage with the *right* pin is a parse error (also W0708).
    let garbage = b"not tfv at all\n";
    root.write("t.tfv", garbage);
    let e = load(&root, &doc("t.tfv", &sha256_hex(garbage), 1, COLS, HOLDS)).unwrap_err();
    assert_eq!(e.code, DiagnosticCode::W0708);
    assert!(format!("{e}").contains("malformed TFV") || format!("{e}").contains("records"), "{e}");
}

#[test]
fn the_pin_must_be_the_files_own_digest_not_merely_well_formed() {
    let text = good();
    let root = Root::new();
    root.write("t.tfv", text.as_bytes());
    let upper = sha256_hex(text.as_bytes());
    // A digest of different data, however well formed, is refused.
    let e = load(&root, &doc("t.tfv", &sha256_hex(b"something else"), 2, COLS, HOLDS)).unwrap_err();
    assert_eq!(e.code, DiagnosticCode::W0708);
    assert_eq!(load(&root, &doc("t.tfv", &upper, 2, COLS, HOLDS)).unwrap(), 2);
}

// ---------------------------------------------------------------------------
// Row counts, malformed TFV
// ---------------------------------------------------------------------------

#[test]
fn a_row_count_off_by_one_in_either_direction_is_w0708() {
    let text = good();
    for rows in [1, 3] {
        assert_eq!(code(load_bytes(text.as_bytes(), rows)), DiagnosticCode::W0708, "{rows}");
    }
}

#[test]
fn a_row_count_mismatch_is_found_before_the_rows_are_parsed() {
    // 50,000 hostile lines against `rows 1`: refused by the line count, which is
    // linear and allocation-free, before the TFV parser builds a record.
    let mut text = String::from("tfv 1\n");
    for i in 0..50_000 {
        text.push_str(&format!("r{i} R2 Kmsg X Kmd X00000000\n"));
    }
    let started = std::time::Instant::now();
    assert_eq!(code(load_bytes(text.as_bytes(), 1)), DiagnosticCode::W0708);
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
}

#[test]
fn malformed_tfv_odd_hex_duplicate_ids_and_a_missing_header_are_w0708() {
    for (what, text) in [
        ("odd hex", "tfv 1\nr1 R2 Kmsg X0 Kmd X01020304\n".to_string()),
        ("uppercase hex", "tfv 1\nr1 R2 Kmsg X0A Kmd X01020304\n".to_string()),
        (
            "duplicate id",
            "tfv 1\nr1 R2 Kmsg X Kmd X01020304\nr1 R2 Kmsg X Kmd X01020304\n".to_string(),
        ),
        ("no header", "r1 R2 Kmsg X Kmd X01020304\n".to_string()),
        ("bad header", "tfv 2\nr1 R2 Kmsg X Kmd X01020304\n".to_string()),
        ("trailing token", "tfv 1\nr1 R2 Kmsg X Kmd X01020304 x\n".to_string()),
        ("count lies", "tfv 1\nr1 R3 Kmsg X Kmd X01020304\n".to_string()),
        ("double space", "tfv 1\nr1  R2 Kmsg X Kmd X01020304\n".to_string()),
        ("carriage return", "tfv 1\r\nr1 R2 Kmsg X Kmd X01020304\r\n".to_string()),
    ] {
        let rows = text.lines().filter(|l| l.starts_with('r')).count().max(1) as u64;
        assert_eq!(code(load_bytes(text.as_bytes(), rows)), DiagnosticCode::W0708, "{what}");
    }
    // Not UTF-8.
    assert_eq!(code(load_bytes(&[0xff, 0xfe, 0x00], 1)), DiagnosticCode::W0708);
}

// ---------------------------------------------------------------------------
// Column checks: W0710
// ---------------------------------------------------------------------------

#[test]
fn a_missing_extra_or_misnamed_field_and_an_out_of_range_value_are_w0710() {
    let root = Root::new();
    let cols = "    column a: U8\n    column b: Bytes<2>\n";
    let holds = "f(b) == f(b) and a == a";
    for (what, text) in [
        ("missing field", "tfv 1\nr1 R1 Ka I1\n"),
        ("extra field", "tfv 1\nr1 R3 Ka I1 Kb X0102 Kc I1\n"),
        ("misnamed", "tfv 1\nr1 R2 Ka I1 Kc X0102\n"),
        ("U8 out of range (I256)", "tfv 1\nr1 R2 Ka I256 Kb X0102\n"),
        ("U8 negative", "tfv 1\nr1 R2 Ka I-1 Kb X0102\n"),
        ("wrong kind", "tfv 1\nr1 R2 Ka T Kb X0102\n"),
        ("Bytes<2> too short", "tfv 1\nr1 R2 Ka I1 Kb X01\n"),
        ("Bytes<2> too long", "tfv 1\nr1 R2 Ka I1 Kb X010203\n"),
    ] {
        root.write("t.tfv", text.as_bytes());
        let r = load(&root, &doc("t.tfv", &sha256_hex(text.as_bytes()), 1, cols, holds));
        assert_eq!(r.expect_err(what).code, DiagnosticCode::W0710, "{what}");
    }
    // The in-range edges are fine.
    for ok in ["tfv 1\nr1 R2 Ka I0 Kb X0102\n", "tfv 1\nr1 R2 Ka I255 Kb X0102\n"] {
        root.write("t.tfv", ok.as_bytes());
        load(&root, &doc("t.tfv", &sha256_hex(ok.as_bytes()), 1, cols, holds)).expect(ok);
    }
}

#[test]
fn every_value_shape_of_section_4_9_has_a_checker() {
    let parse_value = |text: &str| -> Value {
        parse_file(&format!("tfv 1\nr R1 Kv {text}\n")).unwrap().remove(0).fields.remove(0).1
    };
    let u8s = Shape::fixed(coding_adventures_warrant_grammar::fixed::FixedTy::U8);
    let ok = |text: &str, shape: &Shape| check_value(&parse_value(text), shape, 0);
    // Bool, Nat, Int
    assert!(ok("T", &Shape::Bool).is_ok());
    assert!(ok("I1", &Shape::Bool).is_err());
    assert!(ok("I0", &Shape::nat()).is_ok());
    assert!(ok("I-1", &Shape::nat()).is_err());
    assert!(ok("I-1", &Shape::int()).is_ok());
    assert!(ok("I99999999999999999999999999999999999999999999", &Shape::int()).is_err());
    // Option: L0 | L1 x
    let opt = Shape::Opt(Box::new(u8s.clone()));
    assert!(ok("L0", &opt).is_ok() && ok("L1 I5", &opt).is_ok());
    assert!(ok("L1 I500", &opt).is_err() && ok("L2 I1 I2", &opt).is_err() && ok("I5", &opt).is_err());
    // List and tuple/record
    let list = Shape::List(Box::new(u8s.clone()));
    assert!(ok("L3 I1 I2 I3", &list).is_ok() && ok("L2 I1 I300", &list).is_err());
    let tup = Shape::Tuple(vec![u8s.clone(), Shape::Bytes]);
    assert!(ok("L2 I1 X00", &tup).is_ok());
    assert!(ok("L1 I1", &tup).is_err() && ok("L2 X00 I1", &tup).is_err() && ok("T", &tup).is_err());
    // Variant: L2 S<ctor> L<k> fields, by name
    let var = Shape::Variant(vec![("A".into(), vec![u8s.clone()]), ("B".into(), vec![])]);
    assert!(ok("L2 S41 L1 I3", &var).is_ok());
    assert!(ok(&format!("L2 S{} L0", "42"), &var).is_ok());
    assert!(ok(&format!("L2 S{} L0", "43"), &var).is_err(), "unknown constructor");
    assert!(ok(&format!("L2 S{} L0", "41"), &var).is_err(), "A takes a field");
    assert!(ok("L1 I1", &var).is_err());
    // Result: M1 S"ok" v | M1 S"err" S<CASE>
    let res = Shape::Result(Box::new(u8s), vec!["X".into(), "Y".into()]);
    assert!(ok("M1 S6f6b I7", &res).is_ok());
    assert!(ok("M1 S657272 S58", &res).is_ok());
    assert!(ok("M1 S657272 S5a", &res).is_err(), "unknown case");
    assert!(ok("M1 S6f6b I700", &res).is_err());
    assert!(ok("M1 S6e6f I1", &res).is_err(), "neither ok nor err");
    assert!(ok("M0", &res).is_err());
    assert!(ok("I1", &res).is_err());
    // Bytes<N>
    assert!(ok("X0102", &Shape::BytesN(2)).is_ok() && ok("X01", &Shape::BytesN(2)).is_err());
    assert!(ok("X", &Shape::BytesN(0)).is_ok());
    assert!(ok("I1", &Shape::Bytes).is_err() && ok("S6869", &Shape::Bytes).is_err());
}

#[test]
fn values_nested_beyond_the_limit_are_refused_by_the_checker_itself() {
    // check_value caps its own recursion (the parser has its own cap): a hand
    // built value deeper than 64 containers is an error, not a stack overflow.
    let mut v = Value::Bool(true);
    let mut shape = Shape::Bool;
    for _ in 0..200 {
        v = Value::List(vec![v]);
        shape = Shape::Opt(Box::new(shape));
    }
    assert!(check_value(&v, &shape, 0).is_err());
}

// ---------------------------------------------------------------------------
// Shapes TFV allows but no Warrant type has: W0709
// ---------------------------------------------------------------------------

#[test]
fn a_map_with_more_than_one_entry_is_w0709_without_the_quadratic_key_check() {
    // 10,000 two-entry maps on one line would make the TFV parser's pairwise key
    // check quadratic; they are refused before it runs.
    let hostile = format!(
        "tfv 1\nr1 R2 Kmsg X Kmd L{n} {}\n",
        "M2 S61 T S62 T ".repeat(10_000),
        n = 10_000
    );
    let started = std::time::Instant::now();
    assert_eq!(code(load_bytes(hostile.as_bytes(), 1)), DiagnosticCode::W0709);
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
    // A one-entry map is the Result shape and is not refused by the scan.
    let ok = "tfv 1\nr1 R2 Ka I1 Kb M1 S6f6b I7\n";
    let root = Root::new();
    root.write("t.tfv", ok.as_bytes());
    let cols = "    column a: U8\n    column b: Result<U8, E>\n";
    load(&root, &doc("t.tfv", &sha256_hex(ok.as_bytes()), 1, cols, "f(b\"\") == f(b\"\") and a == a and b == b")).unwrap();
}

#[test]
fn nesting_beyond_64_containers_is_w0709() {
    let deep = format!("{}X{}", "L1 ".repeat(65), "");
    let text = format!("tfv 1\nr1 R1 Kmsg {deep}\n");
    let root = Root::new();
    root.write("t.tfv", text.as_bytes());
    let cols = "    column msg: Bytes\n";
    let r = load(&root, &doc("t.tfv", &sha256_hex(text.as_bytes()), 1, cols, "f(msg) == f(msg)"));
    assert_eq!(r.expect_err("too deep").code, DiagnosticCode::W0709);
}

// ---------------------------------------------------------------------------
// Files: size, symlinks, paths
// ---------------------------------------------------------------------------

#[test]
fn a_missing_file_a_directory_and_a_bad_path_are_w0708() {
    let root = Root::new();
    let missing = doc("nope.tfv", &"0".repeat(64), 1, COLS, HOLDS);
    assert_eq!(load(&root, &missing).unwrap_err().code, DiagnosticCode::W0708);
    fs::create_dir(root.path().join("adir")).unwrap();
    let dir = doc("adir", &"0".repeat(64), 1, COLS, HOLDS);
    assert_eq!(load(&root, &dir).unwrap_err().code, DiagnosticCode::W0708);
    // A file in the middle of the path is not a directory.
    root.write("f.tfv", b"x");
    let through = doc("f.tfv/x", &"0".repeat(64), 1, COLS, HOLDS);
    assert_eq!(load(&root, &through).unwrap_err().code, DiagnosticCode::W0708);
    // The fixtures root itself must exist.
    let gone = Root::new();
    let path = gone.path().to_path_buf();
    drop(gone);
    let e = parse(&doc("t.tfv", &"0".repeat(64), 1, COLS, HOLDS), |file| {
        load_document_tables(file, &path).unwrap_err()
    });
    assert_eq!(e.code, DiagnosticCode::W0708);
}

#[test]
fn the_size_cap_is_16_mib_and_is_checked_before_the_file_is_read() {
    assert_eq!(MAX_TABLE_FILE_BYTES, 16 * 1024 * 1024);
    let root = Root::new();
    // A sparse 17 MiB file: refused from its metadata, never read (the pin is
    // the digest of nothing the loader will compute).
    let path = root.path().join("big.tfv");
    let f = fs::File::create(&path).unwrap();
    f.set_len(17 * 1024 * 1024).unwrap();
    drop(f);
    let e = load(&root, &doc("big.tfv", &"0".repeat(64), 1, COLS, HOLDS)).unwrap_err();
    assert_eq!(e.code, DiagnosticCode::W0708);
    assert!(format!("{e}").contains("larger than"), "{e}");
    // The boundary, with a small cap so the test stays cheap: exactly the cap
    // reads, one byte more does not.
    root.write("a.bin", &[7u8; 10]);
    root.write("b.bin", &[7u8; 11]);
    assert_eq!(read_fixture(root.path(), "a.bin", 10).unwrap().len(), 10);
    let e = read_fixture(root.path(), "b.bin", 10).unwrap_err();
    assert!(e.contains("larger than 10 bytes"), "{e}");
    // A file that *grows* after its metadata was read is still bounded by the
    // `take(max + 1)`: simulated by a zero cap.
    assert!(read_fixture(root.path(), "a.bin", 0).is_err());
}

#[test]
fn paths_that_escape_the_fixtures_root_are_refused() {
    let root = Root::new();
    root.write("ok.bin", b"x");
    for bad in ["../x", "a/../ok.bin", "/etc/passwd", "a//b", "./ok.bin", "ok.bin/", "a\\b", ""] {
        let e = read_fixture(root.path(), bad, 100).unwrap_err();
        assert!(e.contains("path rules"), "{bad:?}: {e}");
    }
    assert_eq!(read_fixture(root.path(), "ok.bin", 100).unwrap(), b"x");
}

#[cfg(unix)]
#[test]
fn a_symlink_anywhere_below_the_root_is_refused() {
    use std::os::unix::fs::symlink;
    let root = Root::new();
    let outside = Root::new();
    outside.write("secret.tfv", good().as_bytes());
    root.write("real.tfv", good().as_bytes());
    fs::create_dir(root.path().join("d")).unwrap();
    root.write("d/inner.tfv", good().as_bytes());
    // A symlinked file, to a file inside and a file outside the root.
    symlink(root.path().join("real.tfv"), root.path().join("link-in.tfv")).unwrap();
    symlink(outside.path().join("secret.tfv"), root.path().join("link-out.tfv")).unwrap();
    // A symlinked directory component.
    symlink(outside.path(), root.path().join("dirlink")).unwrap();
    symlink(root.path().join("d"), root.path().join("dirlink-in")).unwrap();
    // A symlink loop.
    symlink(root.path().join("loop-b"), root.path().join("loop-a")).unwrap();
    symlink(root.path().join("loop-a"), root.path().join("loop-b")).unwrap();
    for bad in ["link-in.tfv", "link-out.tfv", "dirlink/secret.tfv", "dirlink-in/inner.tfv", "loop-a"] {
        let e = read_fixture(root.path(), bad, 1 << 20).unwrap_err();
        assert!(e.contains("symlink") || e.contains("does not exist"), "{bad}: {e}");
        let src = doc(bad, &sha256_hex(good().as_bytes()), 2, COLS, HOLDS);
        assert_eq!(load(&root, &src).unwrap_err().code, DiagnosticCode::W0708, "{bad}");
    }
    // The real files still load.
    assert!(read_fixture(root.path(), "real.tfv", 1 << 20).is_ok());
    assert!(read_fixture(root.path(), "d/inner.tfv", 1 << 20).is_ok());
}

#[test]
fn a_document_may_not_exceed_the_row_budget_even_before_a_file_is_read() {
    let root = Root::new();
    let e = load(&root, &doc("t.tfv", &"0".repeat(64), 100_001, COLS, HOLDS)).unwrap_err();
    assert_eq!(e.code, DiagnosticCode::W0602);
    let two = format!(
        "{}  vectors t2:\n    source \"u.tfv\"\n    sha256 b\"{}\"\n    rows 100000\n{COLS}    holds {HOLDS}\n  vectors t3:\n    source \"v.tfv\"\n    sha256 b\"{}\"\n    rows 100000\n{COLS}    holds {HOLDS}\n",
        doc("t.tfv", &"0".repeat(64), 100_000, COLS, HOLDS),
        "0".repeat(64),
        "0".repeat(64)
    );
    let e = load(&root, &two).unwrap_err();
    assert_eq!(e.code, DiagnosticCode::W0602);
}

// ---------------------------------------------------------------------------
// The generator's side: only verified tables, and discrimination
// ---------------------------------------------------------------------------

fn bindings() -> coding_adventures_warrant_grammar::binding::EvidenceBindingFile {
    parse_evidence_bindings(
        "package subject\nrevision 0\nspec code/specs/t.warrant\noutput code/packages/rust/x/tests/generated/t.rs\nseed 1\nproperty_cases 10\nfuzz_iterations 10\nnegctl_fuzz_iterations 10\nexhaustive_budget 400000\nop f export=f faults=flip-result-bit\n",
    )
    .unwrap()
}

fn options() -> EvidenceOptions {
    let b = bindings();
    EvidenceOptions {
        package_crate: "subject".into(),
        spec_path: "code/specs/t.warrant".into(),
        spec_sha256: "0".repeat(64),
        seed: 1,
        property_cases: 10,
        fuzz_iterations: 10,
        negctl_fuzz_iterations: 10,
        exhaustive_budget: 400_000,
        arg_types: b.arg_types.clone(),
        hosts: b.hosts.clone(),
    }
}

fn gen(src: &str, root: &Root, dir: &str) -> Result<String, CodegenError> {
    let d = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).unwrap();
    let layout = capture_layout(&d, LayoutLimits::default()).unwrap();
    let file = parse_document(&layout, &d, ParserLimits::default()).unwrap();
    let checked = check_warrant(&file).expect("type checks");
    let tables = load_document_tables(&file, root.path())?;
    let set = TableSet { dir, tables: &tables };
    compile_rust_evidence_harness_with_tables(&file, &checked, &bindings().manifest, &options(), &set)
}

fn claimed(src: &str) -> String {
    format!("{src}  claim c:\n    forall m: Bytes. f(m) == f(m)\n")
}

#[test]
fn the_generated_harness_embeds_the_copy_pins_it_and_hashes_it() {
    let root = Root::new();
    let text = good();
    root.write("t.tfv", text.as_bytes());
    let src = claimed(&doc("t.tfv", &sha256_hex(text.as_bytes()), 2, COLS, HOLDS));
    let out = gen(&src, &root, "tests/generated/tables").expect("generates");
    assert!(out.contains("include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/tests/generated/tables/t1.tfv\"))"));
    assert!(out.contains(&format!("const TABLE_T1_SHA256: &str = \"{}\";", sha256_hex(text.as_bytes()))));
    assert!(out.contains("const TABLE_T1_ROWS: usize = 2;"));
    assert!(out.contains("pub fn tab_t1(fault: Fault)"));
    assert!(out.contains("warrant/t/table/t1"));
    assert!(out.contains("fn neg_tab_t1(fault: Fault)"));
    assert!(out.contains("negctl/table/f/flip-result-bit"));
    // The table name never reaches generated code except as an identifier part
    // and a quoted id; both are NAME characters.
    assert!(!out.contains("//@"));
}

#[test]
fn the_generator_refuses_tables_the_loader_did_not_verify() {
    let root = Root::new();
    let text = good();
    root.write("t.tfv", text.as_bytes());
    let src = claimed(&doc("t.tfv", &sha256_hex(text.as_bytes()), 2, COLS, HOLDS));
    let d = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).unwrap();
    let layout = capture_layout(&d, LayoutLimits::default()).unwrap();
    let file = parse_document(&layout, &d, ParserLimits::default()).unwrap();
    // No tables supplied at all.
    let e = generate_evidence_harness(&file, &bindings().manifest, &options()).unwrap_err();
    assert_eq!(e.code, DiagnosticCode::W0708);
    assert!(format!("{e}").contains("was not loaded"), "{e}");
    // A loaded table that is not the one the document pins.
    let mut tables = load_document_tables(&file, root.path()).unwrap();
    tables[0].sha256 = "1".repeat(64);
    let set = TableSet {
        dir: "tests/generated/tables",
        tables: &tables,
    };
    let checked = check_warrant(&file).unwrap();
    let e = compile_rust_evidence_harness_with_tables(&file, &checked, &bindings().manifest, &options(), &set)
        .unwrap_err();
    assert_eq!(e.code, DiagnosticCode::W0708);
    // A hostile directory for the copies (it ends up in a string literal).
    for dir in ["../x", "/abs", "a\"b", "a b", "a\\b", ""] {
        let r = gen(&src, &root, dir);
        assert_eq!(r.unwrap_err().code, DiagnosticCode::W0701, "{dir:?}");
    }
}

#[test]
fn a_v0_document_cannot_carry_a_table_even_if_built_by_hand() {
    // The parser already refuses `vectors` in v0 (W0101); the generator checks again.
    let src = "warrant t\n  op f(m: Bytes) -> Bytes<4>\n  labels f:\n    m public\n  claim c:\n    forall m: Bytes. f(m) == f(m)\n";
    let d = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).unwrap();
    let layout = capture_layout(&d, LayoutLimits::default()).unwrap();
    let file = parse_document(&layout, &d, ParserLimits::default()).unwrap();
    assert!(generate_evidence_harness(&file, &bindings().manifest, &options()).is_err());
}

#[test]
fn a_table_whose_expected_column_is_constant_is_not_discriminating_w0601() {
    let root = Root::new();
    // Every row expects the same digest: a constant function passes it.
    let text = "tfv 1\nr1 R2 Kmsg X00 Kmd X01020304\nr2 R2 Kmsg X01 Kmd X01020304\nr3 R2 Kmsg X02 Kmd X01020304\n";
    root.write("t.tfv", text.as_bytes());
    let src = claimed(&doc("t.tfv", &sha256_hex(text.as_bytes()), 3, COLS, HOLDS));
    let e = gen(&src, &root, "tests/generated/tables").unwrap_err();
    assert_eq!(e.code, DiagnosticCode::W0601);
    assert!(format!("{e}").contains("not discriminating"), "{e}");
    // The mirrored spelling is caught too.
    let mirrored = claimed(&doc("t.tfv", &sha256_hex(text.as_bytes()), 3, COLS, "md == f(msg)"));
    assert_eq!(gen(&mirrored, &root, "tests/generated/tables").unwrap_err().code, DiagnosticCode::W0601);
    // One varying row is enough; and so is a table of a single row.
    let varied = "tfv 1\nr1 R2 Kmsg X00 Kmd X01020304\nr2 R2 Kmsg X01 Kmd X01020305\n";
    root.write("t.tfv", varied.as_bytes());
    gen(&claimed(&doc("t.tfv", &sha256_hex(varied.as_bytes()), 2, COLS, HOLDS)), &root, "tests/generated/tables").unwrap();
    let one = "tfv 1\nr1 R2 Kmsg X00 Kmd X01020304\n";
    root.write("t.tfv", one.as_bytes());
    gen(&claimed(&doc("t.tfv", &sha256_hex(one.as_bytes()), 1, COLS, HOLDS)), &root, "tests/generated/tables").unwrap();
}

#[test]
fn tables_whose_names_collide_when_upper_cased_are_refused() {
    let root = Root::new();
    let text = good();
    root.write("t.tfv", text.as_bytes());
    let pin = sha256_hex(text.as_bytes());
    let one = doc("t.tfv", &pin, 2, COLS, HOLDS).replace("vectors t1:", "vectors ab:");
    let second = format!(
        "  vectors AB:\n    source \"t.tfv\"\n    sha256 b\"{pin}\"\n    rows 2\n{COLS}    holds {HOLDS}\n"
    );
    let src = claimed(&format!("{one}{second}"));
    assert_eq!(gen(&src, &root, "tests/generated/tables").unwrap_err().code, DiagnosticCode::W0701);
}

#[test]
fn composite_columns_generate_with_the_composite_prelude() {
    // A record, a variant, an Option, a List and a Result column all decode.
    let root = Root::new();
    let text = "tfv 1\nr1 R5 Kr L2 I1 X00 Ks L2 S41 L1 I9 Ko L1 I3 Kl L2 T F Ke M1 S657272 S58\n";
    root.write("t.tfv", text.as_bytes());
    let cols = "    column r: R\n    column s: S\n    column o: Option<U8>\n    column l: List<Bool>\n    column e: Result<U8, E>\n";
    let holds = "f(b\"\") == f(b\"\") and r == r and s == s and o == o and l == l and e == e";
    let src = claimed(&doc("t.tfv", &sha256_hex(text.as_bytes()), 1, cols, holds));
    let out = gen(&src, &root, "tests/generated/tables").expect("generates");
    assert!(out.contains("Ty::Data(\"R\""), "{out}");
    assert!(out.contains("Ty::Res("));
    assert!(out.contains("(Ty::Res(t, cases), Value::Map(pairs))"), "the composite decoder is emitted");
}

// ---------------------------------------------------------------------------
// Review fixes: record ids, FIFOs, memory caps, write-side symlink guard
// ---------------------------------------------------------------------------

#[test]
fn record_ids_that_start_with_m_are_not_map_headers() {
    let root = Root::new();
    let text = "tfv 1\nMax-case R2 Kmsg X Kmd X00000000\nM5 R2 Kmsg X01 Kmd X00000000\nM R2 Kmsg X02 Kmd X00000000\n";
    root.write("t.tfv", text.as_bytes());
    assert_eq!(load(&root, &doc("t.tfv", &sha256_hex(text.as_bytes()), 3, COLS, HOLDS)).unwrap(), 3);
    // A real big map after such an id is still refused.
    let hostile = "tfv 1\nM5 R2 Kmsg X Kmd M2 S61 T S62 T\n";
    assert_eq!(code(load_bytes(hostile.as_bytes(), 1)), DiagnosticCode::W0709);
}

#[test]
fn a_record_with_too_many_tokens_is_refused_before_parsing() {
    use coding_adventures_warrant_grammar::vectors::MAX_ROW_TOKENS;
    let line = format!("r1 R1 Kmsg L{} {}", MAX_ROW_TOKENS, "I1 ".repeat(MAX_ROW_TOKENS));
    let text = format!("tfv 1\n{}\n", line.trim_end());
    assert_eq!(code(load_bytes(text.as_bytes(), 1)), DiagnosticCode::W0602);
}

#[test]
fn table_count_and_aggregate_bytes_are_capped() {
    use coding_adventures_warrant_grammar::vectors::MAX_DOCUMENT_TABLES;
    let root = Root::new();
    let table = |n: usize, file: &str, pin: &str| {
        format!(
            "  vectors t{n}:\n    source \"{file}\"\n    sha256 b\"{pin}\"\n    rows 1\n    column msg: Bytes\n    holds f(msg) == f(msg)\n"
        )
    };
    let head = "warrant t v1\n  op f(m: Bytes) -> Bytes<4>\n  labels f:\n    m public\n";
    let many: String = (0..=MAX_DOCUMENT_TABLES).map(|i| table(i, "x.tfv", &"0".repeat(64))).collect();
    assert_eq!(load(&root, &format!("{head}{many}")).unwrap_err().code, DiagnosticCode::W0602);
    // Five tables over one ~14 MiB file: 70 MiB held together is over 64 MiB.
    let big = format!("tfv 1\nr1 R1 Kmsg X{}\n", "ab".repeat(7 * 1024 * 1024));
    root.write("big.tfv", big.as_bytes());
    let pin = sha256_hex(big.as_bytes());
    let five: String = (0..5).map(|i| table(i, "big.tfv", &pin)).collect();
    let e = load(&root, &format!("{head}{five}")).unwrap_err();
    assert_eq!(e.code, DiagnosticCode::W0602);
    assert!(format!("{e}").contains("together"), "{e}");
    let four: String = (0..4).map(|i| table(i, "big.tfv", &pin)).collect();
    assert_eq!(load(&root, &format!("{head}{four}")).unwrap(), 4);
}

#[cfg(unix)]
#[test]
fn a_fifo_is_refused_without_blocking() {
    let root = Root::new();
    let fifo = root.path().join("pipe.tfv");
    let made = std::process::Command::new("mkfifo").arg(&fifo).status().expect("mkfifo");
    assert!(made.success());
    let started = std::time::Instant::now();
    let e = read_fixture(root.path(), "pipe.tfv", 1 << 20).unwrap_err();
    assert!(e.contains("not a regular file"), "{e}");
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
}

#[cfg(unix)]
#[test]
fn the_write_side_guard_refuses_symlinks_at_every_level() {
    use coding_adventures_warrant_grammar::vectors::ensure_no_symlinks;
    use std::os::unix::fs::symlink;
    let root = Root::new();
    let other = Root::new();
    fs::create_dir_all(root.path().join("a/b")).unwrap();
    assert!(ensure_no_symlinks(root.path(), "a/b/new.tfv").is_ok());
    assert!(ensure_no_symlinks(root.path(), "not/yet/there.tfv").is_ok());
    symlink(other.path(), root.path().join("dir-link")).unwrap();
    symlink(other.path().join("x"), root.path().join("a/file-link")).unwrap();
    assert!(ensure_no_symlinks(root.path(), "dir-link/x.tfv").is_err());
    assert!(ensure_no_symlinks(root.path(), "a/file-link").is_err());
    assert!(ensure_no_symlinks(root.path(), "../escape").is_err());
}
