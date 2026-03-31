/**
 * SQL Parser -- parses SQL text into ASTs using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from the
 * `@coding-adventures/parser` package. It uses a pre-compiled grammar object
 * (from `_grammar.ts`) and delegates all parsing work to the generic engine.
 *
 * How It Works
 * ------------
 *
 * The parsing pipeline has two stages:
 *
 *   1. **Lexing** -- The sql-lexer reads the source text and produces a flat
 *      array of tokens. Because sql.tokens uses @case_insensitive true, all SQL
 *      keywords are normalized to uppercase regardless of how they were typed.
 *      This means `select`, `SELECT`, and `Select` all produce KEYWORD("SELECT"),
 *      and the parser grammar can always compare against uppercase strings.
 *
 *   2. **Parsing** -- The GrammarParser uses a pre-compiled grammar object that
 *      defines the syntax rules for SQL statements (SELECT, INSERT, UPDATE,
 *      DELETE, CREATE, DROP) and expressions (arithmetic, comparisons, BETWEEN,
 *      IN, LIKE, IS NULL). It applies recursive descent with backtracking to
 *      produce an AST.
 *
 * The AST Structure
 * -----------------
 *
 * The resulting AST is a tree of `ASTNode` objects. Each node has:
 *   - `ruleName` -- the grammar rule that produced this node (e.g., "program",
 *     "select_stmt", "where_clause", "expr")
 *   - `children` -- an array of child nodes or leaf tokens
 *
 * For example, parsing `SELECT id FROM users` produces roughly:
 *
 *     ASTNode("program", [
 *       ASTNode("statement", [
 *         ASTNode("select_stmt", [
 *           Token(KEYWORD, "SELECT"),
 *           ASTNode("select_list", [
 *             ASTNode("select_item", [
 *               ASTNode("expr", [ ... ASTNode("column_ref", [Token(NAME, "id")]) ])
 *             ])
 *           ]),
 *           Token(KEYWORD, "FROM"),
 *           ASTNode("table_ref", [
 *             ASTNode("table_name", [Token(NAME, "users")])
 *           ])
 *         ])
 *       ])
 *     ])
 *
 * SQL Grammar Overview
 * --------------------
 *
 * The `sql.grammar` file defines rules for an ANSI SQL subset:
 *
 *   - **program** -- one or more statements separated by semicolons
 *   - **statement** -- select_stmt | insert_stmt | update_stmt |
 *                       delete_stmt | create_table_stmt | drop_table_stmt
 *   - **select_stmt** -- SELECT [DISTINCT|ALL] columns FROM table [joins]
 *                          [WHERE] [GROUP BY] [HAVING] [ORDER BY] [LIMIT]
 *   - **insert_stmt** -- INSERT INTO table [(cols)] VALUES (vals)
 *   - **update_stmt** -- UPDATE table SET col=expr [WHERE]
 *   - **delete_stmt** -- DELETE FROM table [WHERE]
 *   - **create_table_stmt** -- CREATE TABLE [IF NOT EXISTS] name (col_defs)
 *   - **drop_table_stmt** -- DROP TABLE [IF EXISTS] name
 *   - **expr** -- recursive expression grammar (or/and/not/comparison/arithmetic)
 *
 * The grammar is recursive through expressions: `expr` → `or_expr` → `and_expr` →
 * `not_expr` → `comparison` → `additive` → `multiplicative` → `unary` → `primary`.
 * This recursive structure enforces standard operator precedence: AND binds tighter
 * than OR, comparison operators bind tighter than AND, and arithmetic operators
 * bind tightest of all.
 *
 * Operator Precedence (highest to lowest)
 * ----------------------------------------
 *
 *   Level 1 (highest): Unary minus, primary values (NUMBER, STRING, column_ref)
 *   Level 2: Multiplication, division, modulo  (* / %)
 *   Level 3: Addition, subtraction  (+ -)
 *   Level 4: Comparisons  (= != < > <= >= BETWEEN IN LIKE IS NULL)
 *   Level 5: NOT
 *   Level 6: AND
 *   Level 7 (lowest): OR
 *
 * Browser Compatibility
 * ---------------------
 *
 * This module uses a pre-compiled grammar object imported from `_grammar.ts`.
 * No file system access is needed at runtime — it works in Node.js, browsers,
 * edge runtimes, and any other JavaScript environment.
 */

import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeSQL } from "coding-adventures-sql-lexer";
import { PARSER_GRAMMAR } from "./_grammar.js";

/**
 * Create a configured SQL parser for the given source text.
 *
 * This function is an alias for `parseSQL`. It makes it explicit that a
 * SQL parser is being constructed, which is useful for testing that the
 * factory pattern works.
 *
 * @param source - The SQL text to parse.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"program"`.
 *
 * @example
 *     const ast = createSQLParser("SELECT 1");
 *     console.log(ast.ruleName); // "program"
 */
export function createSQLParser(source: string): ASTNode {
  return parseSQL(source);
}

/**
 * Parse SQL text and return an AST.
 *
 * This function orchestrates the full parsing pipeline:
 *   1. Tokenize the source using the sql-lexer (keywords normalized to uppercase)
 *   2. Parse with the pre-compiled grammar object using recursive descent
 *
 * The top-level grammar rule is `program`, which matches one or more SQL
 * statements separated by semicolons. Every valid SQL input produces an AST
 * with `ruleName === "program"`.
 *
 * @param source - The SQL text to parse.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"program"`.
 *
 * @throws ParseError if the SQL is invalid or unrecognized by the grammar.
 *
 * @example
 *     const ast = parseSQL("SELECT id, name FROM users");
 *     console.log(ast.ruleName); // "program"
 *
 * @example
 *     // Case-insensitive: all three parse identically
 *     parseSQL("SELECT * FROM users");
 *     parseSQL("select * from users");
 *     parseSQL("Select * From Users");
 *
 * @example
 *     // Parse an INSERT statement
 *     const ast = parseSQL("INSERT INTO users (name, age) VALUES ('Alice', 30)");
 *
 * @example
 *     // Multiple statements
 *     const ast = parseSQL("SELECT 1; SELECT 2");
 */
export function parseSQL(source: string): ASTNode {
  /**
   * Step 1: Tokenize.
   * The sql-lexer handles keyword detection, case-insensitive normalization,
   * string literal recognition, comment skipping, and operator matching.
   * All SQL keywords arrive here as KEYWORD tokens with uppercase values.
   */
  const tokens = tokenizeSQL(source);

  /**
   * Step 2: Parse.
   * The PARSER_GRAMMAR object is a pre-compiled ParserGrammar containing
   * the SQL syntax rules in structured form. The GrammarParser takes the
   * token array and grammar rules, then performs recursive descent with
   * backtracking to produce an AST. The starting rule is determined by
   * the grammar (the first rule defined, which for sql.grammar is "program").
   */
  const parser = new GrammarParser(tokens, PARSER_GRAMMAR);
  return parser.parse();
}
