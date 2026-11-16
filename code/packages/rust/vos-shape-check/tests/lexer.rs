//! The Rust lexer wrapper: what the RS01 grammar lexer's tokens become, the
//! two RS01 hooks (shebang, block comments), multi-document batches, and what
//! is an error (fail closed).  Most cases are lexed together with ONE grammar
//! lexer (`lex_documents`), because building one costs about 0.7 s in a debug
//! build.

use vos_shape_check::lexer::{
    lex_documents, pair_brackets, Edition, Kind, LexError, Token, MAX_TOKENS,
};

fn many(srcs: &[&str]) -> Vec<Result<Vec<Token>, LexError>> {
    lex_documents(srcs, Edition::E2021)
}

fn kinds(t: &[Token]) -> Vec<(Kind, String)> {
    t.iter().map(|t| (t.kind, t.text.clone())).collect()
}

fn texts(t: &[Token]) -> Vec<String> {
    t.iter().map(|t| t.text.clone()).collect()
}

fn ok(r: &Result<Vec<Token>, LexError>) -> &Vec<Token> {
    r.as_ref().unwrap_or_else(|e| panic!("{e:?}"))
}

#[test]
fn identifiers_keywords_and_raw_identifiers() {
    let r = many(&[
        "fn _x1 r#unsafe r#match r#unwrap r#f32 r#usize",
        "unsafe r#unsafe",
        "caf\u{e9} \u{3b1}",
    ]);
    // a raw NON-keyword is the plain identifier; a raw keyword keeps `r#`
    assert_eq!(
        kinds(ok(&r[0])),
        vec![
            (Kind::Ident, "fn".into()),
            (Kind::Ident, "_x1".into()),
            (Kind::Ident, "r#unsafe".into()),
            (Kind::Ident, "r#match".into()),
            (Kind::Ident, "unwrap".into()),
            (Kind::Ident, "f32".into()),
            (Kind::Ident, "usize".into()),
        ]
    );
    let t = ok(&r[1]);
    assert!(t[0].is_ident("unsafe") && !t[1].is_ident("unsafe"));
    assert!(!t[0].is_punct("unsafe"));
    assert_eq!(texts(ok(&r[2])), vec!["caf\u{e9}", "\u{3b1}"]);
}

#[test]
fn comments_are_not_code_and_lines_are_per_file() {
    let r = many(&[
        "// unsafe\n/* unsafe */ /* a /* nested */ b */ /// doc\n//! inner\n/** d */ /*! e */ /**/ /***/ x",
        "a /* x\ny */ b",
        "a//c\nb",
        "line1\nline2\n\nline4 /* \n\n */ line6",
    ]);
    assert_eq!(texts(ok(&r[0])), vec!["x"]);
    let t = ok(&r[1]);
    assert_eq!((t[0].line, t[1].line), (1, 2));
    assert_eq!(texts(ok(&r[2])), vec!["a", "b"]);
    let lines: Vec<u32> = ok(&r[3]).iter().map(|t| t.line).collect();
    assert_eq!(lines, vec![1, 2, 4, 6], "every file's lines start at 1");
}

#[test]
fn strings_of_every_flavour_are_one_token_with_their_inner_text() {
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
    ];
    let srcs: Vec<&str> = cases.iter().map(|c| c.0).collect();
    let r = many(&srcs);
    for ((src, want), got) in cases.iter().zip(&r) {
        assert_eq!(
            kinds(ok(got)),
            vec![(Kind::Str, want.to_string())],
            "{src:?}"
        );
    }
    let r = many(&["\"a\nb\" x r#\"c\nd\"# y"]);
    let lines: Vec<u32> = ok(&r[0]).iter().map(|t| t.line).collect();
    assert_eq!(lines, vec![1, 2, 2, 3]);
}

#[test]
fn characters_and_lifetimes() {
    let r = many(&[
        "'a'",
        "'\\n'",
        "'\\u{1F600}'",
        "'\\''",
        "b'x'",
        "'a",
        "'static",
        "'_",
        "'outer: loop",
        "&'a T",
        "'\"'",
        "'\\\\'",
        "'/'",
        "'{'",
    ]);
    let want: Vec<Vec<(Kind, &str)>> = vec![
        vec![(Kind::Char, "a")],
        vec![(Kind::Char, "\\n")],
        vec![(Kind::Char, "\\u{1F600}")],
        vec![(Kind::Char, "\\'")],
        vec![(Kind::Char, "x")],
        vec![(Kind::Lifetime, "'a")],
        vec![(Kind::Lifetime, "'static")],
        vec![(Kind::Lifetime, "'_")],
        vec![
            (Kind::Lifetime, "'outer"),
            (Kind::Punct, ":"),
            (Kind::Ident, "loop"),
        ],
        vec![
            (Kind::Punct, "&"),
            (Kind::Lifetime, "'a"),
            (Kind::Ident, "T"),
        ],
        vec![(Kind::Char, "\"")],
        vec![(Kind::Char, "\\\\")],
        vec![(Kind::Char, "/")],
        vec![(Kind::Char, "{")],
    ];
    for (got, want) in r.iter().zip(want) {
        let w: Vec<(Kind, String)> = want.into_iter().map(|(k, s)| (k, s.to_string())).collect();
        assert_eq!(kinds(ok(got)), w);
    }
}

#[test]
fn numbers_integers_floats_and_tuple_indices() {
    let ints = [
        "0", "42", "1_000", "0xff", "0b1010", "0o17", "1u8", "7usize", "0x1f32",
    ];
    let floats = [
        "1.5", "0.25", "1e3", "1E3", "1e+3", "2.5e-3", "2f32", "3f64", "1_0.0_1", "0f32", "1_f64",
        "1.0_f32",
    ];
    let mut srcs: Vec<&str> = ints.to_vec();
    srcs.extend(floats);
    let r = many(&srcs);
    for (s, got) in srcs.iter().zip(&r) {
        let want = if floats.contains(s) {
            Kind::Float
        } else {
            Kind::Int
        };
        assert_eq!(kinds(ok(got)), vec![(want, s.to_string())], "{s}");
    }
    let r = many(&[
        "0..8", "1.max(2)", "t.0.1", "1.;", "(1.)", "1. foo", "1..=2", "x.0.", "1.e",
    ]);
    assert_eq!(texts(ok(&r[0])), vec!["0", "..", "8"]);
    assert_eq!(texts(ok(&r[1])), vec!["1", ".", "max", "(", "2", ")"]);
    assert_eq!(
        kinds(ok(&r[2])),
        vec![
            (Kind::Ident, "t".into()),
            (Kind::Punct, ".".into()),
            (Kind::Int, "0".into()),
            (Kind::Punct, ".".into()),
            (Kind::Int, "1".into())
        ]
    );
    // a trailing-dot float is a float, but not a tuple index or a range
    assert_eq!(kinds(ok(&r[3]))[0], (Kind::Float, "1.".into()));
    assert_eq!(kinds(ok(&r[4]))[1], (Kind::Float, "1.".into()));
    assert_eq!(kinds(ok(&r[5]))[0], (Kind::Float, "1.".into()));
    assert_eq!(texts(ok(&r[6])), vec!["1", "..", "=", "2"]);
    assert_eq!(texts(ok(&r[7])), vec!["x", ".", "0", "."]);
    assert_eq!(texts(ok(&r[8])), vec!["1", ".", "e"]);
}

#[test]
fn punctuation_merging_and_joint_flags() {
    let r = many(&[
        "a::b->c=>d..e",
        "..=",
        "a<<b > c >> d",
        "a != b",
        "<'a>",
        "f::<T>()",
    ]);
    assert_eq!(
        texts(ok(&r[0])),
        vec!["a", "::", "b", "->", "c", "=>", "d", "..", "e"]
    );
    assert_eq!(texts(ok(&r[1])), vec!["..", "="]);
    let t = ok(&r[2]);
    // `<<`: first `<` joint, second not; `>` alone not joint; `>>` joint
    assert!(t[1].joint && !t[2].joint && !t[4].joint && t[6].joint && !t[7].joint);
    assert!(ok(&r[3])[1].joint, "`!=` is `!` joint with `=`");
    assert!(!ok(&r[4])[0].joint, "a lifetime is not punctuation");
    assert_eq!(texts(ok(&r[5])), vec!["f", "::", "<", "T", ">", "(", ")"]);
}

#[test]
fn shebang_bom_frontmatter_and_inner_attributes() {
    let r = many(&[
        "#!/usr/bin/env rust-script\nfn",
        "#![no_std]",
        "\u{feff}fn",
        "#! [allow(dead_code)] unsafe fn evil() {}",
        "#!/* c */[a]",
        "#!\n// line\n[a]",
        "#!/* /* n */ */ [a]",
        "#! /** doc */ [a]\nfn x",
        "#! /// doc\n[a]\nfn y",
        "---\nfn",
    ]);
    assert_eq!(texts(ok(&r[0])), vec!["fn"]);
    assert_eq!(texts(ok(&r[1])), vec!["#", "!", "[", "no_std", "]"]);
    assert_eq!(texts(ok(&r[2])), vec!["fn"]);
    // `#!` then `[` (past whitespace and non-doc comments) is an attribute
    for i in [3, 4, 5, 6] {
        assert_eq!(&texts(ok(&r[i]))[..3], ["#", "!", "["], "case {i}");
    }
    // a DOC comment after `#!` does not count as skipped: it is a shebang line
    assert_eq!(texts(ok(&r[7])), vec!["fn", "x"]);
    assert_eq!(texts(ok(&r[8]))[0], "[");
    assert!(r[9].is_err(), "frontmatter is refused");
}

#[test]
fn lex_errors_fail_closed() {
    let bad = [
        "\"open",
        "/* open",
        "1e",
        "r#crate",
        "r\"x",
        "/** a /* b */ c */",
    ];
    // A refused file does not hide the others, up to three refused files per
    // batch (each refusal costs one more lexer, and the attempts are bounded).
    for chunk in bad.chunks(3) {
        let mut docs: Vec<&str> = vec!["pub fn ok() {}"];
        docs.extend(chunk);
        docs.push("pub fn also_ok() {}");
        let r = many(&docs);
        assert_eq!(
            texts(ok(&r[0])),
            vec!["pub", "fn", "ok", "(", ")", "{", "}"]
        );
        assert_eq!(texts(ok(r.last().unwrap()))[2], "also_ok");
        for (src, got) in docs[1..docs.len() - 1].iter().zip(&r[1..]) {
            let e = got.as_ref().expect_err(src);
            assert!(e.line >= 1, "{src}");
        }
    }
    // past the attempt bound the remaining files are reported as not lexed
    // (fail closed), never as clean
    let mut docs: Vec<&str> = bad[..5].to_vec();
    docs.push("pub fn late() {}");
    let r = many(&docs);
    assert!(r.iter().all(Result::is_err), "{r:?}");
}

#[test]
fn comment_depth_is_capped_at_256() {
    let deep_ok = format!("{} x {}", "/*".repeat(200), "*/".repeat(200));
    let deep_bad = format!("{} x {}", "/*".repeat(300), "*/".repeat(300));
    let r = many(&[&deep_ok, &deep_bad]);
    assert!(ok(&r[0]).is_empty());
    assert!(r[1].as_ref().unwrap_err().message.contains("256"));
}

#[test]
fn many_documents_share_one_lexer_but_not_their_state() {
    let docs = ["fn a() {}", "\"never closed", "fn c() {}\n\nfn d() {}"];
    let r = many(&docs);
    assert!(r[0].is_ok() && r[2].is_ok());
    assert!(r[1].is_err());
    assert_eq!(
        ok(&r[2]).iter().map(|t| t.line).collect::<Vec<_>>(),
        vec![1, 1, 1, 1, 1, 1, 3, 3, 3, 3, 3, 3]
    );
    // an unterminated string cannot swallow the next file's first line
    let r = many(&["pub fn f() { \"x", "pub fn g() {}"]);
    assert!(r[0].is_err() && r[1].is_ok());
}

#[test]
fn a_forged_or_swallowed_boundary_fails_the_whole_batch() {
    // the boundary identifier appearing inside a file changes the expected set
    let forged = "__vos_shape_check_file_boundary__";
    let r = many(&[forged, "fn a() {}"]);
    assert!(r.iter().all(Result::is_err), "{r:?}");
    let r = many(&["fn a() {}", "fn b() {}", &format!("fn c() {{ {forged}; }}")]);
    assert!(r.iter().all(Result::is_err));
}

#[test]
fn the_edition_selects_the_lexical_pair() {
    // `gen` is a reserved word in edition 2024 only (RS01 section 2.1)
    let r21 = lex_documents(&["fn gen() {}"], Edition::E2021);
    assert!(r21[0].is_ok());
    let r24 = lex_documents(&["fn gen() {}"], Edition::E2024);
    assert_eq!(texts(ok(&r24[0]))[1], "gen");
}

#[test]
fn bracket_pairing() {
    let r = many(&["f(a[1]) { }", ")", "(]", "{ (", "[ ( ] )", "a < b > c"]);
    let p = pair_brackets(ok(&r[0])).unwrap();
    assert_eq!((p[1], p[6], p[3], p[0]), (6, 1, 5, usize::MAX));
    for (i, why) in [
        (1, "unmatched"),
        (2, "closes"),
        (3, "unclosed"),
        (4, "closes"),
    ] {
        let e = pair_brackets(ok(&r[i])).unwrap_err();
        assert!(e.message.contains(why), "{i}: {}", e.message);
    }
    assert!(pair_brackets(ok(&r[5])).is_ok());
}

#[test]
fn token_cap_is_enforced() {
    let too_many = "a ".repeat(MAX_TOKENS + 1);
    let r = many(&[&too_many]);
    assert!(r[0].as_ref().unwrap_err().message.contains("token cap"));
}

#[test]
fn a_nested_comment_inside_a_doc_block_comment_is_refused_by_the_grammar() {
    // The adapter counts depth for doc comments too, so it no longer relies on
    // this; the pin documents that the RS01 pair (divergence RD4) refuses it.
    let r = many(&[
        "/** a /* b */ c */ x",
        "/*! a /* b */ c */ x",
        "/** a */ x",
        "/*! a */ y",
    ]);
    assert!(r[0].is_err() && r[1].is_err());
    assert_eq!(texts(ok(&r[2])), vec!["x"]);
    assert_eq!(texts(ok(&r[3])), vec!["y"]);
}
