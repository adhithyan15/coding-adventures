/**
 * index.ts — Public API for @coding-adventures/register-vm.
 *
 * This package implements a register-based virtual machine modelled on
 * V8's Ignition interpreter. Key concepts:
 *
 * - **Accumulator model**: Most instructions use an implicit accumulator
 *   register as their output, reducing instruction encoding size.
 *
 * - **Feedback vectors**: Each call frame records type information about
 *   binary operations and property accesses, enabling future JIT
 *   compilation.
 *
 * - **Hidden classes**: Objects carry a monotonically-increasing
 *   hiddenClassId that changes when new properties are added, simulating
 *   V8's Map (hidden class) transitions.
 *
 * - **Context chains**: Closures capture lexical scope through a linked
 *   list of Context objects.
 *
 * ## Quick start
 *
 * ```typescript
 * import { RegisterVM, Opcode } from '@coding-adventures/register-vm';
 *
 * const vm = new RegisterVM();
 *
 * // Build a trivial code object: load 42 and halt.
 * const result = vm.execute({
 *   name: 'main',
 *   instructions: [
 *     { opcode: Opcode.LDA_SMI,  operands: [42], feedbackSlot: null },
 *     { opcode: Opcode.HALT,     operands: [],   feedbackSlot: null },
 *   ],
 *   constants: [],
 *   names: [],
 *   registerCount: 0,
 *   feedbackSlotCount: 0,
 *   parameterCount: 0,
 * });
 *
 * console.log(result.returnValue); // 42
 * ```
 */

// Core VM class and object helpers
export { RegisterVM, newObject, objectWithHiddenClass } from './vm.js';

// Types
export type {
  VMValue,
  VMObject,
  VMArray,
  VMFunction,
  CodeObject,
  RegisterInstruction,
  CallFrame,
  FeedbackSlot,
  TypePair,
  VMResult,
  VMError,
  TraceStep,
  Context,
} from './types.js';

// Opcode constants
export { Opcode, opcodeName } from './opcodes.js';

// Feedback vector utilities
export { newVector, valueType, recordBinaryOp, recordPropertyLoad, recordCallSite } from './feedback.js';

// Scope chain utilities
export { newContext, getSlot, setSlot } from './scope.js';
