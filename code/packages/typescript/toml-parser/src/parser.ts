/**
 * TOML Parser -- parses TOML text into ASTs using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from the
 * `@coding-adventures/parser` package. It loads the `toml.grammar` file and
 * delegates all parsing work to the generic engine.
 *
 * How It Works
 * ------------
 *
 * The parsing pipeline has two stages:
 *
 *   1. **Lexing** -- The toml-lexer reads the source text and produces a flat
 *      array of tokens. Each token has a type (BARE_KEY, BASIC_STRING, INTEGER,
 *      LBRACKET, NEWLINE, etc.) and a value (the actual text). TOML lexing is
 *      significantly more complex than JSON because of four string types,
 *      date/time literals, bare keys, comments, and newline sensitivity.
 *
 *   2. **Parsing** -- The GrammarParser reads the toml.grammar file, which
 *      defines the syntax rules (document, expression, keyval, key, value,
 *      array, inline_table, etc.) using EBNF-like notation. It then applies
 *      recursive descent with backtracking to match the token stream against
 *      these rules, producing an AST.
 *
 * The AST Structure
 * -----------------
 *
 * The resulting AST is a tree of `ASTNode` objects. Each node has:
 *   - `ruleName` -- the grammar rule that produced this node (e.g., "document",
 *     "expression", "keyval", "key", "value", "array", "inline_table")
 *   - `children` -- an array of child nodes or leaf tokens
 *
 * For example, parsing `title = "TOML Example"` produces roughly:
 *
 *     ASTNode("document", [
 *       ASTNode("expression", [
 *         ASTNode("keyval", [
 *           ASTNode("key", [
 *             ASTNode("simple_key", [Token(BARE_KEY, "title")])
 *           ]),
 *           Token(EQUALS, "="),
 *           ASTNode("value", [Token(BASIC_STRING, "TOML Example")])
 *         ])
 *       ])
 *     ])
 *
 * TOML Grammar Rules
 * ------------------
 *
 * The TOML grammar (toml.grammar) has ~12 rules:
 *
 *   - **document** -- the top-level rule. A sequence of expressions separated
 *     by newlines. Handles blank lines and leading/trailing newlines.
 *   - **expression** -- one of: array_table_header, table_header, or keyval.
 *   - **keyval** -- key EQUALS value (the fundamental TOML construct).
 *   - **key** -- one or more simple_keys separated by DOTs (dotted keys).
 *   - **simple_key** -- BARE_KEY, quoted string, or any value token that could
 *     appear as a bare key (TRUE, FALSE, INTEGER, FLOAT, dates, etc.).
 *   - **table_header** -- LBRACKET key RBRACKET (e.g., [server]).
 *   - **array_table_header** -- LBRACKET LBRACKET key RBRACKET RBRACKET
 *     (e.g., [[products]]).
 *   - **value** -- any TOML value: string, integer, float, boolean, datetime,
 *     array, or inline table.
 *   - **array** -- LBRACKET array_values RBRACKET (comma-separated values).
 *   - **array_values** -- handles multi-line arrays with optional trailing comma.
 *   - **inline_table** -- LBRACE [ keyval { COMMA keyval } ] RBRACE.
 *
 * The grammar is recursive: `value` references `array` and `inline_table`,
 * which reference `value` and `keyval` respectively, which reference `value`
 * again. This mutual recursion allows TOML to represent arbitrarily deep
 * nested structures.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `toml.grammar` file lives in `code/grammars/` at the repository root.
 *
 *     src/ -> toml-parser/ -> typescript/ -> packages/ -> code/ -> grammars/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeTOML } from "@coding-adventures/toml-lexer";

/**
 * Resolve __dirname for ESM modules.
 * See the toml-lexer tokenizer.ts for a detailed explanation.
 */
const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Navigate from src/ up to the grammars/ directory.
 *
 * The path traversal:
 *   __dirname  = .../toml-parser/src/
 *   ..          = .../toml-parser/
 *   ../..       = .../typescript/
 *   ../../..    = .../packages/
 *   ../../../.. = .../code/
 *   + grammars  = .../code/grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const TOML_GRAMMAR_PATH = join(GRAMMARS_DIR, "toml.grammar");

/**
 * Parse TOML text and return an AST.
 *
 * This function orchestrates the full parsing pipeline:
 *   1. Tokenize the source using the toml-lexer
 *   2. Read and parse the toml.grammar file
 *   3. Run the grammar-driven parser to produce an AST
 *
 * The top-level grammar rule is "document", so the returned AST node
 * always has `ruleName` of `"document"`.
 *
 * @param source - The TOML text to parse.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"document"`.
 *
 * @example
 *     const ast = parseTOML('[server]\nhost = "localhost"');
 *     console.log(ast.ruleName); // "document"
 *
 * @example
 *     // Parse a simple key-value pair:
 *     const ast = parseTOML('title = "TOML Example"');
 *
 * @example
 *     // Parse a document with multiple tables:
 *     const ast = parseTOML('[database]\nserver = "192.168.1.1"\nport = 5432');
 */
export function parseTOML(source: string): ASTNode {
  /**
   * Step 1: Tokenize.
   * The toml-lexer handles all four string types, date/time recognition,
   * multiple integer formats, bare keys, comments, and newline emission.
   */
  const tokens = tokenizeTOML(source);

  /**
   * Step 2: Load the grammar.
   * The grammar file defines the syntax rules in EBNF-like notation.
   * parseParserGrammar converts the text into a structured object that
   * the GrammarParser can use for recursive descent.
   */
  const grammarText = readFileSync(TOML_GRAMMAR_PATH, "utf-8");
  const grammar = parseParserGrammar(grammarText);

  /**
   * Step 3: Parse.
   * The GrammarParser takes the token array and grammar rules, then
   * performs recursive descent with backtracking to produce an AST.
   * The starting rule is "document" (the first rule in toml.grammar),
   * which expects a sequence of expressions separated by newlines.
   */
  const parser = new GrammarParser(tokens, grammar);
  return parser.parse();
}
