//! The lexer and bracket pairing: what is a token, what is not, and what is an
//! error (fail closed).

use vos_shape_check::lexer::{lex, pair_brackets, Kind, MAX_TOKENS};

fn kinds(src: &str) -> Vec<(Kind, String)> {
    lex(src)
        .unwrap()
        .into_iter()
        .map(|t| (t.kind, t.text))
        .collect()
}

fn texts(src: &str) -> Vec<String> {
    kinds(src).into_iter().map(|(_, t)| t).collect()
}

#[test]
fn identifiers_keywords_and_raw_identifiers() {
    assert_eq!(
        kinds("fn _x1 r#unsafe r#match"),
        vec![
            (Kind::Ident, "fn".into()),
            (Kind::Ident, "_x1".into()),
            (Kind::Ident, "r#unsafe".into()),
            (Kind::Ident, "r#match".into()),
        ]
    );
    let toks = lex("unsafe r#unsafe").unwrap();
    assert!(toks[0].is_ident("unsafe"));
    assert!(
        !toks[1].is_ident("unsafe"),
        "a raw identifier is not the keyword"
    );
    assert!(toks[0].is_ident("unsafe") && !toks[0].is_punct("unsafe"));
    // non-ASCII identifiers
    assert_eq!(texts("caf\u{e9} \u{3b1}"), vec!["caf\u{e9}", "\u{3b1}"]);
}

#[test]
fn comments_are_not_code() {
    assert!(
        lex("// unsafe\n/* unsafe */ /* a /* nested */ b */ /// doc\n//! inner\n")
            .unwrap()
            .is_empty()
    );
    let toks = lex("a /* x\ny */ b").unwrap();
    assert_eq!((toks[0].line, toks[1].line), (1, 2));
    assert_eq!(texts("a//c\nb"), vec!["a", "b"]);
}

#[test]
fn strings_of_every_flavour_are_one_token() {
    let cases = [
        ("\"a b\"", "a b"),
        ("\"q\\\"q\"", "q\\\"q"),
        ("b\"by\"", "by"),
        ("c\"cs\"", "cs"),
        ("r\"raw\\\"", "raw\\"),
        ("r#\"a\"b\"#", "a\"b"),
        ("r##\"a\"#b\"##", "a\"#b"),
        ("br#\"x\"#", "x"),
        ("cr\"y\"", "y"),
        ("\"multi\nline\"", "multi\nline"),
        ("\"esc\\\nnl\"", "esc\\\nnl"),
    ];
    for (src, want) in cases {
        let k = kinds(src);
        assert_eq!(k, vec![(Kind::Str, want.to_string())], "{src:?}");
    }
    // the line counter advances through strings
    let toks = lex("\"a\nb\" x r#\"c\nd\"# y").unwrap();
    assert_eq!(
        toks.iter().map(|t| t.line).collect::<Vec<_>>(),
        vec![1, 2, 2, 3]
    );
}

#[test]
fn characters_and_lifetimes() {
    assert_eq!(kinds("'a'"), vec![(Kind::Char, "a".into())]);
    assert_eq!(kinds("'\\n'"), vec![(Kind::Char, "\\n".into())]);
    assert_eq!(
        kinds("'\\u{1F600}'"),
        vec![(Kind::Char, "\\u{1F600}".into())]
    );
    assert_eq!(kinds("'\\''"), vec![(Kind::Char, "\\'".into())]);
    assert_eq!(kinds("b'x'"), vec![(Kind::Char, "x".into())]);
    assert_eq!(kinds("'a"), vec![(Kind::Lifetime, "'a".into())]);
    assert_eq!(kinds("'static"), vec![(Kind::Lifetime, "'static".into())]);
    assert_eq!(kinds("'_"), vec![(Kind::Lifetime, "'_".into())]);
    assert_eq!(
        kinds("'outer: loop"),
        vec![
            (Kind::Lifetime, "'outer".into()),
            (Kind::Punct, ":".into()),
            (Kind::Ident, "loop".into()),
        ]
    );
    assert_eq!(
        kinds("&'a T"),
        vec![
            (Kind::Punct, "&".into()),
            (Kind::Lifetime, "'a".into()),
            (Kind::Ident, "T".into()),
        ]
    );
}

#[test]
fn numbers_integers_floats_and_tuple_indices() {
    let ints = [
        "0",
        "42",
        "1_000",
        "0xff",
        "0b1010",
        "0o17",
        "1u8",
        "0xE+1".split('+').next().unwrap(),
        "7usize",
    ];
    for s in ints {
        assert_eq!(kinds(s), vec![(Kind::Int, s.to_string())], "{s}");
    }
    for s in [
        "1.5", "0.25", "1e3", "1E3", "1e+3", "2.5e-3", "2f32", "3f64", "1_0.0_1", "0f32",
    ] {
        assert_eq!(kinds(s), vec![(Kind::Float, s.to_string())], "{s}");
    }
    // a range is not a float; a method call on an integer is not a float
    assert_eq!(texts("0..8"), vec!["0", "..", "8"]);
    assert_eq!(texts("1.max(2)"), vec!["1", ".", "max", "(", "2", ")"]);
    // tuple indices: after a single `.` only digits are read
    assert_eq!(
        kinds("t.0.1"),
        vec![
            (Kind::Ident, "t".into()),
            (Kind::Punct, ".".into()),
            (Kind::Int, "0".into()),
            (Kind::Punct, ".".into()),
            (Kind::Int, "1".into()),
        ]
    );
    // an exponent with no digits is a suffix, not part of a float
    assert_eq!(kinds("1e"), vec![(Kind::Int, "1e".into())]);
    assert_eq!(
        kinds("1e+"),
        vec![(Kind::Int, "1e".into()), (Kind::Punct, "+".into())]
    );
}

#[test]
fn punctuation_merging_and_joint_flags() {
    assert_eq!(
        texts("a::b->c=>d..e"),
        vec!["a", "::", "b", "->", "c", "=>", "d", "..", "e"]
    );
    assert_eq!(texts("..="), vec!["..", "="]);
    let t = lex("a<<b > c >> d").unwrap();
    // `<<` : first `<` is joint, second is not (followed by `b`)
    assert!(t[1].joint && !t[2].joint);
    // `>` alone is not joint, `>>` is
    assert!(!t[4].joint);
    assert!(t[6].joint && !t[7].joint);
    // `!=` is `!` joint with `=`
    let t = lex("a != b").unwrap();
    assert!(t[1].joint);
    // a quote after punctuation does not make it joint
    let t = lex("<'a>").unwrap();
    assert!(!t[0].joint);
}

#[test]
fn shebang_and_bom_and_inner_attributes() {
    assert_eq!(texts("#!/usr/bin/env rust-script\nfn"), vec!["fn"]);
    assert_eq!(texts("#![no_std]"), vec!["#", "!", "[", "no_std", "]"]);
    assert_eq!(texts("\u{feff}fn"), vec!["fn"]);
}

#[test]
fn lex_errors_fail_closed() {
    let bad: [(&str, &str); 12] = [
        ("\"open", "unterminated"),
        ("'", "unrecognised"),
        ("'1", "unrecognised"),
        ("'\\", "unterminated"),
        ("/* open", "unterminated block comment"),
        ("/* a /* b */", "unterminated block comment"),
        ("r#\"open", "unterminated raw"),
        ("r##x", "malformed raw"),
        ("r#", "malformed raw"),
        ("b'x", "unterminated"),
        ("\u{2603}", "cannot classify"),
        ("\"a\\", "unterminated"),
    ];
    for (src, why) in bad {
        let e = lex(src).unwrap_err();
        assert!(e.message.contains(why), "{src:?}: {}", e.message);
        assert_eq!(e.line, 1);
    }
    let e = lex("a\nb\n\"open").unwrap_err();
    assert_eq!(e.line, 3);
}

#[test]
fn token_cap_is_enforced() {
    let ok = "a ".repeat(MAX_TOKENS);
    assert_eq!(lex(&ok).unwrap().len(), MAX_TOKENS);
    let too_many = "a ".repeat(MAX_TOKENS + 1);
    let e = lex(&too_many).unwrap_err();
    assert!(e.message.contains("token cap"));
}

#[test]
fn bracket_pairing() {
    let toks = lex("f(a[1]) { }").unwrap();
    let p = pair_brackets(&toks).unwrap();
    assert_eq!(p[1], 6);
    assert_eq!(p[6], 1);
    assert_eq!(p[3], 5);
    assert_eq!(p[0], usize::MAX);
    for (src, why) in [
        (")", "unmatched"),
        ("(]", "closes"),
        ("{ (", "unclosed"),
        ("[ ( ] )", "closes"),
    ] {
        let toks = lex(src).unwrap();
        let e = pair_brackets(&toks).unwrap_err();
        assert!(e.message.contains(why), "{src}: {}", e.message);
    }
    // generics and comparison operators are not brackets
    let toks = lex("a < b > c").unwrap();
    assert!(pair_brackets(&toks).is_ok());
}
