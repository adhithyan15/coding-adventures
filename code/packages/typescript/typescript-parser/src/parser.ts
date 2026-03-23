/**
 * TypeScript Parser — parses TypeScript source code into ASTs using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from the
 * `@coding-adventures/parser` package. It loads the `typescript.grammar` file and
 * delegates all parsing work to the generic engine.
 *
 * The TypeScript grammar extends the JavaScript grammar with:
 * - Type annotations (`: number`, `: string`, `: boolean`)
 * - Interface and type alias declarations
 * - Generic syntax
 * - All JavaScript grammar rules carry over (var_declaration, assignment, etc.)
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `typescript.grammar` file lives in `code/grammars/` at the repository root.
 *
 *     src/ -> typescript-parser/ -> typescript/ -> packages/ -> code/ -> grammars/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeTypescript } from "@coding-adventures/typescript-lexer";

const __dirname = dirname(fileURLToPath(import.meta.url));
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const TS_GRAMMAR_PATH = join(GRAMMARS_DIR, "typescript.grammar");

/**
 * Parse TypeScript source code and return an AST.
 *
 * @param source - The TypeScript source code to parse.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"program"`.
 *
 * @example
 *     const ast = parseTypescript("let x = 1 + 2;");
 *     console.log(ast.ruleName); // "program"
 */
export function parseTypescript(source: string): ASTNode {
  const tokens = tokenizeTypescript(source);
  const grammarText = readFileSync(TS_GRAMMAR_PATH, "utf-8");
  const grammar = parseParserGrammar(grammarText);
  const parser = new GrammarParser(tokens, grammar);
  return parser.parse();
}
