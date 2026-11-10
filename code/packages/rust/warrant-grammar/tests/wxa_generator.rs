//! VER04j sections 4.6-4.8 and 10.4: what the generator decides for a v1
//! document with composite types -- binder domains and the budget demotion rule
//! applied to the leaves of a composite, the host bindings (`struct`, `enum`,
//! `ret=`, `errors=`), which faults may attack which results -- and what it
//! refuses.  These tests inspect the generated text and the generator's
//! diagnostics; `tests/wxa_evidence.rs` compiles and runs the output.

use coding_adventures_warrant_grammar::binding::{
    parse_evidence_bindings, parse_host_ty, split_top, HostShape, HostTy,
};
use coding_adventures_warrant_grammar::codegen::DiagnosticCode;
use coding_adventures_warrant_grammar::evidence::{generate_evidence_harness, EvidenceOptions};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

/// The preamble every bindings file needs; the `op`/`struct`/`enum` lines follow.
fn bindings_text(budget: u64, lines: &[&str]) -> String {
    let mut s = format!(
        "package subject\nrevision 0\nspec code/specs/t.warrant\noutput code/packages/rust/x/tests/generated/t.rs\nseed 1\nproperty_cases 10\nfuzz_iterations 10\nnegctl_fuzz_iterations 10\nexhaustive_budget {budget}\n"
    );
    for l in lines {
        s.push_str(l);
        s.push('\n');
    }
    s
}

fn generate(
    src: &str,
    budget: u64,
    lines: &[&str],
) -> Result<String, (DiagnosticCode, String)> {
    let b = parse_evidence_bindings(&bindings_text(budget, lines)).expect("bindings parse");
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    let options = EvidenceOptions {
        package_crate: "subject".into(),
        spec_path: "code/specs/t.warrant".into(),
        spec_sha256: "0".repeat(64),
        seed: 1,
        property_cases: 10,
        fuzz_iterations: 10,
        negctl_fuzz_iterations: 10,
        exhaustive_budget: budget,
        arg_types: b.arg_types.clone(),
        hosts: b.hosts.clone(),
    };
    generate_evidence_harness(&file, &b.manifest, &options).map_err(|e| (e.code, format!("{e}")))
}

fn tys_of(text: &str) -> String {
    let start = text
        .find("const CLAIM_C_TYS: &[Ty] = &[")
        .expect("TYS constant");
    let rest = &text[start + "const CLAIM_C_TYS: &[Ty] = &[".len()..];
    rest[..rest.find("];\n").unwrap()].to_string()
}

fn demoted_of(text: &str) -> String {
    let start = text
        .find("const CLAIM_C_DEMOTED: &[&str] = &[")
        .expect("DEMOTED constant");
    let rest = &text[start + "const CLAIM_C_DEMOTED: &[&str] = &[".len()..];
    rest[..rest.find("];").unwrap()].to_string()
}

const REFERENCE: u64 = 400_000;
const RAISED: u64 = 1 << 26;

/// A document with one composite op `f` (so a claim has an operation to call)
/// and a claim `c` over `binders`.
fn doc(decls: &str, binders: &str) -> String {
    format!(
        "warrant t v1\n  error E = X | Y\n  record R(x: U16, y: U16)\n  variant V = P(a: U16) | Q(a: U16, b: U8) | Z\n{decls}  op f(a: U8) -> (U8, U8)\n  claim c:\n    forall {binders}. f(0u8) == f(0u8)\n"
    )
}

const F_BINDING: &[&str] = &["op f export=f faults=swap-components"];

// ---------------------------------------------------------------------------
// Domains, the budget and the demotion rule inside composites
// ---------------------------------------------------------------------------

#[test]
fn composite_binders_enumerate_their_component_domains() {
    let text = generate(&doc("", "p: (U8, Bool), q: Option<Bool>"), REFERENCE, F_BINDING).unwrap();
    assert_eq!(
        tys_of(&text),
        "Ty::Tuple(&[Ty::Fx(F::U8), Ty::Bool]), Ty::Opt(&Ty::Bool)"
    );
    // Records and variants carry their constructors and field types; aliases
    // resolve; Result carries the error cases in declaration order; List its
    // element type.
    let text = generate(
        &doc(
            "  model Pair = (U8, Bool)\n",
            "b: Pair, c: Result<Bool, E>, d: List<Bool>",
        ),
        REFERENCE,
        F_BINDING,
    )
    .unwrap();
    assert_eq!(
        tys_of(&text),
        "Ty::Tuple(&[Ty::Fx(F::U8), Ty::Bool]), Ty::Res(&Ty::Bool, &[\"X\", \"Y\"]), Ty::List(&Ty::Bool)"
    );
    let text = generate(&doc("", "a: V"), REFERENCE, F_BINDING).unwrap();
    assert_eq!(
        tys_of(&text),
        "Ty::Data(\"V\", &[(\"P\", &[Ty::FxEdge(F::U16)]), (\"Q\", &[Ty::FxEdge(F::U16), Ty::Fx(F::U8)]), (\"Z\", &[])])"
    );
    // The model is composite, so the WX-A prelude is on.
    assert!(text.contains("pub struct Unreachable"));
    assert!(text.contains("fn field_pos("));
}

#[test]
fn the_demotion_rule_reaches_into_composites_leftmost_first() {
    // (U16, U16): 65536 * 65536 -> 281 * 65536 (still over) -> 281 * 281.
    let text = generate(&doc("", "p: (U16, U16)"), REFERENCE, F_BINDING).unwrap();
    assert_eq!(tys_of(&text), "Ty::Tuple(&[Ty::FxEdge(F::U16), Ty::FxEdge(F::U16)])");
    assert_eq!(demoted_of(&text), "\"p.0\", \"p.1\"");
    // A composite binder's domain is materialised whole, so the 64 MiB byte
    // budget bites inside it even when the case budget is raised: both leaves
    // are demoted at 2^26 too (281 * 65536 elements are 1.7 GB).
    let text = generate(&doc("", "p: (U16, U16)"), RAISED, F_BINDING).unwrap();
    assert_eq!(demoted_of(&text), "\"p.0\", \"p.1\"");
    // Within budget nothing is demoted: 65536 * 2 elements.
    let text = generate(&doc("", "p: (U16, Bool)"), REFERENCE, F_BINDING).unwrap();
    assert_eq!(demoted_of(&text), "");
    // (U16, U8): the one U16 demoted gives 281 * 256 = 71,936.
    let text = generate(&doc("", "p: (U16, U8)"), REFERENCE, F_BINDING).unwrap();
    assert_eq!(demoted_of(&text), "\"p.0\"");
    // A record's fields are demoted by index, binders left to right.
    let text = generate(&doc("", "r: R"), REFERENCE, F_BINDING).unwrap();
    assert_eq!(demoted_of(&text), "\"r.0\", \"r.1\"");
    // A nested path: (Bool, (U16, U16)) -> p.1.0, p.1.1.
    let text = generate(&doc("", "p: (Bool, (U16, U16))"), REFERENCE, F_BINDING).unwrap();
    assert_eq!(demoted_of(&text), "\"p.1.0\", \"p.1.1\"");
    // A variant's path names the constructor: V = P(U16) | Q(U16, U8) | Z needs
    // both U16 fields demoted (65536 + 65536 * 256 + 1 is over budget).
    let text = generate(&doc("", "a: V"), REFERENCE, F_BINDING).unwrap();
    assert_eq!(demoted_of(&text), "\"a.P.0\", \"a.Q.0\"");
    // Two such binders cannot be made to fit: demotion is exhausted, W0706.
    assert_eq!(
        generate(&doc("", "a: V, b: V"), REFERENCE, F_BINDING).unwrap_err().0,
        DiagnosticCode::W0706
    );
    // Demotion order is depth first across binders, left to right.
    let text = generate(&doc("", "u: U16, p: (U16, Bool)"), REFERENCE, F_BINDING).unwrap();
    assert_eq!(demoted_of(&text), "\"u\", \"p.0\"");
}

#[test]
fn what_cannot_be_demoted_is_w0706_and_never_sampled() {
    // (U16, U8, U8): the one U16 demoted gives 281 * 256 * 256 = 18.4M elements,
    // over both the case and the byte budget, even raised: W0706.
    let src = doc("", "p: (U16, U8, U8)");
    assert_eq!(generate(&src, REFERENCE, F_BINDING).unwrap_err().0, DiagnosticCode::W0706);
    assert_eq!(generate(&src, RAISED, F_BINDING).unwrap_err().0, DiagnosticCode::W0706);
    // A tuple of eight bytes overflows a u64 product: W0706, not a wrapped 0.
    let src = doc("", "p: (U8, U8, U8, U8, U8, U8, U8, U8)");
    let (code, msg) = generate(&src, RAISED, F_BINDING).unwrap_err();
    assert_eq!(code, DiagnosticCode::W0706, "{msg}");
    // List<List<U8>> is 337 elements (a list domain saturates at 1 + 16 + 256 +
    // 64), so two binders fit and three (38M cases) do not.
    let two = doc("", "l: List<List<U8>>, m: List<List<U8>>");
    assert!(generate(&two, REFERENCE, F_BINDING).is_ok());
    let three = doc("", "l: List<List<U8>>, m: List<List<U8>>, n: List<List<U8>>");
    assert_eq!(generate(&three, REFERENCE, F_BINDING).unwrap_err().0, DiagnosticCode::W0706);
    // The case budget counts the product of component domains.
    let src = doc("", "p: (Bytes, Bytes, Bytes)");
    assert_eq!(generate(&src, REFERENCE, F_BINDING).unwrap_err().0, DiagnosticCode::W0706);
    // The byte budget counts the per-element cost: 337 * 337 lists of bytes
    // fit the case budget but the bytes bound refuses a wide-enough tuple.
    let src = doc("", "p: (List<U8>, List<U8>, List<U8>)");
    assert_eq!(generate(&src, RAISED, F_BINDING).unwrap_err().0, DiagnosticCode::W0706);
}

#[test]
fn the_generator_s_sizes_equal_the_closed_forms_at_the_budget_boundary() {
    // Result<Bool,E> x Option<Bool> x List<Bool> x U8 = 4 * 3 * 15 * 256 = 46,080.
    let src = doc("", "r: Result<Bool, E>, o: Option<Bool>, l: List<Bool>, a: U8");
    assert!(generate(&src, 46_080, F_BINDING).is_ok());
    assert_eq!(generate(&src, 46_079, F_BINDING).unwrap_err().0, DiagnosticCode::W0706);
    // (U64, Nat) is 377 * 65 = 24,505 (the example of section 4.7).
    let src = doc("", "p: (U64, Nat)");
    assert!(generate(&src, 24_505, F_BINDING).is_ok());
    assert_eq!(generate(&src, 24_504, F_BINDING).unwrap_err().0, DiagnosticCode::W0706);
    // A variant is a sum of products: P(Bool) | Q(Bool, Bool) | Z = 2 + 4 + 1.
    let src = "warrant t v1\n  variant W = P(a: Bool) | Q(a: Bool, b: Bool) | Z\n  op f(a: U8) -> (U8, U8)\n  claim c:\n    forall w: W. f(0u8) == f(0u8)\n";
    assert!(generate(src, 7, F_BINDING).is_ok());
    assert_eq!(generate(src, 6, F_BINDING).unwrap_err().0, DiagnosticCode::W0706);
    // Option 1 + n and Result n + cases.
    let src = doc("", "o: Option<U8>");
    assert!(generate(&src, 257, F_BINDING).is_ok());
    assert_eq!(generate(&src, 256, F_BINDING).unwrap_err().0, DiagnosticCode::W0706);
    let src = doc("", "r: Result<U8, E>");
    assert!(generate(&src, 258, F_BINDING).is_ok());
    assert_eq!(generate(&src, 257, F_BINDING).unwrap_err().0, DiagnosticCode::W0706);
    // List<U8> is 1 + 16 + 256 + 64 = 337.
    let src = doc("", "l: List<U8>");
    assert!(generate(&src, 337, F_BINDING).is_ok());
    assert_eq!(generate(&src, 336, F_BINDING).unwrap_err().0, DiagnosticCode::W0706);
}

#[test]
fn only_documents_that_use_composites_switch_the_prelude_on() {
    // A v1 document with scalar-shaped operations only keeps the WX-B/WX-C
    // output (no WX-A machinery in it).
    let src = "warrant t v1\n  op g(a: U8, b: U8) -> U8\n  claim c:\n    forall a: U8, b: U8. g(a, b) == a + b\n";
    let text = generate(src, REFERENCE, &["op g export=g faults=saturate-add"]).unwrap();
    assert!(!text.contains("Unreachable") && !text.contains("pat_match") && !text.contains("domain_take"));
    // A match in a model switches it on; so does a tuple type in a binder.
    let src = "warrant t v1\n  op g(a: U8, b: U8) -> U8\n  model m(o: Option<U8>) -> U8 = match o with None -> 0u8 | Some(x) -> x\n  claim c:\n    forall a: U8, b: U8. g(a, b) == a + b\n";
    let text = generate(src, REFERENCE, &["op g export=g faults=saturate-add"]).unwrap();
    assert!(text.contains("pub fn pat_match") && text.contains("Pat::Some(&Pat::Bind)"));
    // A v0 document never has them.
    let src = "warrant t\n  op g(a: U8, b: U8) -> U8\n  claim c:\n    forall a: U8, b: U8. g(a, b) == g(a, b)\n";
    // (a v0 scalar operation uses the v0 shapes; only the absence matters here)
    if let Ok(text) = generate(src, REFERENCE, &["op g export=g faults=or-for-xor"]) {
        assert!(!text.contains("Unreachable"));
    }
}

// ---------------------------------------------------------------------------
// Which faults may attack which results (section 4.8)
// ---------------------------------------------------------------------------

fn fault_doc(ret: &str) -> String {
    format!(
        "warrant t v1\n  error E = X | Y\n  record Rec(a: U8, flag: Bool)\n  record NoNum(flag: Bool, other: Bool)\n  variant Vr = A | B(x: U8)\n  op h(a: U8) -> {ret}\n  claim c:\n    forall a: U8. h(a) == h(a)\n"
    )
}

fn attack(ret: &str, fault: &str, extra: &str) -> Result<String, (DiagnosticCode, String)> {
    let line = format!("op h export=h {extra} faults={fault}");
    generate(
        &fault_doc(ret),
        REFERENCE,
        &[
            "struct Rec path=subject::Rec fields=a:u8,flag:bool",
            "struct NoNum path=subject::NoNum fields=flag:bool,other:bool",
            "enum Vr path=subject::Vr ctors=A,B(x:u8)",
            &line,
        ],
    )
}

#[test]
fn a_fault_must_apply_to_the_declared_result() {
    // Applicable.
    for (ret, fault, extra) in [
        ("(U8, Nat)", "drop-consumed", ""),
        ("(U8, Nat)", "off-by-one-consumed", ""),
        ("(U8, U8)", "swap-components", ""),
        ("(U8, U8, U8)", "swap-components", ""),
        ("Rec", "field-zero", ""),
        ("Vr", "wrong-ctor", ""),
        ("Result<(U8, U8), E>", "ok-as-err", "error=X"),
        ("Result<Rec, E>", "ok-as-err", "error=Y"),
        ("Result<(U8, U8), E>", "missing-error", "error=X"),
        ("Option<(U8, Nat)>", "off-by-one-consumed", ""),
        ("Option<(U8, U8)>", "swap-components", ""),
    ] {
        let r = attack(ret, fault, extra);
        assert!(r.is_ok(), "{ret} / {fault}: {r:?}");
    }
    // Inapplicable: each is W0701 at generation, never a silently inert fault.
    for (ret, fault, extra) in [
        ("(U8, Bool)", "drop-consumed", ""),          // second component not numeric
        ("(U8, Bool)", "off-by-one-consumed", ""),
        ("(U8, Bool)", "swap-components", ""),         // heterogeneous
        ("Rec", "swap-components", ""),
        ("NoNum", "field-zero", ""),                   // no numeric field
        ("Vr", "field-zero", ""),                      // a variant is not a record
        ("Rec", "wrong-ctor", ""),                     // one constructor
        ("(U8, U8)", "ok-as-err", ""),                 // not a Result
        ("Result<U8, E>", "ok-as-err", "error=X"),     // the payload is not composite
        ("(U8, U8)", "missing-error", ""),             // total operation
        ("(U8, U8)", "saturate-add", ""),              // a scalar fault
        ("(U8, U8)", "no-such-fault", ""),
    ] {
        let r = attack(ret, fault, extra);
        let (code, msg) = r.unwrap_err();
        assert_eq!(code, DiagnosticCode::W0701, "{ret} / {fault}: {msg}");
    }
}

#[test]
fn faults_appear_in_the_generated_adapter_and_the_fault_enum() {
    let text = attack("Result<(U8, Nat), E>", "off-by-one-consumed,drop-consumed,ok-as-err", "error=X").unwrap();
    assert!(text.contains("Fault::OffByOneConsumed => fault_off_by_one_consumed(v),"));
    assert!(text.contains("Fault::DropConsumed => fault_drop_consumed(v),"));
    assert!(text.contains("Fault::OkAsErr => fault_ok_as_err(v, \"X\"),"));
    assert!(text.contains("OffByOneConsumed,") && text.contains("\"off-by-one-consumed\""));
    assert!(!text.contains("WrongCtor"));
    // The adapter obeys a fault only while it is the attacked operation.
    assert!(text.contains("let fault = fault_for(\"h\", fault);"));
}

// ---------------------------------------------------------------------------
// Bindings: host composites (section 4.6)
// ---------------------------------------------------------------------------

#[test]
fn host_types_parse() {
    assert_eq!(parse_host_ty("u8").unwrap(), HostTy::Int("u8".into()));
    assert_eq!(parse_host_ty("usize").unwrap(), HostTy::Int("usize".into()));
    assert_eq!(parse_host_ty("bool").unwrap(), HostTy::Bool);
    assert_eq!(parse_host_ty("bytes").unwrap(), HostTy::Bytes);
    assert_eq!(
        parse_host_ty("(u64,usize)").unwrap(),
        HostTy::Tuple(vec![HostTy::Int("u64".into()), HostTy::Int("usize".into())])
    );
    assert_eq!(
        parse_host_ty("opt<(u8, list<bool>)>").unwrap(),
        HostTy::Opt(Box::new(HostTy::Tuple(vec![
            HostTy::Int("u8".into()),
            HostTy::List(Box::new(HostTy::Bool))
        ])))
    );
    assert_eq!(parse_host_ty("Tlv").unwrap(), HostTy::Named("Tlv".into()));
    // Any other identifier names a declared host type (checked at file level).
    assert_eq!(parse_host_ty("u7").unwrap(), HostTy::Named("u7".into()));
    for bad in ["", "(u8)", "(u8,u8,u8,u8,u8,u8,u8,u8,u8)", "opt<>", "list<u8", "a b", "foo::Bar"] {
        assert!(parse_host_ty(bad).is_err(), "{bad:?}");
    }
    // Nesting is bounded.
    let deep = format!("{}u8{}", "opt<".repeat(9), ">".repeat(9));
    assert!(parse_host_ty(&deep).is_err());
    assert_eq!(split_top("a,(b,c),opt<d,e>,f"), vec!["a", "(b,c)", "opt<d,e>", "f"]);
}

#[test]
fn the_bindings_file_describes_host_structs_enums_returns_and_errors() {
    let b = parse_evidence_bindings(&bindings_text(
        1000,
        &[
            "struct Tlv path=subject::Tlv fields=tag:u8,constructed:bool,0:u32,value:bytes",
            "enum Shape path=subject::Shape ctors=Circle(r:u8),Rect(w:u8,h:(u8,u8)),Empty,Tup(0:u8,1:Tlv)",
            "op decode export=decode args=bytes,usize ret=(u64,usize) errors=subject::LebErr/Truncated=TRUNCATED,Overlong=OVERLONG faults=off-by-one-consumed",
            "op shape export=shape ret=Shape faults=wrong-ctor",
        ],
    ))
    .unwrap();
    let tlv = &b.hosts.types["Tlv"];
    assert_eq!(tlv.path, "subject::Tlv");
    let HostShape::Struct(fields) = &tlv.shape else {
        panic!("struct")
    };
    assert_eq!(
        fields.iter().map(|f| (f.name.as_str(), f.ty.clone())).collect::<Vec<_>>(),
        vec![
            ("tag", HostTy::Int("u8".into())),
            ("constructed", HostTy::Bool),
            ("0", HostTy::Int("u32".into())),
            ("value", HostTy::Bytes),
        ]
    );
    let HostShape::Enum(ctors) = &b.hosts.types["Shape"].shape else {
        panic!("enum")
    };
    assert_eq!(
        ctors.iter().map(|(c, fs)| (c.as_str(), fs.len())).collect::<Vec<_>>(),
        vec![("Circle", 1), ("Rect", 2), ("Empty", 0), ("Tup", 2)]
    );
    let op = &b.hosts.ops["decode"];
    assert_eq!(op.args.as_ref().unwrap().len(), 2);
    assert_eq!(
        op.ret,
        Some(HostTy::Tuple(vec![HostTy::Int("u64".into()), HostTy::Int("usize".into())]))
    );
    let errs = op.errors.as_ref().unwrap();
    assert_eq!(errs.path, "subject::LebErr");
    assert_eq!(
        errs.map,
        vec![("Truncated".into(), "TRUNCATED".into()), ("Overlong".into(), "OVERLONG".into())]
    );
    // The scalar `args=` strings are still recorded as before.
    assert_eq!(b.arg_types["decode"], vec!["bytes".to_string(), "usize".to_string()]);
}

#[test]
fn malformed_host_lines_are_refused() {
    for (line, what) in [
        ("struct", "needs a type name"),
        ("struct T fields=a:u8", "no path"),
        ("struct T path=a::B", "no fields"),
        ("struct T path=a::B fields=a", "name:type"),
        ("struct T path=a::B fields=a:u8,a:u8", "twice"),
        ("struct T path=a::B fields=a-b:u8", "not an identifier"),
        ("struct T path=a::B fields=a:zz9z", "host type"),
        ("struct T path=a-b fields=a:u8", "not a Rust path"),
        ("struct T path=a::B fields=a:Unknown", "has no `struct` or `enum`"),
        ("struct T path=a::B ctors=A", "unknown or malformed"),
        ("enum T path=a::B fields=a:u8", "unknown or malformed"),
        ("enum T path=a::B ctors=A,A", "invalid or repeated"),
        ("enum T path=a::B ctors=A(x:u8", "missing its"),
        ("enum T path=a::B ctors=1x", "invalid or repeated"),
        ("op f export=f ret=9z9 faults=a", "not a host type"),
        ("op f export=f errors=nopath faults=a", "PATH/Variant"),
        ("op f export=f errors=a::B/V faults=a", "Variant=CASE"),
        ("op f export=f errors=a-b/V=C faults=a", "Rust path"),
        ("op f export=f errors=a::B/V=1x faults=a", "identifiers"),
        ("op f export=f args=Missing faults=a", "has no `struct` or `enum`"),
    ] {
        let lines: Vec<&str> = if line.starts_with("op ") {
            vec![line]
        } else {
            vec![line, "op f export=f faults=a"]
        };
        let e = parse_evidence_bindings(&bindings_text(1000, &lines)).unwrap_err();
        assert!(e.contains(what), "{line}: {e}");
    }
    // A type declared twice.
    let two = bindings_text(
        1000,
        &[
            "struct T path=a::B fields=a:u8",
            "struct T path=a::C fields=a:u8",
            "op f export=f faults=a",
        ],
    );
    assert!(parse_evidence_bindings(&two).unwrap_err().contains("twice"));
    // `args=` entries that are not host types do not fail the parse (the
    // scalar adapters have their own check) ...
    let b = parse_evidence_bindings(&bindings_text(1000, &["op f export=f args=9z9 faults=a"])).unwrap();
    assert!(!b.hosts.ops.contains_key("f"));
    assert_eq!(b.arg_types["f"], vec!["9z9".to_string()]);
}

fn host_doc() -> &'static str {
    "warrant t v1\n  error E = X | Y\n  record Rec(a: U8, flag: Bool)\n  variant Vr = A | B(x: U8)\n  op h(a: U8, r: Rec) -> (U8, Nat)\n  op k(a: U8) -> Result<(U8, Nat), E>\n  op s(r: Rec, v: Vr) -> (U8, Nat)\n  claim c:\n    forall a: U8. h(a, Rec(a, true)) == h(a, Rec(a, true))\n  claim d:\n    forall a: U8. k(a) == k(a)\n  claim e:\n    forall a: U8. s(Rec(a, true), A) == s(Rec(a, true), A)\n"
}

const HOST_OPS: [&str; 3] = [
    "op h export=h args=u8,Rec ret=(u8,u8) faults=drop-consumed",
    "op k export=k error=X faults=missing-error",
    "op s export=s args=Rec,Vr ret=(u8,u8) faults=drop-consumed",
];
const HOST_TYPES: [&str; 2] = [
    "struct Rec path=subject::Rec fields=a:u8,flag:bool",
    "enum Vr path=subject::Vr ctors=A,B(x:u8)",
];

fn with(extra: &[&str], ops: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = HOST_TYPES.iter().map(|s| s.to_string()).collect();
    v.extend(extra.iter().map(|s| s.to_string()));
    v.extend(ops.iter().map(|s| s.to_string()));
    v
}

fn gen_host(lines: &[String]) -> Result<String, (DiagnosticCode, String)> {
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    generate(host_doc(), REFERENCE, &refs)
}

#[test]
fn host_conversions_are_generated_from_the_bindings() {
    let text = gen_host(&with(&[], &HOST_OPS)).unwrap();
    // A struct is built field by field and observed field by field.
    assert!(text.contains("subject::Rec { a: host::<u8>(args[1].fields()[0].int()), flag: args[1].fields()[1].boolv() }"), "{text}");
    // An enum is matched by constructor index, with named fields.
    assert!(text.contains("match args[1].ctor() { 0 => subject::Vr::A {  }, 1 => subject::Vr::B { x: host::<u8>(args[1].fields()[0].int()) }, _ => unreachable!"));
    // A bytes parameter at the top is passed by reference; the result tuple
    // is observed once into a model value.
    assert!(text.contains("V::Tup(vec![V::Fixed(F::U8, (t"));
    // Panic normalisation of a fallible export.
    assert!(text.contains("match contained(|| k(p0)) {"));
    assert!(text.contains("None => V::Err(\"X\".into()),"));
    assert!(text.contains("None if fault == Fault::MissingError =>"));
}

#[test]
fn host_binding_errors_are_diagnosed_at_generation() {
    let refused = |lines: Vec<String>, code: DiagnosticCode| {
        let (c, msg) = gen_host(&lines).unwrap_err();
        assert_eq!(c, code, "{msg}");
        msg
    };
    // Wrong argument count in args=.
    refused(
        with(&[], &["op h export=h args=u8 ret=(u8,u8) faults=drop-consumed", HOST_OPS[1], HOST_OPS[2]]),
        DiagnosticCode::W0701,
    );
    // A host integer of the wrong width or signedness for a fixed-width type.
    let m = refused(
        with(&[], &["op h export=h args=u16,Rec ret=(u8,u8) faults=drop-consumed", HOST_OPS[1], HOST_OPS[2]]),
        DiagnosticCode::W0701,
    );
    assert!(m.contains("does not fit"), "{m}");
    // A `Nat` accepts unsigned host integers only.
    refused(
        with(&[], &["op h export=h args=u8,Rec ret=(u8,i32) faults=drop-consumed", HOST_OPS[1], HOST_OPS[2]]),
        DiagnosticCode::W0701,
    );
    // A struct whose fields do not fit the record.
    refused(
        vec![
            "struct Rec path=subject::Rec fields=a:u8".to_string(),
            HOST_TYPES[1].to_string(),
            HOST_OPS[0].to_string(),
            HOST_OPS[1].to_string(),
            HOST_OPS[2].to_string(),
        ],
        DiagnosticCode::W0701,
    );
    // An enum whose constructors do not fit the variant, by name and count.
    refused(
        vec![
            HOST_TYPES[0].to_string(),
            "enum Vr path=subject::Vr ctors=A,C(x:u8)".to_string(),
            HOST_OPS[0].to_string(),
            HOST_OPS[1].to_string(),
            HOST_OPS[2].to_string(),
        ],
        DiagnosticCode::W0701,
    );
    refused(
        vec![
            HOST_TYPES[0].to_string(),
            "enum Vr path=subject::Vr ctors=A".to_string(),
            HOST_OPS[0].to_string(),
            HOST_OPS[1].to_string(),
            HOST_OPS[2].to_string(),
        ],
        DiagnosticCode::W0701,
    );
    // A record bound to an enum and a variant bound to a struct.
    refused(
        vec![
            "enum Rec path=subject::Rec ctors=A,B".to_string(),
            HOST_TYPES[1].to_string(),
            HOST_OPS[0].to_string(),
            HOST_OPS[1].to_string(),
            HOST_OPS[2].to_string(),
        ],
        DiagnosticCode::W0701,
    );
    // A fallible export with neither error= nor errors= is W0704.
    for fault in ["missing-error", "ok-as-err"] {
        let op = format!("op k export=k faults={fault}");
        refused(with(&[], &[HOST_OPS[0], &op, HOST_OPS[2]]), DiagnosticCode::W0704);
    }
    // errors= naming a case that is not an error case of the operation.
    refused(
        with(&[], &[HOST_OPS[0], "op k export=k errors=subject::KE/V=NOPE faults=ok-as-err", HOST_OPS[2]]),
        DiagnosticCode::W0701,
    );
    // errors= is accepted when the cases are right, and maps variants by name.
    let text = gen_host(&with(
        &[],
        &[HOST_OPS[0], "op k export=k errors=subject::KE/Vx=X,Vy=Y faults=ok-as-err", HOST_OPS[2]],
    ))
    .unwrap();
    assert!(text.contains("subject::KE::Vx { .. } => V::Err(\"X\".into()),"));
    assert!(text.contains("subject::KE::Vy { .. } => V::Err(\"Y\".into()),"));
    assert!(text.contains("_ => panic!(\"unmapped host error\")"));
    // A static width or a `fresh` output on a composite operation is refused.
    let src = "warrant t v1\n  op h<N: Nat>(a: Bytes<N>) -> (U8, U8)\n  claim c:\n    forall a: Bytes<2>. h<2>(a) == h<2>(a)\n";
    assert_eq!(
        generate(src, REFERENCE, &["op h export=h static=N faults=swap-components"]).unwrap_err().0,
        DiagnosticCode::W0701
    );
    // A host type missing for a record or variant: composites have no default
    // host type, so `args=` must name a declared `struct`/`enum`.
    let m = refused(
        vec![
            "op h export=h ret=(u8,u8) faults=drop-consumed".to_string(),
            HOST_OPS[1].to_string(),
            "op s export=s ret=(u8,u8) faults=drop-consumed".to_string(),
        ],
        DiagnosticCode::W0701,
    );
    assert!(m.contains("is not declared"), "{m}");
}
