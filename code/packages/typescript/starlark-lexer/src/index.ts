/**
 * Starlark Lexer — tokenizes Starlark source code using the grammar-driven approach.
 *
 * Starlark is a deterministic, hermetic dialect of Python designed for configuration
 * files. It is the language used by Bazel BUILD files, Buck TARGETS files, and other
 * build systems that need a safe, reproducible configuration language.
 *
 * Usage:
 *
 *     import { tokenizeStarlark } from "@coding-adventures/starlark-lexer";
 *
 *     const tokens = tokenizeStarlark("x = 1 + 2");
 */

export { tokenizeStarlark } from "./tokenizer.js";
