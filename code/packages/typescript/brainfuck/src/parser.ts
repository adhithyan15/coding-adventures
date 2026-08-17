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
 * Grammar Source
 * --------------
 *
 * The parser grammar is compiled ahead of time from `brainfuck.grammar`
 * (in `code/grammars/brainfuck/`) into `./_parser_grammar.ts`, a native
 * TypeScript object literal. This avoids reading and parsing a grammar
 * file from disk at runtime -- which would break once this package is
 * published, since a published npm package never ships the monorepo's
 * `code/grammars/` tree.
 *
 * (This package's `lexer.ts` also has a compiled *token* grammar in this
 * same `src/` directory, named `_token_grammar.ts` to avoid colliding
 * with this file.)
 */

import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeBrainfuck } from "./lexer.js";

import { PARSER_GRAMMAR } from "./_parser_grammar.js";

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
   * Step 2: Parse.
   * The GrammarParser takes the token array and the pre-compiled
   * PARSER_GRAMMAR constant, then performs recursive descent with
   * backtracking to produce an AST. The starting rule is determined by
   * the grammar (the first rule defined, which for brainfuck.grammar is
   * "program"):
   *   program = { instruction } ;
   *   instruction = loop | command ;
   *   loop = LOOP_START { instruction } LOOP_END ;
   *   command = RIGHT | LEFT | INC | DEC | OUTPUT | INPUT ;
   *
   * Unmatched brackets cause the parser to throw -- e.g., "[" without
   * a matching "]" will be detected because the loop rule requires
   * LOOP_END after the loop body.
   */
  const parser = new GrammarParser(tokens, PARSER_GRAMMAR);
  return parser.parse();
}
