/**
 * SQL Lexer -- tokenizes SQL text using the grammar-driven approach.
 *
 * SQL (Structured Query Language) is the standard language for relational
 * databases. This lexer produces a flat stream of tokens from SQL text,
 * suitable for feeding into the grammar-driven SQL parser.
 *
 * Features:
 *   - Case-insensitive keyword matching (select = SELECT = Select → KEYWORD("SELECT"))
 *   - Single-quoted string literals ('hello' → STRING)
 *   - Backtick-quoted identifiers (`col` → NAME)
 *   - Multi-character operators (!=, <>, <=, >=)
 *   - Line comments (-- ...) and block comments (/* ... *\/) skipped silently
 *   - Decimal numbers (42, 3.14)
 *
 * Usage:
 *
 *     import { tokenizeSQL } from "@coding-adventures/sql-lexer";
 *
 *     const tokens = tokenizeSQL("SELECT id, name FROM users WHERE age > 18");
 */

export { tokenizeSQL, createSQLLexer } from "./tokenizer.js";
