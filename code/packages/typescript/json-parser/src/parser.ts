/**
 * JSON Parser -- parses JSON text into ASTs using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from the
 * `@coding-adventures/parser` package. It loads the `json.grammar` file and
 * delegates all parsing work to the generic engine.
 *
 * How It Works
 * ------------
 *
 * The parsing pipeline has two stages:
 *
 *   1. **Lexing** -- The json-lexer reads the source text and produces a flat
 *      array of tokens. Each token has a type (STRING, NUMBER, TRUE, LBRACE, etc.)
 *      and a value (the actual text). JSON lexing is simpler than programming
 *      languages because there are no keywords, no comments, and no significant
 *      whitespace.
 *
 *   2. **Parsing** -- The GrammarParser reads the json.grammar file, which
 *      defines the syntax rules (value, object, pair, array) using EBNF-like
 *      notation. It then applies recursive descent with backtracking to match
 *      the token stream against these rules, producing an AST.
 *
 * The AST Structure
 * -----------------
 *
 * The resulting AST is a tree of `ASTNode` objects. Each node has:
 *   - `ruleName` -- the grammar rule that produced this node (e.g., "value",
 *     "object", "pair", "array")
 *   - `children` -- an array of child nodes or leaf tokens
 *
 * For example, parsing `{"name": "Alice"}` produces roughly:
 *
 *     ASTNode("value", [
 *       ASTNode("object", [
 *         Token(LBRACE, "{"),
 *         ASTNode("pair", [
 *           Token(STRING, '"name"'),
 *           Token(COLON, ":"),
 *           ASTNode("value", [Token(STRING, '"Alice"')])
 *         ]),
 *         Token(RBRACE, "}")
 *       ])
 *     ])
 *
 * JSON Grammar Rules
 * ------------------
 *
 * The JSON grammar (json.grammar) has just four rules:
 *
 *   - **value** -- the top-level rule. A value is one of: object, array,
 *     STRING, NUMBER, TRUE, FALSE, or NULL.
 *   - **object** -- LBRACE [ pair { COMMA pair } ] RBRACE
 *   - **pair** -- STRING COLON value (key-value pair in an object)
 *   - **array** -- LBRACKET [ value { COMMA value } ] RBRACKET
 *
 * The grammar is recursive: `value` references `object` and `array`, which
 * reference `value` again. This mutual recursion allows JSON to represent
 * arbitrarily deep nested structures.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `json.grammar` file lives in `code/grammars/` at the repository root.
 *
 *     src/ -> json-parser/ -> typescript/ -> packages/ -> code/ -> grammars/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeJSON } from "@coding-adventures/json-lexer";

/**
 * Resolve __dirname for ESM modules.
 * See the json-lexer tokenizer.ts for a detailed explanation.
 */
const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Navigate from src/ up to the grammars/ directory.
 *
 * The path traversal:
 *   __dirname  = .../json-parser/src/
 *   ..          = .../json-parser/
 *   ../..       = .../typescript/
 *   ../../..    = .../packages/
 *   ../../../.. = .../code/
 *   + grammars  = .../code/grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const JSON_GRAMMAR_PATH = join(GRAMMARS_DIR, "json.grammar");

/**
 * Parse JSON text and return an AST.
 *
 * This function orchestrates the full parsing pipeline:
 *   1. Tokenize the source using the json-lexer
 *   2. Read and parse the json.grammar file
 *   3. Run the grammar-driven parser to produce an AST
 *
 * @param source - The JSON text to parse.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"value"`.
 *
 * @example
 *     const ast = parseJSON('{"name": "Alice"}');
 *     console.log(ast.ruleName); // "value"
 *
 * @example
 *     // Parse an array of numbers:
 *     const ast = parseJSON("[1, 2, 3]");
 *
 * @example
 *     // Parse a deeply nested structure:
 *     const ast = parseJSON('{"users": [{"name": "Alice"}, {"name": "Bob"}]}');
 */
export function parseJSON(source: string): ASTNode {
  /**
   * Step 1: Tokenize.
   * The json-lexer handles string and number recognition, literal matching,
   * whitespace skipping, and all the lexical details.
   */
  const tokens = tokenizeJSON(source);

  /**
   * Step 2: Load the grammar.
   * The grammar file defines the syntax rules in EBNF-like notation.
   * parseParserGrammar converts the text into a structured object that
   * the GrammarParser can use for recursive descent.
   */
  const grammarText = readFileSync(JSON_GRAMMAR_PATH, "utf-8");
  const grammar = parseParserGrammar(grammarText);

  /**
   * Step 3: Parse.
   * The GrammarParser takes the token array and grammar rules, then
   * performs recursive descent with backtracking to produce an AST.
   * The starting rule is determined by the grammar (the first rule
   * defined, which for json.grammar is "value").
   */
  const parser = new GrammarParser(tokens, grammar);
  return parser.parse();
}
