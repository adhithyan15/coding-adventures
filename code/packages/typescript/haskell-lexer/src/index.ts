/**
 * Haskell Lexer â€” tokenizes Haskell source code using the grammar-driven approach.
 *
 * Usage:
 *
 *     import { tokenizeHaskell, createHaskellLexer } from "@coding-adventures/haskell-lexer";
 *
 *     // Default version (Haskell 21)
 *     const tokens = tokenizeHaskell("class Hello { }");
 *
 *     // Version-specific grammar
 *     const tokens = tokenizeHaskell("int x = 1;", "8");
 *     const tokens = tokenizeHaskell("var x = 1;", "10");
 *
 *     // Class-based lexer with on-token callbacks
 *     const lexer = createHaskellLexer("class Hello { }", "21");
 *     const tokens = lexer.tokenize();
 */

export { tokenizeHaskell, createHaskellLexer } from "./lexer.js";

