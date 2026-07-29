use coding_adventures_axiom_lexer::{create_axiom_lexer, tokenize_axiom, try_tokenize_axiom};
use lexer::token::TokenType;

fn token_types(source: &str) -> Vec<String> {
    tokenize_axiom(source)
        .into_iter()
        .filter(|token| token.type_ != TokenType::Eof)
        .map(|token| token.effective_type_name().to_string())
        .collect()
}

fn token_values(source: &str) -> Vec<String> {
    tokenize_axiom(source)
        .into_iter()
        .filter(|token| token.type_ != TokenType::Eof)
        .map(|token| token.value)
        .collect()
}

// ===========================================================================
// Integer and float literals (MA13 §4).
// ===========================================================================

#[test]
fn tokenizes_an_integer_literal() {
    assert_eq!(token_types("123"), vec!["NUMBER"]);
    assert_eq!(token_values("123"), vec!["123"]);
}

#[test]
fn tokenizes_a_float_literal() {
    assert_eq!(token_types("1.5"), vec!["NUMBER"]);
    assert_eq!(token_values("1.5"), vec!["1.5"]);
}

#[test]
fn tokenizes_numbers_with_exponents() {
    assert_eq!(token_values("1e10"), vec!["1e10"]);
    assert_eq!(token_values("2.5E+3"), vec!["2.5E+3"]);
    assert_eq!(token_values("1e-5"), vec!["1e-5"]);
}

#[test]
fn number_requires_a_leading_digit() {
    // A bare `.5` is not a number -- matches every sibling symbolic-family
    // lexer's convention (reduce/derive/maple.tokens). `.` has no token of
    // its own in this cut's grammar, so it falls through to an honest
    // lex error.
    assert!(try_tokenize_axiom(".5").is_err());
}

// ===========================================================================
// Rational spelling: `1/3` is ordinary division, NOT a dedicated rational
// literal token (MA13 §4's own confirmed finding).
// ===========================================================================

#[test]
fn rational_literal_spelling_is_three_ordinary_tokens() {
    assert_eq!(token_types("1/3"), vec!["NUMBER", "SLASH", "NUMBER"]);
    assert_eq!(token_values("1/3"), vec!["1", "/", "3"]);
}

#[test]
fn rational_coercion_combines_ordinary_division_with_coerce() {
    // `3 :: Fraction Integer` -- MA13 §3's own worked coercion example.
    assert_eq!(
        token_types("3 :: Fraction Integer"),
        vec!["NUMBER", "COERCE", "NAME", "NAME"]
    );
}

// ===========================================================================
// String literals (MA13 §4).
// ===========================================================================

#[test]
fn tokenizes_a_string_literal() {
    assert_eq!(token_types("\"hello\""), vec!["STRING"]);
    assert_eq!(token_values("\"hello\""), vec!["hello"]);
}

#[test]
fn tokenizes_an_empty_string_literal() {
    assert_eq!(token_types("\"\""), vec!["STRING"]);
    assert_eq!(token_values("\"\""), vec![String::new()]);
}

#[test]
fn string_literal_containing_ordinary_grammar_characters() {
    assert_eq!(
        token_values("\"a := b :: c\""),
        vec!["a := b :: c"]
    );
}

// ===========================================================================
// Symbols/identifiers (MA13 §4) -- domain/category names are ordinary NAMEs.
// ===========================================================================

#[test]
fn tokenizes_plain_identifiers() {
    assert_eq!(token_types("x foo f"), vec!["NAME", "NAME", "NAME"]);
    assert_eq!(token_values("x foo f"), vec!["x", "foo", "f"]);
}

#[test]
fn builtin_domain_and_category_names_are_ordinary_names_not_keywords() {
    // MA13 §3/§4: the fixed built-in domain/category table is an
    // axiom-runtime-internal (MA-13d) concern, invisible here -- every one
    // of these lexes as a plain NAME, never a KEYWORD.
    for name in [
        "Integer",
        "Boolean",
        "Float",
        "String",
        "Fraction",
        "Polynomial",
        "List",
        "PositiveInteger",
        "NonNegativeInteger",
        "Ring",
        "OrderedSet",
    ] {
        assert_eq!(token_types(name), vec!["NAME"], "expected NAME for {name}");
    }
}

#[test]
fn identifiers_are_case_sensitive() {
    // Unlike idl.tokens's `@case_insensitive true`, Axiom keeps its
    // default case-sensitive behavior (MA13 §4: if/then/else/has are
    // lowercase; IF/THEN/ELSE/HAS in uppercase are ordinary NAMEs, not
    // keywords).
    assert_eq!(token_types("IF"), vec!["NAME"]);
    assert_eq!(token_types("Has"), vec!["NAME"]);
}

// ===========================================================================
// Parens, brackets, comma (MA13 §4).
// ===========================================================================

#[test]
fn tokenizes_parens_and_a_function_call_shape() {
    assert_eq!(
        token_types("f(a, b)"),
        vec!["NAME", "LPAREN", "NAME", "COMMA", "NAME", "RPAREN"]
    );
}

#[test]
fn tokenizes_a_list_literal_with_square_brackets() {
    assert_eq!(
        token_types("[a, b, c]"),
        vec!["LBRACKET", "NAME", "COMMA", "NAME", "COMMA", "NAME", "RBRACKET"]
    );
}

#[test]
fn single_argument_paren_optional_call_is_just_two_names() {
    // `factorial 7` -- MA13 §4's paren-optional single-argument call; at
    // the lexer level this is simply NAME NUMBER, with call-vs-juxtaposition
    // disambiguation left entirely to the parser (MA-13c).
    assert_eq!(token_types("factorial 7"), vec!["NAME", "NUMBER"]);
}

// ===========================================================================
// Arithmetic operators, both power spellings (MA13 §4).
// ===========================================================================

#[test]
fn tokenizes_ordinary_arithmetic_operators() {
    assert_eq!(
        token_types("a + b - c * d / e"),
        vec![
            "NAME", "PLUS", "NAME", "MINUS", "NAME", "TIMES", "NAME", "SLASH", "NAME",
        ]
    );
}

#[test]
fn both_power_spellings_are_real_and_distinct_tokens() {
    // MA13 §4: `^` and `**` are both real, confirmed spellings for power
    // (mirroring Reduce's own CARET/POW pair, MA08 §3) -- kept as distinct
    // token types; the parser collapses them onto one production.
    assert_eq!(token_types("a ^ b"), vec!["NAME", "CARET", "NAME"]);
    assert_eq!(token_types("a ** b"), vec!["NAME", "POW", "NAME"]);
}

#[test]
fn pow_wins_over_bare_times_by_longest_match() {
    // Regression: `**` must lex as ONE POW token, never two TIMES tokens
    // or TIMES+TIMES -- POW is declared before TIMES in axiom.tokens for
    // exactly this reason.
    assert_eq!(token_types("**"), vec!["POW"]);
    assert_eq!(token_types("*"), vec!["TIMES"]);
    assert_eq!(token_types("***"), vec!["POW", "TIMES"]);
    assert_eq!(token_types("****"), vec!["POW", "POW"]);
}

// ===========================================================================
// Comparison operators (MA13 §3/§4) -- `~=` is Axiom's real not-equal
// spelling, NOT Maple's `<>` or Wolfram's `!=`.
// ===========================================================================

#[test]
fn equality_lowers_to_a_single_eq_token() {
    assert_eq!(token_types("a = b"), vec!["NAME", "EQ", "NAME"]);
}

#[test]
fn not_equal_is_spelled_tilde_equals() {
    assert_eq!(token_types("a ~= b"), vec!["NAME", "NE", "NAME"]);
}

#[test]
fn maple_and_wolfram_not_equal_spellings_are_not_recognized_as_one_token() {
    // `<>` is Maple's spelling (not Axiom's) -- it must NOT lex as a single
    // NE-like token here; it lexes as LESS then GREATER, two tokens.
    assert_eq!(token_types("a <> b"), vec!["NAME", "LESS", "GREATER", "NAME"]);
    // `!=` is Wolfram's spelling -- `!` has no token in this grammar at all.
    assert!(try_tokenize_axiom("a != b").is_err());
}

#[test]
fn tokenizes_ordering_comparisons() {
    assert_eq!(token_types("a < b"), vec!["NAME", "LESS", "NAME"]);
    assert_eq!(token_types("a <= b"), vec!["NAME", "LE", "NAME"]);
    assert_eq!(token_types("a > b"), vec!["NAME", "GREATER", "NAME"]);
    assert_eq!(token_types("a >= b"), vec!["NAME", "GE", "NAME"]);
}

#[test]
fn le_and_ge_win_over_bare_less_and_greater_by_longest_match() {
    assert_eq!(token_types("<="), vec!["LE"]);
    assert_eq!(token_types(">="), vec!["GE"]);
    assert_eq!(token_types("<"), vec!["LESS"]);
    assert_eq!(token_types(">"), vec!["GREATER"]);
}

// ===========================================================================
// Assignment `:=`, function-definition `==`, and the declaration/coercion
// colon family `:` / `::` -- the three genuinely new tokens (MA13 §3).
// ===========================================================================

#[test]
fn tokenizes_immediate_assignment() {
    assert_eq!(token_types("a := 3"), vec!["NAME", "ASSIGN", "NUMBER"]);
}

#[test]
fn tokenizes_a_held_body_function_definition() {
    // f(x: T, ...): T == e -- MA13 §4's function-definition row.
    let src = "f(x: T): T == x + 1";
    assert_eq!(
        token_types(src),
        vec![
            "NAME", "LPAREN", "NAME", "COLON", "NAME", "RPAREN", "COLON", "NAME", "DEFINE",
            "NAME", "PLUS", "NUMBER",
        ]
    );
}

#[test]
fn tokenizes_an_undeclared_function_definition() {
    // Undeclared `f x == e` form (MA13 §4).
    assert_eq!(
        token_types("f x == x * x"),
        vec!["NAME", "NAME", "DEFINE", "NAME", "TIMES", "NAME"]
    );
}

#[test]
fn tokenizes_a_plain_declaration() {
    // a : PositiveInteger -- MA13 §3/§4's declaration row.
    assert_eq!(
        token_types("a : PositiveInteger"),
        vec!["NAME", "COLON", "NAME"]
    );
}

#[test]
fn tokenizes_a_tuple_declaration() {
    // (a, b, c) : T -- MA13 §4's tuple-declaration row.
    assert_eq!(
        token_types("(a, b, c) : T"),
        vec![
            "LPAREN", "NAME", "COMMA", "NAME", "COMMA", "NAME", "RPAREN", "COLON", "NAME",
        ]
    );
}

#[test]
fn tokenizes_a_coercion_expression() {
    assert_eq!(
        token_types("e :: T"),
        vec!["NAME", "COERCE", "NAME"]
    );
}

#[test]
fn colon_family_longest_match_regression() {
    // `:=`, `::`, and bare `:` must never be confused with one another --
    // ASSIGN and COERCE are both declared before bare COLON in
    // axiom.tokens for exactly this reason.
    assert_eq!(token_types(":="), vec!["ASSIGN"]);
    assert_eq!(token_types("::"), vec!["COERCE"]);
    assert_eq!(token_types(":"), vec!["COLON"]);
    assert_eq!(token_types(":::"), vec!["COERCE", "COLON"]);
    // "::=" is COERCE ("::") followed by only a single "=" character
    // remaining -- not enough characters left to form a second two-char
    // operator (":=" or "=="), so it falls through to the single-char EQ,
    // not ASSIGN.
    assert_eq!(token_types("::="), vec!["COERCE", "EQ"]);
    assert_eq!(token_types(":=:"), vec!["ASSIGN", "COLON"]);
}

#[test]
fn define_wins_over_bare_eq_by_longest_match() {
    assert_eq!(token_types("=="), vec!["DEFINE"]);
    assert_eq!(token_types("="), vec!["EQ"]);
    assert_eq!(token_types("==="), vec!["DEFINE", "EQ"]);
    assert_eq!(token_types("===="), vec!["DEFINE", "DEFINE"]);
}

// ===========================================================================
// `has` category-membership query (MA13 §3/§4).
// ===========================================================================

#[test]
fn tokenizes_a_has_query() {
    // Polynomial(Integer) has Ring -- MA13 §3's own worked example, `true`.
    assert_eq!(
        token_types("Polynomial(Integer) has Ring"),
        vec!["NAME", "LPAREN", "NAME", "RPAREN", "KEYWORD", "NAME"]
    );
    assert_eq!(token_values("Polynomial(Integer) has Ring")[4], "has");
}

#[test]
fn tokenizes_a_false_has_query() {
    // List(Integer) has Ring -- MA13 §4's own confirmed `false` example.
    assert_eq!(
        token_types("List(Integer) has Ring"),
        vec!["NAME", "LPAREN", "NAME", "RPAREN", "KEYWORD", "NAME"]
    );
}

#[test]
fn has_is_case_sensitive_like_every_other_keyword() {
    assert_eq!(token_types("has"), vec!["KEYWORD"]);
    // Uppercase `Has`/`HAS` are ordinary NAMEs -- Axiom keeps the default
    // case-sensitive behavior (no `@case_insensitive` directive), unlike
    // idl.tokens's own case-insensitive keyword lookup.
    assert_eq!(token_types("Has"), vec!["NAME"]);
    assert_eq!(token_types("HAS"), vec!["NAME"]);
}

// ===========================================================================
// `if`/`then`/`else` conditional keywords (MA13 §4).
// ===========================================================================

#[test]
fn tokenizes_an_if_then_else_conditional() {
    assert_eq!(
        token_types("if a > b then a else b"),
        vec!["KEYWORD", "NAME", "GREATER", "NAME", "KEYWORD", "NAME", "KEYWORD", "NAME"]
    );
}

#[test]
fn keywords_are_exactly_the_closed_four_word_set() {
    for kw in ["if", "then", "else", "has"] {
        assert_eq!(token_types(kw), vec!["KEYWORD"], "expected KEYWORD({kw})");
        assert_eq!(token_values(kw), vec![kw.to_string()]);
    }
}

// ===========================================================================
// Semicolon-separated block (MA13 §4).
// ===========================================================================

#[test]
fn tokenizes_a_parenthesised_semicolon_separated_block() {
    assert_eq!(
        token_types("(a := 1; b := 2; a + b)"),
        vec![
            "LPAREN", "NAME", "ASSIGN", "NUMBER", "SEMI", "NAME", "ASSIGN", "NUMBER", "SEMI",
            "NAME", "PLUS", "NAME", "RPAREN",
        ]
    );
}

// ===========================================================================
// `--` line comments (MA13 §4's task-scoped addition, per FriCAS/SPAD
// convention).
// ===========================================================================

#[test]
fn double_dash_opens_a_comment_to_end_of_line() {
    assert_eq!(
        token_types("a := 1 -- a trailing comment"),
        vec!["NAME", "ASSIGN", "NUMBER"]
    );
}

#[test]
fn comment_does_not_prevent_lexing_the_next_line() {
    assert_eq!(
        token_types("a := 1 -- comment\nb := 2"),
        vec!["NAME", "ASSIGN", "NUMBER", "NAME", "ASSIGN", "NUMBER"]
    );
}

#[test]
fn whole_line_comment_between_two_statements() {
    assert_eq!(
        token_types("a := 1\n-- a whole comment line\nb := 2"),
        vec!["NAME", "ASSIGN", "NUMBER", "NAME", "ASSIGN", "NUMBER"]
    );
}

#[test]
fn comment_may_contain_characters_outside_the_grammars_alphabet() {
    // A comment's content is arbitrary text and must never be re-lexed as
    // code -- `@`/`$`/`\` all have no token anywhere in this grammar, but
    // must not cause a lex error when they only ever appear inside a
    // comment.
    assert!(try_tokenize_axiom("a := 1 -- cost is @5 $ off \\ end\nb := 2").is_ok());
}

#[test]
fn single_minus_is_not_swallowed_by_the_comment_pattern() {
    // A single `-` (MINUS) must not be mistaken for the start of a `--`
    // comment -- GrammarLexer's skip pass tries the WHOLE `--[^\n]*`
    // pattern at the position, which cannot match on just one dash.
    assert_eq!(token_types("a - b"), vec!["NAME", "MINUS", "NAME"]);
    assert_eq!(token_types("a-b"), vec!["NAME", "MINUS", "NAME"]);
}

// ===========================================================================
// Whitespace / no significant newline (MA13 §4/§5 -- blocks are
// `;`-separated, never newline-separated; `axiom-repl`'s numbered-prompt
// step counter is a REPL-layer concern, not a lexer one).
// ===========================================================================

#[test]
fn newlines_are_ordinary_whitespace_not_a_significant_token() {
    assert_eq!(
        token_types("a := 1\nb := 2"),
        vec!["NAME", "ASSIGN", "NUMBER", "NAME", "ASSIGN", "NUMBER"]
    );
}

#[test]
fn skips_ordinary_whitespace() {
    assert_eq!(token_types("a   :=   1"), vec!["NAME", "ASSIGN", "NUMBER"]);
}

// ===========================================================================
// Deferred surface (MA13 §4) -- none of these has any token in this cut's
// grammar; each falls through to an honest lex error.
// ===========================================================================

#[test]
fn deferred_constructs_are_lex_errors_not_silently_accepted() {
    // Package-calling `$` and target-type `@` (deferred) -- neither `$` nor
    // `@` has a token anywhere in this grammar, so these are honest errors.
    assert!(try_tokenize_axiom("content(2)$Polynomial(Integer)").is_err());
    assert!(try_tokenize_axiom("(2 = 3)@Boolean").is_err());
}

#[test]
fn no_dedicated_token_for_multi_char_sequences_this_cut_never_declares() {
    // Neither the anonymous "maps-to" operator `+->` nor block early-exit
    // `=>` (both deferred, MA13 §4) has a DEDICATED token in this cut's
    // grammar -- but unlike `$`/`@` above, every individual CHARACTER in
    // both sequences (`+`, `-`, `>`, `=`) already has its OWN ordinary
    // single-character token, so these decompose into their constituent
    // tokens rather than erroring. This is an honest reflection of the
    // absence of a dedicated production (a future `axiom-parser`, MA-13c,
    // would reject the *sequence* at the grammar level), not a
    // special-cased rejection at the lexer layer.
    assert_eq!(
        token_types("x +-> x * x"),
        vec!["NAME", "PLUS", "MINUS", "GREATER", "NAME", "TIMES", "NAME"]
    );
    assert_eq!(token_types("=>"), vec!["EQ", "GREATER"]);
}

#[test]
fn macro_is_not_a_keyword_here() {
    // `macro` (deferred, MA13 §4) is simply an ordinary NAME in this cut --
    // no reserved word for it exists.
    assert_eq!(token_types("macro"), vec!["NAME"]);
}

#[test]
fn record_union_any_are_ordinary_names_not_reserved_words() {
    // Record/Union/Any (the heterogeneous aggregate/sum-type machinery,
    // deferred whole per MA13 §4) have no lexer-level reservation at all
    // in this cut -- they lex exactly like any other capitalized
    // identifier (indistinguishable from a built-in domain name at this
    // layer), an honest reflection of "not implemented yet" rather than a
    // reserved-but-unusable word.
    assert_eq!(token_types("Record"), vec!["NAME"]);
    assert_eq!(token_types("Union"), vec!["NAME"]);
    assert_eq!(token_types("Any"), vec!["NAME"]);
}

// ===========================================================================
// Errors.
// ===========================================================================

#[test]
fn unrecognized_character_is_an_error() {
    assert!(try_tokenize_axiom("a @ b").is_err());
    assert!(try_tokenize_axiom("a $ b").is_err());
    assert!(try_tokenize_axiom("a ! b").is_err());
    assert!(try_tokenize_axiom("a ? b").is_err());
}

#[test]
fn tokenize_axiom_panics_on_malformed_source() {
    let result = std::panic::catch_unwind(|| tokenize_axiom("@"));
    assert!(result.is_err());
}

#[test]
fn create_axiom_lexer_exposes_the_result_returning_api() {
    assert!(create_axiom_lexer("x := 1").tokenize().is_ok());
    assert!(create_axiom_lexer("@").tokenize().is_err());
}

#[test]
fn handles_eof_mid_string_without_panic() {
    // An unterminated string -- honest failure, not a panic (STRING's
    // pattern simply never finds a closing quote, so no other pattern
    // matches at the opening `"` either... actually the STRING pattern
    // requires a closing quote, so this must fail to tokenize the `"`
    // itself as an isolated character via the UNKNOWN catch-all path).
    let _ = try_tokenize_axiom("a := \"oops");
}

// ===========================================================================
// Adversarial / DoS-guard regression: `axiom-lexer` performs a single
// linear scan with O(1) stack depth regardless of source-level nesting --
// verified directly here rather than merely asserted in prose (MA13b task
// brief's own DoS-guard requirement). See `src/lib.rs`'s own doc comment
// for the fuller argument for why no recursion-depth cap belongs in this
// crate (that is `axiom-parser`'s concern, MA-13c).
// ===========================================================================

#[test]
fn deeply_nested_parens_do_not_overflow_the_lexer_stack() {
    // 50,000 levels of nested parens would blow a naive recursive-descent
    // parser's call stack; a flat single-pass lexer must handle this with
    // no growth in stack usage at all, since it never recurses on bracket
    // structure -- it only ever counts LPAREN/RPAREN as ordinary tokens.
    let depth = 50_000;
    let src = "(".repeat(depth) + "a" + &")".repeat(depth);
    let tokens = tokenize_axiom(&src);
    // depth LPARENs + 1 NAME + depth RPARENs + EOF.
    assert_eq!(tokens.len(), depth * 2 + 2);
    assert_eq!(tokens[0].effective_type_name(), "LPAREN");
    assert_eq!(tokens[depth].effective_type_name(), "NAME");
    assert_eq!(tokens[depth + 1].effective_type_name(), "RPAREN");
}

#[test]
fn a_very_large_flat_token_stream_tokenizes_without_incident() {
    // A wide (not deep) adversarial input -- many thousands of sibling
    // tokens rather than nested structure -- exercises the other half of
    // the DoS surface (unbounded allocation/time, not stack depth). This
    // crate makes no attempt to cap input size or token count (see
    // `src/lib.rs`'s doc comment): that is a resource-budgeting decision
    // for whatever embeds this lexer (a REPL reading a line at a time
    // naturally bounds this; a batch file-reader would need its own cap),
    // not a lexer-level concern any sibling `*-lexer` crate in this repo
    // enforces either. This test only asserts correctness at scale, not
    // a rejection.
    let count = 100_000;
    let src = "a + ".repeat(count) + "a";
    let tokens = tokenize_axiom(&src);
    // count * (NAME, PLUS) + one final NAME + EOF.
    assert_eq!(tokens.len(), count * 2 + 2);
}

#[test]
fn a_very_long_comment_does_not_hang_or_overflow() {
    let body = "x".repeat(500_000);
    let src = format!("a := 1 -- {body}\nb := 2");
    assert_eq!(
        token_types(&src),
        vec!["NAME", "ASSIGN", "NUMBER", "NAME", "ASSIGN", "NUMBER"]
    );
}

// ===========================================================================
// End-to-end: a small, realistic "textbook Axiom session" snippet.
// ===========================================================================

#[test]
fn tokenizes_a_small_end_to_end_snippet() {
    // a : PositiveInteger  -- declare a domain
    // a := 3
    // f(x: Integer): Integer == x * x
    // if a > 0 then f(a) else 0
    // Polynomial(Integer) has Ring
    let src = "a : PositiveInteger -- declare a domain\n\
               a := 3\n\
               f(x: Integer): Integer == x * x\n\
               if a > 0 then f(a) else 0\n\
               Polynomial(Integer) has Ring";
    assert_eq!(
        token_types(src),
        vec![
            "NAME", "COLON", "NAME",
            "NAME", "ASSIGN", "NUMBER",
            "NAME", "LPAREN", "NAME", "COLON", "NAME", "RPAREN", "COLON", "NAME", "DEFINE",
            "NAME", "TIMES", "NAME",
            "KEYWORD", "NAME", "GREATER", "NUMBER", "KEYWORD", "NAME", "LPAREN", "NAME",
            "RPAREN", "KEYWORD", "NUMBER",
            "NAME", "LPAREN", "NAME", "RPAREN", "KEYWORD", "NAME",
        ]
    );
}
