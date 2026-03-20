/**
 * JSON Lexer -- tokenizes JSON text using the grammar-driven approach.
 *
 * JSON (JavaScript Object Notation, RFC 8259) is the most widely used data
 * interchange format. This lexer produces a flat stream of tokens from JSON
 * text, suitable for feeding into the grammar-driven parser.
 *
 * Usage:
 *
 *     import { tokenizeJSON } from "@coding-adventures/json-lexer";
 *
 *     const tokens = tokenizeJSON('{"name": "Alice"}');
 */

export { tokenizeJSON } from "./tokenizer.js";
