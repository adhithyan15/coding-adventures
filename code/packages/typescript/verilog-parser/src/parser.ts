/**
 * Verilog Parser — parses Verilog (IEEE 1364-2005) source code into ASTs
 * using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from
 * the `@coding-adventures/parser` package. It loads the `verilog.grammar`
 * file and delegates all parsing work to the generic engine.
 *
 * Why Verilog Is Different from Software Languages
 * -------------------------------------------------
 *
 * In a software language like JavaScript or Python, parsing produces an AST
 * that represents a sequence of instructions to execute one after another.
 * In Verilog, parsing produces an AST that represents a hierarchy of hardware
 * components that all exist simultaneously.
 *
 * A Verilog "program" is a collection of **modules** — each module is like a
 * chip on a circuit board. Modules have input/output ports (pins), internal
 * wires, registers, and behavioral descriptions. The parser's job is to
 * turn this textual description into a structured tree that later tools
 * (synthesis, simulation) can process.
 *
 * Key Grammar Constructs
 * ----------------------
 *
 * The `verilog.grammar` file defines rules for:
 *
 *   - **source_text**: The root rule — one or more module declarations
 *   - **module_declaration**: `module NAME (...); ... endmodule`
 *   - **continuous_assign**: `assign y = a & b;` (combinational logic)
 *   - **always_construct**: `always @(posedge clk) ...` (sequential logic)
 *   - **expressions**: Full operator precedence from ternary down to unary
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `verilog.grammar` file lives in `code/grammars/` at the repository root.
 * We navigate from this module's source directory up through the package hierarchy:
 *
 *     src/ -> verilog-parser/ -> typescript/ -> packages/ -> code/ -> grammars/
 *
 * That is four levels up from `src/`, which is why we use four `..` segments.
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeVerilog } from "@coding-adventures/verilog-lexer";

/**
 * Resolve __dirname for ESM modules.
 *
 * In CommonJS, __dirname is a global. In ESM, we must derive it from
 * import.meta.url. The `fileURLToPath` function converts a file:// URL
 * to a filesystem path, and `dirname` extracts the directory portion.
 */
const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Navigate from src/ up to the grammars/ directory.
 *
 * The path traversal:
 *   __dirname   = .../verilog-parser/src/
 *   ..          = .../verilog-parser/
 *   ../..       = .../typescript/
 *   ../../..    = .../packages/
 *   ../../../.. = .../code/
 *   + grammars  = .../code/grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const VERILOG_GRAMMAR_PATH = join(GRAMMARS_DIR, "verilog.grammar");

/**
 * Parse Verilog source code and return an AST.
 *
 * This function performs three steps:
 *
 * 1. **Tokenize**: The Verilog source is passed to `tokenizeVerilog()` from
 *    the verilog-lexer package. This handles preprocessor directives (`define,
 *    `ifdef, etc.) and produces a flat list of tokens (KEYWORD, NAME, NUMBER,
 *    SEMICOLON, etc.).
 *
 * 2. **Load grammar**: The `verilog.grammar` file is read and parsed into a
 *    `ParserGrammar` object — a data structure describing all the EBNF rules.
 *
 * 3. **Parse**: The `GrammarParser` walks the grammar rules, matching them
 *    against the token stream to build an AST. Each grammar rule becomes an
 *    `ASTNode` with the rule name and its matched children.
 *
 * @param source - The Verilog source code to parse.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"source_text"`.
 *
 * @example
 *     // Parse a simple module with no ports or body:
 *     const ast = parseVerilog("module empty; endmodule");
 *     console.log(ast.ruleName); // "source_text"
 *
 * @example
 *     // Parse an AND gate module:
 *     const ast = parseVerilog(`
 *       module and_gate(input a, input b, output y);
 *         assign y = a & b;
 *       endmodule
 *     `);
 *     // ast.ruleName === "source_text"
 *     // ast contains module_declaration -> continuous_assign -> ...
 */
export function parseVerilog(source: string): ASTNode {
  /**
   * Step 1: Tokenize the Verilog source.
   *
   * The verilog-lexer handles:
   *   - Preprocessing (`define, `ifdef, `include, `timescale)
   *   - Sized number literals (4'b1010, 8'hFF)
   *   - System tasks ($display, $finish)
   *   - All Verilog operators and punctuation
   */
  const tokens = tokenizeVerilog(source);

  /**
   * Step 2: Load and parse the Verilog grammar.
   *
   * The grammar is an EBNF-style definition with rules like:
   *   source_text = { description } ;
   *   module_declaration = "module" NAME ... "endmodule" ;
   *
   * `parseParserGrammar` converts this text into a structured
   * `ParserGrammar` object that the `GrammarParser` can interpret.
   */
  const grammarText = readFileSync(VERILOG_GRAMMAR_PATH, "utf-8");
  const grammar = parseParserGrammar(grammarText);

  /**
   * Step 3: Run the grammar-driven parser.
   *
   * The `GrammarParser` uses recursive descent with packrat memoization
   * to match the token stream against the grammar rules. It produces a
   * tree of `ASTNode` objects, where each node corresponds to a matched
   * grammar rule and contains child nodes or tokens.
   */
  const parser = new GrammarParser(tokens, grammar);
  return parser.parse();
}
