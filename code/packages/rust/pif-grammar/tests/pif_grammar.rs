//! Parse tests for the Proof Interchange Format grammar (VER32 section 4).
//!
//! Three kinds of test live here:
//!
//! 1. the grammar files are well formed (they validate and compile);
//! 2. positive cases: the VER32 4.6 worked example byte for byte, and one
//!    minimal file per record kind;
//! 3. negative cases: every LEXICAL rule the grammar enforces (VER32 4.1) and
//!    the structural rules of 4.3, each paired with a good twin so a case
//!    cannot pass by failing for an unrelated reason.
//!
//! Rules VER32 5.3 assigns to the reader (dense indices, backward references,
//! counts, limits, scalar-value and canonical-escape value rules) are NOT
//! grammar rules.  A few tests PIN that the grammar accepts them, so that
//! tightening the grammar later is a deliberate, visible change.

use std::collections::HashSet;

use coding_adventures_pif_grammar::{kind, lex, parse, GRAMMAR, TOKENS};
use grammar_tools::{
    cross_validator::cross_validate,
    parser_grammar::{parse_parser_grammar, validate_parser_grammar},
    token_grammar::{parse_token_grammar, token_names, validate_token_grammar},
};
use parser::grammar_parser::find_nodes;

const SPEC: &str = include_str!("../../../../specs/VER32-proof-interchange-format.md");
const WORKED_EXAMPLE: &str = include_str!("fixtures/worked-example.pif");

/// The four fixed header lines every body below sits under.
const HDR: &str = "pif 1\nlogic 1\nprofile native \"x\"\nfeatures\n";

/// A complete file: the header, the given record lines, and an `end` line.
/// The count on `end` is arbitrary because the grammar never checks it (R-CNT-2
/// is a reader rule).
fn doc(lines: &[&str]) -> String {
    let mut s = String::from(HDR);
    for l in lines {
        s.push_str(l);
        s.push('\n');
    }
    s.push_str("end 1\n");
    s
}

const DIGEST: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

fn assert_accepts(label: &str, src: &str) {
    if let Err(e) = parse(src) {
        panic!("{label}: expected to parse, got {e}\n{src:?}");
    }
}

fn assert_rejects(label: &str, src: &str, stage: &str) {
    match parse(src) {
        Ok(_) => panic!("{label}: expected a {stage} error, but it parsed\n{src:?}"),
        Err(e) => assert!(
            stage == "any" || e.starts_with(stage),
            "{label}: expected a {stage} error, got {e}\n{src:?}"
        ),
    }
}

/// Good twin accepted, bad variant rejected at the named stage.
fn pair(label: &str, good: &str, bad: &str, stage: &str) {
    assert_accepts(&format!("{label} (good twin)"), good);
    assert_rejects(label, bad, stage);
}

// ---------------------------------------------------------------------------
// 1. The grammar files themselves
// ---------------------------------------------------------------------------

#[test]
fn grammar_files_validate_and_cross_validate() {
    let tokens = parse_token_grammar(TOKENS).expect("parse pif.tokens");
    let grammar = parse_parser_grammar(GRAMMAR).expect("parse pif.grammar");
    let issues = validate_token_grammar(&tokens);
    assert!(issues.is_empty(), "token issues: {issues:?}");
    let names: HashSet<String> = token_names(&tokens);
    let issues = validate_parser_grammar(&grammar, Some(&names));
    assert!(issues.is_empty(), "parser issues: {issues:?}");
    // NAME is used only for keyword promotion (pif.tokens), never named by a
    // rule, so the cross-validator warns about exactly that one token.
    let issues: Vec<String> = cross_validate(&tokens, &grammar)
        .into_iter()
        .filter(|i| !i.starts_with("Warning: Token 'NAME'"))
        .collect();
    assert!(issues.is_empty(), "cross-validation issues: {issues:?}");
}

#[test]
fn tokens_file_has_the_vers32_token_set() {
    let tokens = parse_token_grammar(TOKENS).unwrap();
    let names: Vec<&str> = tokens.definitions.iter().map(|d| d.name.as_str()).collect();
    for want in [
        "FDEFAULT",
        "FIMPLICIT",
        "FSTRICT",
        "FINST",
        "FABBREV",
        "FOPAQUE",
        "FREGULAR",
        "LB",
        "RB",
        "NEWLINE",
        "NAME",
        "NAT",
        "FNAT",
        "FSTRING",
        "FLSTRING",
        "FSHA256",
        "FIDENT",
    ] {
        assert!(names.contains(&want), "missing token {want}");
    }
    // The literals must precede FIDENT (VER32 4.2 tie rule).
    let pos = |n: &str| names.iter().position(|x| *x == n).unwrap();
    for lit in ["FDEFAULT", "FIMPLICIT", "FSTRICT", "FINST"] {
        assert!(
            pos(lit) < pos("FIDENT"),
            "{lit} must be listed before FIDENT"
        );
    }
    assert!(pos("FSTRING") < pos("FLSTRING"));
    assert!(pos("FSHA256") < pos("FIDENT"));
    // 32 record keywords.
    assert_eq!(tokens.keywords.len(), 32);
}

// ---------------------------------------------------------------------------
// 2. Positive cases
// ---------------------------------------------------------------------------

/// The fixture is the spec's 4.6 block, byte for byte.
#[test]
fn worked_example_fixture_matches_the_spec() {
    let start = SPEC.find("### 4.6 Worked example").expect("4.6 heading");
    let rest = &SPEC[start..];
    let open = rest.find("\n```\n").expect("opening fence") + "\n```\n".len();
    let close = rest[open..].find("\n```").expect("closing fence") + open;
    let from_spec = format!("{}\n", &rest[open..close]);
    assert_eq!(
        from_spec, WORKED_EXAMPLE,
        "tests/fixtures/worked-example.pif has drifted from VER32 section 4.6; \
         re-extract the first fenced block after the '### 4.6 Worked example' heading"
    );
}

#[test]
fn worked_example_parses() {
    assert!(WORKED_EXAMPLE.is_ascii());
    assert!(!WORKED_EXAMPLE.contains('\r') && !WORKED_EXAMPLE.contains('\t'));
    let tree = parse(WORKED_EXAMPLE).expect("worked example parses");
    assert_eq!(tree.rule_name, "file");

    // 102 lines: 4 header + 97 record lines + `end`.  Two of the record lines'
    // groups are inductive blocks (Bool: 4 lines, Eq: 3 lines), each ONE
    // `record` node, so 97 - 3 - 2 = 92 record nodes.
    let tokens = lex(WORKED_EXAMPLE).unwrap();
    let newlines = tokens.iter().filter(|t| kind(t) == "Newline").count();
    assert_eq!(newlines, 102);
    assert_eq!(find_nodes(&tree, "record").len(), 92);
    assert_eq!(find_nodes(&tree, "inductive").len(), 2);
    assert_eq!(find_nodes(&tree, "itype").len(), 2);
    assert_eq!(find_nodes(&tree, "ictor").len(), 3);
    assert_eq!(find_nodes(&tree, "trailer").len(), 1);

    // The `end` line says 97 and the file really has 97 record lines.
    let end_at = tokens.iter().position(|t| t.value == "end").unwrap();
    assert_eq!(tokens[end_at + 1].value, " 97");
}

#[test]
fn string_tokens_keep_their_spacing_and_escapes() {
    // The profile string spells its space as \u{20}; the token keeps the
    // leading field space, the quotes and the escape exactly as written.
    let tokens = lex(WORKED_EXAMPLE).unwrap();
    let t = tokens
        .iter()
        .find(|t| kind(t) == "FSTRING")
        .expect("an FSTRING");
    assert_eq!(t.value, r#" "pif-case-split\u{20}0.1""#);
}

#[test]
fn header_variants_parse() {
    assert_accepts("no features, no imports", &doc(&[]));
    assert_accepts(
        "features",
        "pif 1\nlogic 1\nprofile lean4 \"p\"\nfeatures lits mutual nested quotient\nend 0\n",
    );
    assert_accepts(
        "imports",
        &format!(
            "pif 1\nlogic 1\nprofile native \"p\"\nfeatures\nimport {DIGEST}\nimport {DIGEST}\nend 0\n"
        ),
    );
    // A keyword is only special at the start of a line: as a field it is an
    // ordinary identifier.
    assert_accepts(
        "keyword spelling as a feature",
        "pif 1\nlogic 1\nprofile end \"p\"\nfeatures end quot\nend 0\n",
    );
}

/// One minimal file per record kind (VER32 4.3).
#[test]
fn every_record_kind_has_a_minimal_file() {
    let cases: &[(&str, &[&str])] = &[
        ("nstr", &["nstr 1 0 \"Bool\""]),
        ("nstr empty component", &["nstr 1 0 \"\""]),
        ("nnum", &["nnum 1 0 5"]),
        ("lsucc", &["lsucc 1 0"]),
        ("lmax", &["lmax 1 0 0"]),
        ("limax", &["limax 1 0 0"]),
        ("lparam", &["lparam 1 1"]),
        ("evar", &["evar 1 0"]),
        ("esort", &["esort 1 0"]),
        ("econst empty list", &["econst 1 1 []"]),
        ("econst one level", &["econst 1 1 [1]"]),
        ("econst two levels", &["econst 1 1 [1 2]"]),
        ("econst many levels", &["econst 1 1 [0 1 2 3 10 11 99]"]),
        ("eapp", &["eapp 1 1 1"]),
        ("elam default", &["elam 1 default 1 1 1"]),
        ("elam implicit", &["elam 1 implicit 1 1 1"]),
        ("elam strict", &["elam 1 strict 1 1 1"]),
        ("elam inst", &["elam 1 inst 1 1 1"]),
        ("epi default", &["epi 1 default 1 1 1"]),
        ("epi implicit", &["epi 1 implicit 1 1 1"]),
        ("epi strict", &["epi 1 strict 1 1 1"]),
        ("epi inst", &["epi 1 inst 1 1 1"]),
        ("elet", &["elet 1 1 1 1 1"]),
        ("enat", &["enat 1 12345678901234567890"]),
        ("enat zero", &["enat 1 0"]),
        ("estr", &["estr 1 \"abc\""]),
        ("estr empty", &["estr 1 \"\""]),
        ("estr with NUL escape", &[r#"estr 1 "a\u{0}b""#]),
        ("estr only NUL escape", &[r#"estr 1 "\u{0}""#]),
        ("eproj", &["eproj 1 1 0 1"]),
        ("axiom", &["axiom 1 [] 1"]),
        ("axiom with params", &["axiom 1 [1 2] 1"]),
        ("def abbrev", &["def 1 [] 1 1 @abbrev"]),
        ("def opaque hint", &["def 1 [] 1 1 @opaque"]),
        ("def regular", &["def 1 [] 1 1 @regular 1"]),
        ("def regular height 0", &["def 1 [1] 1 1 @regular 0"]),
        ("theorem", &["theorem 1 [] 1 1"]),
        ("opaque", &["opaque 1 [] 1 1"]),
        (
            "inductive, one type, no constructors",
            &["inductive [] 0 1", "itype 1 1 0"],
        ),
        (
            "inductive, one type, two constructors",
            &["inductive [] 0 1", "itype 1 1 2", "ictor 2 2", "ictor 3 2"],
        ),
        (
            "inductive, mutual types",
            &[
                "inductive [1] 1 2",
                "itype 1 1 1",
                "ictor 2 2",
                "itype 3 3 0",
            ],
        ),
        ("quot", &["quot"]),
        ("prim", &["prim nat.add 1"]),
        ("prim axiom role", &["prim axiom.propext 1"]),
        ("goal", &["goal \"gates.and_comm\" 1 sha256:bf1fa0a0d828b019965f46efa44ff2627bf54a59558a1bc2959b52c161f74676"]),
    ];
    for (label, lines) in cases {
        assert_accepts(label, &doc(lines));
    }
}

#[test]
fn an_inductive_block_is_one_record_with_its_itypes_and_ictors() {
    let src = doc(&[
        "inductive [] 0 2",
        "itype 1 1 1",
        "ictor 2 2",
        "itype 3 3 2",
        "ictor 4 4",
        "ictor 5 5",
        "nstr 1 0 \"after\"",
    ]);
    let tree = parse(&src).unwrap();
    assert_eq!(find_nodes(&tree, "inductive").len(), 1);
    assert_eq!(find_nodes(&tree, "itype").len(), 2);
    assert_eq!(find_nodes(&tree, "ictor").len(), 3);
    assert_eq!(find_nodes(&tree, "record").len(), 2);
}

#[test]
fn string_escapes_the_grammar_accepts() {
    for (label, lit) in [
        ("plain", r#""abc""#),
        (
            "every punctuation mark but quote and backslash",
            r#""!#$%&'()*+,-./:;<=>?@[]^_`{|}~""#,
        ),
        ("escaped quote", r#""a\"b""#),
        ("escaped backslash", r#""a\\b""#),
        ("escaped space", r#""a\u{20}b""#),
        ("escaped non-ASCII", r#""\u{e9}""#),
        ("escaped astral", r#""\u{1f600}""#),
        ("six digits", r#""\u{10ffff}""#),
        ("escape at the end", r#""a\u{20}""#),
    ] {
        assert_accepts(label, &doc(&[&format!("nstr 1 0 {lit}")]));
    }
}

#[test]
fn a_large_file_parses_without_deep_recursion() {
    let mut s = String::from(HDR);
    for _ in 0..3000 {
        s.push_str("esort 1 0\n");
    }
    s.push_str("end 3000\n");
    assert_accepts("3000 records", &s);
}

// ---------------------------------------------------------------------------
// 3. Negative cases: the lexical rules (VER32 4.1)
// ---------------------------------------------------------------------------

#[test]
fn spacing_between_fields_is_exactly_one_space() {
    pair(
        "double space between fields",
        &doc(&["nstr 1 0 \"a\""]),
        &doc(&["nstr 1  0 \"a\""]),
        "lex",
    );
    pair(
        "triple space",
        &doc(&["nstr 1 0 \"a\""]),
        &doc(&["nstr 1 0   \"a\""]),
        "lex",
    );
    pair(
        "leading space on a line",
        &doc(&["esort 1 0"]),
        &doc(&[" esort 1 0"]),
        "parse",
    );
    pair(
        "trailing space",
        &doc(&["esort 1 0"]),
        &doc(&["esort 1 0 "]),
        "lex",
    );
    pair(
        "no space between keyword and field",
        &doc(&["esort 1 0"]),
        &doc(&["esort1 0"]),
        "parse",
    );
    pair(
        "no space between fields",
        &doc(&["nstr 1 0 \"a\""]),
        &doc(&["nstr 1 0\"a\""]),
        "lex",
    );
    pair(
        "a field squeezed against the previous one",
        &doc(&["nstr 1 0 \"a\""]),
        &doc(&["nstr 1 0 \"a\"\"b\""]),
        "lex",
    );
    pair(
        "trailing space on the end line",
        "pif 1\nlogic 1\nprofile native \"x\"\nfeatures\nend 0\n",
        "pif 1\nlogic 1\nprofile native \"x\"\nfeatures\nend 0 \n",
        "lex",
    );
    pair(
        "trailing space on a header line",
        "pif 1\nlogic 1\nprofile native \"x\"\nfeatures\nend 0\n",
        "pif 1 \nlogic 1\nprofile native \"x\"\nfeatures\nend 0\n",
        "lex",
    );
    pair(
        "trailing space after the bare features keyword",
        "pif 1\nlogic 1\nprofile native \"x\"\nfeatures\nend 0\n",
        "pif 1\nlogic 1\nprofile native \"x\"\nfeatures \nend 0\n",
        "lex",
    );
    pair(
        "double space between features",
        "pif 1\nlogic 1\nprofile native \"x\"\nfeatures lits mutual\nend 0\n",
        "pif 1\nlogic 1\nprofile native \"x\"\nfeatures lits  mutual\nend 0\n",
        "lex",
    );
}

#[test]
fn brackets_are_byte_exact() {
    let good = |l: &str| doc(&[&format!("econst 1 1 {l}")]);
    for g in ["[]", "[1]", "[1 2]"] {
        assert_accepts(g, &good(g));
    }
    for (label, bad) in [
        ("space after the opener", "[ 1]"),
        ("space before the closer", "[1 ]"),
        ("space inside an empty list", "[ ]"),
        ("double space between elements", "[1  2]"),
        ("no space between elements", "[1,2]"),
        ("trailing space after two elements", "[1 2 ]"),
        ("no opener space", "x"),
    ] {
        assert_rejects(label, &good(bad), "any");
    }
    // The opener must carry its field space: "econst 1 1[]" is a lexical error.
    assert_rejects("no space before the opener", &doc(&["econst 1 1[]"]), "lex");
    // A bare numeral is the first element only; a spaced first element is wrong.
    assert_rejects("a spaced first element", &good("[ 1 2]"), "parse");
    // Unclosed and unopened.
    assert_rejects("unclosed", &good("[1 2"), "parse");
    assert_rejects("unopened", &good("1 2]"), "parse");
    assert_rejects("comma separated", &good("[1, 2]"), "lex");
    assert_rejects("nested lists", &good("[[1]]"), "any");
}

#[test]
fn only_lf_and_printable_ascii_occur() {
    let good = doc(&["esort 1 0"]);
    assert_accepts("good twin", &good);
    assert_rejects("tab between fields", &doc(&["esort\t1 0"]), "lex");
    assert_rejects("tab instead of a space", &doc(&["esort 1\t0"]), "lex");
    assert_rejects("tab at the end of a line", &doc(&["esort 1 0\t"]), "lex");
    assert_rejects("tab at the start of a line", &doc(&["\tesort 1 0"]), "lex");
    assert_rejects("CRLF line ending", &good.replace('\n', "\r\n"), "lex");
    assert_rejects(
        "CRLF on the end line only",
        "pif 1\nlogic 1\nprofile native \"x\"\nfeatures\nend 0\r\n",
        "lex",
    );
    assert_rejects("lone CR", &doc(&["esort 1 0\r"]), "lex");
    assert_rejects("CR inside a line", &doc(&["esort 1\r0"]), "lex");
    assert_rejects("vertical tab", &doc(&["esort 1\u{b}0"]), "lex");
    assert_rejects("form feed", &doc(&["esort 1\u{c}0"]), "lex");
    assert_rejects("NUL byte", &doc(&["esort 1 0\u{0}"]), "lex");
    assert_rejects("control byte 0x01", &doc(&["esort 1 0\u{1}"]), "lex");
    assert_rejects("DEL in a string", &doc(&["nstr 1 0 \"a\u{7f}b\""]), "lex");
    assert_rejects(
        "control byte in a string",
        &doc(&["nstr 1 0 \"a\u{1}b\""]),
        "lex",
    );
    assert_rejects("raw tab in a string", &doc(&["nstr 1 0 \"a\tb\""]), "lex");
    assert_rejects("raw space in a string", &doc(&["nstr 1 0 \"a b\""]), "lex");
    assert_rejects("raw space in an estr", &doc(&["estr 1 \"a b\""]), "lex");
    assert_rejects(
        "raw space in a goal id",
        &doc(&[&format!("goal \"a b\" 1 {DIGEST}")]),
        "lex",
    );
    assert_rejects(
        "non-ASCII in a string",
        &doc(&["nstr 1 0 \"caf\u{e9}\""]),
        "lex",
    );
    assert_rejects(
        "non-ASCII emoji in a string",
        &doc(&["nstr 1 0 \"\u{1f600}\""]),
        "lex",
    );
    assert_rejects("non-ASCII in an estr", &doc(&["estr 1 \"\u{e9}\""]), "lex");
    assert_rejects(
        "non-ASCII outside a string",
        &doc(&["esort 1 \u{e9}"]),
        "lex",
    );
    assert_rejects("non-breaking space", &doc(&["esort 1\u{a0}0"]), "lex");
    assert_rejects("UTF-8 BOM", &format!("\u{feff}{good}"), "lex");
    assert_rejects(
        "non-ASCII in the profile string",
        "pif 1\nlogic 1\nprofile native \"\u{e9}\"\nfeatures\nend 0\n",
        "lex",
    );
}

#[test]
fn no_blank_lines_comments_or_bytes_after_end() {
    let hdr = "pif 1\nlogic 1\nprofile native \"x\"\nfeatures\n";
    assert_accepts("good twin", &format!("{hdr}esort 1 0\nend 1\n"));
    assert_rejects(
        "blank line between records",
        &format!("{hdr}esort 1 0\n\nend 1\n"),
        "parse",
    );
    assert_rejects("blank line before end", &format!("{hdr}\nend 0\n"), "parse");
    assert_rejects("blank line at the top", &format!("\n{hdr}end 0\n"), "parse");
    assert_rejects("blank line after end", &format!("{hdr}end 0\n\n"), "parse");
    assert_rejects("comment line", &format!("{hdr}# note\nend 0\n"), "lex");
    assert_rejects(
        "comment after a field",
        &format!("{hdr}esort 1 0 # note\nend 1\n"),
        "lex",
    );
    assert_rejects(
        "bytes after end",
        &format!("{hdr}end 0\nesort 1 0\n"),
        "parse",
    );
    assert_rejects("a second end", &format!("{hdr}end 0\nend 0\n"), "parse");
    assert_rejects(
        "text after end without LF",
        &format!("{hdr}end 0\nx"),
        "parse",
    );
    assert_rejects("no final LF", &format!("{hdr}end 0"), "parse");
    assert_rejects("empty input", "", "parse");
    assert_rejects("only LF", "\n", "parse");
}

#[test]
fn numerals_are_canonical_decimal() {
    let line = |n: &str| doc(&[&format!("enat 1 {n}")]);
    for g in ["0", "7", "10", "1234567890"] {
        assert_accepts(g, &line(g));
    }
    for (label, bad, stage) in [
        ("leading zero", "01", "parse"),
        ("two leading zeros", "007", "parse"),
        ("plus sign", "+1", "lex"),
        ("minus sign", "-1", "lex"),
        ("underscore", "1_000", "lex"),
        ("hex", "0x10", "parse"),
        ("decimal point", "1.5", "lex"),
        ("exponent", "1e3", "parse"),
        ("uppercase digit-like letter", "1A", "lex"),
        ("empty numeral", "", "lex"),
    ] {
        assert_rejects(label, &line(bad), stage);
    }
    // The same for an index, a count and a level.
    assert_rejects("leading zero in an id", &doc(&["esort 01 0"]), "parse");
    assert_rejects(
        "leading zero in a list element",
        &doc(&["econst 1 1 [1 01]"]),
        "parse",
    );
    assert_rejects(
        "leading zero as the first list element",
        &doc(&["econst 1 1 [01]"]),
        "parse",
    );
    assert_rejects(
        "leading zero in the trailer",
        "pif 1\nlogic 1\nprofile native \"x\"\nfeatures\nend 00\n",
        "parse",
    );
    assert_rejects(
        "leading zero in the version",
        "pif 01\nlogic 1\nprofile native \"x\"\nfeatures\nend 0\n",
        "parse",
    );
}

#[test]
fn escapes_follow_the_string_regex() {
    let nstr = |lit: &str| doc(&[&format!("nstr 1 0 {lit}")]);
    for (label, lit) in [
        ("leading-zero escape", r#""\u{0041}""#),
        ("leading zero before one digit", r#""\u{01}""#),
        ("two zeros", r#""\u{00}""#),
        ("uppercase hex", r#""\u{E9}""#),
        ("uppercase hex, mixed", r#""\u{1F600}""#),
        ("seven digits", r#""\u{1000000}""#),
        ("empty braces", r#""\u{}""#),
        ("missing brace", r#""\u{41""#),
        ("missing opener brace", r#""\u41""#),
        ("non-hex digit", r#""\u{g}""#),
        ("sign in the escape", r#""\u{+1}""#),
        ("space inside the escape", r#""\u{ 41}""#),
        ("unknown escape \\n", r#""a\nb""#),
        ("unknown escape \\t", r#""a\tb""#),
        ("unknown escape \\x", r#""\x41""#),
        ("unknown escape \\U", r#""\U{41}""#),
        ("lone trailing backslash", r#""a\""#),
        ("unescaped quote inside", r#""a"b""#),
        ("unterminated", r#""abc"#),
        ("single quotes", "'abc'"),
        ("no quotes", "abc"),
    ] {
        assert_rejects(label, &nstr(lit), "any");
    }
    // Escapes the grammar accepts but the READER rejects (R-LEX-2, R-LEX-3):
    // pinned so that a future tightening is deliberate.
    for (label, lit) in [
        ("surrogate (R-LEX-2)", r#""\u{d800}""#),
        ("surrogate, high end (R-LEX-2)", r#""\u{dfff}""#),
        ("above 10FFFF (R-LEX-2)", r#""\u{110000}""#),
        ("above 10FFFF, all f (R-LEX-2)", r#""\u{ffffff}""#),
        ("escaped printable letter (R-LEX-3)", r#""\u{41}""#),
        ("escaped quote spelled as hex (R-LEX-3)", r#""\u{22}""#),
        ("escaped backslash spelled as hex (R-LEX-3)", r#""\u{5c}""#),
        ("escaped tilde (R-LEX-3)", r#""\u{7e}""#),
    ] {
        assert_accepts(label, &nstr(lit));
    }
}

#[test]
fn nul_escape_is_allowed_only_in_estr() {
    let nul = r#""a\u{0}b""#;
    assert_accepts("estr may spell U+0000", &doc(&[&format!("estr 1 {nul}")]));
    for (label, rec) in [
        ("a name component", format!("nstr 1 0 {nul}")),
        ("a goal id", format!("goal {nul} 1 {DIGEST}")),
        ("the profile string", String::new()),
    ] {
        if rec.is_empty() {
            let src = format!("pif 1\nlogic 1\nprofile native {nul}\nfeatures\nend 0\n");
            assert_rejects(label, &src, "parse");
        } else {
            assert_rejects(label, &doc(&[&rec]), "parse");
        }
    }
    // The lexer makes the distinction: FLSTRING only when \u{0} occurs.
    let toks = lex(&doc(&[&format!("estr 1 {nul}"), r#"estr 2 "plain""#])).unwrap();
    let strs: Vec<String> = toks
        .iter()
        .map(kind)
        .filter(|k| k.ends_with("STRING"))
        .collect();
    // profile "x", then the two estr strings.
    assert_eq!(strs, ["FSTRING", "FLSTRING", "FSTRING"]);
}

#[test]
fn digests_are_sha256_and_64_lowercase_hex() {
    let imp =
        |d: &str| format!("pif 1\nlogic 1\nprofile native \"x\"\nfeatures\nimport {d}\nend 0\n");
    assert_accepts("good twin", &imp(DIGEST));
    for (label, bad) in [
        ("63 digits", &DIGEST[..DIGEST.len() - 1]),
        ("uppercase hex", &DIGEST.replace('0', "A")),
        ("wrong algorithm", "md5:00000000000000000000000000000000"),
        ("missing prefix", &DIGEST[7..]),
        ("sha-256 spelling", &DIGEST.replace("sha256", "sha-256")),
        ("non-hex digit", &DIGEST.replace("00000000", "0000000g")),
    ] {
        assert_rejects(label, &imp(bad), "any");
    }
    // 65 digits lexes as a digest plus a stray digit and is a parse error.
    assert_rejects("65 digits", &imp(&format!("{DIGEST}0")), "parse");
    assert_rejects(
        "two digests on one line",
        &imp(&format!("{DIGEST} {DIGEST}")),
        "parse",
    );
}

// ---------------------------------------------------------------------------
// 4. Negative cases: the keyword / FIDENT tie (VER32 4.2, finding N9)
// ---------------------------------------------------------------------------

#[test]
fn the_four_binder_words_are_never_identifiers() {
    for w in ["default", "implicit", "strict", "inst"] {
        let feats =
            |f: &str| format!("pif 1\nlogic 1\nprofile native \"x\"\nfeatures {f}\nend 0\n");
        assert_accepts("a neighbouring word is a feature", &feats(&format!("{w}x")));
        assert_rejects(&format!("feature {w}"), &feats(w), "parse");
        assert_rejects(
            &format!("feature lits {w}"),
            &feats(&format!("lits {w}")),
            "parse",
        );
        assert_rejects(
            &format!("profile id {w}"),
            &format!("pif 1\nlogic 1\nprofile {w} \"x\"\nfeatures\nend 0\n"),
            "parse",
        );
        assert_rejects(
            &format!("prim role {w}"),
            &doc(&[&format!("prim {w} 1")]),
            "parse",
        );
    }
}

#[test]
fn identifiers_that_start_with_a_binder_word_are_still_identifiers() {
    // First-match-wins lexing would split `instance` into `inst` + `ance`;
    // the FIDENT_LONG entry in pif.tokens prevents that.  These are the
    // identifiers the VER32 4.2 tie rule says are valid.
    for id in [
        "instance",
        "inst.x",
        "inst_2",
        "instant",
        "default2",
        "defaults",
        "default.x",
        "implicitly",
        "implicit.a",
        "strictly",
        "strict_mode",
        "inst0",
        "insta",
    ] {
        let src = format!("pif 1\nlogic 1\nprofile {id} \"x\"\nfeatures {id}\nend 0\n");
        assert_accepts(id, &src);
        assert_accepts(id, &doc(&[&format!("prim {id} 1")]));
        let toks = lex(&src).unwrap();
        let idents: Vec<&str> = toks
            .iter()
            .filter(|t| kind(t) == "FIDENT")
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(idents, [format!(" {id}"), format!(" {id}")], "{id}");
    }
}

#[test]
fn the_binder_word_is_required_where_a_binder_goes() {
    pair(
        "elam binder",
        &doc(&["elam 1 default 1 1 1"]),
        &doc(&["elam 1 instance 1 1 1"]),
        "parse",
    );
    pair(
        "epi binder",
        &doc(&["epi 1 strict 1 1 1"]),
        &doc(&["epi 1 strictly 1 1 1"]),
        "parse",
    );
    pair(
        "binder missing",
        &doc(&["elam 1 default 1 1 1"]),
        &doc(&["elam 1 1 1 1"]),
        "parse",
    );
    pair(
        "binder after wrong field",
        &doc(&["elam 1 default 1 1 1"]),
        &doc(&["elam default 1 1 1 1"]),
        "parse",
    );
    pair(
        "uppercase binder",
        &doc(&["elam 1 inst 1 1 1"]),
        &doc(&["elam 1 INST 1 1 1"]),
        "lex",
    );
    pair(
        "hint where binder goes",
        &doc(&["elam 1 default 1 1 1"]),
        &doc(&["elam 1 @abbrev 1 1 1"]),
        "parse",
    );
}

// ---------------------------------------------------------------------------
// 5. Negative cases: structure (VER32 4.3)
// ---------------------------------------------------------------------------

#[test]
fn the_end_trailer_is_required_and_last() {
    let hdr = "pif 1\nlogic 1\nprofile native \"x\"\nfeatures\n";
    assert_accepts("good twin", &format!("{hdr}end 0\n"));
    assert_rejects("missing end trailer", &format!("{hdr}esort 1 0\n"), "parse");
    assert_rejects("header only", hdr, "parse");
    assert_rejects("end without a count", &format!("{hdr}end\n"), "parse");
    assert_rejects("end with two counts", &format!("{hdr}end 0 0\n"), "parse");
    assert_rejects(
        "end in the middle",
        &format!("{hdr}end 0\nesort 1 0\nend 1\n"),
        "parse",
    );
    assert_rejects("uppercase END", &format!("{hdr}END 0\n"), "lex");
    assert_rejects(
        "end with a string count",
        &format!("{hdr}end \"0\"\n"),
        "parse",
    );
}

#[test]
fn the_header_is_four_fixed_lines_then_imports() {
    let ok = "pif 1\nlogic 1\nprofile native \"x\"\nfeatures\nend 0\n";
    assert_accepts("good twin", ok);
    for (label, bad) in [
        ("missing pif line", "logic 1\nprofile native \"x\"\nfeatures\nend 0\n"),
        ("missing logic line", "pif 1\nprofile native \"x\"\nfeatures\nend 0\n"),
        ("missing profile line", "pif 1\nlogic 1\nfeatures\nend 0\n"),
        ("missing features line", "pif 1\nlogic 1\nprofile native \"x\"\nend 0\n"),
        ("logic before pif", "logic 1\npif 1\nprofile native \"x\"\nfeatures\nend 0\n"),
        ("features before profile", "pif 1\nlogic 1\nfeatures\nprofile native \"x\"\nend 0\n"),
        ("profile without a string", "pif 1\nlogic 1\nprofile native\nfeatures\nend 0\n"),
        ("profile without an id", "pif 1\nlogic 1\nprofile \"x\"\nfeatures\nend 0\n"),
        ("pif without a version", "pif\nlogic 1\nprofile native \"x\"\nfeatures\nend 0\n"),
        ("pif with a string version", "pif \"1\"\nlogic 1\nprofile native \"x\"\nfeatures\nend 0\n"),
        ("uppercase feature", "pif 1\nlogic 1\nprofile native \"x\"\nfeatures Lits\nend 0\n"),
        ("numeric feature", "pif 1\nlogic 1\nprofile native \"x\"\nfeatures 1\nend 0\n"),
        ("duplicate pif line", "pif 1\npif 1\nlogic 1\nprofile native \"x\"\nfeatures\nend 0\n"),
        ("import before features", "pif 1\nlogic 1\nprofile native \"x\"\nimport sha256:0000000000000000000000000000000000000000000000000000000000000000\nfeatures\nend 0\n"),
        ("import without a digest", "pif 1\nlogic 1\nprofile native \"x\"\nfeatures\nimport\nend 0\n"),
    ] {
        assert_rejects(label, bad, "any");
    }
    // `import` is a header line: after the first record it is not allowed.
    assert_rejects(
        "import after a record",
        &format!(
            "pif 1\nlogic 1\nprofile native \"x\"\nfeatures\nesort 1 0\nimport {DIGEST}\nend 1\n"
        ),
        "parse",
    );
}

#[test]
fn records_have_their_exact_field_shapes() {
    let cases: &[(&str, &str, &str)] = &[
        (
            "nstr: component not a string",
            "nstr 1 0 \"a\"",
            "nstr 1 0 5",
        ),
        ("nstr: missing component", "nstr 1 0 \"a\"", "nstr 1 0"),
        ("nstr: extra field", "nstr 1 0 \"a\"", "nstr 1 0 \"a\" 1"),
        (
            "nnum: string where a number goes",
            "nnum 1 0 5",
            "nnum 1 0 \"5\"",
        ),
        ("nnum: too few fields", "nnum 1 0 5", "nnum 1 0"),
        ("lsucc: extra field", "lsucc 1 0", "lsucc 1 0 0"),
        ("lmax: missing field", "lmax 1 0 0", "lmax 1 0"),
        ("limax: missing field", "limax 1 0 0", "limax 1 0"),
        ("lparam: extra field", "lparam 1 1", "lparam 1 1 1"),
        ("evar: extra field", "evar 1 0", "evar 1 0 0"),
        ("esort: missing field", "esort 1 0", "esort 1"),
        ("econst: list missing", "econst 1 1 []", "econst 1 1"),
        ("econst: list not a list", "econst 1 1 []", "econst 1 1 5"),
        ("eapp: missing field", "eapp 1 1 1", "eapp 1 1"),
        (
            "elam: missing body",
            "elam 1 default 1 1 1",
            "elam 1 default 1 1",
        ),
        (
            "epi: extra field",
            "epi 1 default 1 1 1",
            "epi 1 default 1 1 1 1",
        ),
        ("elet: missing field", "elet 1 1 1 1 1", "elet 1 1 1 1"),
        ("enat: string literal", "enat 1 5", "enat 1 \"5\""),
        ("estr: numeric literal", "estr 1 \"a\"", "estr 1 5"),
        ("estr: missing literal", "estr 1 \"a\"", "estr 1"),
        ("eproj: missing field", "eproj 1 1 0 1", "eproj 1 1 0"),
        ("axiom: list missing", "axiom 1 [] 1", "axiom 1 1"),
        ("axiom: type missing", "axiom 1 [] 1", "axiom 1 []"),
        ("def: hint missing", "def 1 [] 1 1 @abbrev", "def 1 [] 1 1"),
        (
            "def: unknown hint",
            "def 1 [] 1 1 @abbrev",
            "def 1 [] 1 1 @inline",
        ),
        (
            "def: @regular without a height",
            "def 1 [] 1 1 @regular 1",
            "def 1 [] 1 1 @regular",
        ),
        (
            "def: @abbrev with a height",
            "def 1 [] 1 1 @abbrev",
            "def 1 [] 1 1 @abbrev 1",
        ),
        (
            "def: hint spelled as an identifier",
            "def 1 [] 1 1 @abbrev",
            "def 1 [] 1 1 abbrev",
        ),
        (
            "theorem: missing value",
            "theorem 1 [] 1 1",
            "theorem 1 [] 1",
        ),
        (
            "theorem: has a hint",
            "theorem 1 [] 1 1",
            "theorem 1 [] 1 1 @abbrev",
        ),
        ("opaque: missing value", "opaque 1 [] 1 1", "opaque 1 [] 1"),
        ("quot: has a field", "quot", "quot 1"),
        ("prim: role missing", "prim nat.add 1", "prim 1"),
        ("prim: constant missing", "prim nat.add 1", "prim nat.add"),
        (
            "prim: role has uppercase",
            "prim nat.add 1",
            "prim Nat.add 1",
        ),
        ("prim: role has a dash", "prim nat.add 1", "prim nat-add 1"),
        (
            "goal: id not a string",
            "goal \"g\" 1 sha256:0000000000000000000000000000000000000000000000000000000000000000",
            "goal g 1 sha256:0000000000000000000000000000000000000000000000000000000000000000",
        ),
        (
            "goal: digest missing",
            "goal \"g\" 1 sha256:0000000000000000000000000000000000000000000000000000000000000000",
            "goal \"g\" 1",
        ),
        ("unknown record kind", "esort 1 0", "etuple 1 0"),
        ("record keyword in uppercase", "esort 1 0", "ESORT 1 0"),
        ("a number where a keyword goes", "esort 1 0", "1 esort 0"),
        ("a header keyword in the body", "esort 1 0", "logic 1"),
    ];
    for (label, good, bad) in cases {
        assert_accepts(&format!("{label} (good twin)"), &doc(&[good]));
        match parse(&doc(&[bad])) {
            Ok(_) => panic!("{label}: expected rejection of {bad:?}"),
            Err(e) => assert!(
                e.starts_with("parse") || e.starts_with("lex"),
                "{label}: {e}"
            ),
        }
    }
}

#[test]
fn inductive_blocks_have_the_right_shape() {
    let good = ["inductive [] 0 1", "itype 1 1 1", "ictor 2 2"];
    assert_accepts("good twin", &doc(&good));
    // Head with no itype.
    assert_rejects("no itype", &doc(&["inductive [] 0 1"]), "parse");
    assert_rejects(
        "no itype, then another record",
        &doc(&["inductive [] 0 1", "esort 1 0"]),
        "parse",
    );
    // itype / ictor outside a block.
    assert_rejects("itype alone", &doc(&["itype 1 1 0"]), "parse");
    assert_rejects("ictor alone", &doc(&["ictor 1 1"]), "parse");
    assert_rejects(
        "ictor before itype",
        &doc(&["inductive [] 0 1", "ictor 2 2", "itype 1 1 1"]),
        "parse",
    );
    // Field shapes.
    assert_rejects(
        "inductive: missing counts",
        &doc(&["inductive []", "itype 1 1 0"]),
        "parse",
    );
    assert_rejects(
        "inductive: no list",
        &doc(&["inductive 0 1", "itype 1 1 0"]),
        "parse",
    );
    assert_rejects(
        "itype: missing field",
        &doc(&["inductive [] 0 1", "itype 1 1"]),
        "parse",
    );
    assert_rejects(
        "ictor: extra field",
        &doc(&["inductive [] 0 1", "itype 1 1 1", "ictor 2 2 2"]),
        "parse",
    );
}

// ---------------------------------------------------------------------------
// 6. Reader-only rules (VER32 5.3): the grammar does NOT enforce these.
//    Pinned so that a future tightening is deliberate and visible.
// ---------------------------------------------------------------------------

#[test]
fn reader_only_rules_are_not_grammar_rules() {
    // R-IDX-1: indices need not be dense or start at 1.
    assert_accepts("sparse index", &doc(&["nstr 7 7 \"a\""]));
    // R-IDX-2: backward references only.
    assert_accepts("forward reference", &doc(&["eapp 1 9 9"]));
    // R-CNT-2: the end count.
    assert_accepts("wrong end count", &format!("{HDR}end 999\n"));
    // R-CNT-1: counts in inductive blocks.
    assert_accepts(
        "wrong itype count",
        &doc(&["inductive [] 0 5", "itype 1 1 9"]),
    );
    // R-LEX-4: numeric bounds, unbounded here.
    assert_accepts(
        "huge numeral",
        &doc(&["enat 1 99999999999999999999999999999999"]),
    );
    assert_accepts("huge index", &doc(&["esort 99999999999999999999 0"]));
    // R-VER-1/2/3: versions, feature names and feature order.
    assert_accepts(
        "unknown version and features, unordered",
        "pif 99\nlogic 99\nprofile whatever \"x\"\nfeatures zzz aaa aaa\nend 0\n",
    );
    // R-FEA-1: a record that needs a feature without declaring it.
    assert_accepts("enat without lits", &doc(&["enat 1 5"]));
    assert_accepts("quot without quotient", &doc(&["quot"]));
    // R-PRM-1, R-PRM-2: unknown and duplicate roles.
    assert_accepts(
        "unknown role, twice",
        &doc(&["prim no.such.role 1", "prim no.such.role 1"]),
    );
    // R-GOL-3: the claim id charset (the string regex is wider).
    assert_accepts(
        "goal id with odd characters",
        &doc(&[&format!("goal \"a!b@c\" 1 {DIGEST}")]),
    );
    // R-NAM-1: name component rules (3.8).
    assert_accepts("dot in a name component", &doc(&["nstr 1 0 \"a.b\""]));
}
