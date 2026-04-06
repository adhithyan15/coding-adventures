/**
 * vm.ts — The RegisterVM interpreter.
 *
 * ## Architecture overview
 *
 * The VM follows the V8 Ignition design:
 *
 *   1. Instructions are decoded one at a time (no pre-fetch or pipelining).
 *   2. The accumulator register is implicit in every instruction.
 *   3. A register file provides named local storage per frame.
 *   4. A feedback vector records type information for the JIT.
 *   5. A context chain provides lexical scope for closures.
 *
 * ## Execution model
 *
 * ```
 *   execute(code)
 *     └─ newFrame(code, null)
 *          └─ runFrame(frame)
 *               └─ while not HALT/RETURN:
 *                    instr = code.instructions[frame.ip++]
 *                    dispatch(instr)  →  mutate frame state
 *               └─ return frame.accumulator
 * ```
 *
 * Nested calls (CALL_ANY_RECEIVER) recurse into runFrame with a new child
 * frame that stores the caller frame in `callerFrame` (for debugging; the
 * return path doesn't walk this chain — recursion naturally unwinds it).
 *
 * ## Error handling
 *
 * Runtime errors are thrown as plain objects matching the VMError interface.
 * They are caught in `execute()` and returned in the VMResult.error field
 * so callers don't have to use try/catch.
 */

import { Opcode } from './opcodes.js';
import {
  newVector,
  recordBinaryOp,
  recordCallSite,
  recordPropertyLoad,
  valueType,
} from './feedback.js';
import { getSlot, newContext, setSlot } from './scope.js';
import type {
  CallFrame,
  CodeObject,
  Context,
  FeedbackSlot,
  RegisterInstruction,
  TraceStep,
  VMArray,
  VMError,
  VMFunction,
  VMObject,
  VMResult,
  VMValue,
} from './types.js';

// ─────────────────────────────────────────────────────────────────────────────
// Object allocation helpers
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Monotonically increasing counter used to assign hidden class IDs.
 * Exported so tests can inspect transitions.
 */
let _nextHiddenClassId = 0;

/**
 * Create a new empty object with a fresh hidden class ID.
 * Simulates V8's `JSObject::Create()`.
 */
export function newObject(): VMObject {
  return { hiddenClassId: _nextHiddenClassId++, properties: new Map() };
}

/**
 * Simulate a hidden-class transition for a property write.
 *
 * In V8, adding a new property to an object causes a Map (hidden class)
 * transition because the object's shape has changed.  Subsequent objects
 * that add properties in the same order share transitions — but here we
 * always allocate a new ID for simplicity.
 *
 * @param obj  The object whose shape is changing.
 * @returns    A new VMObject with the same properties but a new hidden class ID.
 *             The caller is responsible for replacing the old reference.
 */
export function objectWithHiddenClass(obj: VMObject): VMObject {
  return { hiddenClassId: _nextHiddenClassId++, properties: obj.properties };
}

// ─────────────────────────────────────────────────────────────────────────────
// Sentinel to signal HALT from inside runFrame
// ─────────────────────────────────────────────────────────────────────────────

const HALT_SENTINEL = Symbol('HALT');

// ─────────────────────────────────────────────────────────────────────────────
// The VM
// ─────────────────────────────────────────────────────────────────────────────

/**
 * A register-based virtual machine modelled on V8's Ignition interpreter.
 *
 * Each RegisterVM instance maintains:
 *   - A globals map (shared across all frames)
 *   - An output buffer (lines "printed" by the program)
 *   - A call-depth counter for stack-overflow protection
 *
 * @example
 * ```typescript
 * const vm = new RegisterVM();
 * const code: CodeObject = {
 *   name: 'main',
 *   instructions: [
 *     { opcode: Opcode.LDA_SMI,  operands: [42], feedbackSlot: null },
 *     { opcode: Opcode.HALT,     operands: [],   feedbackSlot: null },
 *   ],
 *   constants: [], names: [], registerCount: 0,
 *   feedbackSlotCount: 0, parameterCount: 0,
 * };
 * const result = vm.execute(code);
 * // result.returnValue === 42
 * ```
 */
export class RegisterVM {
  private globals = new Map<string, VMValue>();
  private callDepth = 0;
  private readonly maxDepth: number;
  private output: string[] = [];

  constructor(options: { maxDepth?: number } = {}) {
    this.maxDepth = options.maxDepth ?? 500;
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Public API
  // ──────────────────────────────────────────────────────────────────────────

  /**
   * Execute a CodeObject and return the result.
   *
   * Execution continues until a HALT or RETURN instruction is reached, or
   * until a runtime error occurs.
   *
   * @param code The top-level code to run.
   * @returns    A VMResult with the return value, output lines, and any error.
   */
  execute(code: CodeObject): VMResult {
    this.output = [];
    this.callDepth = 0;
    try {
      const frame = this._newFrame(code, null);
      const returnValue = this._runFrame(frame);
      return { returnValue, output: this.output, error: null };
    } catch (e: unknown) {
      if (isVMError(e)) {
        return {
          returnValue: undefined,
          output: this.output,
          error: e,
        };
      }
      // Unexpected JS error — wrap it.
      return {
        returnValue: undefined,
        output: this.output,
        error: {
          message: e instanceof Error ? e.message : String(e),
          instructionIndex: -1,
          opcode: -1,
        },
      };
    }
  }

  /**
   * Execute a CodeObject with full instruction-level tracing.
   *
   * Returns both the final result and a step-by-step trace of every
   * instruction that executed. Useful for debugging and visualisation.
   *
   * @param code The top-level code to run.
   */
  executeWithTrace(code: CodeObject): { result: VMResult; trace: TraceStep[] } {
    this.output = [];
    this.callDepth = 0;
    const trace: TraceStep[] = [];
    try {
      const frame = this._newFrame(code, null);
      const returnValue = this._runFrameWithTrace(frame, trace, 0);
      return { result: { returnValue, output: this.output, error: null }, trace };
    } catch (e: unknown) {
      if (isVMError(e)) {
        return {
          result: { returnValue: undefined, output: this.output, error: e },
          trace,
        };
      }
      return {
        result: {
          returnValue: undefined,
          output: this.output,
          error: {
            message: e instanceof Error ? e.message : String(e),
            instructionIndex: -1,
            opcode: -1,
          },
        },
        trace,
      };
    }
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Frame management
  // ──────────────────────────────────────────────────────────────────────────

  /**
   * Allocate a new call frame for the given CodeObject.
   *
   * @param code        The code to execute in the frame.
   * @param callerFrame The frame that issued the call (null for top-level).
   */
  private _newFrame(code: CodeObject, callerFrame: CallFrame | null): CallFrame {
    return {
      code,
      ip: 0,
      accumulator: undefined,
      registers: new Array<VMValue>(code.registerCount).fill(undefined),
      feedbackVector: newVector(code.feedbackSlotCount),
      context: null,
      callerFrame,
    };
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Main dispatch loop (no trace)
  // ──────────────────────────────────────────────────────────────────────────

  /**
   * Execute all instructions in `frame` until HALT, RETURN, or error.
   *
   * Returns the accumulator value at termination.
   */
  private _runFrame(frame: CallFrame): VMValue {
    const { code } = frame;
    while (frame.ip < code.instructions.length) {
      const instr = code.instructions[frame.ip];
      frame.ip++;
      const result = this._dispatch(frame, instr);
      if (result === HALT_SENTINEL) break;
    }
    return frame.accumulator;
  }

  /**
   * Execute with trace recording.
   */
  private _runFrameWithTrace(
    frame: CallFrame,
    trace: TraceStep[],
    depth: number,
  ): VMValue {
    const { code } = frame;
    while (frame.ip < code.instructions.length) {
      const instr = code.instructions[frame.ip];
      const ip = frame.ip;
      frame.ip++;

      // Snapshot before
      const accBefore = frame.accumulator;
      const regsBefore = [...frame.registers];
      const vecBefore = frame.feedbackVector.map(s => ({ ...s } as FeedbackSlot));

      const result = this._dispatch(frame, instr);

      // Snapshot after
      const accAfter = frame.accumulator;
      const regsAfter = [...frame.registers];

      // Compute feedback deltas.
      // We use JSON serialization for structural equality because the spread
      // copy above always produces new objects, making reference equality
      // (`before !== after`) useless.
      const feedbackDelta: TraceStep['feedbackDelta'] = [];
      for (let i = 0; i < frame.feedbackVector.length; i++) {
        const before = vecBefore[i];
        const after = frame.feedbackVector[i];
        // Fast path: same kind, no nested data (uninitialized or megamorphic).
        if (before.kind !== after.kind) {
          feedbackDelta.push({ slot: i, before, after });
        } else if (
          (before.kind === 'monomorphic' || before.kind === 'polymorphic') &&
          (after.kind === 'monomorphic' || after.kind === 'polymorphic')
        ) {
          // Compare the types arrays structurally.
          if (JSON.stringify(before.types) !== JSON.stringify(after.types)) {
            feedbackDelta.push({ slot: i, before, after });
          }
        }
      }

      trace.push({
        frameDepth: depth,
        ip,
        instruction: instr,
        accBefore,
        accAfter,
        registersBefore: regsBefore,
        registersAfter: regsAfter,
        feedbackDelta,
      });

      if (result === HALT_SENTINEL) break;
    }
    return frame.accumulator;
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Instruction dispatcher
  // ──────────────────────────────────────────────────────────────────────────

  /**
   * Execute one instruction, mutating `frame` in place.
   *
   * Returns HALT_SENTINEL if the frame should stop (HALT or RETURN opcode).
   */
  private _dispatch(
    frame: CallFrame,
    instr: RegisterInstruction,
  ): typeof HALT_SENTINEL | void {
    const { opcode, operands } = instr;
    const { code } = frame;

    switch (opcode) {
      // ────────────────────────────────────────────────────────────────────
      // 0x0_ — Accumulator loads
      // ────────────────────────────────────────────────────────────────────

      case Opcode.LDA_CONSTANT: {
        // Load a value from the constant pool.
        // operands[0] = pool index
        //
        // Constants can be numbers, strings, booleans, nested CodeObjects,
        // or any other VMValue — the compiler decides what to put in the pool.
        frame.accumulator = code.constants[operands[0]];
        break;
      }

      case Opcode.LDA_ZERO: {
        // Specialised zero-load: avoids a constant-pool entry for 0.
        frame.accumulator = 0;
        break;
      }

      case Opcode.LDA_SMI: {
        // Load a small integer (Small Integer, "SMI") directly from the operand.
        // SMIs are integers that fit in 31 bits (V8 uses the low bit as a tag).
        // Here we simply embed the integer as a 32-bit signed value.
        frame.accumulator = operands[0] | 0;
        break;
      }

      case Opcode.LDA_UNDEFINED: {
        frame.accumulator = undefined;
        break;
      }

      case Opcode.LDA_NULL: {
        frame.accumulator = null;
        break;
      }

      case Opcode.LDA_TRUE: {
        frame.accumulator = true;
        break;
      }

      case Opcode.LDA_FALSE: {
        frame.accumulator = false;
        break;
      }

      // ────────────────────────────────────────────────────────────────────
      // 0x1_ — Register moves
      // ────────────────────────────────────────────────────────────────────

      case Opcode.LDAR: {
        // Load accumulator from register.
        // operands[0] = register index
        frame.accumulator = frame.registers[operands[0]];
        break;
      }

      case Opcode.STAR: {
        // Store accumulator to register.
        // operands[0] = destination register index
        frame.registers[operands[0]] = frame.accumulator;
        break;
      }

      case Opcode.MOV: {
        // Register-to-register copy.
        // operands[0] = source register, operands[1] = destination register
        frame.registers[operands[1]] = frame.registers[operands[0]];
        break;
      }

      // ────────────────────────────────────────────────────────────────────
      // 0x2_ — Variable access
      // ────────────────────────────────────────────────────────────────────

      case Opcode.LDA_GLOBAL: {
        // operands[0] = name table index
        const name = code.names[operands[0]];
        frame.accumulator = this.globals.get(name);
        break;
      }

      case Opcode.STA_GLOBAL: {
        const name = code.names[operands[0]];
        this.globals.set(name, frame.accumulator);
        break;
      }

      case Opcode.LDA_LOCAL: {
        // Same as LDAR but semantically marks a named local variable access.
        frame.accumulator = frame.registers[operands[0]];
        break;
      }

      case Opcode.STA_LOCAL: {
        frame.registers[operands[0]] = frame.accumulator;
        break;
      }

      case Opcode.LDA_CONTEXT_SLOT: {
        // operands[0] = depth, operands[1] = slot index
        if (frame.context !== null) {
          frame.accumulator = getSlot(frame.context, operands[0], operands[1]);
        } else {
          frame.accumulator = undefined;
        }
        break;
      }

      case Opcode.STA_CONTEXT_SLOT: {
        if (frame.context !== null) {
          setSlot(frame.context, operands[0], operands[1], frame.accumulator);
        }
        break;
      }

      case Opcode.LDA_CURRENT_CONTEXT_SLOT: {
        // Shortcut: depth = 0
        if (frame.context !== null) {
          frame.accumulator = getSlot(frame.context, 0, operands[0]);
        } else {
          frame.accumulator = undefined;
        }
        break;
      }

      case Opcode.STA_CURRENT_CONTEXT_SLOT: {
        if (frame.context !== null) {
          setSlot(frame.context, 0, operands[0], frame.accumulator);
        }
        break;
      }

      // ────────────────────────────────────────────────────────────────────
      // 0x3_ — Arithmetic & bitwise
      // ────────────────────────────────────────────────────────────────────

      case Opcode.ADD: {
        // operands[0] = right-hand register, operands[1] = feedback slot
        const right = frame.registers[operands[0]];
        const slot = operands[1] ?? instr.feedbackSlot;
        if (slot !== undefined && slot !== null) {
          recordBinaryOp(frame.feedbackVector, slot, frame.accumulator, right);
        }
        frame.accumulator = doAdd(frame.accumulator, right);
        break;
      }

      case Opcode.SUB: {
        const right = frame.registers[operands[0]];
        const slot = operands[1] ?? instr.feedbackSlot;
        if (slot !== undefined && slot !== null) {
          recordBinaryOp(frame.feedbackVector, slot, frame.accumulator, right);
        }
        frame.accumulator = doArith(frame.accumulator, right, (a, b) => a - b);
        break;
      }

      case Opcode.MUL: {
        const right = frame.registers[operands[0]];
        const slot = operands[1] ?? instr.feedbackSlot;
        if (slot !== undefined && slot !== null) {
          recordBinaryOp(frame.feedbackVector, slot, frame.accumulator, right);
        }
        frame.accumulator = doArith(frame.accumulator, right, (a, b) => a * b);
        break;
      }

      case Opcode.DIV: {
        const right = frame.registers[operands[0]];
        const slot = operands[1] ?? instr.feedbackSlot;
        if (slot !== undefined && slot !== null) {
          recordBinaryOp(frame.feedbackVector, slot, frame.accumulator, right);
        }
        frame.accumulator = doArith(frame.accumulator, right, (a, b) => a / b);
        break;
      }

      case Opcode.MOD: {
        const right = frame.registers[operands[0]];
        frame.accumulator = doArith(frame.accumulator, right, (a, b) => a % b);
        break;
      }

      case Opcode.POW: {
        const right = frame.registers[operands[0]];
        frame.accumulator = doArith(frame.accumulator, right, (a, b) => Math.pow(a, b));
        break;
      }

      case Opcode.ADD_SMI: {
        // Fast-path: add a small integer literal to the accumulator.
        // operands[0] = the SMI value (signed 32-bit)
        const smi = operands[0] | 0;
        if (typeof frame.accumulator === 'number') {
          frame.accumulator = frame.accumulator + smi;
        } else {
          frame.accumulator = NaN;
        }
        break;
      }

      case Opcode.SUB_SMI: {
        const smi = operands[0] | 0;
        if (typeof frame.accumulator === 'number') {
          frame.accumulator = frame.accumulator - smi;
        } else {
          frame.accumulator = NaN;
        }
        break;
      }

      case Opcode.BITWISE_AND: {
        const right = frame.registers[operands[0]];
        frame.accumulator = toInt32(frame.accumulator) & toInt32(right);
        break;
      }

      case Opcode.BITWISE_OR: {
        const right = frame.registers[operands[0]];
        frame.accumulator = toInt32(frame.accumulator) | toInt32(right);
        break;
      }

      case Opcode.BITWISE_XOR: {
        const right = frame.registers[operands[0]];
        frame.accumulator = toInt32(frame.accumulator) ^ toInt32(right);
        break;
      }

      case Opcode.BITWISE_NOT: {
        frame.accumulator = ~toInt32(frame.accumulator);
        break;
      }

      case Opcode.SHIFT_LEFT: {
        const right = frame.registers[operands[0]];
        frame.accumulator = toInt32(frame.accumulator) << (toInt32(right) & 0x1f);
        break;
      }

      case Opcode.SHIFT_RIGHT: {
        const right = frame.registers[operands[0]];
        frame.accumulator = toInt32(frame.accumulator) >> (toInt32(right) & 0x1f);
        break;
      }

      case Opcode.SHIFT_RIGHT_LOGICAL: {
        const right = frame.registers[operands[0]];
        frame.accumulator = (toInt32(frame.accumulator) >>> (toInt32(right) & 0x1f));
        break;
      }

      case Opcode.NEGATE: {
        if (typeof frame.accumulator === 'number') {
          frame.accumulator = -frame.accumulator;
        } else {
          frame.accumulator = NaN;
        }
        break;
      }

      // ────────────────────────────────────────────────────────────────────
      // 0x4_ — Comparisons & type tests
      // ────────────────────────────────────────────────────────────────────

      case Opcode.TEST_EQUAL: {
        // Abstract equality (==): we use === for simplicity (no coercion).
        const right = frame.registers[operands[0]];
        frame.accumulator = frame.accumulator === right;
        break;
      }

      case Opcode.TEST_NOT_EQUAL: {
        const right = frame.registers[operands[0]];
        frame.accumulator = frame.accumulator !== right;
        break;
      }

      case Opcode.TEST_STRICT_EQUAL: {
        const right = frame.registers[operands[0]];
        frame.accumulator = frame.accumulator === right;
        break;
      }

      case Opcode.TEST_STRICT_NOT_EQUAL: {
        const right = frame.registers[operands[0]];
        frame.accumulator = frame.accumulator !== right;
        break;
      }

      case Opcode.TEST_LESS_THAN: {
        const right = frame.registers[operands[0]];
        frame.accumulator = (frame.accumulator as number) < (right as number);
        break;
      }

      case Opcode.TEST_GREATER_THAN: {
        const right = frame.registers[operands[0]];
        frame.accumulator = (frame.accumulator as number) > (right as number);
        break;
      }

      case Opcode.TEST_LESS_THAN_OR_EQUAL: {
        const right = frame.registers[operands[0]];
        frame.accumulator = (frame.accumulator as number) <= (right as number);
        break;
      }

      case Opcode.TEST_GREATER_THAN_OR_EQUAL: {
        const right = frame.registers[operands[0]];
        frame.accumulator = (frame.accumulator as number) >= (right as number);
        break;
      }

      case Opcode.TEST_IN: {
        // key in obj — acc is key, operands[0] is obj register
        const obj = frame.registers[operands[0]];
        if (isVMObject(obj)) {
          frame.accumulator = obj.properties.has(String(frame.accumulator));
        } else {
          frame.accumulator = false;
        }
        break;
      }

      case Opcode.TEST_INSTANCEOF: {
        // Simplified: check if value is an object (no prototype chain)
        frame.accumulator = isVMObject(frame.accumulator);
        break;
      }

      case Opcode.TEST_UNDETECTABLE: {
        frame.accumulator =
          frame.accumulator === null || frame.accumulator === undefined;
        break;
      }

      case Opcode.LOGICAL_NOT: {
        frame.accumulator = !toBoolean(frame.accumulator);
        break;
      }

      case Opcode.TYPEOF: {
        frame.accumulator = vmTypeof(frame.accumulator);
        break;
      }

      // ────────────────────────────────────────────────────────────────────
      // 0x5_ — Control flow
      // ────────────────────────────────────────────────────────────────────

      case Opcode.JUMP: {
        // Unconditional relative jump.
        // operands[0] = signed offset from the NEXT instruction.
        frame.ip += operands[0];
        break;
      }

      case Opcode.JUMP_IF_TRUE: {
        if (toBoolean(frame.accumulator)) frame.ip += operands[0];
        break;
      }

      case Opcode.JUMP_IF_FALSE: {
        if (!toBoolean(frame.accumulator)) frame.ip += operands[0];
        break;
      }

      case Opcode.JUMP_IF_NULL: {
        if (frame.accumulator === null) frame.ip += operands[0];
        break;
      }

      case Opcode.JUMP_IF_UNDEFINED: {
        if (frame.accumulator === undefined) frame.ip += operands[0];
        break;
      }

      case Opcode.JUMP_IF_NULL_OR_UNDEFINED: {
        if (frame.accumulator === null || frame.accumulator === undefined) {
          frame.ip += operands[0];
        }
        break;
      }

      case Opcode.JUMP_IF_TO_BOOLEAN_TRUE: {
        if (toBoolean(frame.accumulator)) frame.ip += operands[0];
        break;
      }

      case Opcode.JUMP_IF_TO_BOOLEAN_FALSE: {
        if (!toBoolean(frame.accumulator)) frame.ip += operands[0];
        break;
      }

      case Opcode.JUMP_LOOP: {
        // Backward jump — semantically same as JUMP but tagged for OSR detection.
        frame.ip += operands[0];
        break;
      }

      // ────────────────────────────────────────────────────────────────────
      // 0x6_ — Function calls
      // ────────────────────────────────────────────────────────────────────

      case Opcode.CALL_ANY_RECEIVER:
      case Opcode.CALL_PROPERTY:
      case Opcode.CALL_UNDEFINED_RECEIVER: {
        // operands[0] = register holding the callable
        // operands[1] = first argument register
        // operands[2] = argc
        // operands[3] = feedback slot (optional)
        const callableReg = operands[0];
        const firstArgReg = operands[1];
        const argc = operands[2];
        const fbSlot = operands[3];

        const callee = frame.registers[callableReg];

        // Record call-site feedback.
        if (fbSlot !== undefined && fbSlot !== null && frame.feedbackVector[fbSlot]) {
          recordCallSite(frame.feedbackVector, fbSlot, valueType(callee));
        }

        if (!isVMFunction(callee)) {
          throwVMError(
            `Not a function: ${vmTypeof(callee)}`,
            frame.ip - 1,
            opcode,
          );
        }

        // Gather arguments from the register file.
        const args: VMValue[] = [];
        for (let i = 0; i < argc; i++) {
          args.push(frame.registers[firstArgReg + i]);
        }

        // Build child frame and execute.
        const childFrame = this._newFrame(callee.code, frame);
        // Populate parameter registers with arguments.
        for (let i = 0; i < callee.code.parameterCount; i++) {
          childFrame.registers[i] = args[i] ?? undefined;
        }
        // Restore the closure's captured context.
        childFrame.context = callee.context;

        this.callDepth++;
        try {
          frame.accumulator = this._runFrame(childFrame);
        } finally {
          this.callDepth--;
        }
        break;
      }

      case Opcode.CONSTRUCT: {
        // Simplified: create a new object and call the constructor on it.
        const calleeReg = operands[0];
        const firstArgReg = operands[1];
        const argc = operands[2];

        const ctor = frame.registers[calleeReg];
        if (!isVMFunction(ctor)) {
          throwVMError(`Not a constructor: ${vmTypeof(ctor)}`, frame.ip - 1, opcode);
        }

        const instance = newObject();
        const args: VMValue[] = [];
        for (let i = 0; i < argc; i++) {
          args.push(frame.registers[firstArgReg + i]);
        }

        const childFrame = this._newFrame(ctor.code, frame);
        for (let i = 0; i < ctor.code.parameterCount; i++) {
          childFrame.registers[i] = args[i] ?? undefined;
        }
        childFrame.context = ctor.context;
        // Register 'this' — by convention register 0 for constructors.
        if (childFrame.registers.length > ctor.code.parameterCount) {
          childFrame.registers[ctor.code.parameterCount] = instance;
        }

        this.callDepth++;
        try {
          this._runFrame(childFrame);
        } finally {
          this.callDepth--;
        }
        frame.accumulator = instance;
        break;
      }

      case Opcode.CONSTRUCT_WITH_SPREAD:
      case Opcode.CALL_WITH_SPREAD: {
        // Simplified: treat as CALL_ANY_RECEIVER without spread expansion.
        const calleeReg = operands[0];
        const firstArgReg = operands[1];
        const argc = operands[2];

        const callee = frame.registers[calleeReg];
        if (!isVMFunction(callee)) {
          throwVMError(`Not a function: ${vmTypeof(callee)}`, frame.ip - 1, opcode);
        }

        const args: VMValue[] = [];
        for (let i = 0; i < argc; i++) {
          args.push(frame.registers[firstArgReg + i]);
        }

        const childFrame = this._newFrame(callee.code, frame);
        for (let i = 0; i < callee.code.parameterCount; i++) {
          childFrame.registers[i] = args[i] ?? undefined;
        }
        childFrame.context = callee.context;

        this.callDepth++;
        try {
          frame.accumulator = this._runFrame(childFrame);
        } finally {
          this.callDepth--;
        }
        break;
      }

      case Opcode.RETURN: {
        // Stop the current frame and return to the caller.
        return HALT_SENTINEL;
      }

      case Opcode.SUSPEND_GENERATOR:
      case Opcode.RESUME_GENERATOR: {
        // Generator support is a placeholder in this educational VM.
        // A full implementation would suspend the frame and store a continuation.
        break;
      }

      // ────────────────────────────────────────────────────────────────────
      // 0x7_ — Property access
      // ────────────────────────────────────────────────────────────────────

      case Opcode.LDA_NAMED_PROPERTY: {
        // operands[0] = object register
        // operands[1] = name table index
        // operands[2] = feedback slot
        const obj = frame.registers[operands[0]];
        const propName = code.names[operands[1]];
        const fbSlot = operands[2] ?? instr.feedbackSlot;

        if (isVMObject(obj)) {
          if (fbSlot !== undefined && fbSlot !== null) {
            recordPropertyLoad(frame.feedbackVector, fbSlot, obj.hiddenClassId);
          }
          frame.accumulator = obj.properties.get(propName);
        } else if (Array.isArray(obj)) {
          // Handle array property access (e.g. .length)
          if (propName === 'length') {
            frame.accumulator = (obj as VMArray).length;
          } else {
            frame.accumulator = undefined;
          }
        } else {
          frame.accumulator = undefined;
        }
        break;
      }

      case Opcode.STA_NAMED_PROPERTY: {
        // operands[0] = object register
        // operands[1] = name table index
        // operands[2] = feedback slot
        const obj = frame.registers[operands[0]];
        const propName = code.names[operands[1]];

        if (isVMObject(obj)) {
          const isNewProp = !obj.properties.has(propName);
          obj.properties.set(propName, frame.accumulator);
          if (isNewProp) {
            // New property causes a hidden class transition.
            // Mutate the hiddenClassId in place (since we have 'readonly' on the
            // interface we cast to mutable here — acceptable for simulation).
            (obj as { hiddenClassId: number }).hiddenClassId = _nextHiddenClassId++;
          }
        }
        break;
      }

      case Opcode.LDA_KEYED_PROPERTY: {
        // acc = key, operands[0] = object register
        const obj = frame.registers[operands[0]];
        const key = frame.accumulator;

        if (isVMObject(obj)) {
          frame.accumulator = obj.properties.get(String(key));
        } else if (Array.isArray(obj) && typeof key === 'number') {
          frame.accumulator = (obj as VMArray)[key];
        } else {
          frame.accumulator = undefined;
        }
        break;
      }

      case Opcode.STA_KEYED_PROPERTY: {
        // operands[0] = object register, operands[1] = key register
        const obj = frame.registers[operands[0]];
        const key = frame.registers[operands[1]];

        if (isVMObject(obj)) {
          const keyStr = String(key);
          const isNewProp = !obj.properties.has(keyStr);
          obj.properties.set(keyStr, frame.accumulator);
          if (isNewProp) {
            (obj as { hiddenClassId: number }).hiddenClassId = _nextHiddenClassId++;
          }
        } else if (Array.isArray(obj) && typeof key === 'number') {
          (obj as VMArray)[key] = frame.accumulator;
        }
        break;
      }

      case Opcode.LDA_NAMED_PROPERTY_NO_FEEDBACK: {
        const obj = frame.registers[operands[0]];
        const propName = code.names[operands[1]];
        if (isVMObject(obj)) {
          frame.accumulator = obj.properties.get(propName);
        } else {
          frame.accumulator = undefined;
        }
        break;
      }

      case Opcode.STA_NAMED_PROPERTY_NO_FEEDBACK: {
        const obj = frame.registers[operands[0]];
        const propName = code.names[operands[1]];
        if (isVMObject(obj)) {
          obj.properties.set(propName, frame.accumulator);
        }
        break;
      }

      case Opcode.DELETE_PROPERTY_STRICT:
      case Opcode.DELETE_PROPERTY_SLOPPY: {
        // operands[0] = object register, operands[1] = key register
        const obj = frame.registers[operands[0]];
        const key = frame.registers[operands[1]];
        if (isVMObject(obj)) {
          frame.accumulator = obj.properties.delete(String(key));
        } else {
          frame.accumulator = true;
        }
        break;
      }

      // ────────────────────────────────────────────────────────────────────
      // 0x8_ — Object / array / closure creation
      // ────────────────────────────────────────────────────────────────────

      case Opcode.CREATE_OBJECT_LITERAL: {
        frame.accumulator = newObject();
        break;
      }

      case Opcode.CREATE_ARRAY_LITERAL: {
        frame.accumulator = [] as VMArray;
        break;
      }

      case Opcode.CREATE_REGEXP_LITERAL: {
        // Return the regex literal from the constant pool as a VMObject.
        // operands[0] = constants index
        const pattern = code.constants[operands[0]];
        const obj = newObject();
        obj.properties.set('source', pattern);
        frame.accumulator = obj;
        break;
      }

      case Opcode.CREATE_CLOSURE: {
        // Create a closure capturing the current context.
        // operands[0] = constants index of the nested CodeObject
        const nestedCode = code.constants[operands[0]] as CodeObject;
        const closure: VMFunction = {
          kind: 'function',
          code: nestedCode,
          context: frame.context,
        };
        frame.accumulator = closure;
        break;
      }

      case Opcode.CREATE_CONTEXT: {
        // operands[0] = slot count for the new context
        const slotCount = operands[0] ?? 0;
        frame.context = newContext(frame.context, slotCount);
        break;
      }

      case Opcode.CLONE_OBJECT: {
        // Shallow clone: copies property references, assigns new hiddenClassId.
        const src = frame.accumulator;
        if (isVMObject(src)) {
          const cloned = newObject();
          for (const [k, v] of src.properties) {
            cloned.properties.set(k, v);
          }
          frame.accumulator = cloned;
        }
        // If not an object, accumulator is unchanged.
        break;
      }

      // ────────────────────────────────────────────────────────────────────
      // 0x9_ — Iteration protocol
      // ────────────────────────────────────────────────────────────────────

      case Opcode.GET_ITERATOR: {
        // Simplified: if the value is an array, return a state object.
        const iterable = frame.accumulator;
        if (Array.isArray(iterable)) {
          const iterObj = newObject();
          iterObj.properties.set('__items__', iterable);
          iterObj.properties.set('__index__', 0);
          frame.accumulator = iterObj;
        } else {
          throwVMError('Value is not iterable', frame.ip - 1, opcode);
        }
        break;
      }

      case Opcode.CALL_ITERATOR_STEP: {
        // Call iterator.next() — acc should be the iterator object.
        const iter = frame.accumulator;
        if (!isVMObject(iter)) break;
        const items = iter.properties.get('__items__');
        const index = iter.properties.get('__index__');
        if (!Array.isArray(items) || typeof index !== 'number') break;
        const resultObj = newObject();
        if (index < (items as VMArray).length) {
          resultObj.properties.set('value', (items as VMArray)[index]);
          resultObj.properties.set('done', false);
          iter.properties.set('__index__', index + 1);
        } else {
          resultObj.properties.set('value', undefined);
          resultObj.properties.set('done', true);
        }
        frame.accumulator = resultObj;
        break;
      }

      case Opcode.GET_ITERATOR_DONE: {
        const iterResult = frame.accumulator;
        if (isVMObject(iterResult)) {
          frame.accumulator = iterResult.properties.get('done') ?? false;
        } else {
          frame.accumulator = true;
        }
        break;
      }

      case Opcode.GET_ITERATOR_VALUE: {
        const iterResult = frame.accumulator;
        if (isVMObject(iterResult)) {
          frame.accumulator = iterResult.properties.get('value');
        } else {
          frame.accumulator = undefined;
        }
        break;
      }

      // ────────────────────────────────────────────────────────────────────
      // 0xA_ — Exception control
      // ────────────────────────────────────────────────────────────────────

      case Opcode.THROW: {
        const msg = frame.accumulator;
        throwVMError(String(msg), frame.ip - 1, opcode);
        break;
      }

      case Opcode.RETHROW: {
        // In a full VM this would re-raise the current exception.
        // Here we just throw a placeholder.
        throwVMError('rethrow', frame.ip - 1, opcode);
        break;
      }

      // ────────────────────────────────────────────────────────────────────
      // 0xB_ — Context / scope
      // ────────────────────────────────────────────────────────────────────

      case Opcode.PUSH_CONTEXT: {
        const slotCount = operands[0] ?? 0;
        frame.context = newContext(frame.context, slotCount);
        break;
      }

      case Opcode.POP_CONTEXT: {
        if (frame.context !== null) {
          frame.context = frame.context.parent;
        }
        break;
      }

      case Opcode.LDA_MODULE_VARIABLE: {
        // Treat module variables like globals for simplicity.
        const name = code.names[operands[0]];
        frame.accumulator = this.globals.get(name);
        break;
      }

      case Opcode.STA_MODULE_VARIABLE: {
        const name = code.names[operands[0]];
        this.globals.set(name, frame.accumulator);
        break;
      }

      // ────────────────────────────────────────────────────────────────────
      // 0xF_ — VM meta-instructions
      // ────────────────────────────────────────────────────────────────────

      case Opcode.STACK_CHECK: {
        // V8 inserts this at the start of every function to detect infinite
        // recursion before the native call stack overflows.
        //
        // We use callDepth (incremented on each CALL) as a proxy for stack depth.
        if (this.callDepth > this.maxDepth) {
          throwVMError(
            `Maximum call stack size exceeded (depth ${this.callDepth})`,
            frame.ip - 1,
            opcode,
          );
        }
        break;
      }

      case Opcode.DEBUGGER: {
        // Trigger a JS debugger breakpoint if a debugger is attached.
        // In production V8, this causes a trap to the V8 Inspector API.
        // eslint-disable-next-line no-debugger
        // (intentionally a no-op in this VM)
        break;
      }

      case Opcode.HALT: {
        // Terminate the entire VM and return the accumulator.
        return HALT_SENTINEL;
      }

      default: {
        // Unknown opcode — this indicates a bug in the compiler or a corrupted
        // bytecode stream.
        throwVMError(
          `Unknown opcode 0x${opcode.toString(16).toUpperCase()}`,
          frame.ip - 1,
          opcode,
        );
      }
    }
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Arithmetic helpers
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Addition with JS-style string coercion.
 *
 * Truth table:
 *   number + number → number  (numeric addition)
 *   string + any    → string  (string concatenation)
 *   any    + string → string
 *   other           → NaN
 */
function doAdd(left: VMValue, right: VMValue): VMValue {
  if (typeof left === 'number' && typeof right === 'number') {
    return left + right;
  }
  if (typeof left === 'string' || typeof right === 'string') {
    return String(left) + String(right);
  }
  return NaN;
}

/**
 * Generic binary arithmetic (sub, mul, div, mod, pow).
 * Converts both operands to numbers; returns NaN if either is non-numeric.
 */
function doArith(left: VMValue, right: VMValue, op: (a: number, b: number) => number): VMValue {
  if (typeof left !== 'number' || typeof right !== 'number') return NaN;
  return op(left, right);
}

/**
 * Convert a VMValue to a 32-bit signed integer (for bitwise ops).
 */
function toInt32(v: VMValue): number {
  if (typeof v === 'number') return v | 0;
  if (typeof v === 'boolean') return v ? 1 : 0;
  return 0;
}

// ─────────────────────────────────────────────────────────────────────────────
// Boolean / typeof helpers
// ─────────────────────────────────────────────────────────────────────────────

/**
 * JavaScript-style ToBoolean conversion.
 *
 * Falsy values: false, 0, '', null, undefined, NaN
 * Everything else is truthy (including empty objects, empty arrays).
 */
function toBoolean(v: VMValue): boolean {
  if (v === null || v === undefined || v === false) return false;
  if (typeof v === 'number') return v !== 0 && !isNaN(v);
  if (typeof v === 'string') return v.length > 0;
  return true; // objects, arrays, functions
}

/**
 * VM typeof — returns the JS typeof string for a VMValue.
 * Matches JS behaviour: null → 'object', function → 'function'.
 */
function vmTypeof(v: VMValue): string {
  if (v === null) return 'object';
  if (v === undefined) return 'undefined';
  if (typeof v === 'number') return 'number';
  if (typeof v === 'string') return 'string';
  if (typeof v === 'boolean') return 'boolean';
  if (isVMFunction(v)) return 'function';
  return 'object';
}

// ─────────────────────────────────────────────────────────────────────────────
// Type guards
// ─────────────────────────────────────────────────────────────────────────────

function isVMObject(v: VMValue): v is VMObject {
  return (
    typeof v === 'object' &&
    v !== null &&
    !Array.isArray(v) &&
    'hiddenClassId' in v &&
    'properties' in v
  );
}

function isVMFunction(v: VMValue): v is VMFunction {
  return (
    typeof v === 'object' &&
    v !== null &&
    !Array.isArray(v) &&
    'kind' in v &&
    (v as VMFunction).kind === 'function'
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Error helpers
// ─────────────────────────────────────────────────────────────────────────────

/** Type guard for VMError plain objects (to distinguish from native Error). */
function isVMError(v: unknown): v is VMError {
  return (
    typeof v === 'object' &&
    v !== null &&
    'message' in v &&
    'instructionIndex' in v &&
    'opcode' in v
  );
}

/**
 * Throw a VMError.
 *
 * We throw a plain object (not an `Error` instance) so it can be caught
 * and pattern-matched by the execute() wrapper without losing information
 * through `instanceof` checks.
 */
function throwVMError(message: string, instructionIndex: number, opcode: number): never {
  throw { message, instructionIndex, opcode } satisfies VMError;
}

// Re-export Opcode for convenience so consumers only need to import from vm.ts.
export { Opcode } from './opcodes.js';
export type {
  CallFrame,
  CodeObject,
  Context,
  FeedbackSlot,
  RegisterInstruction,
  TraceStep,
  TypePair,
  VMError,
  VMFunction,
  VMObject,
  VMArray,
  VMResult,
  VMValue,
} from './types.js';
