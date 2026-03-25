/**
 * Verilog Lexer — tokenizes Verilog (IEEE 1364-2005) source code.
 *
 * This module is a wrapper around the generic `grammarTokenize` function
 * from the `@coding-adventures/lexer` package. It loads the `verilog.tokens`
 * grammar file and delegates all tokenization to the generic engine.
 *
 * What Is Verilog?
 * ----------------
 *
 * Verilog is a Hardware Description Language (HDL) for designing digital
 * circuits. Unlike software languages that describe sequential computation,
 * Verilog describes physical hardware structures — gates, wires, registers,
 * flip-flops — that exist simultaneously and operate in parallel.
 *
 * A simple Verilog module:
 *
 *     module adder(input [7:0] a, b, output [7:0] sum);
 *       assign sum = a + b;
 *     endmodule
 *
 * This describes a physical 8-bit adder circuit, not a function that adds
 * two numbers. The `assign` statement creates a continuous connection —
 * whenever `a` or `b` changes, `sum` updates instantly (like wiring two
 * inputs to an adder chip).
 *
 * Verilog vs Software Language Tokenization
 * ------------------------------------------
 *
 * Verilog has several unique lexical features:
 *
 * 1. **Sized number literals**: `4'b1010`, `8'hFF`, `32'd42`
 *    Numbers carry bit-width information because hardware signals have
 *    fixed widths. `4'b1010` means "a 4-bit value in binary = 10 in decimal."
 *
 * 2. **System tasks**: `$display`, `$finish`, `$time`
 *    Built-in simulation functions prefixed with `$`. Not synthesizable
 *    (they don't map to hardware) but essential for simulation/debugging.
 *
 * 3. **Compiler directives**: `` `define ``, `` `ifdef ``, `` `include ``
 *    Prefixed with backtick (`), not hash (#) like C. Handled by the
 *    preprocessor before tokenization.
 *
 * 4. **Four-value logic**: `x` (unknown) and `z` (high-impedance)
 *    These appear in number literals: `4'bxxzz`. They represent physical
 *    states that don't exist in software.
 *
 * 5. **Escaped identifiers**: `\my.odd.name`
 *    Backslash-prefixed identifiers that can contain any character except
 *    whitespace. Used when identifiers need special characters.
 *
 * Preprocessor Integration
 * ------------------------
 *
 * Verilog source often contains preprocessor directives:
 *
 *     `define WIDTH 8
 *     `ifdef DEBUG
 *       // debug-only code
 *     `endif
 *
 * The `tokenizeVerilog` function can optionally run the preprocessor
 * before tokenizing, which expands macros and evaluates conditionals.
 * This is controlled by the `preprocess` option (defaults to true).
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `verilog.tokens` file lives in `code/grammars/` at the repository root.
 * We navigate from this module's location up to that directory:
 *
 *     src/tokenizer.ts -> verilog-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseTokenGrammar } from "@coding-adventures/grammar-tools";
import { grammarTokenize, GrammarLexer } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

import { verilogPreprocess } from "./preprocessor.js";

/**
 * Resolve __dirname for ESM modules.
 *
 * In CommonJS, __dirname is a global. In ESM, it does not exist — we must
 * derive it from import.meta.url, which gives the file URL of the current
 * module (e.g., "file:///path/to/tokenizer.ts"). The fileURLToPath + dirname
 * pattern converts this to a directory path.
 */
const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Navigate from src/ up to the grammars/ directory.
 *
 * The path traversal:
 *   __dirname = .../verilog-lexer/src/
 *   ..         = .../verilog-lexer/
 *   ../..      = .../typescript/
 *   ../../..   = .../packages/
 *   ../../../.. = .../code/
 *   + grammars  = .../code/grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const VERILOG_TOKENS_PATH = join(GRAMMARS_DIR, "verilog.tokens");

/**
 * Options for the Verilog tokenizer.
 */
export interface VerilogTokenizeOptions {
  /**
   * Whether to run the preprocessor before tokenizing.
   *
   * When true (the default), the source is first passed through
   * `verilogPreprocess()` which expands `define macros, evaluates
   * `ifdef/`ifndef conditionals, stubs `include, and strips `timescale.
   *
   * Set to false if:
   *   - The source has already been preprocessed
   *   - You want to tokenize raw directive syntax
   *   - You're building a tool that needs to see directives as tokens
   */
  preprocess?: boolean;
}

/**
 * Load and parse the Verilog grammar.
 *
 * This is factored out so both `tokenizeVerilog` and `createVerilogLexer`
 * can share it. In a production system you would cache this, but for an
 * educational codebase, clarity beats performance.
 */
function loadVerilogGrammar() {
  const grammarText = readFileSync(VERILOG_TOKENS_PATH, "utf-8");
  return parseTokenGrammar(grammarText);
}

/**
 * Tokenize Verilog source code and return an array of tokens.
 *
 * This is the primary entry point for the Verilog lexer. It reads the
 * `verilog.tokens` grammar file, optionally preprocesses the source,
 * then passes everything to the generic grammar-driven tokenizer.
 *
 * @param source - The Verilog source code to tokenize.
 * @param options - Optional configuration (default: { preprocess: true }).
 * @returns An array of Token objects. The last token is always EOF.
 *
 * @example
 *     const tokens = tokenizeVerilog("assign y = a & b;");
 *     // [KEYWORD("assign"), NAME("y"), EQUALS("="), NAME("a"),
 *     //  AMP("&"), NAME("b"), SEMICOLON(";"), EOF("")]
 *
 * @example
 *     // With preprocessor (default):
 *     const tokens = tokenizeVerilog("`define W 8\nreg [`W-1:0] data;");
 *     // Preprocessor expands `W to 8, then tokenizes: reg [8-1:0] data;
 *
 * @example
 *     // Without preprocessor:
 *     const tokens = tokenizeVerilog("`define W 8", { preprocess: false });
 *     // Tokenizes the raw directive, producing DIRECTIVE("`define"), etc.
 */
export function tokenizeVerilog(
  source: string,
  options?: VerilogTokenizeOptions,
): Token[] {
  const shouldPreprocess = options?.preprocess ?? true;

  /**
   * Step 1: Optionally preprocess the source.
   * This expands macros, evaluates conditionals, stubs includes,
   * and strips timescale directives.
   */
  const processedSource = shouldPreprocess
    ? verilogPreprocess(source)
    : source;

  /**
   * Step 2: Load and parse the Verilog grammar.
   */
  const grammar = loadVerilogGrammar();

  /**
   * Step 3: Run the generic tokenizer with the Verilog grammar.
   */
  return grammarTokenize(processedSource, grammar);
}

/**
 * Create a GrammarLexer instance for Verilog source code.
 *
 * Unlike `tokenizeVerilog` which returns an array of all tokens at once,
 * `createVerilogLexer` returns a `GrammarLexer` object that allows
 * incremental tokenization and event-driven processing via `onToken`.
 *
 * This is useful for:
 *   - Large files where you don't want all tokens in memory at once
 *   - Streaming/incremental processing
 *   - Custom token filtering or transformation during lexing
 *
 * @param source - The Verilog source code to tokenize.
 * @param options - Optional configuration (default: { preprocess: true }).
 * @returns A GrammarLexer instance ready to tokenize.
 *
 * @example
 *     const lexer = createVerilogLexer("module top; endmodule");
 *     const tokens = lexer.tokenize();
 *
 * @example
 *     // With event-driven processing:
 *     const lexer = createVerilogLexer("module top; endmodule");
 *     lexer.onToken((token, ctx) => {
 *       console.log(token.type, token.value);
 *     });
 *     lexer.tokenize();
 */
export function createVerilogLexer(
  source: string,
  options?: VerilogTokenizeOptions,
): GrammarLexer {
  const shouldPreprocess = options?.preprocess ?? true;

  const processedSource = shouldPreprocess
    ? verilogPreprocess(source)
    : source;

  const grammar = loadVerilogGrammar();

  return new GrammarLexer(processedSource, grammar);
}
