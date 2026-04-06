/**
 * types.ts — Core type definitions for the Register VM.
 *
 * Every value the VM can hold, every data structure involved in execution,
 * and every observable output is defined here. Keeping types in one place
 * prevents circular imports and makes the shape of the VM legible at a glance.
 *
 * ## Value representation
 *
 * JavaScript has seven primitive types plus objects. This VM models them
 * with a TypeScript union (VMValue). The union deliberately excludes
 * `bigint` and `symbol` for simplicity.
 *
 * ## Hidden classes
 *
 * V8 tracks the "shape" of an object (which properties it has and in what
 * order they were added) using a hidden class (also called a Map in V8 source).
 * When two objects have the same hidden class, the JIT compiler can generate
 * a single optimised property-access path for both — a technique called
 * inline caching (IC).
 *
 * This VM assigns a monotonically-increasing integer hiddenClassId to each
 * new object. When a property is added the id changes (simulating a class
 * transition).  The feedback vector records which class IDs it has seen at
 * each property-access site.
 *
 * ## Feedback vectors
 *
 * Each CodeObject carries a feedbackSlotCount.  At runtime every CallFrame
 * allocates a corresponding FeedbackSlot[].  As instructions execute they
 * record type information into the appropriate slot, following the state
 * machine:
 *
 *   uninitialized → monomorphic(1 type pair)
 *   monomorphic   → polymorphic(≤4 type pairs) when a new pair arrives
 *   polymorphic   → megamorphic when a 5th distinct pair arrives
 *   megamorphic   → megamorphic (terminal state)
 *
 * A JIT compiler would use this information to emit specialised machine
 * code for the monomorphic case and fall back to generic slow paths for
 * the rest.
 */

// ─────────────────────────────────────────────────────────────────────────────
// VMValue — the value domain
// ─────────────────────────────────────────────────────────────────────────────

/**
 * A heap-allocated JavaScript object with a hidden class ID.
 *
 * The hiddenClassId is assigned at creation time and changes whenever a
 * new property is added for the first time, modelling V8's Map transitions.
 */
export interface VMObject {
  /** Monotonically-increasing integer; uniquely identifies the object's shape. */
  readonly hiddenClassId: number;
  /** The mutable property store. */
  properties: Map<string, VMValue>;
}

/**
 * A JavaScript array — modelled as a plain TypeScript array.
 * In a production VM arrays would have a separate ElementsKind, but for
 * this educational implementation a plain array is sufficient.
 */
export type VMArray = VMValue[];

/**
 * A first-class function value created by CREATE_CLOSURE.
 *
 * Every closure captures the lexical context (scope chain) in effect at
 * the moment it is created.  When the closure is later called, that
 * captured context becomes the starting point for variable lookup.
 */
export interface VMFunction {
  readonly kind: 'function';
  /** The compiled bytecode to execute when this function is called. */
  code: CodeObject;
  /** The lexical scope captured at closure-creation time (may be null for top-level). */
  context: Context | null;
}

/**
 * Any value the VM can hold.
 *
 * - number   : IEEE-754 double (same as JS)
 * - string   : immutable UTF-16 text
 * - boolean  : true / false
 * - null     : intentional absence of a value
 * - undefined: uninitialised / missing
 * - VMObject : a keyed property bag with a hidden class
 * - VMArray  : an ordered sequence
 * - VMFunction: a callable closure
 */
export type VMValue =
  | number
  | string
  | boolean
  | null
  | undefined
  | VMObject
  | VMArray
  | VMFunction;

// ─────────────────────────────────────────────────────────────────────────────
// CodeObject — compiled bytecode
// ─────────────────────────────────────────────────────────────────────────────

/**
 * A single decoded instruction.
 *
 * The opcode is a numeric constant from opcodes.ts.
 * Operands carry register indices, constant-pool indices, name-table
 * indices, SMI literals, jump offsets, or feedback-slot numbers —
 * depending on the opcode.
 */
export interface RegisterInstruction {
  /** Opcode number (see Opcode enum in opcodes.ts). */
  opcode: number;
  /**
   * Variable-length operand list.
   * Interpretation depends on the opcode — see individual opcode docs.
   */
  operands: number[];
  /**
   * Index into the frame's feedbackVector, or null if this instruction
   * does not update type feedback.
   */
  feedbackSlot: number | null;
}

/**
 * A compiled unit of code (analogous to V8's SharedFunctionInfo + BytecodeArray).
 *
 * This is an immutable description of what a function does.  Multiple
 * CallFrames can point to the same CodeObject if the function is called
 * recursively.
 *
 * @example
 * ```
 * const code: CodeObject = {
 *   name: 'add',
 *   instructions: [
 *     { opcode: Opcode.LDA_CONSTANT, operands: [0], feedbackSlot: null }, // acc = 42
 *     { opcode: Opcode.RETURN,       operands: [],  feedbackSlot: null },
 *   ],
 *   constants: [42],
 *   names: [],
 *   registerCount: 0,
 *   feedbackSlotCount: 0,
 *   parameterCount: 0,
 * };
 * ```
 */
export interface CodeObject {
  /** Constant pool: strings, numbers, nested CodeObjects, etc. */
  constants: VMValue[];
  /**
   * Name table: property names / global names referenced by this code.
   * LDA_NAMED_PROPERTY and LDA_GLOBAL index into this array.
   */
  names: string[];
  /** The instruction stream. */
  instructions: RegisterInstruction[];
  /** How many local registers this function needs. */
  registerCount: number;
  /** How many feedback slots the feedback vector needs. */
  feedbackSlotCount: number;
  /** How many named parameters the function declares. */
  parameterCount: number;
  /** Debug name (function name or '<anonymous>'). */
  name: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// CallFrame — execution state for one function invocation
// ─────────────────────────────────────────────────────────────────────────────

/**
 * A call frame represents one activation of a function.
 *
 * Think of it as a stack frame in a traditional native call stack, but
 * heap-allocated so closures can outlive the call.
 *
 * Fields that change on every instruction:
 *   ip          — instruction pointer (advances each cycle)
 *   accumulator — implicit first operand and output of most instructions
 *   registers   — named local values
 *   feedbackVector — type profile, updated by IC instructions
 */
export interface CallFrame {
  /** The code being executed in this frame. */
  code: CodeObject;
  /** Current instruction index (indexes into code.instructions). */
  ip: number;
  /** The implicit accumulator register. */
  accumulator: VMValue;
  /** The register file — preallocated to code.registerCount entries. */
  registers: VMValue[];
  /**
   * Per-frame feedback vector.
   * Survives GC because it is tied to the CodeObject not the frame
   * in a production VM; here it lives on the frame for simplicity.
   */
  feedbackVector: FeedbackSlot[];
  /** The lexical context (scope chain) for this invocation. */
  context: Context | null;
  /** The caller's frame, or null for the top-level frame. */
  callerFrame: CallFrame | null;
}

// ─────────────────────────────────────────────────────────────────────────────
// FeedbackSlot — type profile state machine
// ─────────────────────────────────────────────────────────────────────────────

/**
 * A pair of type strings representing the left-hand and right-hand side
 * types seen at a binary operation site.
 *
 * @example ['number', 'number'] means both operands were numbers.
 * @example ['string', 'number'] means a string+number mix was seen.
 */
export type TypePair = [string, string];

/**
 * The state of a single feedback slot.
 *
 * The state machine (from uninitialized to megamorphic) models how V8's
 * inline caches transition as they see more type variety:
 *
 * ```
 * uninitialized  ──(first type pair)──→  monomorphic
 * monomorphic    ──(new type pair)────→  polymorphic
 * polymorphic    ──(4th new pair)─────→  megamorphic
 * megamorphic    ──(any)──────────────→  megamorphic  (terminal)
 * ```
 *
 * The JIT compiler emits fast-path code for monomorphic sites and
 * fallback code for polymorphic/megamorphic ones.
 */
export type FeedbackSlot =
  | { readonly kind: 'uninitialized' }
  | { readonly kind: 'monomorphic'; types: TypePair[] }
  | { readonly kind: 'polymorphic'; types: TypePair[] }
  | { readonly kind: 'megamorphic' };

// ─────────────────────────────────────────────────────────────────────────────
// Context — lexical scope chain
// ─────────────────────────────────────────────────────────────────────────────

/**
 * A context object represents one lexical scope in the scope chain.
 *
 * When a JavaScript function closes over a variable, that variable is
 * "context-allocated" — it moves from the register file into a Context
 * object on the heap. The context object is shared between the outer
 * function and all of its closures.
 *
 * Contexts form a singly-linked list (the scope chain).  To look up a
 * variable at depth `d`, walk `d` parent links and then index into slots.
 *
 * @example
 *   // outer function's context:
 *   { slots: [42], parent: null }      // depth 0: x = 42
 *
 *   // inner closure's context:
 *   { slots: [7],  parent: outerCtx }  // depth 0: y = 7; depth 1: x
 */
export interface Context {
  /** The variable values stored at this scope level. */
  slots: VMValue[];
  /** The enclosing scope, or null at the top of the chain. */
  parent: Context | null;
}

// ─────────────────────────────────────────────────────────────────────────────
// Result types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * A VM runtime error (thrown as an exception, not a JS Error).
 *
 * Wrapping errors in a plain object (rather than using `throw new Error(...)`)
 * lets callers pattern-match on the fields without catching and re-checking.
 */
export interface VMError {
  /** Human-readable description of what went wrong. */
  message: string;
  /** Index of the failing instruction in code.instructions. */
  instructionIndex: number;
  /** The opcode that triggered the error. */
  opcode: number;
}

/**
 * The result of a top-level VM.execute() call.
 *
 * Even on error, partial output is preserved so callers can debug what
 * happened before the fault.
 */
export interface VMResult {
  /** The value in the accumulator when the VM stopped. */
  returnValue: VMValue;
  /** Lines written to the VM's output stream (via a hypothetical print opcode). */
  output: string[];
  /** null on success; set on any runtime error. */
  error: VMError | null;
}

// ─────────────────────────────────────────────────────────────────────────────
// Trace — per-instruction execution record
// ─────────────────────────────────────────────────────────────────────────────

/**
 * A snapshot of the VM state before and after one instruction executes.
 *
 * Collecting a trace allows step-by-step debugging and visualisation.
 * It is intentionally verbose — every observable piece of state is
 * captured so the caller doesn't need to re-run the VM.
 */
export interface TraceStep {
  /** Nesting depth (0 = top-level). */
  frameDepth: number;
  /** Instruction index within code.instructions when this step ran. */
  ip: number;
  /** The instruction that ran. */
  instruction: RegisterInstruction;
  /** Accumulator value BEFORE the instruction. */
  accBefore: VMValue;
  /** Accumulator value AFTER the instruction. */
  accAfter: VMValue;
  /** Snapshot of the register file BEFORE the instruction. */
  registersBefore: VMValue[];
  /** Snapshot of the register file AFTER the instruction. */
  registersAfter: VMValue[];
  /**
   * Feedback slots that changed during this instruction.
   * Empty array if no feedback was recorded.
   */
  feedbackDelta: Array<{ slot: number; before: FeedbackSlot; after: FeedbackSlot }>;
}
