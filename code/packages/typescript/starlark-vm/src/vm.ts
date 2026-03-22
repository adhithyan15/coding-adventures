/**
 * Starlark VM -- A complete Starlark bytecode interpreter.
 *
 * ==========================================================================
 * Chapter 1: The Full Pipeline
 * ==========================================================================
 *
 * This module ties everything together. The ``createStarlarkVM()`` factory
 * creates a ``GenericVM`` that's fully configured for Starlark execution:
 *
 * 1. All 46 opcodes have registered handlers.
 * 2. All 23 built-in functions are registered.
 * 3. Starlark-specific restrictions are configured (recursion limits, etc.).
 *
 * The ``executeStarlark()`` convenience function goes even further: it takes
 * a CodeObject (pre-compiled bytecode) and executes it in one call.
 *
 * ==========================================================================
 * Chapter 2: How to Use
 * ==========================================================================
 *
 * **Quick start -- create a VM and run bytecode:**
 *
 *     const vm = createStarlarkVM();
 *     const code: CodeObject = { instructions: [...], constants: [...], names: [...] };
 *     const traces = vm.execute(code);
 *     console.log(vm.variables["x"]);  // inspect results
 *
 * **Even quicker -- use executeStarlark():**
 *
 *     const result = executeStarlark(code);
 *     console.log(result.variables["x"]);
 *     console.log(result.output);  // captured print output
 *
 * @module
 */

import type { CodeObject, VMValue } from "@coding-adventures/virtual-machine";
import { GenericVM } from "@coding-adventures/virtual-machine";

import { Op, type StarlarkResult } from "./types.js";
import { getAllBuiltins } from "./builtins.js";
import {
  handleLoadConst,
  handlePop,
  handleDup,
  handleLoadNone,
  handleLoadTrue,
  handleLoadFalse,
  handleStoreName,
  handleLoadName,
  handleStoreLocal,
  handleLoadLocal,
  handleStoreClosure,
  handleLoadClosure,
  handleAdd,
  handleSub,
  handleMul,
  handleDiv,
  handleFloorDiv,
  handleMod,
  handlePower,
  handleNegate,
  handleBitAnd,
  handleBitOr,
  handleBitXor,
  handleBitNot,
  handleLshift,
  handleRshift,
  handleCmpEq,
  handleCmpNe,
  handleCmpLt,
  handleCmpGt,
  handleCmpLe,
  handleCmpGe,
  handleCmpIn,
  handleCmpNotIn,
  handleNot,
  handleJump,
  handleJumpIfFalse,
  handleJumpIfTrue,
  handleJumpIfFalseOrPop,
  handleJumpIfTrueOrPop,
  handleMakeFunction,
  handleCallFunction,
  handleCallFunctionKw,
  handleReturn,
  handleBuildList,
  handleBuildDict,
  handleBuildTuple,
  handleListAppend,
  handleDictSet,
  handleLoadSubscript,
  handleStoreSubscript,
  handleLoadAttr,
  handleStoreAttr,
  handleLoadSlice,
  handleGetIter,
  handleForIter,
  handleUnpackSequence,
  handleLoadModule,
  handleImportFrom,
  handlePrint,
  handleHalt,
} from "./handlers.js";

// =========================================================================
// VM Factory
// =========================================================================

/**
 * Create a ``GenericVM`` fully configured for Starlark execution.
 *
 * This is the main factory function. It:
 * 1. Creates a fresh GenericVM.
 * 2. Registers all 46 Starlark opcode handlers.
 * 3. Registers all 23 Starlark built-in functions.
 * 4. Configures Starlark-specific restrictions.
 *
 * The factory pattern is used (instead of subclassing GenericVM) because:
 *
 * - **Composition over inheritance** -- We configure a generic engine
 *   rather than creating a specialized subclass. This keeps the GenericVM
 *   truly generic and reusable for other languages.
 *
 * - **Testability** -- Each handler is a standalone function that can be
 *   tested independently. If we baked them into a subclass, they'd be
 *   harder to test in isolation.
 *
 * - **Real-world parallel** -- This is how the JVM works. The JVM itself
 *   is a generic execution engine. Java-specific semantics are defined
 *   by the Java Language Specification and implemented by the compiler,
 *   not hardcoded into the VM.
 *
 * @param maxRecursionDepth - Maximum call stack depth. Default 200.
 * @param frozen - Whether to start in frozen mode.
 * @returns A GenericVM ready to execute Starlark bytecode.
 */
export function createStarlarkVM(
  maxRecursionDepth: number = 200,
  frozen: boolean = false,
): GenericVM {
  const vm = new GenericVM();

  // -- Register all opcode handlers --

  // Stack operations
  vm.registerOpcode(Op.LOAD_CONST, handleLoadConst);
  vm.registerOpcode(Op.POP, handlePop);
  vm.registerOpcode(Op.DUP, handleDup);
  vm.registerOpcode(Op.LOAD_NONE, handleLoadNone);
  vm.registerOpcode(Op.LOAD_TRUE, handleLoadTrue);
  vm.registerOpcode(Op.LOAD_FALSE, handleLoadFalse);

  // Variable operations
  vm.registerOpcode(Op.STORE_NAME, handleStoreName);
  vm.registerOpcode(Op.LOAD_NAME, handleLoadName);
  vm.registerOpcode(Op.STORE_LOCAL, handleStoreLocal);
  vm.registerOpcode(Op.LOAD_LOCAL, handleLoadLocal);
  vm.registerOpcode(Op.STORE_CLOSURE, handleStoreClosure);
  vm.registerOpcode(Op.LOAD_CLOSURE, handleLoadClosure);

  // Arithmetic
  vm.registerOpcode(Op.ADD, handleAdd);
  vm.registerOpcode(Op.SUB, handleSub);
  vm.registerOpcode(Op.MUL, handleMul);
  vm.registerOpcode(Op.DIV, handleDiv);
  vm.registerOpcode(Op.FLOOR_DIV, handleFloorDiv);
  vm.registerOpcode(Op.MOD, handleMod);
  vm.registerOpcode(Op.POWER, handlePower);
  vm.registerOpcode(Op.NEGATE, handleNegate);
  vm.registerOpcode(Op.BIT_AND, handleBitAnd);
  vm.registerOpcode(Op.BIT_OR, handleBitOr);
  vm.registerOpcode(Op.BIT_XOR, handleBitXor);
  vm.registerOpcode(Op.BIT_NOT, handleBitNot);
  vm.registerOpcode(Op.LSHIFT, handleLshift);
  vm.registerOpcode(Op.RSHIFT, handleRshift);

  // Comparisons
  vm.registerOpcode(Op.CMP_EQ, handleCmpEq);
  vm.registerOpcode(Op.CMP_NE, handleCmpNe);
  vm.registerOpcode(Op.CMP_LT, handleCmpLt);
  vm.registerOpcode(Op.CMP_GT, handleCmpGt);
  vm.registerOpcode(Op.CMP_LE, handleCmpLe);
  vm.registerOpcode(Op.CMP_GE, handleCmpGe);
  vm.registerOpcode(Op.CMP_IN, handleCmpIn);
  vm.registerOpcode(Op.CMP_NOT_IN, handleCmpNotIn);

  // Boolean
  vm.registerOpcode(Op.NOT, handleNot);

  // Control flow
  vm.registerOpcode(Op.JUMP, handleJump);
  vm.registerOpcode(Op.JUMP_IF_FALSE, handleJumpIfFalse);
  vm.registerOpcode(Op.JUMP_IF_TRUE, handleJumpIfTrue);
  vm.registerOpcode(Op.JUMP_IF_FALSE_OR_POP, handleJumpIfFalseOrPop);
  vm.registerOpcode(Op.JUMP_IF_TRUE_OR_POP, handleJumpIfTrueOrPop);

  // Functions
  vm.registerOpcode(Op.MAKE_FUNCTION, handleMakeFunction);
  vm.registerOpcode(Op.CALL_FUNCTION, handleCallFunction);
  vm.registerOpcode(Op.CALL_FUNCTION_KW, handleCallFunctionKw);
  vm.registerOpcode(Op.RETURN, handleReturn);

  // Collections
  vm.registerOpcode(Op.BUILD_LIST, handleBuildList);
  vm.registerOpcode(Op.BUILD_DICT, handleBuildDict);
  vm.registerOpcode(Op.BUILD_TUPLE, handleBuildTuple);
  vm.registerOpcode(Op.LIST_APPEND, handleListAppend);
  vm.registerOpcode(Op.DICT_SET, handleDictSet);

  // Subscript & attribute
  vm.registerOpcode(Op.LOAD_SUBSCRIPT, handleLoadSubscript);
  vm.registerOpcode(Op.STORE_SUBSCRIPT, handleStoreSubscript);
  vm.registerOpcode(Op.LOAD_ATTR, handleLoadAttr);
  vm.registerOpcode(Op.STORE_ATTR, handleStoreAttr);
  vm.registerOpcode(Op.LOAD_SLICE, handleLoadSlice);

  // Iteration
  vm.registerOpcode(Op.GET_ITER, handleGetIter);
  vm.registerOpcode(Op.FOR_ITER, handleForIter);
  vm.registerOpcode(Op.UNPACK_SEQUENCE, handleUnpackSequence);

  // Module
  vm.registerOpcode(Op.LOAD_MODULE, handleLoadModule);
  vm.registerOpcode(Op.IMPORT_FROM, handleImportFrom);

  // I/O
  vm.registerOpcode(Op.PRINT, handlePrint);

  // VM control
  vm.registerOpcode(Op.HALT, handleHalt);

  // -- Register built-in functions --
  for (const [name, impl] of Object.entries(getAllBuiltins())) {
    vm.registerBuiltin(name, impl);
  }

  // Override print() with a closure that captures output to the VM.
  // The default builtinPrint returns null without side effects because
  // builtins don't have access to the VM instance. This closure fixes
  // that by writing to vm.output directly, matching the PRINT opcode's
  // behavior for calls made via ``print("hello")``.
  vm.registerBuiltin("print", (...args: VMValue[]): VMValue => {
    const outputStr = args.map(String).join(" ");
    vm.output.push(outputStr);
    return null;
  });

  // -- Configure restrictions --
  vm.setMaxRecursionDepth(maxRecursionDepth);
  if (frozen) {
    vm.setFrozen(true);
  }

  return vm;
}

// =========================================================================
// Convenience Function
// =========================================================================

/**
 * Execute a CodeObject using a fresh Starlark VM.
 *
 * This is the highest-level API. Pass in a compiled CodeObject,
 * get back the execution result with variables, output, and traces.
 *
 * @param code - A compiled Starlark CodeObject.
 * @returns The execution result.
 */
export function executeStarlark(code: CodeObject): StarlarkResult {
  const vm = createStarlarkVM();
  const traces = vm.execute(code);

  return {
    variables: { ...vm.variables },
    output: [...vm.output],
    traces,
  };
}
