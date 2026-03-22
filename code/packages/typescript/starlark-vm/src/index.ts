/**
 * Starlark VM -- A complete Starlark bytecode virtual machine.
 *
 * This package implements a Starlark bytecode interpreter on top of the
 * GenericVM framework. It provides:
 *
 * - **59 opcode handlers** covering the full Starlark instruction set
 * - **23 built-in functions** (type, len, range, print, sorted, etc.)
 * - **Factory function** ``createStarlarkVM()`` to create configured VMs
 * - **Convenience function** ``executeStarlark()`` for one-call execution
 *
 * Key types:
 * - {@link StarlarkFunction} -- User-defined functions created by ``def``
 * - {@link StarlarkIterator} -- For-loop iteration wrapper
 * - {@link StarlarkResult} -- Execution output (variables, output, traces)
 * - {@link Op} -- The 46-opcode instruction set enumeration
 *
 * @module
 */

// Types and helpers
export {
  Op,
  type OpValue,
  StarlarkFunction,
  StarlarkIterator,
  type StarlarkResult,
  isTruthy,
  starlarkRepr,
  starlarkValueRepr,
  starlarkTypeName,
} from "./types.js";

// Built-in functions
export {
  getAllBuiltins,
  builtinType,
  builtinBool,
  builtinInt,
  builtinFloat,
  builtinStr,
  builtinLen,
  builtinList,
  builtinDict,
  builtinTuple,
  builtinRange,
  builtinSorted,
  builtinReversed,
  builtinEnumerate,
  builtinZip,
  builtinMin,
  builtinMax,
  builtinAbs,
  builtinAll,
  builtinAny,
  builtinRepr,
  builtinHasattr,
  builtinGetattr,
  builtinPrint,
} from "./builtins.js";

// Opcode handlers
export {
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

// VM factory and execution
export { createStarlarkVM, executeStarlark } from "./vm.js";
