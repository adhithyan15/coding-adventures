/**
 * @coding-adventures/brainfuck-ir-compiler — Brainfuck AOT Compiler Frontend
 *
 * =============================================================================
 * Overview
 * =============================================================================
 *
 * This package is the Brainfuck-specific **frontend** of the AOT compiler
 * pipeline. It knows Brainfuck semantics — tape, cells, pointer arithmetic,
 * loops, I/O — and translates them into target-independent IR.
 *
 * The pipeline:
 *
 *   parseBrainfuck(source)        ← @coding-adventures/brainfuck
 *       ↓
 *   compile(ast, filename, config) ← this package
 *       ↓
 *   { program: IrProgram, sourceMap: SourceMapChain }
 *       ↓
 *   optimizer (compiler-ir-optimizer) — future package
 *       ↓
 *   backend (codegen-riscv) — future package
 *
 * =============================================================================
 * Usage
 * =============================================================================
 *
 *     import { parseBrainfuck } from "@coding-adventures/brainfuck";
 *     import {
 *       compile,
 *       releaseConfig,
 *       debugConfig
 *     } from "@coding-adventures/brainfuck-ir-compiler";
 *     import { printIr } from "@coding-adventures/compiler-ir";
 *
 *     const ast = parseBrainfuck("++[-].");
 *
 *     // Release build: bounds checks off, byte masking on
 *     const { program, sourceMap } = compile(ast, "hello.bf", releaseConfig());
 *
 *     // Print the IR to text
 *     console.log(printIr(program));
 */

export type { BuildConfig } from "./build_config.js";
export { debugConfig, releaseConfig } from "./build_config.js";
export type { CompileResult } from "./compiler.js";
export { compile } from "./compiler.js";
