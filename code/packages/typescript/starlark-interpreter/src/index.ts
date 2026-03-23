/**
 * Starlark Interpreter — The complete execution pipeline.
 *
 * Chains the lexer, parser, compiler, and VM together with ``load()`` support.
 * This is the top-level entry point for executing Starlark programs.
 *
 * Key exports:
 *
 *   - {@link StarlarkInterpreter} — Configurable interpreter class with
 *     load caching, file resolvers, and pluggable compiler/VM.
 *
 *   - {@link interpret} — One-call convenience function for source execution.
 *
 *   - {@link interpretBytecode} — Execute pre-compiled bytecode directly.
 *
 *   - {@link interpretFile} — Execute a Starlark file by path.
 *
 *   - {@link dictResolver} — Create a file resolver from a dictionary
 *     (useful for testing).
 *
 *   - {@link Op} — Starlark opcode constants (for constructing bytecode
 *     in tests or for use by the compiler).
 *
 *   - {@link createMiniStarlarkVM} — A minimal VM factory with basic
 *     Starlark handlers (for testing without the full starlark-vm package).
 *
 * Usage:
 *
 * ```typescript
 * import { interpretBytecode, Op } from "@coding-adventures/starlark-interpreter";
 *
 * const code = {
 *   instructions: [
 *     { opcode: Op.LOAD_CONST, operand: 0 },
 *     { opcode: Op.STORE_NAME, operand: 0 },
 *     { opcode: Op.HALT },
 *   ],
 *   constants: [42],
 *   names: ["x"],
 * };
 *
 * const result = interpretBytecode(code);
 * console.log(result.variables["x"]); // 42
 * ```
 */

export {
  StarlarkInterpreter,
  interpret,
  interpretBytecode,
  interpretFile,
  dictResolver,
  resolveFile,
  createMiniStarlarkVM,
  registerMiniStarlarkHandlers,
  FileNotFoundError,
  Op,
} from "./interpreter.js";

export type {
  FileResolver,
  FileResolverFn,
  StarlarkResult,
  CompileFn,
  CreateVMFn,
} from "./interpreter.js";
