/**
 * Java Lexer — tokenizes Java source code using the grammar-driven approach.
 *
 * Usage:
 *
 *     import { tokenizeJava, createJavaLexer } from "@coding-adventures/java-lexer";
 *
 *     // Default version (Java 21)
 *     const tokens = tokenizeJava("class Hello { }");
 *
 *     // Version-specific grammar
 *     const tokens = tokenizeJava("int x = 1;", "8");
 *     const tokens = tokenizeJava("var x = 1;", "10");
 *
 *     // Class-based lexer with on-token callbacks
 *     const lexer = createJavaLexer("class Hello { }", "21");
 *     const tokens = lexer.tokenize();
 */

export { tokenizeJava, createJavaLexer } from "./tokenizer.js";
