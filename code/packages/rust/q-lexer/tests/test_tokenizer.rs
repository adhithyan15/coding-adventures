use coding_adventures_q_lexer::{create_q_lexer, tokenize_q, try_tokenize_q};
use lexer::token::TokenType;

fn token_types(source: &str) -> Vec<String> {
    tokenize_q(source)
        .into_iter()
        .filter(|token| token.type_ != TokenType::Eof)
        .map(|token| token.effective_type_name().to_string())
        .collect()
}

fn token_values(source: &str) -> Vec<String> {
    tokenize_q(source)
        .into_iter()
        .filter(|token| token.type_ != TokenType::Eof)
        .map(|token| token.value)
        .collect()
}

// ===========================================================================
// Primitive verb glyphs -- MA11 §4's table, checked one row at a time.
// ===========================================================================

#[test]
fn tokenizes_every_primitive_verb_glyph() {
    // + - * % ! , # _ & | ~ in MA11 §4's listed order. Space-separated so
    // none of the whitespace-sensitive rules (MINUS, REDUCE) kick in.
    assert_eq!(
        token_types("+ - * % ! , # _ & | ~"),
        vec![
            "PLUS", "MINUS", "STAR", "PERCENT", "BANG", "COMMA", "HASH", "UNDERSCORE", "AMP",
            "PIPE", "TILDE",
        ]
    );
}

#[test]
fn tokenizes_all_six_comparison_glyphs() {
    assert_eq!(
        token_types("= < > <= >= <>"),
        vec!["EQ", "LT", "GT", "LE", "GE", "NE"]
    );
}

#[test]
fn not_equal_is_spelled_angle_brackets_not_tilde_equals_or_hash() {
    // MA11 §4's explicit callout: Q spells not-equal `<>`, never `~=` (that's
    // MATLAB/Scilab's spelling) and never `#` (Q's own count/take glyph).
    assert_eq!(token_types("<>"), vec!["NE"]);
    // `~=` lexes as TWO separate tokens in this grammar (TILDE then EQ) --
    // there is no NE_ALT-style digraph for it, unlike Scilab's own `<>`+`~=`
    // pair, because Q genuinely only has one spelling.
    assert_eq!(token_types("~="), vec!["TILDE", "EQ"]);
    // `#` is its own unrelated primitive (count/take), never comparison.
    assert_eq!(token_types("#"), vec!["HASH"]);
}

#[test]
fn le_ge_digraphs_win_over_bare_lt_gt() {
    // The core longest-match-first hazard: `<=`/`>=` must win over the bare
    // `<`/`>` prefix character they start with, at every position.
    assert_eq!(token_types("<="), vec!["LE"]);
    assert_eq!(token_types("<"), vec!["LT"]);
    assert_eq!(token_types(">="), vec!["GE"]);
    assert_eq!(token_types(">"), vec!["GT"]);
}

// ===========================================================================
// Adverbs, assignment, grouping, function-literal delimiters
// ===========================================================================

#[test]
fn tokenizes_the_three_adverbs() {
    // ' (each) / (over/reduce) \ (scan) -- MA11 §4.
    assert_eq!(token_types("'"), vec!["EACH"]);
    assert_eq!(token_types("+/"), vec!["PLUS", "REDUCE"]);
    assert_eq!(token_types("+\\"), vec!["PLUS", "SCAN"]);
}

#[test]
fn tokenizes_colon_assignment_not_j_or_apl_spelling() {
    // Q spells assignment with a colon (`name:expr`) -- not J's `=.`/`=:`
    // pair and not APL's `←`.
    assert_eq!(token_types("x:1"), vec!["NAME", "COLON", "NUMBER"]);
}

#[test]
fn tokenizes_parenthesised_grouping() {
    assert_eq!(
        token_types("(1+2)"),
        vec!["LPAREN", "NUMBER", "PLUS", "NUMBER", "RPAREN"]
    );
}

#[test]
fn tokenizes_explicit_list_literal_with_semicolons() {
    // (a;b;c) -- MA11 §3 bullet 3's second list-literal syntax.
    assert_eq!(
        token_types("(1;2;3)"),
        vec![
            "LPAREN", "NUMBER", "SEMICOLON", "NUMBER", "SEMICOLON", "NUMBER", "RPAREN",
        ]
    );
}

#[test]
fn tokenizes_function_literal_delimiters_with_explicit_params() {
    // {[x;y] x+y} -- MA11 §3 bullet 1 / §4.
    assert_eq!(
        token_types("{[x;y] x+y}"),
        vec![
            "LBRACE", "LBRACKET", "NAME", "SEMICOLON", "NAME", "RBRACKET", "NAME", "PLUS", "NAME",
            "RBRACE",
        ]
    );
}

#[test]
fn tokenizes_function_literal_with_implicit_params() {
    // {x+y} -- the bracket-omitted implicit x/y/z form (MA11 §4). At the
    // lexer layer x/y/z are ordinary NAME tokens -- the implicit-parameter
    // convenience is a parser/runtime concern, not a lexer one.
    assert_eq!(
        token_types("{x+y}"),
        vec!["LBRACE", "NAME", "PLUS", "NAME", "RBRACE"]
    );
}

#[test]
fn semicolon_separates_statements_inside_a_function_body_too() {
    // The SAME token, two grammar-level uses (MA11 §3 bullets 1 and 3):
    // list-literal separator AND function-body statement separator.
    assert_eq!(
        token_types("{a:1;a+1}"),
        vec![
            "LBRACE", "NAME", "COLON", "NUMBER", "SEMICOLON", "NAME", "PLUS", "NUMBER", "RBRACE",
        ]
    );
}

// ===========================================================================
// Numeric literals and stranding
// ===========================================================================

#[test]
fn tokenizes_integers_and_decimals() {
    assert_eq!(token_types("42"), vec!["NUMBER"]);
    assert_eq!(token_values("3.14"), vec!["3.14"]);
}

#[test]
fn tokenizes_numbers_with_exponents() {
    // The exponent's own sign is written directly into NUMBER's pattern
    // (unambiguous -- see q.tokens SECTION 5) unlike the leading sign.
    assert_eq!(token_values("1e10"), vec!["1e10"]);
    assert_eq!(token_values("1e-5"), vec!["1e-5"]);
    assert_eq!(token_values("2.5E+3"), vec!["2.5E+3"]);
}

#[test]
fn tokenizes_dense_numeric_arrays_via_vector_stranding() {
    // Adjacent numeric literals separated only by whitespace -- MA11 §3
    // bullet 3's first list-literal syntax. Grouping into one array value is
    // the parser's job; the lexer just emits adjacent NUMBER tokens.
    assert_eq!(token_types("1 2 3"), vec!["NUMBER", "NUMBER", "NUMBER"]);
    assert_eq!(token_values("1 2 3"), vec!["1", "2", "3"]);
}

#[test]
fn name_excludes_underscore_since_underscore_is_its_own_primitive() {
    // `_` is the floor/drop verb glyph (UNDERSCORE), not a name-continuation
    // character -- `a_b` must lex as NAME("a") UNDERSCORE NAME("b"), never
    // as a single identifier, or a real primitive application would be
    // silently swallowed into a name (q.tokens SECTION 5).
    assert_eq!(token_types("a_b"), vec!["NAME", "UNDERSCORE", "NAME"]);
    assert_eq!(token_values("a_b"), vec!["a", "_", "b"]);
}

// ===========================================================================
// Whitespace-sensitive rule 1: negative-literal vs. subtraction
// (MA11 §3 bullet 2, first item)
// ===========================================================================

#[test]
fn space_before_minus_none_after_folds_to_negative_literal() {
    // 2 -1 -- the two-element strand [2, -1].
    assert_eq!(token_types("2 -1"), vec!["NUMBER", "NUMBER"]);
    assert_eq!(token_values("2 -1"), vec!["2", "-1"]);
}

#[test]
fn space_on_both_sides_of_minus_is_subtraction() {
    // 2 - 1 -- ordinary subtraction, both operands spaced away from the verb.
    assert_eq!(token_types("2 - 1"), vec!["NUMBER", "MINUS", "NUMBER"]);
}

#[test]
fn no_space_at_all_is_also_subtraction() {
    // 2-1 -- glued on both sides. This is the case that rules out "no space
    // after the minus" as the whole story: it must still read as ordinary
    // subtraction (real q evaluates 2-1 to 1), not as NUMBER(2) NUMBER(-1).
    assert_eq!(token_types("2-1"), vec!["NUMBER", "MINUS", "NUMBER"]);
    assert_eq!(token_values("2-1"), vec!["2", "-", "1"]);
}

#[test]
fn space_after_minus_only_is_subtraction() {
    // 2- 1 -- there IS a space between - and the digit, so the "no
    // intervening space" precondition already fails regardless of what
    // precedes the minus.
    assert_eq!(token_types("2- 1"), vec!["NUMBER", "MINUS", "NUMBER"]);
}

#[test]
fn negative_literal_at_start_of_line() {
    // A bare `-1` with nothing before it -- the start of input trivially
    // counts as "a position where a new list-stranding element may start".
    assert_eq!(token_types("-1"), vec!["NUMBER"]);
    assert_eq!(token_values("-1"), vec!["-1"]);
}

#[test]
fn negative_literal_after_assignment_colon() {
    // x:-1 -- COLON is not a noun-ending token, so there is no ambiguity to
    // resolve via spacing at all; this is an extremely common real-q idiom.
    assert_eq!(token_types("x:-1"), vec!["NAME", "COLON", "NUMBER"]);
    assert_eq!(token_values("x:-1"), vec!["x", ":", "-1"]);
}

#[test]
fn negative_literal_after_open_paren_and_semicolon() {
    // (-1;2) -- inside an explicit list literal, `-1` opens a new element
    // right after `(`, and `2` is an ordinary positive element after `;`.
    assert_eq!(
        token_types("(-1;2)"),
        vec!["LPAREN", "NUMBER", "SEMICOLON", "NUMBER", "RPAREN"]
    );
    assert_eq!(token_values("(-1;2)"), vec!["(", "-1", ";", "2", ")"]);
}

#[test]
fn subtraction_glued_to_a_closing_paren() {
    // (2+3)-1 -- RPAREN glued directly to `-` blocks the fold (ordinary
    // subtraction of 1 from the parenthesised result).
    assert_eq!(
        token_types("(2+3)-1"),
        vec!["LPAREN", "NUMBER", "PLUS", "NUMBER", "RPAREN", "MINUS", "NUMBER"]
    );
}

#[test]
fn negative_literal_stranded_after_a_spaced_closing_paren() {
    // (2+3) -1 -- a space before `-` means RPAREN is NOT glued to it, so
    // this strands a second, negative element after the parenthesised value.
    assert_eq!(
        token_types("(2+3) -1"),
        vec!["LPAREN", "NUMBER", "PLUS", "NUMBER", "RPAREN", "NUMBER"]
    );
    assert_eq!(token_values("(2+3) -1"), vec!["(", "2", "+", "3", ")", "-1"]);
}

#[test]
fn subtraction_glued_to_a_name() {
    // f-1 -- NAME glued directly to `-` blocks the fold, same as a NUMBER.
    assert_eq!(token_types("f-1"), vec!["NAME", "MINUS", "NUMBER"]);
}

#[test]
fn negative_literal_stranded_after_a_spaced_name() {
    assert_eq!(token_types("f -1"), vec!["NAME", "NUMBER"]);
}

#[test]
fn chained_minus_signs_resolve_independently() {
    // 1 - -1 -- the FIRST `-` has a space on both sides (stays MINUS); the
    // SECOND `-` is glued to `1` with a space before it and its "previous
    // token" is the first (un-folded) MINUS, which is not noun-ending, so it
    // folds. Net: 1 - (-1).
    assert_eq!(
        token_types("1 - -1"),
        vec!["NUMBER", "MINUS", "NUMBER"]
    );
    assert_eq!(token_values("1 - -1"), vec!["1", "-", "-1"]);
}

#[test]
fn three_element_negative_strand() {
    // 2 -1 -2 -- MA11 §3's own worked example, extended to three elements.
    assert_eq!(
        token_types("2 -1 -2"),
        vec!["NUMBER", "NUMBER", "NUMBER"]
    );
    assert_eq!(token_values("2 -1 -2"), vec!["2", "-1", "-2"]);
}

#[test]
fn negative_literal_after_a_function_literal_value() {
    // {x+1} -1 -- RBRACE is a noun-ending token (a lambda is itself an
    // ordinary noun, MA11 §3 bullet 1), so the same spacing rule applies to
    // it as to RPAREN/NUMBER/NAME.
    assert_eq!(
        token_types("{x+1} -1"),
        vec!["LBRACE", "NAME", "PLUS", "NUMBER", "RBRACE", "NUMBER"]
    );
    assert_eq!(
        token_types("{x+1}-1"),
        vec!["LBRACE", "NAME", "PLUS", "NUMBER", "RBRACE", "MINUS", "NUMBER"]
    );
}

#[test]
fn negative_decimal_and_exponent_literals_fold_too() {
    assert_eq!(token_values("x: -3.5"), vec!["x", ":", "-3.5"]);
    assert_eq!(token_values("2 -1e10"), vec!["2", "-1e10"]);
}

// ===========================================================================
// Whitespace-sensitive rule 2: `/` comment marker vs. REDUCE adverb
// (MA11 §3 bullet 2, second item)
// ===========================================================================

#[test]
fn reduce_glued_to_a_preceding_verb_with_no_space() {
    // +/x -- the canonical sum-reduce idiom.
    assert_eq!(token_types("+/x"), vec!["PLUS", "REDUCE", "NAME"]);
}

#[test]
fn reduce_glued_to_a_preceding_noun_with_no_space() {
    assert_eq!(token_types("x/"), vec!["NAME", "REDUCE"]);
}

#[test]
fn comment_at_start_of_line() {
    // A `/` at the very start of a line opens a comment to end of line.
    assert_eq!(token_types("/ comment to end of line"), Vec::<String>::new());
}

#[test]
fn comment_preceded_by_a_single_space() {
    assert_eq!(token_types("x:1 / trailing comment"), vec!["NAME", "COLON", "NUMBER"]);
}

#[test]
fn comment_preceded_by_multiple_spaces_behaves_the_same_as_one() {
    // Presence/absence of whitespace is what matters, not how much of it --
    // one space and five spaces before `/` must both open a comment.
    assert_eq!(
        token_types("x:1     / trailing comment"),
        vec!["NAME", "COLON", "NUMBER"]
    );
    assert_eq!(
        token_types("x:1 / trailing comment"),
        token_types("x:1     / trailing comment"),
    );
}

#[test]
fn reduce_immediately_after_a_closing_paren_with_no_space() {
    // (+/)y or similar -- a `/` glued directly to `)` is still the reduce
    // adverb, never a comment, because there is no space before it.
    assert_eq!(
        token_types("(+/)y"),
        vec!["LPAREN", "PLUS", "REDUCE", "RPAREN", "NAME"]
    );
}

#[test]
fn comment_after_a_closing_paren_with_a_preceding_space() {
    // The same `)` followed by `/`, but now with a space before the slash --
    // this opens a comment instead.
    assert_eq!(token_types("(1+2) / a comment"), vec!["LPAREN", "NUMBER", "PLUS", "NUMBER", "RPAREN"]);
}

#[test]
fn comment_does_not_swallow_the_terminating_newline() {
    // The comment ends just before the newline -- NEWLINE is still its own
    // significant token afterward, exactly like j.tokens' NB. comments.
    assert_eq!(
        token_types("x:1 / a comment\ny:2"),
        vec!["NAME", "COLON", "NUMBER", "NEWLINE", "NAME", "COLON", "NUMBER"]
    );
}

#[test]
fn whole_line_comment_between_two_statements() {
    assert_eq!(
        token_types("x:1\n/ a whole comment line\ny:2"),
        vec!["NAME", "COLON", "NUMBER", "NEWLINE", "NEWLINE", "NAME", "COLON", "NUMBER"]
    );
}

#[test]
fn comment_containing_characters_outside_the_grammars_alphabet() {
    // A comment's content is arbitrary text -- it must never be re-lexed as
    // code, so characters this grammar has no token for at all (here: `$`
    // and `@`, both out of scope/deferred per MA11 §4) must not cause a
    // lexer error when they only ever appear inside a comment.
    assert!(try_tokenize_q("x:1 / cost is $5 @ 10% off\ny:2").is_ok());
    assert_eq!(
        token_types("x:1 / cost is $5 @ 10% off\ny:2"),
        vec!["NAME", "COLON", "NUMBER", "NEWLINE", "NAME", "COLON", "NUMBER"]
    );
}

#[test]
fn reduce_scan_and_each_all_respect_the_same_comment_rule() {
    // The comment-vs-adverb distinction is specific to `/` (REDUCE) -- EACH
    // (') and SCAN (\) have no such ambiguity and are unaffected either way.
    assert_eq!(token_types("+\\ / a comment"), vec!["PLUS", "SCAN"]);
    assert_eq!(token_types("+' / a comment"), vec!["PLUS", "EACH"]);
}

// ===========================================================================
// Newlines and whitespace
// ===========================================================================

#[test]
fn newline_is_its_own_significant_token() {
    assert_eq!(
        token_types("x:1\ny:2"),
        vec!["NAME", "COLON", "NUMBER", "NEWLINE", "NAME", "COLON", "NUMBER"]
    );
}

#[test]
fn skips_ordinary_whitespace() {
    assert_eq!(token_types("x   :   1"), vec!["NAME", "COLON", "NUMBER"]);
}

// ===========================================================================
// Errors
// ===========================================================================

#[test]
fn unrecognized_character_is_an_error() {
    // `@`, `?`, `.`, and `$` are all deferred whole per MA11 §4 -- none of
    // them has a token in this cut's grammar.
    assert!(try_tokenize_q("a@b").is_err());
    assert!(try_tokenize_q("a?b").is_err());
    assert!(try_tokenize_q("a.b").is_err());
    assert!(try_tokenize_q("a$b").is_err());
}

#[test]
fn tokenize_q_panics_on_malformed_source() {
    let result = std::panic::catch_unwind(|| tokenize_q("@"));
    assert!(result.is_err());
}

#[test]
fn create_q_lexer_exposes_the_result_returning_api() {
    assert!(create_q_lexer("x:1").tokenize().is_ok());
    assert!(create_q_lexer("@").tokenize().is_err());
}

// ===========================================================================
// End-to-end: a small, realistic "textbook q session" snippet
// ===========================================================================

#[test]
fn tokenizes_a_small_end_to_end_snippet() {
    // x:2 -1        / a two-element strand, second element negative
    // sum:{[a] +/a} / a function literal summing its argument
    // sum x
    let src = "x:2 -1\nsum:{[a] +/a}\nsum x";
    assert_eq!(
        token_types(src),
        vec![
            "NAME", "COLON", "NUMBER", "NUMBER", "NEWLINE", "NAME", "COLON", "LBRACE", "LBRACKET",
            "NAME", "RBRACKET", "PLUS", "REDUCE", "NAME", "RBRACE", "NEWLINE", "NAME", "NAME",
        ]
    );
    assert_eq!(token_values(src)[2], "2");
    assert_eq!(token_values(src)[3], "-1");
}
