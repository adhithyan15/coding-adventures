/**
 * Starlark Parser — parses Starlark source code into ASTs using the grammar-driven approach.
 *
 * Starlark is a deterministic, hermetic dialect of Python used by Bazel BUILD files,
 * Buck TARGETS files, and other build system configuration. This parser produces
 * abstract syntax trees (ASTs) that represent the structure of Starlark programs.
 *
 * Usage:
 *
 *     import { parseStarlark } from "@coding-adventures/starlark-parser";
 *
 *     const ast = parseStarlark("x = 1 + 2");
 *     console.log(ast.ruleName); // "file"
 */

export { parseStarlark } from "./parser.js";
