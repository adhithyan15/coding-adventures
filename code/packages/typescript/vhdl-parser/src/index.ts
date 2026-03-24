/**
 * VHDL Parser — parses VHDL (IEEE 1076-2008) source code into ASTs
 * using the grammar-driven approach.
 *
 * VHDL (VHSIC Hardware Description Language) is an HDL designed by the
 * US Department of Defense for documenting and simulating digital systems.
 * Unlike Verilog, which is terse and C-like, VHDL is verbose and Ada-like,
 * with strong typing, explicit declarations, and case-insensitive identifiers.
 *
 * This package is a **thin wrapper** around the generic `GrammarParser` from
 * the `@coding-adventures/parser` package. It loads the `vhdl.grammar` file
 * and delegates all parsing to the generic engine.
 *
 * Usage:
 *
 *     import { parseVhdl } from "@coding-adventures/vhdl-parser";
 *
 *     const ast = parseVhdl("entity empty is end entity;");
 *     console.log(ast.ruleName); // "design_file"
 */

export { parseVhdl } from "./parser.js";
