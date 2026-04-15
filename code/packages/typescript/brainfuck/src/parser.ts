/**
 * Brainfuck Parser -- parses Brainfuck source into ASTs using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from the
 * `@coding-adventures/parser` package. It loads the `brainfuck.grammar` file and
 * delegates all parsing work to the generic engine.
 *
 * How It Works
 * ------------
 *
 * The parsing pipeline has two stages:
 *
 *   1. **Lexing** -- `tokenizeBrainfuck` reads the source text and produces a flat
 *      array of tokens. Each token has a type (RIGHT, LEFT, INC, DEC, OUTPUT,
 *      INPUT, LOOP_START, LOOP_END) and a value (the single command character).
 *      Comments and whitespace are discarded during this stage -- they never reach
 *      the parser.
 *
 *   2. **Parsing** -- The GrammarParser reads the `brainfuck.grammar` file, which
 *      defines syntax rules in EBNF-like notation. It then applies recursive descent
 *      with backtracking to match the token stream against these rules, producing an AST.
 *
 * The AST Structure
 * -----------------
 *
 * The resulting AST is a tree of `ASTNode` objects. Each node has:
 *   - `ruleName` -- the grammar rule that produced this node (e.g., "program",
 *     "instruction", "loop", "command")
 *   - `children` -- an array of child nodes or leaf tokens
 *
 * For example, parsing `++[>+<-]` produces roughly:
 *
 *     ASTNode("program", [
 *       ASTNode("instruction", [ASTNode("command", [Token(INC, "+")])]),
 *       ASTNode("instruction", [ASTNode("command", [Token(INC, "+")])]),
 *       ASTNode("instruction", [
 *         ASTNode("loop", [
 *           Token(LOOP_START, "["),
 *           ASTNode("instruction", [ASTNode("command", [Token(RIGHT, ">")])]),
 *           ASTNode("instruction", [ASTNode("command", [Token(INC, "+")])]),
 *           ASTNode("instruction", [ASTNode("command", [Token(LEFT, "<")])]),
 *           ASTNode("instruction", [ASTNode("command", [Token(DEC, "-")])]),
 *           Token(LOOP_END, "]")
 *         ])
 *       ])
 *     ])
 *
 * Brainfuck Grammar Rules
 * -----------------------
 *
 * The Brainfuck grammar (brainfuck.grammar) has four rules:
 *
 *   - **program** -- the top-level rule. A sequence of zero or more instructions.
 *     An empty file is a valid Brainfuck program.
 *   - **instruction** -- either a `loop` or a `command`. Loops come first because
 *     their leading token (LOOP_START) is unambiguous.
 *   - **loop** -- LOOP_START followed by zero or more instructions, then LOOP_END.
 *     Loops can be nested to arbitrary depth.
 *   - **command** -- one of the six non-bracket operators: RIGHT, LEFT, INC, DEC,
 *     OUTPUT, or INPUT.
 *
 * The grammar is recursive: `program` contains `instruction`s, `instruction`
 * contains `loop`, and `loop` contains `instruction`s again. This mutual
 * recursion allows Brainfuck to represent arbitrarily deep nested loops.
 *
 * Unmatched Brackets
 * ------------------
 *
 * If the source contains unmatched brackets (e.g., `[` without a matching `]`
 * or `]` without a leading `[`), the generic parser will throw an error. This
 * is caught at parse time, not at run time. This is one advantage of the
 * grammar-driven approach over the direct-translation approach in `translator.ts`.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `brainfuck.grammar` file lives in `code/grammars/` at the repository root.
 *
 *     src/parser.ts -> brainfuck/ -> typescript/ -> packages/ -> code/ -> grammars/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeBrainfuck } from "./lexer.js";

/**
 * Resolve __dirname for ESM modules.
 * See lexer.ts for a detailed explanation of the ESM __dirname pattern.
 */
const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Navigate from src/ up to the grammars/ directory.
 *
 * The path traversal:
 *   __dirname   = .../brainfuck/src/
 *   ..          = .../brainfuck/
 *   ../..       = .../typescript/
 *   ../../..    = .../packages/
 *   ../../../.. = .../code/
 *   + grammars  = .../code/grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const BF_GRAMMAR_PATH = join(GRAMMARS_DIR, "brainfuck.grammar");

/**
 * Parse Brainfuck source text and return an AST.
 *
 * This function orchestrates the full parsing pipeline:
 *   1. Tokenize the source using the Brainfuck lexer (discards comments/whitespace)
 *   2. Read and parse the brainfuck.grammar file
 *   3. Run the grammar-driven parser to produce an AST
 *
 * The root node of the returned AST always has `ruleName` of `"program"`.
 *
 * @param source - The Brainfuck source text to parse.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"program"`.
 * @throws If the source contains unmatched brackets or other structural errors.
 *
 * @example
 *     const ast = parseBrainfuck("++");
 *     console.log(ast.ruleName); // "program"
 *
 * @example
 *     // Parse a loop that decrements cell 0 while incrementing cell 1:
 *     const ast = parseBrainfuck(">++<[->+<]");
 *
 * @example
 *     // Comments are stripped before parsing:
 *     const ast = parseBrainfuck("+ increment\n- decrement");
 *     // Equivalent to parseBrainfuck("+-")
 */
export function parseBrainfuck(source: string): ASTNode {
  /**
   * Step 1: Tokenize.
   * The Brainfuck lexer handles single-character literal matching,
   * whitespace skipping, and comment discarding. By the time the token
   * array reaches the parser, only command tokens and EOF remain.
   */
  const tokens = tokenizeBrainfuck(source);

  /**
   * Step 2: Load the grammar.
   * The grammar file defines the syntax rules in EBNF-like notation:
   *   program = { instruction } ;
   *   instruction = loop | command ;
   *   loop = LOOP_START { instruction } LOOP_END ;
   *   command = RIGHT | LEFT | INC | DEC | OUTPUT | INPUT ;
   *
   * parseParserGrammar converts the text into a structured object that
   * the GrammarParser can use for recursive descent.
   */
  const grammarText = readFileSync(BF_GRAMMAR_PATH, "utf-8");
  const grammar = parseParserGrammar(grammarText);

  /**
   * Step 3: Parse.
   * The GrammarParser takes the token array and grammar rules, then
   * performs recursive descent with backtracking to produce an AST.
   * The starting rule is determined by the grammar (the first rule
   * defined, which for brainfuck.grammar is "program").
   *
   * Unmatched brackets cause the parser to throw -- e.g., "[" without
   * a matching "]" will be detected because the loop rule requires
   * LOOP_END after the loop body.
   */
  const parser = new GrammarParser(tokens, grammar);
  return parser.parse();
}
