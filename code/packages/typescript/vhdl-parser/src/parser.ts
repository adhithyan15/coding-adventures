/**
 * VHDL Parser — parses VHDL (IEEE 1076-2008) source code into ASTs
 * using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from
 * the `@coding-adventures/parser` package. It loads the `vhdl.grammar`
 * file and delegates all parsing work to the generic engine.
 *
 * Why VHDL Is Different from Verilog
 * -----------------------------------
 *
 * While both are Hardware Description Languages, VHDL and Verilog take
 * opposite design philosophies:
 *
 *   - **Verilog** is implicit and concise (like C). Signals don't need
 *     type declarations; ports can be declared inline; the module keyword
 *     covers both interface and implementation.
 *
 *   - **VHDL** is explicit and verbose (like Ada). Every signal has a
 *     declared type. The interface (entity) is separate from the
 *     implementation (architecture). Strong typing catches errors at
 *     compile time that would be silent bugs in Verilog.
 *
 * VHDL Structural Overview
 * ------------------------
 *
 * A VHDL "design file" contains one or more design units:
 *
 *   - **entity_declaration**: Defines the INTERFACE — ports (pins) and
 *     generics (compile-time parameters). Like the pin diagram on a chip
 *     datasheet.
 *
 *   - **architecture_body**: Defines the IMPLEMENTATION — signals,
 *     concurrent assignments, processes, and component instantiations.
 *     One entity can have multiple architectures (behavioral, structural,
 *     rtl).
 *
 *   - **package_declaration** / **package_body**: Groups reusable
 *     declarations (types, constants, functions) for sharing across
 *     entities. The IEEE standard library is defined this way.
 *
 * Key Grammar Constructs
 * ----------------------
 *
 * The `vhdl.grammar` file defines rules for:
 *
 *   - **design_file**: The root rule — one or more design units
 *   - **entity_declaration**: `entity NAME is ... end entity NAME;`
 *   - **architecture_body**: `architecture NAME of NAME is ... begin ... end;`
 *   - **process_statement**: `process (...) begin ... end process;`
 *   - **signal_assignment**: `y <= a and b;` (concurrent or sequential)
 *   - **if_statement**: `if ... then ... elsif ... else ... end if;`
 *   - **case_statement**: `case expr is when ... => ... end case;`
 *   - **expressions**: Full operator precedence from logical down to primary
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `vhdl.grammar` file lives in `code/grammars/` at the repository root.
 * We navigate from this module's source directory up through the package hierarchy:
 *
 *     src/ -> vhdl-parser/ -> typescript/ -> packages/ -> code/ -> grammars/
 *
 * That is four levels up from `src/`, which is why we use four `..` segments.
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeVhdl } from "@coding-adventures/vhdl-lexer";

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
 *   __dirname   = .../vhdl-parser/src/
 *   ..          = .../vhdl-parser/
 *   ../..       = .../typescript/
 *   ../../..    = .../packages/
 *   ../../../.. = .../code/
 *   + grammars  = .../code/grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const VHDL_GRAMMAR_PATH = join(GRAMMARS_DIR, "vhdl.grammar");

/**
 * Parse VHDL source code and return an AST.
 *
 * This function performs three steps:
 *
 * 1. **Tokenize**: The VHDL source is passed to `tokenizeVhdl()` from
 *    the vhdl-lexer package. This handles case normalization (VHDL is
 *    case-insensitive) and produces a flat list of tokens (KEYWORD, NAME,
 *    NUMBER, SEMICOLON, etc.).
 *
 * 2. **Load grammar**: The `vhdl.grammar` file is read and parsed into a
 *    `ParserGrammar` object — a data structure describing all the EBNF rules.
 *
 * 3. **Parse**: The `GrammarParser` walks the grammar rules, matching them
 *    against the token stream to build an AST. Each grammar rule becomes an
 *    `ASTNode` with the rule name and its matched children.
 *
 * @param source - The VHDL source code to parse.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"design_file"`.
 *
 * @example
 *     // Parse a simple entity with no ports:
 *     const ast = parseVhdl("entity empty is end entity;");
 *     console.log(ast.ruleName); // "design_file"
 *
 * @example
 *     // Parse an AND gate entity and architecture:
 *     const ast = parseVhdl(`
 *       entity and_gate is
 *         port (a, b : in std_logic; y : out std_logic);
 *       end entity and_gate;
 *
 *       architecture rtl of and_gate is
 *       begin
 *         y <= a and b;
 *       end architecture rtl;
 *     `);
 *     // ast.ruleName === "design_file"
 *     // ast contains entity_declaration and architecture_body nodes
 */
export function parseVhdl(source: string): ASTNode {
  /**
   * Step 1: Tokenize the VHDL source.
   *
   * The vhdl-lexer handles:
   *   - Case normalization (ENTITY -> entity, Std_Logic -> std_logic)
   *   - Bit string literals (X"FF", B"1010")
   *   - Character literals ('0', '1')
   *   - All VHDL operators and punctuation
   *   - Keywords like entity, architecture, process, signal, etc.
   */
  const tokens = tokenizeVhdl(source);

  /**
   * Step 2: Load and parse the VHDL grammar.
   *
   * The grammar is an EBNF-style definition with rules like:
   *   design_file = { design_unit } ;
   *   entity_declaration = "entity" NAME "is" ... "end" ... SEMICOLON ;
   *
   * `parseParserGrammar` converts this text into a structured
   * `ParserGrammar` object that the `GrammarParser` can interpret.
   */
  const grammarText = readFileSync(VHDL_GRAMMAR_PATH, "utf-8");
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
