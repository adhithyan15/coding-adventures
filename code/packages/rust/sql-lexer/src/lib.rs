//! # SQL Lexer — tokenizing SQL source text.
//!
//! [SQL](https://en.wikipedia.org/wiki/SQL) (Structured Query Language) is the
//! universal language for relational databases. This crate implements a lexer
//! for an ANSI SQL subset covering SELECT, INSERT, UPDATE, DELETE, CREATE TABLE,
//! and DROP TABLE statements.
//!
//! This crate provides a lexer (tokenizer) for SQL. It does **not** hand-write
//! tokenization rules. Instead, it loads the `sql.tokens` grammar file — a
//! declarative description of every token in SQL — and feeds it to the generic
//! [`GrammarLexer`] from the `lexer` crate.
//!
//! # Architecture
//!
//! The tokenization pipeline has three layers:
//!
//! ```text
//! sql.tokens           (grammar file on disk)
//!        |
//!        v
//! grammar-tools        (parses .tokens -> TokenGrammar struct)
//!        |
//!        v
//! lexer::GrammarLexer  (tokenizes source using TokenGrammar)
//! ```
//!
//! This crate is the thin glue layer that wires these components together
//! for SQL specifically. It knows where to find `sql.tokens` and provides
//! two public entry points:
//!
//! - [`create_sql_lexer`] — returns a `GrammarLexer` for fine-grained control.
//! - [`tokenize_sql`] — convenience function that returns `Result<Vec<Token>, String>`.
//!
//! # Case-insensitive keywords
//!
//! SQL is case-insensitive for keywords: `SELECT`, `select`, and `Select`
//! are all equivalent. The `sql.tokens` grammar file declares
//! `# @case_insensitive true`, which instructs the `GrammarLexer` to:
//!
//! 1. Compare NAME tokens against the keyword list in a case-insensitive way.
//! 2. Normalize all matched keyword values to **UPPERCASE** before emitting them.
//!
//! So regardless of how the user wrote the keyword in the source, the resulting
//! token always has `TokenType::Keyword` and `value` equal to the uppercased form.
//!
//! # Token types
//!
//! SQL tokens fall into these broad categories:
//!
//! - **Keywords** — `SELECT`, `FROM`, `WHERE`, `GROUP`, `BY`, etc. (50+ keywords)
//! - **Identifiers** — `NAME` tokens for table names, column names, aliases
//! - **Literals** — `NUMBER` and `STRING` (single-quoted, quotes stripped)
//! - **Operators** — `=`, `!=`, `<>`, `<`, `>`, `<=`, `>=`, `+`, `-`, `*`, `/`, `%`
//! - **Punctuation** — `(`, `)`, `,`, `;`, `.`
//! - **Comments** — Line comments (`-- ...`) and block comments (`/* ... */`) are skipped.
//!
//! # Why grammar-driven?
//!
//! A hand-written SQL lexer would need to handle:
//! - 50+ keyword lookups with case folding
//! - Two forms of inequality (`!=` and `<>`) mapping to the same token type
//! - Two comment styles
//! - Quoted identifiers (backtick-quoted names)
//! - Single-quoted strings with backslash escapes
//!
//! The grammar-driven approach encodes all of this in the 97-line `sql.tokens`
//! file, with only ~30 lines of Rust glue code needed here.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;
mod _grammar;

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarLexer` configured for SQL source text.
///
/// This function:
/// 1. Reads the `sql.tokens` grammar file from disk.
/// 2. Parses it into a `TokenGrammar` using `grammar-tools`.
/// 3. Constructs a `GrammarLexer` with the grammar and the given source.
///
/// The returned lexer is ready to call `.tokenize()` on. Use this when you
/// need access to the lexer object itself (e.g., for incremental tokenization
/// or custom error handling).
///
/// # Errors
///
/// Returns `Err(String)` if:
/// - The grammar file cannot be read from disk (file not found, permission error).
/// - The grammar file content cannot be parsed (invalid syntax).
///
/// # Example
///
/// ```no_run
/// use coding_adventures_sql_lexer::create_sql_lexer;
///
/// let mut lexer = create_sql_lexer("SELECT id FROM users");
/// let tokens = lexer.tokenize().expect("tokenization failed");
/// for token in &tokens {
///     println!("{}", token);
/// }
/// ```
pub fn create_sql_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    GrammarLexer::new(source, &grammar)
}

/// Tokenize SQL source text into a vector of tokens.
///
/// This is the most convenient entry point — it handles grammar loading,
/// lexer creation, and tokenization in one call. The returned vector always
/// ends with an `EOF` token.
///
/// Keywords are normalized to uppercase regardless of how they appear in
/// the source (`select`, `SELECT`, and `Select` all produce a keyword token
/// with value `"SELECT"`).
///
/// # Errors
///
/// Returns `Err(String)` if:
/// - The grammar file cannot be read or parsed.
/// - The source contains an unexpected character that no token pattern matches.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_sql_lexer::tokenize_sql;
///
/// let tokens = tokenize_sql("SELECT name, age FROM users WHERE age > 18").unwrap();
/// for token in &tokens {
///     println!("{:?} {:?}", token.type_, token.value);
/// }
/// ```
pub fn tokenize_sql(source: &str) -> Result<Vec<Token>, String> {
    // Create a fresh lexer for this source text.
    let mut sql_lexer = create_sql_lexer(source);

    // Tokenize and propagate any LexerError as a String.
    sql_lexer
        .tokenize()
        .map_err(|e| format!("SQL tokenization failed: {e}"))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    // -----------------------------------------------------------------------
    // Helper: tokenize and unwrap, filtering out EOF.
    // -----------------------------------------------------------------------

    /// Tokenize a SQL string and return (type_, value) pairs excluding EOF.
    ///
    /// Panics on lexer error — for error-path tests use `tokenize_sql` directly.
    fn lex(source: &str) -> Vec<(TokenType, String)> {
        let tokens = tokenize_sql(source).expect("tokenize_sql failed");
        tokens
            .into_iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.type_, t.value.clone()))
            .collect()
    }

    // -----------------------------------------------------------------------
    // Test 1: SELECT keyword (uppercase)
    // -----------------------------------------------------------------------

    /// `SELECT` in uppercase produces a Keyword token with value "SELECT".
    ///
    /// Token type: `TokenType::Keyword`
    /// Value: `"SELECT"` (normalized to uppercase)
    #[test]
    fn test_select_uppercase() {
        let pairs = lex("SELECT");
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "SELECT");
    }

    // -----------------------------------------------------------------------
    // Test 2: Case-insensitive keywords
    // -----------------------------------------------------------------------

    /// Because `sql.tokens` declares `# @case_insensitive true`, all three
    /// forms of `select` produce the same Keyword token:
    ///
    /// | Input    | type_       | value    |
    /// |----------|-------------|----------|
    /// | "select" | Keyword     | "SELECT" |
    /// | "SELECT" | Keyword     | "SELECT" |
    /// | "Select" | Keyword     | "SELECT" |
    ///
    /// This normalization makes SQL queries portable regardless of style.
    #[test]
    fn test_select_case_insensitive() {
        for input in &["select", "SELECT", "Select"] {
            let pairs = lex(input);
            assert_eq!(pairs.len(), 1, "Expected 1 token for input {:?}", input);
            assert_eq!(
                pairs[0].0,
                TokenType::Keyword,
                "Input {:?} should be Keyword",
                input
            );
            assert_eq!(
                pairs[0].1, "SELECT",
                "Input {:?} should normalize to SELECT",
                input
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 3: NUMBER token
    // -----------------------------------------------------------------------

    /// SQL integer literals produce NUMBER tokens with the original text.
    ///
    /// | Input | type_  | value |
    /// |-------|--------|-------|
    /// | "42"  | Number | "42"  |
    /// | "0"   | Number | "0"   |
    /// | "100" | Number | "100" |
    #[test]
    fn test_number_integer() {
        for (input, expected) in &[("42", "42"), ("0", "0"), ("100", "100")] {
            let pairs = lex(input);
            assert_eq!(pairs.len(), 1);
            assert_eq!(pairs[0].0, TokenType::Number);
            assert_eq!(pairs[0].1, *expected);
        }
    }

    // -----------------------------------------------------------------------
    // Test 4: NUMBER with decimal
    // -----------------------------------------------------------------------

    /// SQL supports decimal literals like `3.14` and `0.5`.
    #[test]
    fn test_number_decimal() {
        let pairs = lex("3.14");
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, TokenType::Number);
        assert_eq!(pairs[0].1, "3.14");
    }

    // -----------------------------------------------------------------------
    // Test 5: STRING token (single-quoted, quotes stripped)
    // -----------------------------------------------------------------------

    /// SQL string literals are enclosed in single quotes. The lexer strips
    /// the surrounding quotes and returns the inner content as the token value.
    ///
    /// | Input       | type_  | value |
    /// |-------------|--------|-------|
    /// | `'hello'`   | String | hello |
    /// | `'Alice'`   | String | Alice |
    #[test]
    fn test_string_single_quoted() {
        let pairs = lex("'hello'");
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, TokenType::String);
        assert_eq!(pairs[0].1, "hello");
    }

    // -----------------------------------------------------------------------
    // Test 6: String with space
    // -----------------------------------------------------------------------

    /// Single-quoted strings can contain spaces and special characters.
    #[test]
    fn test_string_with_space() {
        let pairs = lex("'hello world'");
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, TokenType::String);
        assert_eq!(pairs[0].1, "hello world");
    }

    // -----------------------------------------------------------------------
    // Test 7: Operator = (EQUALS)
    // -----------------------------------------------------------------------

    /// The equality operator `=` is its own token type.
    #[test]
    fn test_operator_equals() {
        let pairs = lex("=");
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, TokenType::Equals);
    }

    // -----------------------------------------------------------------------
    // Test 8: NOT_EQUALS operator (both forms)
    // -----------------------------------------------------------------------

    /// SQL has two ways to write "not equal": `!=` and `<>`.
    /// Both are aliased to NOT_EQUALS in the grammar via `NEQ_ANSI = "<>" -> NOT_EQUALS`.
    /// They produce a Name token with type_name "NOT_EQUALS".
    ///
    /// | Input | type_name   |
    /// |-------|-------------|
    /// | `!=`  | NOT_EQUALS  |
    /// | `<>`  | NOT_EQUALS  |
    #[test]
    fn test_not_equals_both_forms() {
        for op in &["!=", "<>"] {
            let tokens = tokenize_sql(op).expect("tokenize_sql failed");
            let non_eof: Vec<_> = tokens
                .iter()
                .filter(|t| t.type_ != TokenType::Eof)
                .collect();
            assert_eq!(non_eof.len(), 1, "Expected 1 token for {:?}", op);
            assert_eq!(
                non_eof[0].type_name.as_deref(),
                Some("NOT_EQUALS"),
                "Both != and <> should be NOT_EQUALS, got {:?} for {:?}",
                non_eof[0].type_name,
                op
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 9: LESS_EQUALS and GREATER_EQUALS operators
    // -----------------------------------------------------------------------

    /// Compound comparison operators `<=` and `>=` have their own token types.
    ///
    /// | Input | type_name      |
    /// |-------|----------------|
    /// | `<=`  | LESS_EQUALS    |
    /// | `>=`  | GREATER_EQUALS |
    #[test]
    fn test_less_equals_and_greater_equals() {
        let tokens_le = tokenize_sql("<=").expect("tokenize_sql failed");
        let non_eof_le: Vec<_> = tokens_le
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .collect();
        assert_eq!(non_eof_le.len(), 1);
        assert_eq!(non_eof_le[0].type_name.as_deref(), Some("LESS_EQUALS"));

        let tokens_ge = tokenize_sql(">=").expect("tokenize_sql failed");
        let non_eof_ge: Vec<_> = tokens_ge
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .collect();
        assert_eq!(non_eof_ge.len(), 1);
        assert_eq!(non_eof_ge[0].type_name.as_deref(), Some("GREATER_EQUALS"));
    }

    // -----------------------------------------------------------------------
    // Test 10: < and > operators (LESS_THAN / GREATER_THAN)
    // -----------------------------------------------------------------------

    /// Simple comparison operators `<` and `>`.
    ///
    /// LESS_THAN and GREATER_THAN are not in the built-in TokenType enum,
    /// so they map to `TokenType::Name` with `type_name` set to the
    /// grammar name ("LESS_THAN" / "GREATER_THAN").
    ///
    /// | Input | type_ | type_name     |
    /// |-------|-------|---------------|
    /// | `<`   | Name  | "LESS_THAN"   |
    /// | `>`   | Name  | "GREATER_THAN"|
    #[test]
    fn test_less_than_and_greater_than() {
        let tokens_lt = tokenize_sql("<").expect("tokenize_sql failed");
        let non_eof_lt: Vec<_> = tokens_lt
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .collect();
        assert_eq!(non_eof_lt.len(), 1);
        assert_eq!(non_eof_lt[0].type_name.as_deref(), Some("LESS_THAN"));

        let tokens_gt = tokenize_sql(">").expect("tokenize_sql failed");
        let non_eof_gt: Vec<_> = tokens_gt
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .collect();
        assert_eq!(non_eof_gt.len(), 1);
        assert_eq!(non_eof_gt[0].type_name.as_deref(), Some("GREATER_THAN"));
    }

    // -----------------------------------------------------------------------
    // Test 11: Punctuation tokens ( ) , ; .
    // -----------------------------------------------------------------------

    /// SQL uses five punctuation characters: parentheses, comma, semicolon,
    /// and dot (for qualified names like `schema.table`).
    ///
    /// | Character | TokenType    |
    /// |-----------|--------------|
    /// | `(`       | LParen       |
    /// | `)`       | RParen       |
    /// | `,`       | Comma        |
    /// | `;`       | Semicolon    |
    /// | `.`       | Dot          |
    #[test]
    fn test_punctuation() {
        let pairs = lex("( ) , ; .");
        assert_eq!(pairs.len(), 5);
        assert_eq!(pairs[0].0, TokenType::LParen);
        assert_eq!(pairs[1].0, TokenType::RParen);
        assert_eq!(pairs[2].0, TokenType::Comma);
        assert_eq!(pairs[3].0, TokenType::Semicolon);
        assert_eq!(pairs[4].0, TokenType::Dot);
    }

    // -----------------------------------------------------------------------
    // Test 12: Line comment skipping (-- ...)
    // -----------------------------------------------------------------------

    /// SQL line comments start with `--` and extend to the end of the line.
    /// They are in the `skip:` section of the grammar, so they produce no tokens.
    ///
    /// Truth table:
    ///
    /// | Input                      | Tokens (excl. EOF) |
    /// |----------------------------|--------------------|
    /// | `"-- this is a comment\n"` | []                 |
    /// | `"SELECT -- comment\n id"` | [Keyword, Name]    |
    #[test]
    fn test_line_comment_skipped() {
        // A standalone line comment should produce no tokens.
        let pairs = lex("-- this is a comment\n");
        assert_eq!(pairs.len(), 0, "Line comment should produce no tokens");

        // Tokens before and after the comment should still appear.
        let pairs2 = lex("SELECT -- pick a column\n id");
        assert_eq!(pairs2.len(), 2, "Should have SELECT and id tokens");
        assert_eq!(pairs2[0].0, TokenType::Keyword);
        assert_eq!(pairs2[0].1, "SELECT");
        assert_eq!(pairs2[1].0, TokenType::Name);
        assert_eq!(pairs2[1].1, "id");
    }

    // -----------------------------------------------------------------------
    // Test 13: Block comment skipping (/* ... */)
    // -----------------------------------------------------------------------

    /// SQL block comments are delimited by `/*` and `*/`. They can span
    /// multiple lines and are consumed without producing tokens.
    #[test]
    fn test_block_comment_skipped() {
        // A standalone block comment.
        let pairs = lex("/* this is\na block comment */");
        assert_eq!(pairs.len(), 0, "Block comment should produce no tokens");

        // Block comment embedded in a statement.
        let pairs2 = lex("SELECT /* columns: */ id FROM t");
        assert_eq!(pairs2.len(), 4);
        assert_eq!(pairs2[0].1, "SELECT");
        assert_eq!(pairs2[1].0, TokenType::Name); // id
        assert_eq!(pairs2[2].1, "FROM");
        assert_eq!(pairs2[3].0, TokenType::Name); // t
    }

    // -----------------------------------------------------------------------
    // Test 14: NULL, TRUE, FALSE as keywords
    // -----------------------------------------------------------------------

    /// NULL, TRUE, and FALSE are SQL keywords. They tokenize as
    /// `TokenType::Keyword` with normalized uppercase values.
    ///
    /// | Input   | type_   | value   |
    /// |---------|---------|---------|
    /// | "NULL"  | Keyword | "NULL"  |
    /// | "null"  | Keyword | "NULL"  |
    /// | "TRUE"  | Keyword | "TRUE"  |
    /// | "false" | Keyword | "FALSE" |
    #[test]
    fn test_null_true_false_keywords() {
        for (input, expected_value) in &[
            ("NULL", "NULL"),
            ("null", "NULL"),
            ("TRUE", "TRUE"),
            ("true", "TRUE"),
            ("FALSE", "FALSE"),
            ("false", "FALSE"),
        ] {
            let pairs = lex(input);
            assert_eq!(pairs.len(), 1, "Expected 1 token for {:?}", input);
            assert_eq!(
                pairs[0].0,
                TokenType::Keyword,
                "Input {:?} should be Keyword",
                input
            );
            assert_eq!(
                pairs[0].1, *expected_value,
                "Input {:?} should normalize to {:?}",
                input, expected_value
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 15: NAME token (identifier)
    // -----------------------------------------------------------------------

    /// Non-keyword identifiers produce NAME tokens with their original case.
    /// Examples: table names, column names, aliases.
    #[test]
    fn test_name_identifier() {
        let pairs = lex("users");
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, TokenType::Name);
        assert_eq!(pairs[0].1, "users");
    }

    // -----------------------------------------------------------------------
    // Test 16: Full SELECT statement
    // -----------------------------------------------------------------------

    /// A realistic SELECT query exercises multiple token types: keywords,
    /// names, operators, numbers, and punctuation.
    ///
    /// `SELECT name, age FROM users WHERE age > 18`
    ///
    /// Expected tokens:
    /// SELECT(Keyword) name(Name) ,(Comma) age(Name) FROM(Keyword) users(Name)
    /// WHERE(Keyword) age(Name) >(GreaterThan) 18(Number)
    #[test]
    fn test_full_select_statement() {
        let pairs = lex("SELECT name, age FROM users WHERE age > 18");
        assert_eq!(pairs.len(), 10);

        assert_eq!(pairs[0], (TokenType::Keyword, "SELECT".to_string()));
        assert_eq!(pairs[1], (TokenType::Name, "name".to_string()));
        assert_eq!(pairs[2], (TokenType::Comma, ",".to_string()));
        assert_eq!(pairs[3], (TokenType::Name, "age".to_string()));
        assert_eq!(pairs[4], (TokenType::Keyword, "FROM".to_string()));
        assert_eq!(pairs[5], (TokenType::Name, "users".to_string()));
        assert_eq!(pairs[6], (TokenType::Keyword, "WHERE".to_string()));
        assert_eq!(pairs[7], (TokenType::Name, "age".to_string()));
        // GREATER_THAN maps to TokenType::Name with type_name "GREATER_THAN"
        assert_eq!(pairs[8].0, TokenType::Name);
        assert_eq!(pairs[8].1, ">");
        assert_eq!(pairs[9], (TokenType::Number, "18".to_string()));
    }

    // -----------------------------------------------------------------------
    // Test 17: Arithmetic operators
    // -----------------------------------------------------------------------

    /// SQL arithmetic operators: `+`, `-`, `*`, `/`, `%`.
    ///
    /// Plus, Minus, Star, and Slash map to built-in TokenType variants.
    /// Percent is not in the built-in TokenType enum, so it maps to
    /// `TokenType::Name` with `type_name = Some("PERCENT")`.
    #[test]
    fn test_arithmetic_operators() {
        let pairs = lex("+ - * / %");
        assert_eq!(pairs.len(), 5);
        assert_eq!(pairs[0].0, TokenType::Plus);
        assert_eq!(pairs[1].0, TokenType::Minus);
        assert_eq!(pairs[2].0, TokenType::Star);
        assert_eq!(pairs[3].0, TokenType::Slash);
        // PERCENT is a named token type that maps to Name with type_name "PERCENT"
        assert_eq!(pairs[4].0, TokenType::Name);

        let tokens_pct = tokenize_sql("%").expect("tokenize_sql failed");
        let non_eof_pct: Vec<_> = tokens_pct
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .collect();
        assert_eq!(non_eof_pct[0].type_name.as_deref(), Some("PERCENT"));
    }

    // -----------------------------------------------------------------------
    // Test 18: Qualified column reference (table.column)
    // -----------------------------------------------------------------------

    /// Qualified references use dot notation: `t.column_name`.
    /// This should lex as NAME DOT NAME, not as a float literal.
    #[test]
    fn test_qualified_column_ref() {
        let pairs = lex("users.name");
        assert_eq!(pairs.len(), 3);
        assert_eq!(pairs[0], (TokenType::Name, "users".to_string()));
        assert_eq!(pairs[1], (TokenType::Dot, ".".to_string()));
        assert_eq!(pairs[2], (TokenType::Name, "name".to_string()));
    }

    // -----------------------------------------------------------------------
    // Test 19: Whitespace is skipped
    // -----------------------------------------------------------------------

    /// Spaces, tabs, and newlines between tokens are silently consumed.
    #[test]
    fn test_whitespace_skipped() {
        let pairs_compact = lex("SELECT id FROM t");
        let pairs_spaced = lex("  SELECT   id   FROM   t  ");
        let pairs_newlines = lex("SELECT\n  id\nFROM\n  t");

        assert_eq!(pairs_compact.len(), pairs_spaced.len());
        assert_eq!(pairs_compact.len(), pairs_newlines.len());
        for i in 0..pairs_compact.len() {
            assert_eq!(pairs_compact[i].1, pairs_spaced[i].1);
            assert_eq!(pairs_compact[i].1, pairs_newlines[i].1);
        }
    }

    // -----------------------------------------------------------------------
    // Test 20: create_sql_lexer factory function
    // -----------------------------------------------------------------------

    /// The `create_sql_lexer` factory returns a working `GrammarLexer`.
    #[test]
    fn test_create_sql_lexer_success() {
        let mut lexer = create_sql_lexer("SELECT 1");
        let tokens = lexer.tokenize().expect("tokenize should succeed");
        assert!(tokens.len() >= 2); // SELECT, NUMBER, EOF
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Test 21: Error path — non-existent grammar file
    // -----------------------------------------------------------------------

    /// `create_sql_lexer_with_path` returns `Err` when the grammar file does
    /// not exist. This exercises the error branch in the file-read logic.
    ///
    /// | Path                  | Result |
    /// |-----------------------|--------|
    /// | "/no/such/file.tokens"| Err(_) |

    // -----------------------------------------------------------------------
    // Test 22: Error path — tokenize_sql with bad grammar path
    // -----------------------------------------------------------------------

    /// `tokenize_sql` uses the default grammar path. When the grammar path
    /// is good, it returns `Ok`. This test ensures the `Err` variant of
    /// the `Result` is exercised via the `create_sql_lexer_with_path` helper.

    // -----------------------------------------------------------------------
    // Test 23: INSERT keywords
    // -----------------------------------------------------------------------

    /// INSERT INTO VALUES keywords are case-normalized.
    #[test]
    fn test_insert_keywords() {
        let pairs = lex("INSERT INTO VALUES");
        assert_eq!(pairs.len(), 3);
        assert_eq!(pairs[0], (TokenType::Keyword, "INSERT".to_string()));
        assert_eq!(pairs[1], (TokenType::Keyword, "INTO".to_string()));
        assert_eq!(pairs[2], (TokenType::Keyword, "VALUES".to_string()));
    }

    // -----------------------------------------------------------------------
    // Test 24: Mixed-case query tokenization
    // -----------------------------------------------------------------------

    /// SQL keywords are matched case-insensitively, but string literals keep
    /// their original source spelling.
    /// `insert into users values ('Bob', 30)` → INSERT, INTO, users(Name),
    /// VALUES, (, 'Bob'(String), comma, 30(Number), )
    #[test]
    fn test_mixed_case_query() {
        let pairs = lex("insert into users values ('Bob', 30)");
        assert_eq!(pairs[0], (TokenType::Keyword, "INSERT".to_string()));
        assert_eq!(pairs[1], (TokenType::Keyword, "INTO".to_string()));
        assert_eq!(pairs[2], (TokenType::Name, "users".to_string()));
        assert_eq!(pairs[3], (TokenType::Keyword, "VALUES".to_string()));
        assert_eq!(pairs[4], (TokenType::LParen, "(".to_string()));
        assert_eq!(pairs[5], (TokenType::String, "Bob".to_string()));
        assert_eq!(pairs[6], (TokenType::Comma, ",".to_string()));
        assert_eq!(pairs[7], (TokenType::Number, "30".to_string()));
        assert_eq!(pairs[8], (TokenType::RParen, ")".to_string()));
    }

    // -----------------------------------------------------------------------
    // Test 25: Star token in SELECT *
    // -----------------------------------------------------------------------

    /// `SELECT *` uses the STAR token, which is also the multiplication operator.
    #[test]
    fn test_select_star() {
        let pairs = lex("SELECT *");
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], (TokenType::Keyword, "SELECT".to_string()));
        assert_eq!(pairs[1], (TokenType::Star, "*".to_string()));
    }

    // -----------------------------------------------------------------------
    // Test 26: Semicolon as statement terminator
    // -----------------------------------------------------------------------

    /// SQL statements are separated by semicolons. Multiple statements in
    /// sequence produce SEMICOLON tokens between them.
    #[test]
    fn test_semicolon_separator() {
        let pairs = lex("SELECT 1 ; SELECT 2");
        let semicolons: Vec<_> = pairs
            .iter()
            .filter(|(t, _)| *t == TokenType::Semicolon)
            .collect();
        assert_eq!(semicolons.len(), 1);
    }

    // -----------------------------------------------------------------------
    // Test 27: tokenize_sql returns Ok on valid input
    // -----------------------------------------------------------------------

    /// `tokenize_sql` returns `Ok(Vec<Token>)` on valid SQL.
    #[test]
    fn test_tokenize_sql_ok() {
        let result = tokenize_sql("SELECT 1");
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
        let tokens = result.unwrap();
        // Should have: Keyword("SELECT"), Number("1"), Eof
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].type_, TokenType::Keyword);
        assert_eq!(tokens[0].value, "SELECT");
        assert_eq!(tokens[1].type_, TokenType::Number);
        assert_eq!(tokens[1].value, "1");
        assert_eq!(tokens[2].type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Test 28: STAR type_name check
    // -----------------------------------------------------------------------

    /// The STAR token type: check first_type_name helper for STAR.
    #[test]
    fn test_star_type_name() {
        // STAR is a named token type — verify it lexes correctly
        let pairs = lex("*");
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, TokenType::Star);
    }
}
