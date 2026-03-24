/**
 * SQL Parser -- parses SQL text into ASTs using the grammar-driven approach.
 *
 * SQL (Structured Query Language) is the standard language for relational
 * databases. This parser produces abstract syntax trees (ASTs) from SQL text,
 * supporting SELECT, INSERT, UPDATE, DELETE, CREATE TABLE, and DROP TABLE.
 *
 * Usage:
 *
 *     import { parseSQL } from "coding-adventures-sql-parser";
 *
 *     const ast = parseSQL("SELECT id, name FROM users WHERE age > 18");
 *     console.log(ast.ruleName); // "program"
 */

export { parseSQL, createSQLParser } from "./parser.js";
