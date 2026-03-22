/**
 * TOML Parser -- parses TOML text into ASTs using the grammar-driven approach.
 *
 * TOML (Tom's Obvious Minimal Language) is a configuration file format that
 * maps unambiguously to a hash table. This parser produces abstract syntax
 * trees (ASTs) from TOML text, suitable for analysis, transformation, or
 * evaluation into a nested key-value structure.
 *
 * Usage:
 *
 *     import { parseTOML } from "@coding-adventures/toml-parser";
 *
 *     const ast = parseTOML('[server]\nhost = "localhost"\nport = 8080');
 *     console.log(ast.ruleName); // "document"
 */

export { parseTOML } from "./parser.js";
