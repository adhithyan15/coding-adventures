/**
 * TOML Lexer -- tokenizes TOML text using the grammar-driven approach.
 *
 * TOML (Tom's Obvious Minimal Language) is a configuration file format that
 * is designed to be easy to read due to its clear semantics. This lexer
 * produces a flat stream of tokens from TOML text, suitable for feeding
 * into the grammar-driven parser.
 *
 * Usage:
 *
 *     import { tokenizeTOML } from "@coding-adventures/toml-lexer";
 *
 *     const tokens = tokenizeTOML('title = "TOML Example"');
 */

export { tokenizeTOML } from "./tokenizer.js";
