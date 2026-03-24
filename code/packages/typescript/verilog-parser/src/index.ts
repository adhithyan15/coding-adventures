/**
 * Verilog Parser — parses Verilog (IEEE 1364-2005) source code into ASTs
 * using the grammar-driven approach.
 *
 * Verilog is a Hardware Description Language (HDL) for designing digital
 * circuits. Unlike software languages that describe sequential computation,
 * Verilog describes physical hardware — gates, wires, registers, flip-flops —
 * that exist simultaneously and operate in parallel.
 *
 * This package is a **thin wrapper** around the generic `GrammarParser` from
 * the `@coding-adventures/parser` package. It loads the `verilog.grammar` file
 * and delegates all parsing to the generic engine.
 *
 * Usage:
 *
 *     import { parseVerilog } from "@coding-adventures/verilog-parser";
 *
 *     const ast = parseVerilog("module top; endmodule");
 *     console.log(ast.ruleName); // "source_text"
 */

export { parseVerilog } from "./parser.js";
