use coding_adventures_idl_lexer::{create_idl_lexer, tokenize_idl, try_tokenize_idl};
use lexer::token::TokenType;

fn token_types(source: &str) -> Vec<String> {
    tokenize_idl(source)
        .into_iter()
        .filter(|token| token.type_ != TokenType::Eof)
        .map(|token| token.effective_type_name().to_string())
        .collect()
}

fn token_values(source: &str) -> Vec<String> {
    tokenize_idl(source)
        .into_iter()
        .filter(|token| token.type_ != TokenType::Eof)
        .map(|token| token.value)
        .collect()
}

// ===========================================================================
// `;` line comments (MA12 Â§3/Â§6) -- an ordinary skip: pattern, no hook.
// ===========================================================================

#[test]
fn semicolon_opens_a_comment_to_end_of_line() {
    assert_eq!(
        token_types("x = 1 ; a trailing comment"),
        vec!["NAME", "EQUALS", "NUMBER"]
    );
}

#[test]
fn comment_does_not_swallow_the_terminating_newline() {
    assert_eq!(
        token_types("x = 1 ; a comment\ny = 2"),
        vec!["NAME", "EQUALS", "NUMBER", "NEWLINE", "NAME", "EQUALS", "NUMBER"]
    );
}

#[test]
fn whole_line_comment_between_two_statements() {
    assert_eq!(
        token_types("x = 1\n; a whole comment line\ny = 2"),
        vec!["NAME", "EQUALS", "NUMBER", "NEWLINE", "NEWLINE", "NAME", "EQUALS", "NUMBER"]
    );
}

#[test]
fn comment_containing_characters_outside_the_grammars_alphabet() {
    // A comment's content is arbitrary text and must never be re-lexed as
    // code -- characters this grammar has no token for at all (here `@`
    // and `\`, both out of scope) must not cause a lexer error when they
    // only ever appear inside a comment.
    assert!(try_tokenize_idl("x = 1 ; cost is @5 \\ off\ny = 2").is_ok());
}

// ===========================================================================
// `$` line continuation -- a real token, not stripped (MA12 Â§5).
// ===========================================================================

#[test]
fn dollar_is_its_own_continuation_token_not_silently_stripped() {
    assert_eq!(
        token_types("x = 1 + $\n    2"),
        vec![
            "NAME",
            "EQUALS",
            "NUMBER",
            "PLUS",
            "CONTINUATION",
            "NEWLINE",
            "NUMBER"
        ]
    );
}

// ===========================================================================
// `&` statement separator (MA12 Â§3/Â§4/Â§6).
// ===========================================================================

#[test]
fn ampersand_separates_statements_on_one_line() {
    assert_eq!(
        token_types("x = 1 & y = 2"),
        vec!["NAME", "EQUALS", "NUMBER", "STMT_SEP", "NAME", "EQUALS", "NUMBER"]
    );
}

// ===========================================================================
// Single- and double-quoted strings (MA12 Â§2/Â§4) -- unified STRING type.
// ===========================================================================

#[test]
fn single_quoted_string_literal() {
    assert_eq!(token_types("'hello'"), vec!["STRING"]);
    assert_eq!(token_values("'hello'"), vec!["hello"]);
}

#[test]
fn double_quoted_string_literal() {
    assert_eq!(token_types("\"hello\""), vec!["STRING"]);
    assert_eq!(token_values("\"hello\""), vec!["hello"]);
}

#[test]
fn both_quote_styles_produce_the_same_token_type() {
    assert_eq!(token_types("'flux'"), token_types("\"flux\""));
}

#[test]
fn empty_string_literals_of_both_styles() {
    assert_eq!(token_types("''"), vec!["STRING"]);
    assert_eq!(token_values("''"), vec![String::new()]);
    assert_eq!(token_types("\"\""), vec!["STRING"]);
    assert_eq!(token_values("\"\""), vec![String::new()]);
}

#[test]
fn a_double_quote_may_appear_inside_a_single_quoted_string_and_vice_versa() {
    // No escape mechanism, per the header note in idl.tokens -- but a
    // string delimited by one quote style may freely contain the OTHER
    // quote character verbatim (the real-IDL idiom for embedding a quote).
    assert_eq!(token_values("'say \"hi\"'"), vec!["say \"hi\""]);
    assert_eq!(token_values("\"it's fine\""), vec!["it's fine"]);
}

// ===========================================================================
// Word operators -- comparison and logical (MA12 Â§4), case-insensitive.
// ===========================================================================

#[test]
fn tokenizes_every_comparison_word_operator() {
    assert_eq!(
        token_types("EQ NE LT LE GT GE"),
        vec!["KEYWORD", "KEYWORD", "KEYWORD", "KEYWORD", "KEYWORD", "KEYWORD"]
    );
    assert_eq!(
        token_values("EQ NE LT LE GT GE"),
        vec!["EQ", "NE", "LT", "LE", "GT", "GE"]
    );
}

#[test]
fn tokenizes_every_logical_word_operator() {
    assert_eq!(
        token_types("AND OR NOT XOR"),
        vec!["KEYWORD", "KEYWORD", "KEYWORD", "KEYWORD"]
    );
    assert_eq!(
        token_values("AND OR NOT XOR"),
        vec!["AND", "OR", "NOT", "XOR"]
    );
}

#[test]
fn word_operators_have_no_symbolic_alternative_spelling() {
    // MA12 Â§4 lists ONLY the word forms for IDL's relational operators --
    // no `<`/`<=`/`!=` symbolic spelling is in this cut's scope, and
    // idl.tokens has no LT/LE/NE glyph token at all -- `<` falls through to
    // an honest lex error rather than silently meaning something.
    assert!(try_tokenize_idl("a < b").is_err());
    assert!(try_tokenize_idl("a <= b").is_err());
    // `==` is NOT an error, but not for the reason a MATLAB-family reader
    // might expect: `=` (EQUALS) is a real, single-character token here
    // (assignment/keyword-binding, SECTION 3), so "==" simply lexes as two
    // separate EQUALS tokens -- there is no EQUALS_EQUALS digraph. Whether
    // that is meaningful is a parser-level question this crate does not
    // answer.
    assert_eq!(
        token_types("a == b"),
        vec!["NAME", "EQUALS", "EQUALS", "NAME"]
    );
}

#[test]
fn word_operator_keyword_lookup_is_case_insensitive() {
    assert_eq!(token_types("eq"), vec!["KEYWORD"]);
    assert_eq!(token_values("eq"), vec!["EQ"]);
    assert_eq!(token_values("Eq"), vec!["EQ"]);
    assert_eq!(token_values("and"), vec!["AND"]);
}

#[test]
fn a_comparison_expression_using_word_operators() {
    assert_eq!(
        token_types("IF x EQ 1 THEN y = 2"),
        vec!["KEYWORD", "NAME", "KEYWORD", "NUMBER", "KEYWORD", "NAME", "EQUALS", "NUMBER",]
    );
}

// ===========================================================================
// Control-flow / definition keywords (MA12 Â§3/Â§4/Â§6), case-insensitive.
// ===========================================================================

#[test]
fn tokenizes_if_then_else_endif_endelse() {
    assert_eq!(
        token_types("IF THEN ELSE ENDIF ENDELSE"),
        vec!["KEYWORD", "KEYWORD", "KEYWORD", "KEYWORD", "KEYWORD"]
    );
    assert_eq!(
        token_values("IF THEN ELSE ENDIF ENDELSE"),
        vec!["IF", "THEN", "ELSE", "ENDIF", "ENDELSE"]
    );
}

#[test]
fn tokenizes_for_do_endfor_and_while_endwhile_sharing_do() {
    assert_eq!(token_values("FOR DO ENDFOR"), vec!["FOR", "DO", "ENDFOR"]);
    assert_eq!(
        token_values("WHILE DO ENDWHILE"),
        vec!["WHILE", "DO", "ENDWHILE"]
    );
}

#[test]
fn tokenizes_repeat_until_endrep() {
    assert_eq!(
        token_values("REPEAT UNTIL ENDREP"),
        vec!["REPEAT", "UNTIL", "ENDREP"]
    );
}

#[test]
fn tokenizes_break_and_continue() {
    assert_eq!(token_types("BREAK"), vec!["KEYWORD"]);
    assert_eq!(token_types("CONTINUE"), vec!["KEYWORD"]);
}

#[test]
fn tokenizes_begin_and_generic_end() {
    assert_eq!(token_values("BEGIN END"), vec!["BEGIN", "END"]);
}

#[test]
fn tokenizes_pro_function_return() {
    assert_eq!(
        token_values("PRO FUNCTION RETURN"),
        vec!["PRO", "FUNCTION", "RETURN"]
    );
}

#[test]
fn keywords_are_case_insensitive_but_names_are_not_altered() {
    assert_eq!(token_types("pro"), vec!["KEYWORD"]);
    assert_eq!(token_values("pro"), vec!["PRO"]);
    assert_eq!(token_types("Pro"), vec!["KEYWORD"]);
    assert_eq!(token_values("Pro"), vec!["PRO"]);

    // A non-keyword identifier keeps its EXACT original casing -- only
    // `keywords:`-block lookup folds case, per idl.tokens's own header note.
    assert_eq!(token_types("MyProc"), vec!["NAME"]);
    assert_eq!(token_values("MyProc"), vec!["MyProc"]);
}

// ===========================================================================
// A PRO definition, a FUNCTION definition (MA12 Â§3/Â§4).
// ===========================================================================

#[test]
fn tokenizes_a_pro_definition_header_and_end() {
    // PRO GREET, name
    //   PRINT, name
    // END
    //
    // PRINT is an ordinary library-routine NAME, not a reserved keyword --
    // MA12 Â§4 lists it alongside PLOT/SIN/TOTAL/etc. as an intrinsic
    // procedure/function resolved by name at a later layer, distinct from
    // the closed keyword set Â§3/Â§6 fix (PRO/END/IF/.../EQ/AND/...).
    let src = "PRO GREET, name\n  PRINT, name\nEND";
    assert_eq!(
        token_types(src),
        vec![
            "KEYWORD", "NAME", "COMMA", "NAME", "NEWLINE", "NAME", "COMMA", "NAME", "NEWLINE",
            "KEYWORD",
        ]
    );
}

#[test]
fn tokenizes_a_function_definition_with_return() {
    // FUNCTION DOUBLE, x
    //   RETURN, x*2
    // END
    let src = "FUNCTION DOUBLE, x\n  RETURN, x*2\nEND";
    assert_eq!(
        token_types(src),
        vec![
            "KEYWORD", "NAME", "COMMA", "NAME", "NEWLINE", "KEYWORD", "COMMA", "NAME", "STAR",
            "NUMBER", "NEWLINE", "KEYWORD",
        ]
    );
}

// ===========================================================================
// Procedure call with keyword arguments and the `/KEYWORD` shorthand
// (MA12 Â§3 items 2-3) -- the LEXER'S job is only to emit the ordinary
// token stream; the boolean-shorthand PRODUCTION itself is idl-parser's.
// ===========================================================================

#[test]
fn tokenizes_a_procedure_call_with_keyword_arguments_and_boolean_shorthand() {
    // PLOT, x, TITLE='flux', /YLOG
    let src = "PLOT, x, TITLE='flux', /YLOG";
    assert_eq!(
        token_types(src),
        vec![
            "NAME", "COMMA", "NAME", "COMMA", "NAME", "EQUALS", "STRING", "COMMA", "SLASH", "NAME",
        ]
    );
    assert_eq!(token_values(src)[0], "PLOT");
    assert_eq!(token_values(src)[6], "flux");
    assert_eq!(token_values(src)[9], "YLOG");
}

#[test]
fn slash_before_an_identifier_is_always_plain_division_at_this_layer() {
    // The exact same zero-whitespace "/IDENT" shape as the boolean
    // shorthand above, but in ordinary expression position -- MA12 Â§3
    // item 3 frames telling these apart as a parse-context (grammatical
    // position) concern, not a lexer one, so both must tokenize IDENTICALLY
    // here: SLASH then NAME, with no special-casing. Compare the trailing
    // two tokens of each snippet -- one is a call argument, the other an
    // ordinary division expression, yet the lexer sees the same shape.
    assert_eq!(token_types("a/YLOG"), vec!["NAME", "SLASH", "NAME"]);
    // "PLOT, x, /YLOG"  -> NAME COMMA NAME COMMA SLASH NAME  (6 tokens)
    // "y = a/YLOG"      -> NAME EQUALS NAME SLASH NAME       (5 tokens)
    let call_tail = &token_types("PLOT, x, /YLOG")[4..];
    let division_tail = &token_types("y = a/YLOG")[3..];
    assert_eq!(call_tail, vec!["SLASH", "NAME"]);
    assert_eq!(division_tail, vec!["SLASH", "NAME"]);
    assert_eq!(call_tail, division_tail);
}

// ===========================================================================
// IF...THEN...ENDIF block (MA12 Â§4).
// ===========================================================================

#[test]
fn tokenizes_an_if_then_endif_block() {
    // IF x EQ 1 THEN BEGIN
    //   y = 2
    // ENDIF
    let src = "IF x EQ 1 THEN BEGIN\n  y = 2\nENDIF";
    assert_eq!(
        token_types(src),
        vec![
            "KEYWORD", "NAME", "KEYWORD", "NUMBER", "KEYWORD", "KEYWORD", "NEWLINE", "NAME",
            "EQUALS", "NUMBER", "NEWLINE", "KEYWORD",
        ]
    );
}

// ===========================================================================
// Array literals and subscripting (MA12 Â§4).
// ===========================================================================

#[test]
fn tokenizes_an_array_literal() {
    assert_eq!(
        token_types("[1, 2, 3]"),
        vec!["LBRACKET", "NUMBER", "COMMA", "NUMBER", "COMMA", "NUMBER", "RBRACKET"]
    );
}

#[test]
fn tokenizes_simple_subscripting() {
    assert_eq!(
        token_types("a[0]"),
        vec!["NAME", "LBRACKET", "NUMBER", "RBRACKET"]
    );
}

#[test]
fn tokenizes_negative_from_end_subscripting() {
    // a[-1] -- unary MINUS, resolved at a later layer (MA12 Â§2 note 2).
    assert_eq!(
        token_types("a[-1]"),
        vec!["NAME", "LBRACKET", "MINUS", "NUMBER", "RBRACKET"]
    );
}

#[test]
fn tokenizes_ranged_and_strided_subscripts() {
    assert_eq!(
        token_types("a[s0:s1]"),
        vec!["NAME", "LBRACKET", "NAME", "COLON", "NAME", "RBRACKET"]
    );
    assert_eq!(
        token_types("a[s0:s1:n]"),
        vec!["NAME", "LBRACKET", "NAME", "COLON", "NAME", "COLON", "NAME", "RBRACKET"]
    );
}

#[test]
fn tokenizes_the_all_rest_wildcard_subscript() {
    // a[*] and a[s0:*] -- STAR is reused; the wildcard MEANING is a parser
    // concern, per idl.tokens's own SECTION 3 note.
    assert_eq!(
        token_types("a[*]"),
        vec!["NAME", "LBRACKET", "STAR", "RBRACKET"]
    );
    assert_eq!(
        token_types("a[s0:*]"),
        vec!["NAME", "LBRACKET", "NAME", "COLON", "STAR", "RBRACKET"]
    );
}

#[test]
fn tokenizes_two_dimensional_subscripting() {
    assert_eq!(
        token_types("a[i, j]"),
        vec!["NAME", "LBRACKET", "NAME", "COMMA", "NAME", "RBRACKET"]
    );
}

// ===========================================================================
// Matrix-product operators: the `##` vs `#` longest-match regression
// (MA12 Â§4/Â§6).
// ===========================================================================

#[test]
fn double_hash_wins_over_bare_hash_by_longest_match() {
    assert_eq!(token_types("##"), vec!["HASH_HASH"]);
    assert_eq!(token_types("#"), vec!["HASH"]);
}

#[test]
fn matrix_product_operators_between_operands() {
    assert_eq!(token_types("a # b"), vec!["NAME", "HASH", "NAME"]);
    assert_eq!(token_types("a ## b"), vec!["NAME", "HASH_HASH", "NAME"]);
}

#[test]
fn three_hashes_lex_as_double_hash_then_bare_hash() {
    // A regression on the greedy/first-match interaction: "###" must lex
    // as HASH_HASH then HASH (longest match at each position), never as
    // HASH three times or HASH_HASH swallowing all three characters.
    assert_eq!(token_types("###"), vec!["HASH_HASH", "HASH"]);
}

#[test]
fn four_hashes_lex_as_two_double_hashes() {
    assert_eq!(token_types("####"), vec!["HASH_HASH", "HASH_HASH"]);
}

// ===========================================================================
// Arithmetic and assignment (MA12 Â§4).
// ===========================================================================

#[test]
fn tokenizes_ordinary_arithmetic_operators() {
    assert_eq!(
        token_types("a + b - c * d / e ^ f"),
        vec![
            "NAME", "PLUS", "NAME", "MINUS", "NAME", "STAR", "NAME", "SLASH", "NAME", "CARET",
            "NAME",
        ]
    );
}

#[test]
fn equals_is_one_token_for_both_assignment_and_keyword_binding() {
    // MA12 Â§3 item 2: telling these apart is idl-parser's job, not this
    // lexer's -- both contexts emit the same EQUALS token here.
    assert_eq!(token_types("x = 1"), vec!["NAME", "EQUALS", "NUMBER"]);
    assert_eq!(
        token_types("TITLE='flux'"),
        vec!["NAME", "EQUALS", "STRING"]
    );
}

// ===========================================================================
// Numeric literals (MA12 Â§2/Â§4).
// ===========================================================================

#[test]
fn tokenizes_integers_and_decimals() {
    assert_eq!(token_types("42"), vec!["NUMBER"]);
    assert_eq!(token_values("3.14"), vec!["3.14"]);
}

#[test]
fn tokenizes_leading_dot_floats() {
    assert_eq!(token_types(".5"), vec!["NUMBER"]);
    assert_eq!(token_values(".5"), vec![".5"]);
}

#[test]
fn tokenizes_numbers_with_exponents() {
    assert_eq!(token_values("1e10"), vec!["1e10"]);
    assert_eq!(token_values("1e-5"), vec!["1e-5"]);
    assert_eq!(token_values("2.5E+3"), vec!["2.5E+3"]);
}

#[test]
fn typed_numeric_suffixes_are_not_part_of_this_cuts_number_token() {
    // MA12 Â§2/Â§4 defer IDL's typed numeric tower entirely -- a literal
    // spelled with a type suffix (`5L`) lexes as NUMBER("5") followed by a
    // separate NAME("L"), an honest reflection of the un-typed f64 model
    // rather than a silent guess at what the suffix should mean.
    assert_eq!(token_types("5L"), vec!["NUMBER", "NAME"]);
    assert_eq!(token_values("5L"), vec!["5", "L"]);
}

// ===========================================================================
// Identifiers.
// ===========================================================================

#[test]
fn identifier_must_start_with_a_letter() {
    assert_eq!(token_types("N_ELEMENTS"), vec!["NAME"]);
    assert_eq!(token_values("x9"), vec!["x9"]);
}

#[test]
fn underscore_is_a_valid_continuation_character() {
    assert_eq!(token_types("PTR_NEW"), vec!["NAME"]);
    assert_eq!(token_values("PTR_NEW"), vec!["PTR_NEW"]);
}

#[test]
fn leading_underscore_identifier_is_not_a_valid_name_this_cut() {
    // _EXTRA/_REF_EXTRA (deferred, MA12 Â§4) would need a leading
    // underscore, which NAME does not allow in this cut -- a bare leading
    // `_` has no token at all and falls through to an honest lex error.
    assert!(try_tokenize_idl("_EXTRA").is_err());
}

// ===========================================================================
// Newlines and whitespace.
// ===========================================================================

#[test]
fn newline_is_its_own_significant_token() {
    assert_eq!(
        token_types("x = 1\ny = 2"),
        vec!["NAME", "EQUALS", "NUMBER", "NEWLINE", "NAME", "EQUALS", "NUMBER"]
    );
}

#[test]
fn skips_ordinary_whitespace() {
    assert_eq!(token_types("x   =   1"), vec!["NAME", "EQUALS", "NUMBER"]);
}

// ===========================================================================
// Errors.
// ===========================================================================

#[test]
fn unrecognized_character_is_an_error() {
    // `@`, `?`, `.` (outside a NUMBER), `\`, `{`, `}` are all deferred/out
    // of scope per MA12 Â§4 -- none has a token in this cut's grammar.
    assert!(try_tokenize_idl("a@b").is_err());
    assert!(try_tokenize_idl("a?b").is_err());
    assert!(try_tokenize_idl("s.tag").is_err());
    assert!(try_tokenize_idl("{1:2}").is_err());
}

#[test]
fn tokenize_idl_panics_on_malformed_source() {
    let result = std::panic::catch_unwind(|| tokenize_idl("@"));
    assert!(result.is_err());
}

#[test]
fn create_idl_lexer_exposes_the_result_returning_api() {
    assert!(create_idl_lexer("x = 1").tokenize().is_ok());
    assert!(create_idl_lexer("@").tokenize().is_err());
}

// ===========================================================================
// End-to-end: a small, realistic "textbook IDL session" snippet.
// ===========================================================================

#[test]
fn tokenizes_a_small_end_to_end_snippet() {
    // PRO GREET, name        ; say hello
    //   PRINT, 'Hello, ', name
    // END
    // PRINT is an ordinary library-routine NAME, not a keyword (see
    // `tokenizes_a_pro_definition_header_and_end` above).
    let src = "PRO GREET, name        ; say hello\n  PRINT, 'Hello, ', name\nEND";
    assert_eq!(
        token_types(src),
        vec![
            "KEYWORD", "NAME", "COMMA", "NAME", "NEWLINE", "NAME", "COMMA", "STRING", "COMMA",
            "NAME", "NEWLINE", "KEYWORD",
        ]
    );
    assert_eq!(token_values(src)[0], "PRO");
    assert_eq!(token_values(src)[7], "Hello, ");
}
