/**
 * JSON Parser -- parses JSON text into ASTs using the grammar-driven approach.
 *
 * JSON (JavaScript Object Notation, RFC 8259) is the most widely used data
 * interchange format. This parser produces abstract syntax trees (ASTs) from
 * JSON text, suitable for analysis, transformation, or evaluation.
 *
 * Usage:
 *
 *     import { parseJSON } from "@coding-adventures/json-parser";
 *
 *     const ast = parseJSON('{"name": "Alice"}');
 *     console.log(ast.ruleName); // "value"
 */

export { parseJSON } from "./parser.js";
