/**
 * C# Lexer — tokenizes C# source code using the grammar-driven approach.
 *
 * Usage:
 *
 *     import { tokenizeCSharp, createCSharpLexer } from "@coding-adventures/csharp-lexer";
 *
 *     // Default version (C# 12.0)
 *     const tokens = tokenizeCSharp("class Hello { }");
 *
 *     // Version-specific grammar
 *     const tokens = tokenizeCSharp("int x = 1;", "8.0");
 *     const tokens = tokenizeCSharp("var x = 1;", "3.0");
 *
 *     // Class-based lexer with on-token callbacks
 *     const lexer = createCSharpLexer("class Hello { }", "12.0");
 *     const tokens = lexer.tokenize();
 */

export { tokenizeCSharp, createCSharpLexer } from "./tokenizer.js";
