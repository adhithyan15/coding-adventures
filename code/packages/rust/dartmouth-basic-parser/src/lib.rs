//! # Dartmouth BASIC Parser — parsing 1964 BASIC source text into an AST.
//!
//! This crate is the second half of the Dartmouth BASIC front-end pipeline.
//! Where the `dartmouth-basic-lexer` crate breaks source text into tokens,
//! this crate arranges those tokens into a tree that reflects the **structure**
//! of the program — an Abstract Syntax Tree (AST).
//!
//! # Historical Context
//!
//! Dartmouth BASIC was created by John G. Kemeny and Thomas E. Kurtz at
//! Dartmouth College in 1964. Running on a GE-225 mainframe accessed through
//! uppercase-only teletypes, it was the first programming language designed
//! explicitly for non-science students. Its numbered-line structure made it
//! easy to learn: every program statement lived on its own numbered line, and
//! you could type lines in any order — the system sorted them for you.
//!
//! The 17 statement types from the original 1964 manual are:
//!
//!   LET, PRINT, INPUT, IF-THEN, GOTO, GOSUB, RETURN,
//!   FOR, NEXT, END, STOP, REM, READ, DATA, RESTORE, DIM, DEF
//!
//! # The parsing pipeline
//!
//! ```text
//! Source text  ("10 LET X = 5\n20 PRINT X\n30 END\n")
//!       |
//!       v
//! dartmouth-basic-lexer   → Vec<Token>
//!       |                  [LINE_NUM("10"), KEYWORD("LET"), NAME("X"), EQ("="),
//!       |                   NUMBER("5"), NEWLINE, LINE_NUM("20"), ...]
//!       v
//! dartmouth_basic.grammar → ParserGrammar (rules: program, line, statement, ...)
//!       |
//!       v
//! GrammarParser           → GrammarASTNode tree
//!       |
//!       |                   program
//!       |                     ├── line
//!       |                     │     ├── LINE_NUM("10")
//!       |                     │     ├── statement
//!       |                     │     │     └── let_stmt
//!       |                     │     │           ├── KEYWORD("LET")
//!       |                     │     │           ├── variable
//!       |                     │     │           │     └── NAME("X")
//!       |                     │     │           ├── EQ("=")
//!       |                     │     │           └── expr
//!       |                     │     │                 └── ...
//!       |                     │     └── NEWLINE
//!       |                     └── line
//!       |                           └── ...
//!       v
//! [compiler or interpreter consumes the AST]
//! ```
//!
//! This crate is the thin glue layer that wires these components together.
//! It knows where to find the `dartmouth_basic.grammar` file and provides
//! two public entry points.
//!
//! # Grammar-driven parsing
//!
//! The `GrammarParser` is a **recursive descent parser with backtracking and
//! packrat memoization**. The BASIC grammar has ~25 rules covering:
//!
//! - `program` — the start symbol: zero or more numbered lines
//! - `line` — LINE_NUM [ statement ] NEWLINE
//! - `statement` — one of the 17 statement types
//! - `expr`, `term`, `power`, `unary`, `primary` — the expression hierarchy
//! - `variable`, `relop`, `print_list`, `dim_decl`, etc.
//!
//! Expression precedence is encoded in the rule nesting:
//!
//! ```text
//! expr (+ -)
//!   └── term (* /)
//!         └── power (^ right-assoc)
//!               └── unary (unary -)
//!                     └── primary (atom)
//! ```

use coding_adventures_dartmouth_basic_lexer::tokenize_dartmouth_basic;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};
mod _grammar;

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarParser` configured for Dartmouth BASIC source text.
///
/// This function performs two major steps:
///
/// 1. **Tokenization** — uses `tokenize_dartmouth_basic` from the
///    dartmouth-basic-lexer crate to break the source into tokens
///    (LINE_NUM, KEYWORD, NAME, NUMBER, STRING, EQ, LT, GT, etc.).
///
/// 2. **Grammar loading** — reads and parses the `dartmouth_basic.grammar`
///    file, which defines ~25 rules covering the full 1964 BASIC syntax.
///
/// The returned `GrammarParser` is ready to call `.parse()` on.
///
/// # Panics
///
/// Panics if:
/// - The `dartmouth_basic.grammar` file cannot be read or parsed.
/// - The source text fails tokenization (unexpected character).
///
/// # Example
///
/// ```no_run
/// use coding_adventures_dartmouth_basic_parser::create_dartmouth_basic_parser;
///
/// let mut parser = create_dartmouth_basic_parser("10 LET X = 5\n");
/// let ast = parser.parse().expect("parse failed");
/// println!("{:?}", ast.rule_name);
/// ```
pub fn create_dartmouth_basic_parser(source: &str) -> GrammarParser {
    // Step 1: Tokenize the source using the dartmouth-basic-lexer.
    //
    // The lexer handles all BASIC token types:
    //   LINE_NUM   — line number at the start of each statement (10, 20, ...)
    //   KEYWORD    — reserved words: LET, PRINT, IF, THEN, GOTO, ...
    //   NAME       — variable names: A–Z, A0–Z9
    //   NUMBER     — numeric literals: 42, 3.14, 1E-5
    //   STRING     — double-quoted string literals: "HELLO WORLD"
    //   BUILTIN_FN — built-in functions: SIN, COS, TAN, ATN, EXP, LOG, ABS, SQR, INT, RND, SGN
    //   USER_FN    — user-defined functions: FNA through FNZ
    //   EQ, LT, GT, LE, GE, NE — comparison and assignment operators
    //   PLUS, MINUS, STAR, SLASH, CARET — arithmetic operators
    //   LPAREN, RPAREN, COMMA, SEMICOLON — structural punctuation
    //   NEWLINE    — line terminator (significant in BASIC!)
    //   EOF        — end of input
    let tokens = tokenize_dartmouth_basic(source);

    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar)
}

/// Parse Dartmouth BASIC source text into an AST.
///
/// This is the most convenient entry point — it handles tokenization,
/// grammar loading, parser creation, and parsing in one call.
///
/// The returned `GrammarASTNode` has `rule_name` set to `"program"` (the
/// start symbol of the BASIC grammar). Its children are `line` nodes, each
/// containing a LINE_NUM, an optional `statement`, and a NEWLINE.
///
/// # Panics
///
/// Panics if tokenization fails, the grammar file is missing/invalid,
/// or the source text has a syntax error.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_dartmouth_basic_parser::parse_dartmouth_basic;
///
/// let ast = parse_dartmouth_basic("10 LET X = 5\n20 PRINT X\n30 END\n");
/// assert_eq!(ast.rule_name, "program");
/// ```
pub fn parse_dartmouth_basic(source: &str) -> GrammarASTNode {
    // Create a parser wired to the BASIC grammar and tokens.
    let mut basic_parser = create_dartmouth_basic_parser(source);

    // Parse and unwrap — any GrammarParseError becomes a panic.
    //
    // In a production tool, you would propagate the error via Result.
    // For this educational codebase, panicking with a descriptive message
    // is sufficient.
    basic_parser
        .parse()
        .unwrap_or_else(|e| panic!("Dartmouth BASIC parse failed: {e}"))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    // -----------------------------------------------------------------------
    // Helper functions
    // -----------------------------------------------------------------------

    /// All Dartmouth BASIC programs parse to a root node with rule_name
    /// "program", since that is the start symbol of the grammar.
    fn assert_program_root(ast: &GrammarASTNode) {
        assert_eq!(
            ast.rule_name, "program",
            "Expected root rule 'program', got '{}'",
            ast.rule_name
        );
    }

    /// Recursively search the AST for a node with the given rule name.
    /// Returns true if found anywhere in the tree.
    ///
    /// This is the primary inspection tool for these tests. Rather than
    /// checking the exact tree structure (which may vary with grammar
    /// changes), we verify that expected rule nodes are present in the tree.
    fn find_rule(node: &GrammarASTNode, target_rule: &str) -> bool {
        if node.rule_name == target_rule {
            return true;
        }
        for child in &node.children {
            if let ASTNodeOrToken::Node(child_node) = child {
                if find_rule(child_node, target_rule) {
                    return true;
                }
            }
        }
        false
    }

    /// Count how many nodes with the given rule name exist in the tree.
    fn count_rule(node: &GrammarASTNode, target_rule: &str) -> usize {
        let mut count = if node.rule_name == target_rule { 1 } else { 0 };
        for child in &node.children {
            if let ASTNodeOrToken::Node(child_node) = child {
                count += count_rule(child_node, target_rule);
            }
        }
        count
    }

    // -----------------------------------------------------------------------
    // Test group 1: LET statement
    // -----------------------------------------------------------------------

    /// LET is the assignment statement. In 1964 BASIC, every variable
    /// assignment requires the LET keyword (unlike modern BASIC dialects
    /// that drop it).
    ///
    ///   10 LET X = 5
    ///   20 LET A(3) = X + 1
    #[test]
    fn test_let_scalar() {
        let ast = parse_dartmouth_basic("10 LET X = 5\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "let_stmt"), "Expected let_stmt node");
        assert!(find_rule(&ast, "variable"), "Expected variable node");
        assert!(find_rule(&ast, "expr"), "Expected expr node");
    }

    #[test]
    fn test_let_array_element() {
        // Assigning to an array element: LET A(3) = X + 1
        // The variable rule handles both scalar (NAME) and array (NAME(expr)) forms.
        let ast = parse_dartmouth_basic("10 LET A(3) = X + 1\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "let_stmt"), "Expected let_stmt node");
        assert!(find_rule(&ast, "variable"), "Expected variable node");
    }

    // -----------------------------------------------------------------------
    // Test group 2: PRINT statement
    // -----------------------------------------------------------------------

    /// PRINT outputs values to the terminal. The 1964 spec supports:
    ///   PRINT             — blank line
    ///   PRINT expr        — print a number
    ///   PRINT "STRING"    — print a string
    ///   PRINT a, b        — print a, tab to next zone, print b
    ///   PRINT a; b        — print a immediately followed by b
    ///   PRINT a,          — trailing comma: suppress final newline

    #[test]
    fn test_print_bare() {
        // "10 PRINT\n" — just a blank line output
        let ast = parse_dartmouth_basic("10 PRINT\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "print_stmt"), "Expected print_stmt node");
    }

    #[test]
    fn test_print_expression() {
        // "10 PRINT X + 1\n" — print an arithmetic expression
        let ast = parse_dartmouth_basic("10 PRINT X + 1\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "print_stmt"), "Expected print_stmt node");
        assert!(find_rule(&ast, "print_list"), "Expected print_list node");
    }

    #[test]
    fn test_print_string() {
        // "10 PRINT \"HELLO\"\n" — print a string literal
        let ast = parse_dartmouth_basic("10 PRINT \"HELLO\"\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "print_stmt"), "Expected print_stmt node");
        assert!(find_rule(&ast, "print_list"), "Expected print_list node");
    }

    #[test]
    fn test_print_comma_separator() {
        // "10 PRINT X, Y\n" — comma advances to the next print zone (every 15 chars)
        let ast = parse_dartmouth_basic("10 PRINT X, Y\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "print_stmt"), "Expected print_stmt node");
        assert!(find_rule(&ast, "print_sep"), "Expected print_sep node");
    }

    #[test]
    fn test_print_semicolon_separator() {
        // "10 PRINT X; Y\n" — semicolon: print items immediately adjacent
        let ast = parse_dartmouth_basic("10 PRINT X; Y\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "print_stmt"), "Expected print_stmt node");
        assert!(find_rule(&ast, "print_sep"), "Expected print_sep node");
    }

    // -----------------------------------------------------------------------
    // Test group 3: INPUT statement
    // -----------------------------------------------------------------------

    /// INPUT pauses execution and reads values from the user (or from the
    /// :input_queue in tests). The teletype would print a question mark
    /// prompt and wait for the user to type a value.
    #[test]
    fn test_input_single() {
        // "10 INPUT X\n" — read a single value
        let ast = parse_dartmouth_basic("10 INPUT X\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "input_stmt"), "Expected input_stmt node");
        assert!(find_rule(&ast, "variable"), "Expected variable node");
    }

    #[test]
    fn test_input_multiple() {
        // "10 INPUT A, B, C\n" — read three values in one INPUT statement
        let ast = parse_dartmouth_basic("10 INPUT A, B, C\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "input_stmt"), "Expected input_stmt node");
    }

    // -----------------------------------------------------------------------
    // Test group 4: IF-THEN statement — all 6 relational operators
    // -----------------------------------------------------------------------

    /// IF-THEN is the sole conditional in 1964 BASIC. The form is always:
    ///   IF expr relop expr THEN LINE_NUM
    ///
    /// There is no ELSE clause and no multi-statement THEN body — these
    /// features came in later BASIC dialects. The branch target is a
    /// literal line number, not an expression.
    ///
    /// The six relational operators: = < > <= >= <>
    #[test]
    fn test_if_eq() {
        let ast = parse_dartmouth_basic("10 IF X = 5 THEN 100\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "if_stmt"), "Expected if_stmt node");
        assert!(find_rule(&ast, "relop"), "Expected relop node");
    }

    #[test]
    fn test_if_lt() {
        let ast = parse_dartmouth_basic("10 IF X < 5 THEN 100\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "if_stmt"), "Expected if_stmt node");
        assert!(find_rule(&ast, "relop"), "Expected relop node");
    }

    #[test]
    fn test_if_gt() {
        let ast = parse_dartmouth_basic("10 IF X > 5 THEN 100\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "if_stmt"), "Expected if_stmt node");
    }

    #[test]
    fn test_if_le() {
        let ast = parse_dartmouth_basic("10 IF X <= 5 THEN 100\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "if_stmt"), "Expected if_stmt node");
    }

    #[test]
    fn test_if_ge() {
        let ast = parse_dartmouth_basic("10 IF X >= 5 THEN 100\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "if_stmt"), "Expected if_stmt node");
    }

    #[test]
    fn test_if_ne() {
        // <> is the "not equal" operator in BASIC (unlike C's !=)
        let ast = parse_dartmouth_basic("10 IF X <> 5 THEN 100\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "if_stmt"), "Expected if_stmt node");
    }

    // -----------------------------------------------------------------------
    // Test group 5: GOTO statement
    // -----------------------------------------------------------------------

    /// GOTO is the unconditional jump. In 1964 BASIC, it is spelled "GOTO"
    /// (no space), not "GO TO". The target is a literal line number.
    ///
    /// Dijkstra's famous 1968 letter "Go To Statement Considered Harmful"
    /// was partly inspired by BASIC's heavy reliance on GOTO.
    #[test]
    fn test_goto() {
        let ast = parse_dartmouth_basic("10 GOTO 50\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "goto_stmt"), "Expected goto_stmt node");
    }

    // -----------------------------------------------------------------------
    // Test group 6: GOSUB / RETURN
    // -----------------------------------------------------------------------

    /// GOSUB is the subroutine call. It pushes the return address onto a
    /// call stack and jumps to the target line. RETURN pops the return
    /// address and continues from there.
    ///
    /// BASIC subroutines don't have names — they're identified by line
    /// number. Nesting subroutines is possible because GOSUB/RETURN use
    /// an actual stack (not a single register).
    #[test]
    fn test_gosub() {
        let ast = parse_dartmouth_basic("10 GOSUB 200\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "gosub_stmt"), "Expected gosub_stmt node");
    }

    #[test]
    fn test_return() {
        let ast = parse_dartmouth_basic("200 RETURN\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "return_stmt"), "Expected return_stmt node");
    }

    // -----------------------------------------------------------------------
    // Test group 7: FOR / NEXT loop
    // -----------------------------------------------------------------------

    /// FOR/NEXT is the counted loop. In 1964 BASIC, the loop variable
    /// must be a scalar variable (not an array element), and the bounds
    /// and step can be arbitrary expressions.
    ///
    ///   10 FOR I = 1 TO 10      (step defaults to +1)
    ///   20 FOR I = 10 TO 1 STEP -1
    ///
    /// The NEXT statement ends the loop body and names the loop variable.
    #[test]
    fn test_for_without_step() {
        let ast = parse_dartmouth_basic("10 FOR I = 1 TO 10\n20 NEXT I\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "for_stmt"), "Expected for_stmt node");
        assert!(find_rule(&ast, "next_stmt"), "Expected next_stmt node");
    }

    #[test]
    fn test_for_with_step() {
        // STEP -1 is the classic countdown loop. The STEP value can be
        // any expression, including negative numbers.
        let ast = parse_dartmouth_basic("10 FOR I = 10 TO 1 STEP -1\n20 NEXT I\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "for_stmt"), "Expected for_stmt node");
        assert!(find_rule(&ast, "next_stmt"), "Expected next_stmt node");
    }

    // -----------------------------------------------------------------------
    // Test group 8: END and STOP
    // -----------------------------------------------------------------------

    /// END is the normal program terminator — every BASIC program must end
    /// with an END statement (usually on the highest line number).
    ///
    /// STOP halts execution with a "STOP IN LINE n" message and was used in
    /// the DTSS (Dartmouth Time-Sharing System) to allow resuming execution
    /// via a CONTINUE command.
    #[test]
    fn test_end() {
        let ast = parse_dartmouth_basic("999 END\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "end_stmt"), "Expected end_stmt node");
    }

    #[test]
    fn test_stop() {
        let ast = parse_dartmouth_basic("500 STOP\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "stop_stmt"), "Expected stop_stmt node");
    }

    // -----------------------------------------------------------------------
    // Test group 9: REM statement
    // -----------------------------------------------------------------------

    /// REM (remark) is BASIC's comment syntax. Everything from REM to the
    /// end of the line is a comment. The lexer strips the comment content,
    /// so by the time the parser sees the token stream, a REM line is just:
    ///   LINE_NUM KEYWORD("REM") NEWLINE
    #[test]
    fn test_rem() {
        let ast = parse_dartmouth_basic("10 REM THIS IS A COMMENT\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "rem_stmt"), "Expected rem_stmt node");
    }

    // -----------------------------------------------------------------------
    // Test group 10: READ / DATA / RESTORE
    // -----------------------------------------------------------------------

    /// READ/DATA/RESTORE implement a sequential data pool:
    ///
    ///   DATA defines values inline in the program text
    ///   READ consumes values from the DATA pool in order
    ///   RESTORE resets the pool pointer to the beginning
    ///
    /// This mechanism lets programs encode test data directly in the source.
    /// It was particularly useful on the DTSS system where files were not
    /// easily accessible from within programs.
    #[test]
    fn test_read_single() {
        let ast = parse_dartmouth_basic("10 READ X\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "read_stmt"), "Expected read_stmt node");
    }

    #[test]
    fn test_read_multiple() {
        let ast = parse_dartmouth_basic("10 READ A, B, C\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "read_stmt"), "Expected read_stmt node");
    }

    #[test]
    fn test_data() {
        let ast = parse_dartmouth_basic("10 DATA 1, 2, 3\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "data_stmt"), "Expected data_stmt node");
    }

    #[test]
    fn test_restore() {
        let ast = parse_dartmouth_basic("10 RESTORE\n");
        assert_program_root(&ast);
        assert!(
            find_rule(&ast, "restore_stmt"),
            "Expected restore_stmt node"
        );
    }

    // -----------------------------------------------------------------------
    // Test group 11: DIM statement
    // -----------------------------------------------------------------------

    /// DIM declares arrays with a maximum index. Without DIM, all arrays
    /// default to 10 elements. DIM is needed for larger arrays.
    ///
    ///   10 DIM A(100)
    ///   20 DIM A(10), B(20), C(5)
    ///
    /// The 1964 spec supports only one-dimensional arrays. The subscript
    /// starts at 1 (not 0) in the original spec, though the GE-225
    /// implementation allowed index 0 as well.
    #[test]
    fn test_dim_single() {
        let ast = parse_dartmouth_basic("10 DIM A(100)\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "dim_stmt"), "Expected dim_stmt node");
        assert!(find_rule(&ast, "dim_decl"), "Expected dim_decl node");
    }

    #[test]
    fn test_dim_multiple() {
        let ast = parse_dartmouth_basic("10 DIM A(10), B(20), C(5)\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "dim_stmt"), "Expected dim_stmt node");
        // Three dim_decls: A(10), B(20), C(5)
        let count = count_rule(&ast, "dim_decl");
        assert!(
            count >= 3,
            "Expected at least 3 dim_decl nodes, got {count}"
        );
    }

    // -----------------------------------------------------------------------
    // Test group 12: DEF statement
    // -----------------------------------------------------------------------

    /// DEF defines a user function. Each function has:
    ///   - A name: FNA through FNZ (26 possible functions)
    ///   - A single formal parameter: a scalar variable name
    ///   - A body: any arithmetic expression (may reference global variables)
    ///
    ///   10 DEF FNA(X) = X * X
    ///   20 DEF FNB(T) = SIN(T) / COS(T)
    ///
    /// User functions are called like built-ins: FNA(5) evaluates to 25.
    #[test]
    fn test_def() {
        let ast = parse_dartmouth_basic("10 DEF FNA(X) = X * X\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "def_stmt"), "Expected def_stmt node");
        assert!(find_rule(&ast, "expr"), "Expected expr node");
    }

    // -----------------------------------------------------------------------
    // Test group 13: Expressions — arithmetic operators
    // -----------------------------------------------------------------------

    /// The expression hierarchy in 1964 BASIC follows standard mathematical
    /// precedence: ^ (highest), then * /, then + - (lowest).
    ///
    /// Each level is a separate grammar rule, which encodes the precedence
    /// relationship through rule nesting.
    #[test]
    fn test_expr_addition() {
        let ast = parse_dartmouth_basic("10 LET X = A + B\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "expr"), "Expected expr node");
    }

    #[test]
    fn test_expr_subtraction() {
        let ast = parse_dartmouth_basic("10 LET X = A - B\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "expr"), "Expected expr node");
    }

    #[test]
    fn test_expr_multiplication() {
        let ast = parse_dartmouth_basic("10 LET X = A * B\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "term"), "Expected term node");
    }

    #[test]
    fn test_expr_division() {
        let ast = parse_dartmouth_basic("10 LET X = A / B\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "term"), "Expected term node");
    }

    #[test]
    fn test_expr_exponentiation() {
        // ^ is right-associative: 2^3^2 = 2^(3^2) = 512
        let ast = parse_dartmouth_basic("10 LET X = 2 ^ 3 ^ 2\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "power"), "Expected power node");
    }

    #[test]
    fn test_expr_unary_minus() {
        // Unary minus: -X, -3.14, -(X + 1)
        let ast = parse_dartmouth_basic("10 LET X = -Y\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "unary"), "Expected unary node");
    }

    #[test]
    fn test_expr_parentheses() {
        // Parentheses override precedence. (A + B) * C is different from A + B * C.
        let ast = parse_dartmouth_basic("10 LET X = (A + B) * C\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "primary"), "Expected primary node");
    }

    // -----------------------------------------------------------------------
    // Test group 14: Built-in functions
    // -----------------------------------------------------------------------

    /// 1964 BASIC has 11 built-in mathematical functions. They cover the
    /// most common scientific calculations (sine, cosine, logarithm, etc.)
    /// plus useful utilities (ABS for absolute value, INT for truncation).
    ///
    /// Built-in functions are single-argument: BUILTIN_FN ( expr ).

    #[test]
    fn test_builtin_sin() {
        let ast = parse_dartmouth_basic("10 LET X = SIN(3.14159)\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "primary"), "Expected primary node");
    }

    #[test]
    fn test_builtin_cos() {
        let ast = parse_dartmouth_basic("10 LET X = COS(0)\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "primary"), "Expected primary node");
    }

    #[test]
    fn test_builtin_tan() {
        let ast = parse_dartmouth_basic("10 LET X = TAN(X)\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "primary"), "Expected primary node");
    }

    #[test]
    fn test_builtin_atn() {
        // ATN is arctangent. ATN(1)*4 ≈ π
        let ast = parse_dartmouth_basic("10 LET X = ATN(1)\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "primary"), "Expected primary node");
    }

    #[test]
    fn test_builtin_exp() {
        // EXP(X) = e^X. EXP(1) ≈ 2.71828
        let ast = parse_dartmouth_basic("10 LET X = EXP(1)\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "primary"), "Expected primary node");
    }

    #[test]
    fn test_builtin_log() {
        // LOG is natural logarithm (base e), not base 10.
        // This was a common source of confusion for students!
        let ast = parse_dartmouth_basic("10 LET X = LOG(2.71828)\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "primary"), "Expected primary node");
    }

    #[test]
    fn test_builtin_abs() {
        let ast = parse_dartmouth_basic("10 LET X = ABS(Y - 5)\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "primary"), "Expected primary node");
    }

    #[test]
    fn test_builtin_sqr() {
        // SQR (square root) — equivalent to X^0.5
        let ast = parse_dartmouth_basic("10 LET X = SQR(2)\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "primary"), "Expected primary node");
    }

    #[test]
    fn test_builtin_int() {
        // INT truncates toward negative infinity (not toward zero).
        // INT(3.9) = 3, INT(-3.1) = -4
        let ast = parse_dartmouth_basic("10 LET X = INT(Y)\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "primary"), "Expected primary node");
    }

    #[test]
    fn test_builtin_rnd() {
        // RND(X) returns a random number between 0 and 1.
        // The argument was historically required but its value is ignored.
        let ast = parse_dartmouth_basic("10 LET X = RND(1)\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "primary"), "Expected primary node");
    }

    #[test]
    fn test_builtin_sgn() {
        // SGN returns -1, 0, or +1 depending on the sign of its argument.
        let ast = parse_dartmouth_basic("10 LET X = SGN(Y)\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "primary"), "Expected primary node");
    }

    // -----------------------------------------------------------------------
    // Test group 15: User-defined functions
    // -----------------------------------------------------------------------

    /// User-defined functions (FNA through FNZ) are declared with DEF and
    /// called like built-in functions: FNA(expr).
    #[test]
    fn test_user_fn_fna() {
        let ast = parse_dartmouth_basic("10 DEF FNA(X) = X * X\n20 LET Y = FNA(5)\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "def_stmt"), "Expected def_stmt node");
        assert!(find_rule(&ast, "primary"), "Expected primary node");
    }

    #[test]
    fn test_user_fn_fnz() {
        let ast = parse_dartmouth_basic("10 DEF FNZ(T) = SIN(T) / COS(T)\n20 LET X = FNZ(1)\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "def_stmt"), "Expected def_stmt node");
    }

    // -----------------------------------------------------------------------
    // Test group 16: Array subscript in expressions
    // -----------------------------------------------------------------------

    /// Arrays can be used on the right side of expressions too.
    /// The variable rule handles: NAME | NAME ( expr )
    #[test]
    fn test_array_subscript_expr() {
        let ast = parse_dartmouth_basic("10 LET X = A(I)\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "variable"), "Expected variable node");
    }

    #[test]
    fn test_array_subscript_expr_complex() {
        // Array indices can be expressions: A(I+1)
        let ast = parse_dartmouth_basic("10 LET X = A(I + 1)\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "variable"), "Expected variable node");
    }

    // -----------------------------------------------------------------------
    // Test group 17: Multi-line programs
    // -----------------------------------------------------------------------

    /// The real power of a parser is handling complete programs. These
    /// tests exercise multiple statements working together.

    #[test]
    fn test_hello_world() {
        // The quintessential first BASIC program — printed on the teletype
        // at Dartmouth College by students learning to use the computer for
        // the first time.
        let src = "10 PRINT \"HELLO, WORLD\"\n20 END\n";
        let ast = parse_dartmouth_basic(src);
        assert_program_root(&ast);
        let line_count = count_rule(&ast, "line");
        assert!(
            line_count >= 2,
            "Expected at least 2 line nodes, got {line_count}"
        );
    }

    #[test]
    fn test_counting_loop() {
        // A simple counting loop: print numbers 1 through 10.
        // This was a common example in the original Dartmouth BASIC manual.
        let src = "10 FOR I = 1 TO 10\n20 PRINT I\n30 NEXT I\n40 END\n";
        let ast = parse_dartmouth_basic(src);
        assert_program_root(&ast);
        assert!(find_rule(&ast, "for_stmt"), "Expected for_stmt");
        assert!(find_rule(&ast, "next_stmt"), "Expected next_stmt");
        assert!(find_rule(&ast, "print_stmt"), "Expected print_stmt");
        assert!(find_rule(&ast, "end_stmt"), "Expected end_stmt");
    }

    #[test]
    fn test_conditional_program() {
        // A program that uses IF-THEN and GOTO to implement a bounded loop.
        // Before FOR was common, this was how loops were written in BASIC.
        let src = "10 LET I = 1\n20 IF I > 10 THEN 60\n30 PRINT I\n40 LET I = I + 1\n50 GOTO 20\n60 END\n";
        let ast = parse_dartmouth_basic(src);
        assert_program_root(&ast);
        assert!(find_rule(&ast, "if_stmt"), "Expected if_stmt");
        assert!(find_rule(&ast, "goto_stmt"), "Expected goto_stmt");
    }

    #[test]
    fn test_subroutine_program() {
        // A program that uses GOSUB/RETURN to implement a subroutine.
        // Subroutines let you reuse code without duplicating it.
        let src = "10 GOSUB 100\n20 GOSUB 100\n30 END\n100 PRINT \"SUBROUTINE\"\n110 RETURN\n";
        let ast = parse_dartmouth_basic(src);
        assert_program_root(&ast);
        assert!(find_rule(&ast, "gosub_stmt"), "Expected gosub_stmt");
        assert!(find_rule(&ast, "return_stmt"), "Expected return_stmt");
    }

    // -----------------------------------------------------------------------
    // Test group 18: Edge case — bare line number
    // -----------------------------------------------------------------------

    /// A bare line number with no statement is valid BASIC. In the DTSS
    /// interactive environment, typing just a line number deleted that line.
    /// When parsing a stored program, it produces a no-op line node.
    #[test]
    fn test_bare_line_number() {
        let ast = parse_dartmouth_basic("10\n");
        assert_program_root(&ast);
        let line_count = count_rule(&ast, "line");
        assert_eq!(line_count, 1, "Expected exactly 1 line node");
    }

    // -----------------------------------------------------------------------
    // Test group 19: Factory function
    // -----------------------------------------------------------------------

    /// The `create_dartmouth_basic_parser` factory returns a working
    /// GrammarParser that can successfully parse BASIC programs.
    #[test]
    fn test_create_parser() {
        let mut parser = create_dartmouth_basic_parser("10 LET X = 5\n");
        let result = parser.parse();
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());

        let ast = result.unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    // -----------------------------------------------------------------------
    // Test group 20: READ/DATA round-trip program
    // -----------------------------------------------------------------------

    #[test]
    fn test_read_data_program() {
        // A program that reads from a DATA pool — the classic way to embed
        // test data or lookup tables in a BASIC program.
        let src = "10 READ X\n20 PRINT X\n30 DATA 42\n40 END\n";
        let ast = parse_dartmouth_basic(src);
        assert_program_root(&ast);
        assert!(find_rule(&ast, "read_stmt"), "Expected read_stmt");
        assert!(find_rule(&ast, "data_stmt"), "Expected data_stmt");
    }

    // -----------------------------------------------------------------------
    // Test group 21: Complex expression — combined operators
    // -----------------------------------------------------------------------

    #[test]
    fn test_complex_expression() {
        // Test that operator precedence cascading works correctly:
        // 2 + 3 * 4 ^ 2 should parse as 2 + (3 * (4 ^ 2)) = 50
        let ast = parse_dartmouth_basic("10 LET X = 2 + 3 * 4 ^ 2\n");
        assert_program_root(&ast);
        assert!(find_rule(&ast, "expr"), "Expected expr");
        assert!(find_rule(&ast, "term"), "Expected term");
        assert!(find_rule(&ast, "power"), "Expected power");
    }
}
