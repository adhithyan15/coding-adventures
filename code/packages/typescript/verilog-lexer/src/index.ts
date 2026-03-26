/**
 * Verilog Lexer — tokenizes Verilog (IEEE 1364-2005) source code using
 * the grammar-driven approach, with built-in preprocessor support.
 *
 * Verilog is a Hardware Description Language (HDL) for designing digital
 * circuits. This package provides two main entry points:
 *
 *   - `tokenizeVerilog(source)` — convenience function that returns all tokens
 *   - `createVerilogLexer(source)` — returns a GrammarLexer for incremental use
 *
 * Both functions optionally run the Verilog preprocessor before tokenization,
 * handling `define, `ifdef, `ifndef, `else, `endif, `include, and `timescale.
 *
 * Usage:
 *
 *     import { tokenizeVerilog } from "@coding-adventures/verilog-lexer";
 *
 *     const tokens = tokenizeVerilog("assign y = a & b;");
 */

export { tokenizeVerilog, createVerilogLexer } from "./tokenizer.js";
export type { VerilogTokenizeOptions } from "./tokenizer.js";
export { verilogPreprocess } from "./preprocessor.js";
