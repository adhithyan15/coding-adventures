//! R-LEX-1 to R-LEX-4: the bytes the reader accepts are exactly the bytes the PIF
//! tokens match. One negative test per lexical rule, each asserting the exact error kind,
//! plus the positive spellings that must still be accepted.

mod pif_common;

use lean_kernel::pif::PifError as E;
use pif_common::{file, lib, lib_err, ok, B};

const HEAD: &str = "pif 1\nlogic 1\nprofile native \"t\"\nfeatures\n";

fn with(body: &str, count: usize) -> String {
    format!("{HEAD}{body}end {count}\n")
}

#[test]
fn a_minimal_file_is_accepted() {
    let r = ok(with("", 0).as_bytes(), &lib());
    assert!(r.decls.is_empty());
}

#[test]
fn r_lex_1_whitespace_and_stray_bytes() {
    let cases: &[(&str, String)] = &[
        ("CRLF line ends", with("", 0).replace('\n', "\r\n")),
        ("tab as separator", with("nstr\t1 0 \"a\"\n", 1)),
        ("double space", with("nstr  1 0 \"a\"\n", 1)),
        ("trailing space", with("nstr 1 0 \"a\" \n", 1)),
        ("leading space", with(" nstr 1 0 \"a\"\n", 1)),
        ("blank line in the body", with("\nnstr 1 0 \"a\"\n", 2)),
        ("comment line", with("# note\n", 1)),
        ("no space before a string", with("nstr 1 0\"a\"\n", 1)),
        ("space inside brackets", with("econst 1 0 [ 1 2 ]\n", 1)),
        ("space after the bracket", with("econst 1 0 [1 ]\n", 1)),
        ("doubled space in a list", with("econst 1 0 [1  2]\n", 1)),
        ("no space before a bracket", with("econst 1 0[]\n", 1)),
        ("raw space inside a string", with("nstr 1 0 \"a b\"\n", 1)),
        ("a non-ASCII byte", with("nstr 1 0 \"é\"\n", 1)),
        ("a NUL byte", with("nstr 1 0 \"a\u{0}\"\n", 1)),
        ("a DEL byte", with("nstr 1 0 \"a\u{7f}\"\n", 1)),
        ("leading zero", with("nstr 01 0 \"a\"\n", 1)),
        ("a plus sign", with("nstr +1 0 \"a\"\n", 1)),
        ("an underscore in a numeral", with("nstr 1_0 0 \"a\"\n", 1)),
        ("unknown record keyword", with("nstring 1 0 \"a\"\n", 1)),
        ("a keyword in upper case", with("NSTR 1 0 \"a\"\n", 1)),
        ("a missing field", with("nstr 1 0\n", 1)),
        ("an extra field", with("nstr 1 0 \"a\" 3\n", 1)),
        ("an unterminated string", with("nstr 1 0 \"a\n", 1)),
        ("an unescaped quote inside a string", with("nstr 1 0 \"a\"b\"\n", 1)),
        ("a lone backslash", with("nstr 1 0 \"a\\\"\n", 1)),
        ("an unknown escape", with("nstr 1 0 \"a\\n\"\n", 1)),
        ("an uppercase hex escape", with("nstr 1 0 \"\\u{4A}\"\n", 1)),
        ("a leading-zero escape", with("nstr 1 0 \"\\u{0041}\"\n", 1)),
        ("a seven-digit escape", with("nstr 1 0 \"\\u{1000000}\"\n", 1)),
        ("an unclosed escape", with("nstr 1 0 \"\\u{e9\"\n", 1)),
        ("an empty escape", with("nstr 1 0 \"\\u{}\"\n", 1)),
        ("a bad binder word", with("elam 1 plain 1 1 1\n", 1)),
        ("a missing final newline", with("", 0).trim_end().to_string()),
        ("an empty file", String::new()),
        ("an uppercase digest", with("", 0).replace("features\n", "features\nimport sha256:ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789\n")),
        ("a short digest", with("", 0).replace("features\n", "features\nimport sha256:abcd\n")),
    ];
    for (what, text) in cases {
        assert_eq!(lib_err(text), E::BadLexicalForm, "{what}");
    }
}

#[test]
fn r_lex_1_the_header_is_four_fixed_lines_in_order() {
    let swapped = "logic 1\npif 1\nprofile native \"t\"\nfeatures\nend 0\n";
    assert_eq!(lib_err(swapped), E::BadLexicalForm);
    let no_profile = "pif 1\nlogic 1\nfeatures\nend 0\n";
    assert_eq!(lib_err(no_profile), E::BadLexicalForm);
    let bad_profile_id = "pif 1\nlogic 1\nprofile Native \"t\"\nfeatures\nend 0\n";
    assert_eq!(lib_err(bad_profile_id), E::BadLexicalForm);
    let unquoted = "pif 1\nlogic 1\nprofile native t\nfeatures\nend 0\n";
    assert_eq!(lib_err(unquoted), E::BadLexicalForm);
    let extra = "pif 1 2\nlogic 1\nprofile native \"t\"\nfeatures\nend 0\n";
    assert_eq!(lib_err(extra), E::BadLexicalForm);
}

#[test]
fn r_lex_1_nothing_may_follow_the_end_line() {
    let ok_text = with("", 0);
    for tail in ["\n", " ", "x", "end 0\n", "# c\n"] {
        let text = format!("{ok_text}{tail}");
        assert_eq!(lib_err(&text), E::BadLexicalForm, "{tail:?}");
    }
}

#[test]
fn the_four_binder_words_are_never_identifiers() {
    // the grammar lexes these as binder tokens, so they cannot be a profile id or a feature
    for w in ["default", "implicit", "strict", "inst"] {
        let as_feature = format!("pif 1\nlogic 1\nprofile native \"t\"\nfeatures {w}\nend 0\n");
        assert_eq!(lib_err(&as_feature), E::BadLexicalForm, "{w}");
        let as_profile = format!("pif 1\nlogic 1\nprofile {w} \"t\"\nfeatures\nend 0\n");
        assert_eq!(lib_err(&as_profile), E::BadLexicalForm, "{w}");
    }
    // ... but a longer identifier that merely starts with one is an ordinary identifier
    let longer = "pif 1\nlogic 1\nprofile instance \"t\"\nfeatures\nend 0\n";
    assert_eq!(ok(longer.as_bytes(), &lib()).profile.0, "instance");
}

#[test]
fn r_lex_2_an_escape_must_be_a_scalar_value() {
    for esc in ["\\u{d800}", "\\u{dfff}", "\\u{110000}", "\\u{ffffff}"] {
        let text = with(&format!("nstr 1 0 \"{esc}\"\n"), 1);
        assert_eq!(lib_err(&text), E::BadEscape, "{esc}");
    }
}

#[test]
fn r_lex_3_an_escape_only_for_a_character_with_no_direct_spelling() {
    // printable ASCII (0x21 to 0x7E), including the quote and the backslash
    for esc in ["\\u{41}", "\\u{22}", "\\u{5c}", "\\u{21}", "\\u{7e}"] {
        let text = with(&format!("nstr 1 0 \"{esc}\"\n"), 1);
        assert_eq!(lib_err(&text), E::NonCanonicalEscape, "{esc}");
    }
    // \u{0} is allowed only in an estr literal, never in a name or a profile string
    assert_eq!(
        lib_err(&with("nstr 1 0 \"\\u{0}\"\n", 1)),
        E::NonCanonicalEscape
    );
    let profile = "pif 1\nlogic 1\nprofile native \"\\u{0}\"\nfeatures\nend 0\n";
    assert_eq!(lib_err(profile), E::NonCanonicalEscape);
}

#[test]
fn escapes_that_are_canonical_are_accepted() {
    // a space has no direct spelling; so do non-ASCII characters and DEL
    let r = ok(
        "pif 1\nlogic 1\nprofile native \"a\\u{20}b\"\nfeatures\nnstr 1 0 \"\\u{e9}\"\nnstr 2 0 \"q\\\"\\\\\"\nend 2\n".as_bytes(),
        &lib(),
    );
    assert_eq!(r.profile.1, "a b");
}

#[test]
fn r_lex_4_numbers_fit_their_bounds() {
    // indices, counts and versions are 32-bit
    assert_eq!(
        lib_err(&with("nstr 4294967296 0 \"a\"\n", 1)),
        E::NumberOutOfRange
    );
    assert_eq!(
        lib_err(&with("nstr 1 0 \"a\"\n", 4294967296)),
        E::NumberOutOfRange
    );
    let version = "pif 99999999999999999999999\nlogic 1\nprofile native \"t\"\nfeatures\nend 0\n";
    assert_eq!(lib_err(version), E::NumberOutOfRange);
    // an nnum component is 64-bit: the maximum is fine, one more is not
    let max = "nnum 1 0 18446744073709551615\n";
    ok(with(max, 1).as_bytes(), &lib());
    let over = "nnum 1 0 18446744073709551616\n";
    assert_eq!(lib_err(&with(over, 1)), E::NumberOutOfRange);
    // the largest 32-bit index parses and is then simply not the next index
    assert_eq!(
        lib_err(&with("nstr 4294967295 0 \"a\"\n", 1)),
        E::IndexNotDense
    );
}

#[test]
fn a_bad_reducibility_hint_is_lexical() {
    // a fully valid definition, with only the hint word wrong
    let mut b = B::new();
    let n = b.name("d");
    let prop = b.sort(0);
    let ty = b.pi(n, prop, prop);
    b.def(n, &[], ty, ty, "@fancy");
    assert_eq!(pif_common::err(&b.bytes(), &lib()), E::BadLexicalForm);
}

#[test]
fn a_string_with_a_quote_inside_is_one_component() {
    let r = ok(with("nstr 1 0 \"a\\\"b\"\n", 1).as_bytes(), &lib());
    assert!(r.decls.is_empty());
}

#[test]
fn the_builder_produces_what_the_reader_accepts() {
    // a sanity check on the test producer itself
    let mut b = B::new();
    let (_, _) = b.prop_axiom("A");
    ok(&b.bytes(), &lib());
    assert_eq!(file(&["nstr 1 0 \"a\""]).matches('\n').count(), 6);
}
