/**
 * Bytecode Compiler — Layer 4a of the computing stack.
 *
 * Compiles ASTs (from the parser) into stack-machine bytecode (for the VM).
 *
 * The compiler is the bridge between human-readable syntax and machine-executable
 * instructions. It walks the Abstract Syntax Tree produced by the parser and emits
 * a flat sequence of stack operations that the Virtual Machine can execute.
 *
 * This package includes multiple backends that compile the same AST to different
 * bytecode formats:
 *
 * - **BytecodeCompiler** — Targets our custom VM (the original backend).
 * - **JVMCompiler** — Targets the Java Virtual Machine (real JVM bytecode bytes).
 * - **CLRCompiler** — Targets the .NET Common Language Runtime (real CLR IL bytes).
 * - **WASMCompiler** — Targets WebAssembly (real WASM bytecode bytes).
 *
 * Usage:
 *
 *     import { BytecodeCompiler, compileSource } from "@coding-adventures/bytecode-compiler";
 *
 *     // End-to-end: source code -> CodeObject
 *     const code = compileSource("x = 1 + 2");
 *
 *     // Or step by step: AST -> CodeObject
 *     import { Parser } from "@coding-adventures/parser";
 *     import { tokenize } from "@coding-adventures/lexer";
 *
 *     const tokens = tokenize("x = 1 + 2");
 *     const ast = new Parser(tokens).parse();
 *     const compiler = new BytecodeCompiler();
 *     const code2 = compiler.compile(ast);
 *
 *     // JVM backend:
 *     import { JVMCompiler } from "@coding-adventures/bytecode-compiler";
 *     const jvmCode = new JVMCompiler().compile(ast);
 *
 *     // CLR backend:
 *     import { CLRCompiler } from "@coding-adventures/bytecode-compiler";
 *     const clrCode = new CLRCompiler().compile(ast);
 *
 *     // WASM backend:
 *     import { WASMCompiler } from "@coding-adventures/bytecode-compiler";
 *     const wasmCode = new WASMCompiler().compile(ast);
 */

// Custom VM compiler
export { BytecodeCompiler, compileSource } from "./compiler.js";

// VM types (CodeObject, Instruction, OpCode, VirtualMachine)
export { OpCode, VirtualMachine } from "./vm-types.js";
export type {
  OpCodeValue,
  Instruction,
  CodeObject,
} from "./vm-types.js";

// JVM compiler
export { JVMCompiler } from "./jvm-compiler.js";
export type { JVMCodeObject } from "./jvm-compiler.js";

// CLR compiler
export { CLRCompiler } from "./clr-compiler.js";
export type { CLRCodeObject } from "./clr-compiler.js";

// WASM compiler
export { WASMCompiler } from "./wasm-compiler.js";
export type { WASMCodeObject } from "./wasm-compiler.js";

// Generic compiler framework
export {
  GenericCompiler,
  CompilerError,
  UnhandledRuleError,
  DefaultCompilerScope,
} from "./generic-compiler.js";
export type {
  CompileHandler,
  ASTNode,
  TokenNode,
  CompilerScope,
} from "./generic-compiler.js";
