//! Edges of WX-E and WX-H that the larger suites do not reach: diagnostic text,
//! the name walker, the loader's defensive branches, the discrimination
//! predicate, and generator paths for tables over composite columns and over
//! observed operations.

use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::PathBuf;

use coding_adventures_sha256::sha256_hex;
use coding_adventures_test_bootstrap::tfv::parse_file;
use coding_adventures_warrant_grammar::binding::parse_evidence_bindings;
use coding_adventures_warrant_grammar::checker::{check_warrant, DiagnosticCode as CheckCode};
use coding_adventures_warrant_grammar::codegen::DiagnosticCode as GenCode;
use coding_adventures_warrant_grammar::evidence::{
    compile_rust_evidence_harness_with_tables, generate_evidence_harness, load_document_tables,
    EvidenceOptions, TableSet,
};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{
    parse_document, Declaration, Expression, ParserLimits, Version, VectorsDecl, WarrantFile,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};
use coding_adventures_warrant_grammar::vectors::{
    self, check_value, expected_column, fixtures_root, is_constant_output, load_table,
    LoadedTable, Shape, FIXTURES_ROOT,
};

fn parse<R>(src: &str, f: impl FnOnce(&WarrantFile) -> R) -> R {
    let d = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&d, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &d, ParserLimits::default()).expect("parse");
    f(&file)
}

struct Dir(PathBuf);
impl Dir {
    fn new(tag: &str) -> Dir {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("wxe-edges-{}-{tag}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Dir(p)
    }
}
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// ---------------------------------------------------------------------------
// Diagnostic text
// ---------------------------------------------------------------------------

#[test]
fn the_new_diagnostic_codes_print_their_ids() {
    for (code, text) in [(CheckCode::W0601, "W0601"), (CheckCode::W0710, "W0710")] {
        assert_eq!(code.to_string(), text);
    }
    for (code, text) in [
        (GenCode::W0601, "W0601"),
        (GenCode::W0602, "W0602"),
        (GenCode::W0708, "W0708"),
        (GenCode::W0710, "W0710"),
        (GenCode::W0709, "W0709"),
    ] {
        assert_eq!(code.to_string(), text);
    }
    // A bad pin prints W0708 and never echoes the source.
    let src = "warrant t v1\n  op f(m: Bytes) -> Bytes<4>\n  vectors a:\n    source \"x\"\n    sha256 b\"zz\"\n    rows 1\n    column m: Bytes\n    holds f(m) == f(m)\n";
    let d = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).unwrap();
    let layout = capture_layout(&d, LayoutLimits::default()).unwrap();
    let e = parse_document(&layout, &d, ParserLimits::default()).unwrap_err();
    let shown = e.to_string();
    assert!(shown.starts_with("W0708"), "{shown}");
    assert!(!shown.contains("zz"), "{shown}");
    // An unterminated pin is a syntax error, not an integrity error.
    let src = src.replace("b\"zz\"", "b\"01");
    let d = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).unwrap();
    let layout = capture_layout(&d, LayoutLimits::default()).unwrap();
    assert_eq!(parse_document(&layout, &d, ParserLimits::default()).unwrap_err().code(), "W0001");
}

// ---------------------------------------------------------------------------
// Expression::mentioned_names
// ---------------------------------------------------------------------------

#[test]
fn mentioned_names_walks_every_expression_form() {
    let src = "warrant t v1\n  record R(a: U8, b: U8)\n  model m(a: U8, b: U8, c: U8, d: U8) -> U8 = if a == b then c else d\n  example e:\n    (match (x, y) with (p, q) -> p + z | _ -> w) == (if t then (u, v).0 else m(k, k, k, k)) and (forall n: U8 | n == g. n == h) and R(r1, r2).a == s.a and -neg == not_ and (-5 == 3)\n";
    parse(src, |file| {
        let Some(Declaration::Example(ex)) = file
            .declarations
            .iter()
            .find(|d| matches!(d, Declaration::Example(_)))
        else {
            panic!("example")
        };
        let names: HashSet<&str> = [
            "x", "y", "z", "w", "t", "u", "v", "k", "g", "h", "r1", "r2", "s", "neg", "not_", "p", "q", "nothing",
        ]
        .into_iter()
        .collect();
        let mut seen = BTreeSet::new();
        ex.body.mentioned_names(&names, &mut seen);
        let want: BTreeSet<String> = [
            "x", "y", "z", "w", "t", "u", "v", "k", "g", "h", "r1", "r2", "s", "neg", "not_",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        // `p` and `q` are pattern binders: they occur as references in the arm
        // body (`p + z`), where they are mentioned by the walker too.
        let mut want = want;
        want.insert("p".into());
        assert_eq!(seen, want);
    });
}

// ---------------------------------------------------------------------------
// Loader defences
// ---------------------------------------------------------------------------

/// Run `f` on a mutable copy of the document's first table.  (The declaration
/// borrows the parsed source, so a test that edits its strings does so inside
/// the closure, with `&'static str` replacements.)
fn with_table<R>(src: &str, f: impl FnOnce(&mut VectorsDecl) -> R) -> R {
    parse(src, |file| {
        let Some(Declaration::Vectors(v)) = file
            .declarations
            .iter()
            .find(|d| matches!(d, Declaration::Vectors(_)))
        else {
            panic!("table")
        };
        let mut copy = v.clone();
        f(&mut copy)
    })
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

const DOC: &str = "warrant t v1\n  op f(m: Bytes) -> Bytes<4>\n  labels f:\n    m public\n  vectors a:\n    source \"a.tfv\"\n    sha256 b\"0000000000000000000000000000000000000000000000000000000000000000\"\n    rows 1\n    column msg: Bytes\n    holds f(msg) == f(msg)\n";

#[test]
fn a_hand_built_declaration_cannot_skip_the_parsers_rules() {
    let dir = Dir::new("hand");
    let shapes = vec![("msg".to_string(), Shape::Bytes)];
    with_table(DOC, |v| {
        // A path outside the rules.
        for bad in ["../a.tfv", "/a.tfv", "a//b", ""] {
            v.source = bad;
            let e = load_table(v, &shapes, &dir.0).unwrap_err();
            assert_eq!(e.code, GenCode::W0708, "{bad:?}");
        }
        v.source = "a.tfv";
        // A pin that is not 64 lowercase hex digits.
        for bad in [
            "",
            "ab",
            leak("A".repeat(64)),
            leak("g".repeat(64)),
            leak("0".repeat(65)),
        ] {
            v.sha256 = bad;
            let e = load_table(v, &shapes, &dir.0).unwrap_err();
            assert_eq!(e.code, GenCode::W0708, "{bad:?}");
        }
        // The wrong number of column shapes.
        v.sha256 = leak("0".repeat(64));
        let e = load_table(v, &[], &dir.0).unwrap_err();
        assert_eq!(e.code, GenCode::W0710);
    });
}

#[cfg(target_os = "linux")]
#[test]
fn a_file_that_reports_no_size_is_still_bounded_by_the_read_cap() {
    // /proc files claim a length of 0 but produce data: only the `take(max+1)`
    // and the length check after it stand between them and an unbounded read.
    let e = vectors::read_fixture(std::path::Path::new("/proc/self"), "status", 10).unwrap_err();
    assert!(e.contains("larger than 10 bytes"), "{e}");
}

#[test]
fn fixtures_root_is_below_the_repository_root() {
    let p = fixtures_root(std::path::Path::new("/repo"));
    assert_eq!(p, PathBuf::from("/repo").join(FIXTURES_ROOT));
    assert_eq!(FIXTURES_ROOT, "code/specs/fixtures");
}

// ---------------------------------------------------------------------------
// Shape mismatches name what was expected
// ---------------------------------------------------------------------------

#[test]
fn every_shape_names_what_it_expected() {
    let v = |t: &str| {
        parse_file(&format!("tfv 1\nr R1 Kv {t}\n")).unwrap().remove(0).fields.remove(0).1
    };
    let cases: Vec<(Shape, &str, &str)> = vec![
        (Shape::BytesN(4), "I1", "4 bytes"),
        (Shape::List(Box::new(Shape::Bool)), "I1", "a list"),
        (Shape::Variant(vec![("A".into(), vec![])]), "I1", "a variant"),
        (Shape::Opt(Box::new(Shape::Bool)), "I1", "an Option"),
        (Shape::Tuple(vec![Shape::Bool]), "I1", "a tuple or record"),
        (Shape::Result(Box::new(Shape::Bool), vec![]), "I1", "a Result"),
        (Shape::Bool, "I1", "a Bool"),
        (Shape::nat(), "T", "an integer"),
        (Shape::Bytes, "T", "bytes"),
    ];
    for (shape, text, want) in cases {
        let e = check_value(&v(text), &shape, 0).unwrap_err();
        assert!(e.contains(want), "{shape:?} on {text}: {e}");
    }
    // A variant whose body is the wrong list shape.
    let var = Shape::Variant(vec![("A".into(), vec![])]);
    assert!(check_value(&v("L1 I1"), &var, 0).is_err());
    assert!(check_value(&v("L2 I1 L0"), &var, 0).is_err());
}

// ---------------------------------------------------------------------------
// The discrimination predicate
// ---------------------------------------------------------------------------

fn table_over(cols: &[&str], rows: &[&[&str]]) -> LoadedTable {
    let mut text = String::from("tfv 1\n");
    for (i, r) in rows.iter().enumerate() {
        text.push_str(&format!("r{i} R{}", cols.len()));
        for (c, v) in cols.iter().zip(*r) {
            text.push_str(&format!(" K{c} {v}"));
        }
        text.push('\n');
    }
    LoadedTable {
        name: "t".into(),
        source: "t.tfv".into(),
        sha256: sha256_hex(text.as_bytes()),
        bytes: text.clone().into_bytes(),
        rows: parse_file(&text).unwrap(),
        columns: cols.iter().map(|c| c.to_string()).collect(),
    }
}

fn holds_of(src: &str, f: impl FnOnce(&Expression) -> bool) -> bool {
    let doc = format!(
        "warrant t v1\n  op f(m: Bytes) -> Bytes<4>\n  op g(m: Bytes) -> Bytes<4>\n  labels f:\n    m public\n  labels g:\n    m public\n  vectors a:\n    source \"a.tfv\"\n    sha256 b\"{}\"\n    rows 2\n    column msg: Bytes\n    column md: Bytes<4>\n    column other: Bytes<4>\n    holds {src}\n",
        "0".repeat(64)
    );
    parse(&doc, |file| {
        let Some(Declaration::Vectors(v)) = file
            .declarations
            .iter()
            .find(|d| matches!(d, Declaration::Vectors(_)))
        else {
            panic!("table")
        };
        f(&v.holds)
    })
}

#[test]
fn the_expected_column_is_the_one_compared_with_an_expression() {
    let cols: Vec<String> = ["msg", "md", "other"].iter().map(|s| s.to_string()).collect();
    let name = |src: &str| holds_of(src, |h| expected_column(h, &cols).map(str::to_string).as_deref() == Some("md"));
    assert!(name("f(msg) == md"));
    assert!(name("md == f(msg)"));
    assert!(name("(f(msg) == md)"));
    assert!(name("(md == f(msg))"));
    // Two columns compared: no subject on either side.
    assert!(!name("md == other"));
    assert!(holds_of("md == other", |h| expected_column(h, &cols).is_none()));
    // Neither side a column, or not an equality at all.
    assert!(holds_of("f(msg) == g(msg)", |h| expected_column(h, &cols).is_none()));
    assert!(holds_of("f(msg) != md", |h| expected_column(h, &cols).is_none()));
    assert!(holds_of("f(msg) == md and f(msg) == md", |h| expected_column(h, &cols).is_none()));
    // A reference with arguments is a call, not a column.
    assert!(holds_of("f(msg) == f(md)", |h| expected_column(h, &cols).is_none()));
}

#[test]
fn constant_output_needs_two_rows_a_column_and_equal_values() {
    let t = table_over(&["msg", "md", "other"], &[&["X00", "X01", "X09"], &["X02", "X01", "X08"]]);
    assert!(holds_of("f(msg) == md", |h| is_constant_output(&t, h)));
    assert!(!holds_of("f(msg) == other", |h| is_constant_output(&t, h)));
    assert!(!holds_of("f(msg) == g(msg)", |h| is_constant_output(&t, h)));
    // A column that is absent from a later row is not "the same value".
    let mut ragged = t.clone();
    ragged.rows[1].fields.retain(|(n, _)| n != "md");
    assert!(!holds_of("f(msg) == md", |h| is_constant_output(&ragged, h)));
    // One row can never be constant.
    let one = table_over(&["msg", "md", "other"], &[&["X00", "X01", "X09"]]);
    assert!(!holds_of("f(msg) == md", |h| is_constant_output(&one, h)));
}

// ---------------------------------------------------------------------------
// Generator paths
// ---------------------------------------------------------------------------

fn options() -> EvidenceOptions {
    EvidenceOptions {
        package_crate: "subject".into(),
        spec_path: "code/specs/t.warrant".into(),
        spec_sha256: "0".repeat(64),
        seed: 1,
        property_cases: 10,
        fuzz_iterations: 10,
        negctl_fuzz_iterations: 10,
        exhaustive_budget: 400_000,
        arg_types: Default::default(),
        hosts: Default::default(),
    }
}

fn bindings_text(lines: &[&str]) -> String {
    let mut s = String::from(
        "package subject\nrevision 0\nspec code/specs/t.warrant\noutput code/packages/rust/x/tests/generated/t.rs\nseed 1\nproperty_cases 10\nfuzz_iterations 10\nnegctl_fuzz_iterations 10\nexhaustive_budget 400000\n",
    );
    for l in lines {
        s.push_str(l);
        s.push('\n');
    }
    s
}

fn manifest(lines: &[&str]) -> coding_adventures_warrant_grammar::binding::BindingManifest {
    parse_evidence_bindings(&bindings_text(lines)).unwrap().manifest
}

#[test]
fn a_v1_document_marked_v0_by_hand_cannot_generate_a_table() {
    let dir = Dir::new("v0");
    let text = "tfv 1\nr1 R1 Kmsg X\n";
    fs::write(dir.0.join("a.tfv"), text).unwrap();
    let src = DOC.replace(&"0".repeat(64), &sha256_hex(text.as_bytes()));
    parse(&src, |file| {
        let tables = load_document_tables(file, &dir.0).unwrap();
        let mut v0 = file.clone();
        v0.version = Version::V0;
        let set = TableSet {
            dir: "tests/generated/tables",
            tables: &tables,
        };
        let e = coding_adventures_warrant_grammar::evidence::generate_evidence_harness_with_tables(
            &v0,
            &manifest(&["op f export=f faults=flip-result-bit"]),
            &options(),
            &set,
        )
        .unwrap_err();
        assert_eq!(e.code, GenCode::W0701);
        assert!(format!("{e}").contains("v0 document"), "{e}");
    });
}

fn generate(src: &str, files: &[(&str, &str)], lines: &[&str]) -> Result<String, String> {
    let dir = Dir::new("gen");
    for (name, content) in files {
        fs::write(dir.0.join(name), content).unwrap();
    }
    parse(src, |file| {
        let checked = check_warrant(file).map_err(|e| format!("{e}"))?;
        let tables = load_document_tables(file, &dir.0).map_err(|e| format!("{e}"))?;
        let set = TableSet {
            dir: "tests/generated/tables",
            tables: &tables,
        };
        let b = parse_evidence_bindings(&bindings_text(lines)).unwrap();
        let mut opts = options();
        opts.hosts = b.hosts.clone();
        opts.arg_types = b.arg_types.clone();
        compile_rust_evidence_harness_with_tables(file, &checked, &b.manifest, &opts, &set)
            .map_err(|e| format!("{e}"))
    })
}

#[test]
fn a_table_over_a_tuple_column_decodes_through_the_composite_prelude() {
    let text = "tfv 1\nr1 R2 Kmsg X00 Kpair L2 I1 T\nr2 R2 Kmsg X Kpair L2 I2 F\n";
    let src = format!(
        "warrant t v1\n  op f(m: Bytes) -> (U8, Bool)\n  labels f:\n    m public\n  vectors a:\n    source \"a.tfv\"\n    sha256 b\"{}\"\n    rows 2\n    column msg: Bytes\n    column pair: (U8, Bool)\n    holds f(msg) == pair\n  claim c:\n    forall m: Bytes. f(m) == f(m)\n",
        sha256_hex(text.as_bytes())
    );
    let out = generate(&src, &[("a.tfv", text)], &["op f export=f faults=swap-components"]);
    // swap-components needs a homogeneous pair: (U8, Bool) is not, so generation
    // refuses the fault; the table itself is fine with another fault set.
    assert!(out.is_err());
    let src_ok = src.replace("(U8, Bool)", "(U8, U8)").replace("I1 T", "I1 I3").replace("I2 F", "I2 I4");
    let text_ok = text.replace("I1 T", "I1 I3").replace("I2 F", "I2 I4");
    let src_ok = src_ok.replace(&sha256_hex(text.as_bytes()), &sha256_hex(text_ok.as_bytes()));
    let out = generate(&src_ok, &[("a.tfv", &text_ok)], &["op f export=f faults=swap-components"]).unwrap();
    assert!(out.contains("Ty::Tuple("), "{out}");
    assert!(out.contains("(Ty::Tuple(cs), Value::List(items))"));
}

#[test]
fn a_table_over_an_observed_operation_turns_observer_failures_into_failed_rows() {
    let text = "tfv 1\nr1 R2 Kmsg X00 Kout X00\nr2 R2 Kmsg X01 Kout X01\n";
    let src = format!(
        "warrant t v1\n  op f(m: Bytes) -> Bytes\n  labels f:\n    m public\n  bound f:\n    heap <= 4096\n  vectors a:\n    source \"a.tfv\"\n    sha256 b\"{}\"\n    rows 2\n    column msg: Bytes\n    column out: Bytes\n    holds f(msg) == out\n  claim c:\n    forall m: Bytes. f(m) == f(m)\n",
        sha256_hex(text.as_bytes())
    );
    let out = generate(&src, &[("a.tfv", text)], &["op f export=f faults=flip-result-bit"]).unwrap();
    assert!(out.contains("fn table_a_holds"));
    assert!(out.contains("Err(p) if p.is::<ObsFailure>() => obs_failed(p),\n        Err(_) => false,"), "{out}");
    // A bounded operation without a scoped adapter is observed without
    // `fault_for` (the byte-pair shapes are not attacked one at a time).
    let eq = "warrant t v1\n  op eq(a: Bytes, b: Bytes) -> Bool\n  labels eq:\n    a public\n    b public\n  bound eq:\n    heap <= 64\n  claim c:\n    forall a: Bytes, b: Bytes. eq(a, b) == eq(b, a)\n";
    let out = generate(eq, &[], &["op eq export=eq faults=invert"]).unwrap();
    assert!(out.contains("fn op_eq_raw("), "{out}");
    assert!(!out.contains("fault_for(\"eq\""), "{out}");
}

#[test]
fn an_observed_scalar_claim_catches_observer_failures_without_the_composite_prelude() {
    let src = "warrant t v1\n  op add(a: U16, b: U16) -> U16\n  labels add:\n    a public\n    b public\n  bound add:\n    heap <= 64\n  claim c:\n    forall a: U16, b: U16. add(a, b) == add(b, a)\n";
    let out = generate(src, &[], &["op add export=add faults=saturate-add"]).unwrap();
    assert!(out.contains("Err(p) => obs_failed(p),"), "{out}");
    assert!(!out.contains("count_unreach"), "this document is not composite");
}

#[test]
fn a_nat_limit_parameter_is_clamped_to_its_host_type_by_the_fault() {
    let src = "warrant t v1\n  error LzErr = LIMIT | BAD\n  op dec(d: Bytes, m: Nat) -> Result<Bytes, LzErr>\n  labels dec:\n    d public\n    m public\n  bound dec:\n    output <= m\n  claim c:\n    forall d: Bytes. match dec(d, 0) with Ok(r) -> len(r) == 0 | Err(e) -> true\n";
    let out = generate(
        src,
        &[],
        &["op dec export=dec errors=subject::E/Limit=LIMIT,Bad=BAD faults=ignore-limit,off-by-one-limit,limit-after-alloc"],
    )
    .unwrap();
    assert!(out.contains("Fault::IgnoreLimit => u32::MAX"), "{out}");
    assert!(out.contains("vec![0u8; p1 as usize]"), "{out}");
    let _ = vectors::MAX_NESTING;
}

#[test]
fn generate_evidence_harness_refuses_a_document_with_tables_and_none_loaded() {
    parse(DOC, |file| {
        let e = generate_evidence_harness(
            file,
            &manifest(&["op f export=f faults=flip-result-bit"]),
            &options(),
        )
        .unwrap_err();
        assert_eq!(e.code, GenCode::W0708);
    });
}
