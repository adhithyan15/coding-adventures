//! VER04j sections 2.3, 3.2-3.4 and 10.2: the type rules of WX-B and WX-C, the
//! negative corpus, the exhaustive conversion matrix, and version scoping of
//! the new names.

use coding_adventures_warrant_grammar::checker::{
    check_warrant, CheckedWarrantModule, CheckerError, CheckerErrorKind, DiagnosticCode, Type,
};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::parse_document;
use coding_adventures_warrant_grammar::parser::ParserLimits;
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

fn check(src: &str) -> Result<CheckedWarrantModule, CheckerError> {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    check_warrant(&file)
}

/// `model m(<params>) -> <ret> = <body>` in a document of the given header.
fn model(header: &str, params: &str, ret: &str, body: &str) -> String {
    format!("warrant t{header}\n  model m({params}) -> {ret} = {body}\n")
}

fn ty_of(params: &str, ret: &str, body: &str) -> Result<Type, CheckerError> {
    check(&model(" v1", params, ret, body)).map(|m| m.models["m"].body_type.clone())
}

fn rejected(params: &str, ret: &str, body: &str) -> CheckerError {
    match check(&model(" v1", params, ret, body)) {
        Ok(_) => panic!("accepted: ({params}) -> {ret} = {body}"),
        Err(e) => e,
    }
}

fn assert_w0301(params: &str, ret: &str, body: &str) {
    let e = rejected(params, ret, body);
    assert_eq!(e.code, DiagnosticCode::W0301, "{params} / {body}: {e:?}");
}

const NAMES: [&str; 8] = ["U8", "U16", "U32", "U64", "I8", "I16", "I32", "I64"];

fn t(name: &str) -> Type {
    match name {
        "U8" => Type::U8,
        "U16" => Type::U16,
        "U32" => Type::U32,
        "U64" => Type::U64,
        "I8" => Type::I8,
        "I16" => Type::I16,
        "I32" => Type::I32,
        "I64" => Type::I64,
        other => panic!("{other}"),
    }
}

/// The ranges of the eight types, written out independently of `FixedTy`.
fn range(name: &str) -> (i128, i128) {
    match name {
        "U8" => (0, 255),
        "U16" => (0, 65_535),
        "U32" => (0, 4_294_967_295),
        "U64" => (0, 18_446_744_073_709_551_615),
        "I8" => (-128, 127),
        "I16" => (-32_768, 32_767),
        "I32" => (-2_147_483_648, 2_147_483_647),
        "I64" => (-9_223_372_036_854_775_808, 9_223_372_036_854_775_807),
        other => panic!("{other}"),
    }
}

#[test]
fn arithmetic_and_comparison_need_the_same_fixed_width_type() {
    for n in NAMES {
        let p = format!("a: {n}, b: {n}");
        for op in ["+", "-", "*"] {
            assert_eq!(
                ty_of(&p, n, &format!("a {op} b")).unwrap(),
                t(n),
                "{n} {op}"
            );
        }
        for op in ["<", "<=", ">", ">=", "==", "!="] {
            assert_eq!(
                ty_of(&p, "Bool", &format!("a {op} b")).unwrap(),
                Type::Bool,
                "{n} {op}"
            );
        }
        // No implicit conversion in either direction.
        for other in NAMES.iter().filter(|o| **o != n) {
            let p = format!("a: {n}, b: {other}");
            for op in ["+", "<", "=="] {
                let ret = if op == "+" { n } else { "Bool" };
                assert_w0301(&p, ret, &format!("a {op} b"));
            }
        }
        // ... nor against Int or Nat.
        for lit in ["1", "a"] {
            let _ = lit;
        }
        assert_w0301(&format!("a: {n}, b: Int"), n, "a + b");
        assert_w0301(&format!("a: {n}, b: Nat"), n, "a + b");
        assert_w0301(&format!("a: {n}, b: Int"), "Bool", "a < b");
        assert_w0301(&format!("a: {n}"), n, "a + 1");
        assert_w0301(&format!("a: {n}"), "Bool", "a == 1");
    }
}

#[test]
fn unary_minus_is_wrapping_negation_of_signed_values_only() {
    for n in ["I8", "I16", "I32", "I64"] {
        assert_eq!(ty_of(&format!("a: {n}"), n, "-a").unwrap(), t(n));
    }
    for n in ["U8", "U16", "U32", "U64"] {
        let e = rejected(&format!("a: {n}"), n, "-a");
        assert_eq!(e.code, DiagnosticCode::W0301, "{n}");
    }
    assert_eq!(ty_of("a: Int", "Int", "-a").unwrap(), Type::Int);
    assert_eq!(ty_of("", "I8", "-128i8").unwrap(), Type::I8);
    assert_eq!(ty_of("", "I8", "-5i8").unwrap(), Type::I8);
}

#[test]
fn literal_types_and_hex() {
    for (lit, ty) in [
        ("5u8", "U8"),
        ("5u16", "U16"),
        ("5u32", "U32"),
        ("5u64", "U64"),
        ("5i8", "I8"),
        ("5i16", "I16"),
        ("5i32", "I32"),
        ("5i64", "I64"),
        ("0x1du8", "U8"),
        ("0x7fffi16", "I16"),
    ] {
        assert_eq!(ty_of("", ty, lit).unwrap(), t(ty), "{lit}");
    }
    // An unsuffixed hex literal is an Int (Nat-widenable) like a decimal one.
    assert_eq!(ty_of("", "Int", "0x1d").unwrap(), Type::Int);
    assert_eq!(
        ty_of("", "Nat", "0x1d").unwrap(),
        Type::Nat,
        "an expected Nat types it as a Nat, like 29"
    );
}

#[test]
fn conversion_matrix_widen_wrap_try_over_all_64_pairs() {
    for src in NAMES {
        for dst in NAMES {
            let p = format!("x: {src}");
            let (smin, smax) = range(src);
            let (dmin, dmax) = range(dst);
            let contained = dmin <= smin && smax <= dmax;
            let widen = check(&model(" v1", &p, dst, &format!("widen<{dst}>(x)")));
            match (contained, widen) {
                (true, Ok(m)) => assert_eq!(m.models["m"].body_type, t(dst), "widen {src}->{dst}"),
                (false, Err(e)) => {
                    assert_eq!(e.code, DiagnosticCode::W0301, "widen {src}->{dst}");
                }
                (c, r) => panic!("widen<{dst}>({src}): contained={c} but {:?}", r.map(|_| ())),
            }
            // wrap and try accept every pair.
            assert_eq!(ty_of(&p, dst, &format!("wrap<{dst}>(x)")).unwrap(), t(dst));
            assert_eq!(
                ty_of(&p, &format!("Option<{dst}>"), &format!("try<{dst}>(x)")).unwrap(),
                Type::Option(Box::new(t(dst)))
            );
        }
        // Int and Nat sources: wrap and try, never widen.
        for source in ["Int", "Nat"] {
            let p = format!("x: {source}");
            assert_eq!(ty_of(&p, src, &format!("wrap<{src}>(x)")).unwrap(), t(src));
            assert_eq!(
                ty_of(&p, &format!("Option<{src}>"), &format!("try<{src}>(x)")).unwrap(),
                Type::Option(Box::new(t(src)))
            );
            assert_w0301(&p, src, &format!("widen<{src}>(x)"));
        }
        // to_int is total over F; to_nat only over the unsigned types.
        assert_eq!(
            ty_of(&format!("x: {src}"), "Int", "to_int(x)").unwrap(),
            Type::Int
        );
        let nat = check(&model(" v1", &format!("x: {src}"), "Nat", "to_nat(x)"));
        if src.starts_with('U') {
            assert_eq!(nat.unwrap().models["m"].body_type, Type::Nat);
        } else {
            assert_eq!(
                nat.unwrap_err().code,
                DiagnosticCode::W0301,
                "to_nat({src})"
            );
        }
        assert_eq!(
            ty_of("", src, &format!("max_value<{src}>()")).unwrap(),
            t(src)
        );
        assert_eq!(
            ty_of("", src, &format!("min_value<{src}>()")).unwrap(),
            t(src)
        );
    }
}

#[test]
fn the_named_negative_cases_of_section_2_10() {
    assert_w0301("a: U8, b: U16", "U8", "a + b"); // 1u8 + 1u16
    assert_w0301("x: U32", "U32", "x + 1");
    assert_w0301("a: U8", "U8", "-a");
    assert_w0301("x: U16", "U8", "widen<U8>(x)");
    assert_w0301("x: I8", "U32", "widen<U32>(x)");
    assert_w0301("a: I32, b: I32", "Option<I32>", "div(a, b)");
    assert_w0301("a: I32, b: I32", "Option<I32>", "rem(a, b)");
    assert_w0301("x: U32", "Option<Int>", "try<Int>(x)");
    assert_w0301("", "Nat", "max_value<Nat>()");
    assert_w0301("x: I8", "Nat", "to_nat(x)");
    assert_w0301("a: I8, b: U8", "Bool", "a < b");
    assert_w0301("a: I8, b: U8", "Bool", "a == b");
}

#[test]
fn division_is_floor_on_int_unsigned_checked_and_truncating_on_signed() {
    // Int/Nat: unchanged from v0.
    assert_eq!(
        ty_of("a: Int, b: Nat", "Option<Int>", "div(a, b)").unwrap(),
        Type::Option(Box::new(Type::Int))
    );
    assert_eq!(
        ty_of("a: Int, b: Nat", "Option<Int>", "rem(a, b)").unwrap(),
        Type::Option(Box::new(Type::Int))
    );
    for n in ["U8", "U16", "U32", "U64"] {
        let p = format!("a: {n}, b: {n}");
        let want = Type::Option(Box::new(t(n)));
        assert_eq!(
            ty_of(&p, &format!("Option<{n}>"), "div(a, b)").unwrap(),
            want
        );
        assert_eq!(
            ty_of(&p, &format!("Option<{n}>"), "rem(a, b)").unwrap(),
            want
        );
        assert_w0301(&p, &format!("Option<{n}>"), "div_trunc(a, b)");
        assert_w0301(&p, &format!("Option<{n}>"), "rem_trunc(a, b)");
    }
    for n in ["I8", "I16", "I32", "I64"] {
        let p = format!("a: {n}, b: {n}");
        let want = Type::Option(Box::new(t(n)));
        assert_eq!(
            ty_of(&p, &format!("Option<{n}>"), "div_trunc(a, b)").unwrap(),
            want
        );
        assert_eq!(
            ty_of(&p, &format!("Option<{n}>"), "rem_trunc(a, b)").unwrap(),
            want
        );
        assert_w0301(&p, &format!("Option<{n}>"), "div(a, b)");
        assert_w0301(&p, &format!("Option<{n}>"), "rem(a, b)");
    }
    // Mixed or fixed-versus-Int operands are rejected.
    assert_w0301("a: U8, b: U16", "Option<U8>", "div(a, b)");
    assert_w0301("a: U8, b: Int", "Option<U8>", "div(a, b)");
    assert_w0301("a: Int, b: U8", "Option<Int>", "rem(a, b)");
}

#[test]
fn checked_arithmetic_and_unwrap_or() {
    for n in NAMES {
        let p = format!("a: {n}, b: {n}");
        for f in ["checked_add", "checked_sub", "checked_mul"] {
            assert_eq!(
                ty_of(&p, &format!("Option<{n}>"), &format!("{f}(a, b)")).unwrap(),
                Type::Option(Box::new(t(n)))
            );
        }
        assert_eq!(
            ty_of(&format!("o: Option<{n}>, d: {n}"), n, "unwrap_or(o, d)").unwrap(),
            t(n)
        );
        assert_w0301(&format!("o: Option<{n}>, d: Int"), n, "unwrap_or(o, d)");
    }
    assert_w0301("a: U8, b: U16", "Option<U8>", "checked_add(a, b)");
    assert_w0301("a: Int, b: Int", "Option<Int>", "checked_add(a, b)");
    assert_w0301("a: U8, d: U8", "U8", "unwrap_or(a, d)");
    assert_eq!(
        ty_of("o: Option<Bool>, d: Bool", "Bool", "unwrap_or(o, d)").unwrap(),
        Type::Bool
    );
}

#[test]
fn bitwise_builtins_follow_section_3_2() {
    for n in NAMES {
        let signed = n.starts_with('I');
        let p = format!("a: {n}, b: {n}");
        for f in ["bit_and", "bit_or", "bit_xor"] {
            assert_eq!(ty_of(&p, n, &format!("{f}(a, b)")).unwrap(), t(n));
        }
        assert_eq!(ty_of(&format!("a: {n}"), n, "bit_not(a)").unwrap(), t(n));
        let pc = format!("a: {n}, c: Nat");
        for f in ["shl", "shr", "rotl", "rotr"] {
            assert_eq!(ty_of(&pc, n, &format!("{f}(a, c)")).unwrap(), t(n), "{f}");
        }
        assert_eq!(ty_of(&pc, "Bool", "bit(a, c)").unwrap(), Type::Bool);
        // sar: signed only.
        let sar = check(&model(" v1", &pc, n, "sar(a, c)"));
        if signed {
            assert_eq!(sar.unwrap().models["m"].body_type, t(n));
        } else {
            assert_eq!(sar.unwrap_err().code, DiagnosticCode::W0301, "sar on {n}");
        }
        for f in ["popcount", "clz", "ctz"] {
            assert_eq!(
                ty_of(&format!("a: {n}"), "Nat", &format!("{f}(a)")).unwrap(),
                Type::Nat
            );
        }
        // Operands of different widths are rejected.
        let other = if n == "U8" { "U16" } else { "U8" };
        assert_w0301(&format!("a: {n}, b: {other}"), n, "bit_xor(a, b)");
        // The count must be a Nat: not a fixed-width value, not an Int.
        assert_w0301(&format!("a: {n}, c: U8"), n, "shl(a, c)");
        assert_w0301(&format!("a: {n}, c: Int"), n, "shl(a, c)");
        assert_w0301(&format!("a: {n}"), n, "shl(a, 1u8)");
        assert_eq!(
            ty_of(&format!("a: {n}"), n, "shl(a, 1)").unwrap(),
            t(n),
            "a literal count is a Nat"
        );
    }
    // popcount and friends need a fixed-width operand, not a Nat.
    assert_w0301("n: Nat", "Nat", "popcount(n)");
    assert_w0301("n: Int", "Nat", "clz(n)");
    assert_w0301("n: Nat", "Nat", "bit_not(n)");
    assert_w0301("n: Nat, c: Nat", "Nat", "shl(n, c)");
}

#[test]
fn endian_conversion_is_exact_width_only() {
    for n in NAMES {
        let bytes: u64 = match &n[1..] {
            "8" => 1,
            "16" => 2,
            "32" => 4,
            _ => 8,
        };
        assert_eq!(
            ty_of(
                &format!("a: {n}"),
                &format!("Bytes<{bytes}>"),
                "be_bytes(a)"
            )
            .unwrap(),
            Type::FixedBytes(bytes)
        );
        assert_eq!(
            ty_of(
                &format!("a: {n}"),
                &format!("Bytes<{bytes}>"),
                "le_bytes(a)"
            )
            .unwrap(),
            Type::FixedBytes(bytes)
        );
        let p = format!("b: Bytes<{bytes}>");
        assert_eq!(ty_of(&p, n, &format!("from_be<{n}>(b)")).unwrap(), t(n));
        assert_eq!(ty_of(&p, n, &format!("from_le<{n}>(b)")).unwrap(), t(n));
        // A dynamic Bytes, or a Bytes of the wrong width, is W0301.
        assert_w0301("b: Bytes", n, &format!("from_be<{n}>(b)"));
        assert_w0301(
            &format!("b: Bytes<{}>", bytes + 1),
            n,
            &format!("from_le<{n}>(b)"),
        );
    }
    assert_w0301("b: Bytes<4>", "Bytes<4>", "be_bytes(b)");
}

#[test]
fn arity_and_static_argument_errors_for_every_builtin() {
    let calls: [(&str, usize); 31] = [
        ("widen", 1),
        ("wrap", 1),
        ("try", 1),
        ("to_int", 1),
        ("to_nat", 1),
        ("max_value", 0),
        ("min_value", 0),
        ("checked_add", 2),
        ("checked_sub", 2),
        ("checked_mul", 2),
        ("rem", 2),
        ("div_trunc", 2),
        ("rem_trunc", 2),
        ("unwrap_or", 2),
        ("bit_and", 2),
        ("bit_or", 2),
        ("bit_xor", 2),
        ("bit_not", 1),
        ("shl", 2),
        ("shr", 2),
        ("sar", 2),
        ("rotl", 2),
        ("rotr", 2),
        ("popcount", 1),
        ("clz", 1),
        ("ctz", 1),
        ("bit", 2),
        ("be_bytes", 1),
        ("le_bytes", 1),
        ("from_be", 1),
        ("from_le", 1),
    ];
    let statics = [
        "widen",
        "wrap",
        "try",
        "max_value",
        "min_value",
        "from_be",
        "from_le",
    ];
    for (name, arity) in calls {
        let stat = if statics.contains(&name) { "<U8>" } else { "" };
        for given in [arity + 1, arity.saturating_sub(1)] {
            if given == arity {
                continue;
            }
            let args = vec!["a"; given].join(", ");
            let e = rejected("a: U8", "Bool", &format!("{name}{stat}({args})"));
            assert_eq!(
                e.code,
                DiagnosticCode::W0301,
                "{name} with {given} operands: {e:?}"
            );
        }
        // No call parentheses at all.
        let e = rejected("a: U8", "Bool", &format!("{name}{stat}"));
        assert_eq!(e.code, DiagnosticCode::W0301, "{name} bare: {e:?}");
        if statics.contains(&name) {
            // Missing, symbolic and non-type static arguments.
            for bad in ["", "<5>", "<Bool>", "<Bytes>", "<U8, U8>", "<Int>", "<Nat>"] {
                let args = vec!["a"; arity].join(", ");
                let e = rejected("a: U8", "Bool", &format!("{name}{bad}({args})"));
                assert_eq!(e.code, DiagnosticCode::W0301, "{name}{bad}: {e:?}");
            }
        } else {
            let args = vec!["a"; arity].join(", ");
            let e = rejected("a: U8", "Bool", &format!("{name}<U8>({args})"));
            assert_eq!(
                e.code,
                DiagnosticCode::W0301,
                "{name} takes no static argument"
            );
        }
    }
}

#[test]
fn quantifier_binders_may_have_any_fixed_width_type() {
    for n in NAMES {
        let src = format!(
            "warrant t v1\n  op f(a: {n}) -> {n}\n  labels f:\n    a public\n  claim c:\n    forall x: {n}, y: {n}. f(x) + y == y + f(x)\n"
        );
        check(&src).unwrap_or_else(|e| panic!("{n}: {e:?}"));
    }
}

// ---------------------------------------------------------------------------
// Version scoping (VER04j sections 1.2 and 2.9)
// ---------------------------------------------------------------------------

#[test]
fn the_new_types_and_builtins_exist_only_in_a_v1_document() {
    // v0: the new type names are unbound (W0202); the new built-ins unbound.
    for n in ["U16", "U32", "I8", "I16", "I32", "I64"] {
        let e = check(&model("", &format!("a: {n}"), n, "a")).unwrap_err();
        assert_eq!(e.code, DiagnosticCode::W0202, "{n}");
        assert!(matches!(e.kind, CheckerErrorKind::UnboundType(_)));
    }
    for call in [
        "bit_xor(a, a)",
        "shl(a, 1)",
        "widen<U8>(a)",
        "popcount(a)",
        "wrap<U8>(a)",
        "max_value<U8>()",
    ] {
        let e = check(&model("", "a: U8", "U8", call)).unwrap_err();
        assert_eq!(e.code, DiagnosticCode::W0202, "{call}: {e:?}");
    }
    // A v0 document may define a model with one of the new names and keeps
    // its own meaning; `div` stays reserved.
    for name in ["rem", "shl", "wrap", "try", "bit", "clz", "widen"] {
        let src = format!(
            "warrant t\n  model {name}(a: U8) -> U8 = a\n  model use(a: U8) -> U8 = {name}(a)\n"
        );
        check(&src).unwrap_or_else(|e| panic!("{name}: {e:?}"));
    }
    // v1: defining one of them is a duplicate symbol (W0201); an arm binder
    // may not shadow one (W0203).
    for name in ["rem", "shl", "wrap", "U16", "from_le"] {
        let src = format!("warrant t v1\n  model {name}(a: U8) -> U8 = a\n");
        assert_eq!(
            check(&src).unwrap_err().code,
            DiagnosticCode::W0201,
            "{name}"
        );
    }
    let src = "warrant t v1\n  model m(o: Option<U8>) -> U8 = match o with None -> 0u8 | Some(shl) -> shl\n";
    assert_eq!(check(src).unwrap_err().code, DiagnosticCode::W0203);
    let src =
        "warrant t\n  model m(o: Option<U8>) -> U8 = match o with None -> 0u8 | Some(shl) -> shl\n";
    check(src).expect("a v0 arm binder may be called shl");
}

#[test]
fn v0_typing_is_unchanged_for_u8_and_u64() {
    for header in ["", " v1"] {
        assert_eq!(
            check(&model(header, "a: U8, b: U8", "U8", "a + b * 2u8"))
                .unwrap()
                .models["m"]
                .body_type,
            Type::U8
        );
        assert_eq!(
            check(&model(header, "a: U64", "Bool", "a >= 5u64"))
                .unwrap()
                .models["m"]
                .body_type,
            Type::Bool
        );
        assert_eq!(
            check(&model(header, "a: Nat, b: Int", "Int", "a - b"))
                .unwrap()
                .models["m"]
                .body_type,
            Type::Int
        );
        // Unary minus on an unsigned value was never allowed.
        assert_eq!(
            check(&model(header, "a: U8", "U8", "-a")).unwrap_err().code,
            DiagnosticCode::W0301
        );
        assert_eq!(
            check(&model(header, "a: U8, b: U64", "U8", "a + b"))
                .unwrap_err()
                .code,
            DiagnosticCode::W0301
        );
    }
}

#[test]
fn conversion_operands_must_be_integers() {
    assert_w0301("b: Bool", "U8", "wrap<U8>(b)");
    assert_w0301("b: Bool", "Option<U8>", "try<U8>(b)");
    assert_w0301("b: Bool", "Int", "to_int(b)");
    assert_w0301("a: Int", "Int", "to_int(a)");
    assert_w0301("a: Nat", "Nat", "to_nat(a)");
    assert_w0301("b: Bytes<1>", "U8", "widen<U8>(b)");
}
