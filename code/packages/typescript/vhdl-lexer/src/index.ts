/**
 * VHDL Lexer — tokenizes VHDL (IEEE 1076-2008) source code using
 * the grammar-driven approach, with case normalization.
 *
 * VHDL (VHSIC Hardware Description Language) is an HDL designed by the
 * US Department of Defense for documenting and simulating digital systems.
 * Unlike Verilog, which is terse and C-like, VHDL is verbose and Ada-like,
 * with strong typing, explicit declarations, and case-insensitive identifiers.
 *
 * This package provides two main entry points:
 *
 *   - `tokenizeVhdl(source)` — convenience function that returns all tokens
 *   - `createVhdlLexer(source)` — returns a GrammarLexer for incremental use
 *
 * Unlike the Verilog lexer, VHDL has NO preprocessor. Instead, VHDL relies
 * on libraries and packages for code reuse, and configurations for conditional
 * compilation — all handled at the language level, not by text substitution.
 *
 * The key VHDL-specific behavior is case normalization: after tokenization,
 * all NAME and KEYWORD token values are lowercased. This reflects VHDL's
 * case-insensitive nature: ENTITY, Entity, and entity are all the same.
 *
 * Usage:
 *
 *     import { tokenizeVhdl } from "@coding-adventures/vhdl-lexer";
 *
 *     const tokens = tokenizeVhdl("ENTITY my_chip IS END ENTITY;");
 */

export { tokenizeVhdl, createVhdlLexer } from "./tokenizer.js";
