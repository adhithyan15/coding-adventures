/**
 * Mosaic Lexer — tokenizes `.mosaic` source using the grammar-driven approach.
 *
 * Mosaic is a UI component description language. A `.mosaic` file declares
 * one component with typed data slots and a visual node tree. This lexer
 * converts raw Mosaic source text into a flat stream of typed tokens.
 *
 * Usage:
 *
 *     import { tokenizeMosaic } from "@coding-adventures/mosaic-lexer";
 *
 *     const tokens = tokenizeMosaic('component Button { slot label: text; }');
 *     console.log(tokens[0]); // Token { type: "KEYWORD", value: "component", ... }
 */

export { tokenizeMosaic } from "./tokenizer.js";
export { TOKEN_GRAMMAR } from "./_grammar.js";
