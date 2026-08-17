/**
 * Dartmouth BASIC Parser -- parses 1964 BASIC source text into ASTs.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from
 * the `@coding-adventures/parser` package. It loads the
 * `dartmouth_basic.grammar` file and delegates all parsing work to the
 * generic engine.
 *
 * Historical Context
 * ------------------
 *
 * Dartmouth BASIC was created by John G. Kemeny and Thomas E. Kurtz at
 * Dartmouth College in 1964. It ran on a GE-225 mainframe, accessed via
 * uppercase-only teletypes. It was the first programming language designed
 * specifically for non-science students — the goal was to make computing
 * accessible to everyone, not just mathematicians and engineers.
 *
 * A typical 1964 BASIC program looked like this:
 *
 *     10 LET X = 1
 *     20 PRINT X
 *     30 LET X = X + 1
 *     40 IF X <= 10 THEN 20
 *     50 END
 *
 * Every line begins with a line number, contains exactly one statement,
 * and the NEWLINE at the end is syntactically significant (unlike most
 * languages, where whitespace is ignored).
 *
 * How It Works
 * ------------
 *
 * The parsing pipeline has two stages:
 *
 *   1. **Lexing** -- The dartmouth-basic-lexer reads the source text and
 *      produces a flat array of tokens. Token types include LINE_NUM,
 *      KEYWORD, NAME, NUMBER, STRING, BUILTIN_FN, USER_FN, EQ, LT, GT,
 *      LE, GE, NE, PLUS, MINUS, STAR, SLASH, CARET, LPAREN, RPAREN,
 *      COMMA, SEMICOLON, NEWLINE, and EOF.
 *
 *   2. **Parsing** -- The GrammarParser reads the dartmouth_basic.grammar
 *      file, which defines ~25 syntax rules in EBNF-like notation. It then
 *      applies recursive descent with backtracking to match the token
 *      stream against these rules, producing an AST.
 *
 * The AST Structure
 * -----------------
 *
 * The resulting AST is a tree of `ASTNode` objects. The root has
 * `ruleName = "program"`. Its children are `line` nodes, each containing:
 *   - A LINE_NUM token
 *   - An optional `statement` node (one of the 17 statement types)
 *   - A NEWLINE token
 *
 * For example, parsing `"10 LET X = 5\n"` produces roughly:
 *
 *     ASTNode("program", [
 *       ASTNode("line", [
 *         Token(LINE_NUM, "10"),
 *         ASTNode("statement", [
 *           ASTNode("let_stmt", [
 *             Token(KEYWORD, "LET"),
 *             ASTNode("variable", [Token(NAME, "X")]),
 *             Token(EQ, "="),
 *             ASTNode("expr", [...])
 *           ])
 *         ]),
 *         Token(NEWLINE, "\n")
 *       ])
 *     ])
 *
 * Grammar Overview
 * ----------------
 *
 * The dartmouth_basic.grammar has ~25 rules:
 *
 *   **Structure rules:**
 *   - program    — { line } (zero or more numbered lines)
 *   - line       — LINE_NUM [ statement ] NEWLINE
 *   - statement  — one of the 17 statement type rules
 *
 *   **Statement rules (17 total):**
 *   - let_stmt, print_stmt, input_stmt, if_stmt, goto_stmt, gosub_stmt,
 *     return_stmt, for_stmt, next_stmt, end_stmt, stop_stmt, rem_stmt,
 *     read_stmt, data_stmt, restore_stmt, dim_stmt, def_stmt
 *
 *   **Expression rules (precedence cascade):**
 *   - expr    — addition and subtraction (lowest precedence)
 *   - term    — multiplication and division
 *   - power   — exponentiation (right-associative)
 *   - unary   — unary minus
 *   - primary — atomic values (number, function call, variable, parentheses)
 *
 *   **Helper rules:**
 *   - variable  — scalar NAME or array NAME(expr)
 *   - relop     — relational operator (=, <, >, <=, >=, <>)
 *   - print_list, print_item, print_sep — PRINT argument structure
 *   - dim_decl  — array dimension declaration
 *
 * Grammar Data
 * ------------
 *
 * The `dartmouth_basic.grammar` file under `code/grammars/dartmouth_basic/`
 * at the repository root is compiled ahead of time into `./_grammar.ts`
 * (see `code/scripts/_ts_grammar_compile.ts`). This module statically
 * imports it rather than reading the monorepo's `code/grammars/` tree at
 * runtime, so a published npm package works standalone.
 */

import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeDartmouthBasic } from "@coding-adventures/dartmouth-basic-lexer";

import { PARSER_GRAMMAR } from "./_grammar.js";

/**
 * Parse Dartmouth BASIC source text and return an AST.
 *
 * This function orchestrates the full parsing pipeline:
 *   1. Tokenize the source using the dartmouth-basic-lexer
 *   2. Read and parse the dartmouth_basic.grammar file
 *   3. Run the grammar-driven parser to produce an AST
 *
 * The root node has `ruleName = "program"`. Its children are `line` nodes,
 * each wrapping a LINE_NUM token, an optional statement, and a NEWLINE token.
 *
 * @param source - The BASIC source text to parse. Lines must end with `\n`.
 * @returns An ASTNode representing the parse tree, with `ruleName = "program"`.
 *
 * @example
 *     // Parse a simple assignment
 *     const ast = parseDartmouthBasic("10 LET X = 5\n");
 *     console.log(ast.ruleName); // "program"
 *
 * @example
 *     // Parse a complete counting program
 *     const src = "10 FOR I = 1 TO 10\n20 PRINT I\n30 NEXT I\n40 END\n";
 *     const ast = parseDartmouthBasic(src);
 *
 * @example
 *     // Parse a program using GOSUB/RETURN
 *     const src = "10 GOSUB 100\n20 END\n100 PRINT \"HI\"\n110 RETURN\n";
 *     const ast = parseDartmouthBasic(src);
 */
export function parseDartmouthBasic(source: string): ASTNode {
  /**
   * Step 1: Tokenize.
   *
   * The dartmouth-basic-lexer handles all the lexical details:
   *   - Recognizing LINE_NUM at the start of each line
   *   - Distinguishing KEYWORD ("LET", "PRINT", "IF", ...) from NAME (A–Z)
   *   - Distinguishing BUILTIN_FN (SIN, COS, ...) from USER_FN (FNA–FNZ)
   *   - Producing NEWLINE tokens (significant in BASIC — they terminate stmts)
   *   - Stripping REM content (everything after REM to end of line)
   */
  const tokens = tokenizeDartmouthBasic(source);

  /**
   * Step 2: Use the pre-compiled grammar.
   *
   * `PARSER_GRAMMAR` (compiled ahead of time from dartmouth_basic.grammar)
   * defines ~25 rules in EBNF-like notation. The rules encode the full
   * syntax of 1964 Dartmouth BASIC, including:
   *   - All 17 statement types
   *   - Expression precedence (expr > term > power > unary > primary)
   *   - Right-associativity of ^ via self-reference in the `power` rule
   *   - Operator alternations (PLUS | MINUS for addition/subtraction, etc.)
   */
  const grammar = PARSER_GRAMMAR;

  /**
   * Step 3: Parse.
   *
   * The GrammarParser takes the token array and grammar rules, then
   * performs recursive descent with backtracking to produce an AST.
   * The starting rule is the first rule defined in the grammar, which
   * for dartmouth_basic.grammar is "program".
   *
   * The GrammarParser is a packrat parser (memoized recursive descent),
   * so backtracking is efficient — each (rule, position) pair is only
   * evaluated once, giving O(n) worst-case complexity.
   */
  const parser = new GrammarParser(tokens, grammar);
  return parser.parse();
}
