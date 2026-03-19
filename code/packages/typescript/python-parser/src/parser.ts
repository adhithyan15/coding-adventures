/**
 * Python Parser — parses Python source code into ASTs using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from the
 * `@coding-adventures/parser` package. It demonstrates the same core idea as the
 * Python lexer: the *same* parser engine that handles one language can handle
 * Python — just swap the `.grammar` file.
 *
 * How the Grammar-Driven Parser Works (Brief Recap)
 * --------------------------------------------------
 *
 * The `GrammarParser` interprets EBNF grammar rules at runtime. For each rule,
 * it tries to match the token stream against the rule's body using a recursive
 * descent approach with backtracking:
 *
 * - **Sequences** (`A B C`) must match all elements in order.
 * - **Alternations** (`A | B`) try each choice; first match wins.
 * - **Repetitions** (`{ A }`) match zero or more times.
 * - **Optionals** (`[ A ]`) match zero or one time.
 * - **Token references** (`NUMBER`, `NAME`) match tokens by type.
 * - **Literals** (`"puts"`) match tokens by exact text value.
 * - **Rule references** (`expression`) recursively parse another rule.
 *
 * What This Module Provides
 * -------------------------
 *
 * One convenience function:
 *
 * - `parsePython(source)` — the all-in-one function. Pass in Python source
 *   code, get back an AST. This is the function most callers want.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `python.grammar` file lives in `code/grammars/` at the repository root.
 * We locate it relative to this module's file path, similar to how the Python
 * lexer locates `python.tokens`.
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizePython } from "@coding-adventures/python-lexer";

// ---------------------------------------------------------------------------
// Grammar File Location
// ---------------------------------------------------------------------------
//
// We navigate from this file's directory (src/) up four levels to reach
// the code/ directory, then into grammars/.
//
//   src/ -> python-parser/ -> typescript/ -> packages/ -> code/ -> grammars/
// ---------------------------------------------------------------------------

const __dirname = dirname(fileURLToPath(import.meta.url));
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const PYTHON_GRAMMAR_PATH = join(GRAMMARS_DIR, "python.grammar");

/**
 * Parse Python source code and return an AST.
 *
 * This is the main entry point for the Python parser. Pass in a string of
 * Python source code, and get back an `ASTNode` representing the complete
 * parse tree.
 *
 * The pipeline is:
 * 1. Tokenize the source using the Python lexer (which loads `python.tokens`).
 * 2. Read and parse the `python.grammar` file.
 * 3. Feed the tokens and grammar into `GrammarParser`.
 * 4. Return the resulting AST.
 *
 * @param source - The Python source code to parse.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"program"`.
 *
 * @example
 *     const ast = parsePython("x = 1 + 2");
 *     console.log(ast.ruleName); // "program"
 */
export function parsePython(source: string): ASTNode {
  const tokens = tokenizePython(source);
  const grammarText = readFileSync(PYTHON_GRAMMAR_PATH, "utf-8");
  const grammar = parseParserGrammar(grammarText);
  const parser = new GrammarParser(tokens, grammar);
  return parser.parse();
}
