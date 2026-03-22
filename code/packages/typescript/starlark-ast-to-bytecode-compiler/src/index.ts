/**
 * Starlark AST-to-Bytecode Compiler -- Compiles Starlark ASTs into bytecode.
 *
 * This package is the bridge between the Starlark parser and the virtual machine.
 * It takes the Abstract Syntax Tree (AST) produced by the starlark-parser and
 * transforms it into a flat sequence of bytecode instructions that the VM can
 * execute.
 *
 * The compilation pipeline:
 *
 *     Starlark source code
 *         | (starlark-lexer)
 *     Token stream
 *         | (starlark-parser)
 *     AST (ASTNode tree)
 *         | (THIS PACKAGE)
 *     CodeObject (bytecode)
 *         | (virtual-machine)
 *     Execution result
 *
 * Usage:
 *
 *     import { compileStarlark } from "@coding-adventures/starlark-ast-to-bytecode-compiler";
 *
 *     const code = compileStarlark("x = 1 + 2\n");
 *     // code.instructions, code.constants, code.names are ready for the VM
 *
 *     // Or step by step:
 *     import { createStarlarkCompiler, Op } from "@coding-adventures/starlark-ast-to-bytecode-compiler";
 *     import { parseStarlark } from "@coding-adventures/starlark-parser";
 *
 *     const ast = parseStarlark("x = 1 + 2\n");
 *     const compiler = createStarlarkCompiler();
 *     const code = compiler.compile(ast, Op.HALT);
 */

export { createStarlarkCompiler, compileStarlark, parseStringLiteral } from "./compiler.js";

export {
  Op,
  type OpValue,
  BINARY_OP_MAP,
  COMPARE_OP_MAP,
  AUGMENTED_ASSIGN_MAP,
  UNARY_OP_MAP,
} from "./opcodes.js";
