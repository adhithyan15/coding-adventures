/**
 * GenericVM — A Pluggable Stack-Based Bytecode Interpreter.
 *
 * ==========================================================================
 * Chapter 1: Why a "Generic" VM?
 * ==========================================================================
 *
 * The {@link VirtualMachine} in ``vm.ts`` is a concrete interpreter — it has
 * a fixed instruction set (LOAD_CONST, ADD, PRINT, etc.) hardcoded into a
 * giant switch statement. That works great for running programs compiled
 * from one specific language, but what happens when you want to support
 * *multiple* languages, each with different semantics?
 *
 * Consider the problem:
 *
 *   - Python has ``//`` for integer division and ``/`` for float division.
 *   - Ruby has ``puts`` (adds newline) vs ``print`` (no newline).
 *   - A custom language might have matrix opcodes or concurrency primitives.
 *
 * If we hardcode every opcode into one giant switch, we end up with a
 * monolithic "God VM" that knows about every language — a maintenance
 * nightmare that violates the Open/Closed Principle.
 *
 * The **GenericVM** solves this with the **Strategy Pattern**: instead of
 * hardcoding opcode handlers, it lets languages *register* their own
 * handlers at runtime. Each handler is a function that receives the VM
 * instance, the current instruction, and the code object, and performs
 * whatever operation that opcode means in its language.
 *
 * ```
 *   ┌──────────────────────────────────────────────────┐
 *   │                   GenericVM                       │
 *   │                                                   │
 *   │   handlers: Map<opcode, OpcodeHandler>           │
 *   │   builtins: Map<name, BuiltinFunction>           │
 *   │                                                   │
 *   │   ┌────────────┐  ┌────────────┐                 │
 *   │   │  stack[]   │  │ variables  │                 │
 *   │   └────────────┘  └────────────┘                 │
 *   │                                                   │
 *   │   execute(code) → fetch → lookup → dispatch      │
 *   └──────────────────────────────────────────────────┘
 *          ▲                    ▲
 *          │                    │
 *   registerOpcode(0x01, ...)  registerBuiltin("print", ...)
 *          │                    │
 *   ┌──────┴──────┐     ┌──────┴──────┐
 *   │  Python     │     │   Ruby      │
 *   │  opcodes    │     │  builtins   │
 *   └─────────────┘     └─────────────┘
 * ```
 *
 * This is the same pattern used by real VMs:
 *
 * - The **JVM** has a fixed bytecode set but supports "native methods"
 *   registered via JNI — external handlers for operations the bytecode
 *   can't express.
 *
 * - **.NET's CLR** uses "PInvoke" and "internal calls" to extend the
 *   base instruction set with platform-specific operations.
 *
 * - **WebAssembly** has "host functions" — handlers registered by the
 *   embedding environment (the browser or Node.js) that wasm modules
 *   can call.
 *
 * Our GenericVM is a simplified version of the same idea: the core VM
 * provides the execution engine (stack, PC, eval loop), and languages
 * plug in their specific behavior through registered handlers.
 *
 * ==========================================================================
 * Chapter 2: The Handler Protocol
 * ==========================================================================
 *
 * An {@link OpcodeHandler} is a function with this signature:
 *
 *     (vm: GenericVM, instruction: Instruction, code: CodeObject) => string | null
 *
 * The parameters are:
 *
 * - **vm** — The GenericVM instance. The handler uses ``vm.push()``,
 *   ``vm.pop()``, ``vm.advancePc()``, etc. to manipulate VM state.
 *
 * - **instruction** — The current instruction being executed, including
 *   its opcode and optional operand.
 *
 * - **code** — The full CodeObject, so the handler can access the
 *   constants pool, names pool, or other instructions if needed.
 *
 * The return value is a **description string** (for trace recording) or
 * ``null`` if no description is needed. This follows the same pattern as
 * the VMTrace descriptions in the base VM.
 *
 * **Important contract:** The handler is responsible for advancing the
 * program counter. If the handler doesn't call ``vm.advancePc()`` or
 * ``vm.jumpTo()``, the PC stays where it is, and the VM will re-execute
 * the same instruction forever (an infinite loop). This gives handlers
 * full control over control flow.
 *
 * ==========================================================================
 * Chapter 3: Built-in Functions
 * ==========================================================================
 *
 * Beyond opcodes, languages often need "built-in functions" — functions
 * like ``print()``, ``len()``, or ``type()`` that are implemented in the
 * host language (TypeScript) rather than in bytecode.
 *
 * The GenericVM supports this through {@link BuiltinFunction} registration.
 * A language compiler can emit a CALL instruction targeting a built-in
 * name, and the corresponding opcode handler can look up the built-in
 * via ``vm.getBuiltin()`` and invoke it.
 *
 * This mirrors how Python's ``builtins`` module works — ``print``,
 * ``len``, ``int``, etc. are all "built-in functions" implemented in C
 * that the bytecode interpreter dispatches to when it encounters a CALL
 * instruction targeting those names.
 *
 * ==========================================================================
 * Chapter 4: Safety Features
 * ==========================================================================
 *
 * The GenericVM includes two safety mechanisms:
 *
 * **Max Recursion Depth:**
 * Every function call pushes a frame onto the call stack. Without a limit,
 * infinite recursion would consume all available memory and crash the host
 * process. The ``maxRecursionDepth`` setting caps how deep the call stack
 * can go, mirroring Python's ``sys.setrecursionlimit()`` and the JVM's
 * ``-Xss`` stack size flag.
 *
 * **Frozen State:**
 * Sometimes you want to "freeze" a VM — prevent any further execution.
 * This is useful for debugging (freeze after hitting a breakpoint) or
 * for sandboxing (freeze a VM that has been running too long). The
 * ``frozen`` flag causes ``execute()`` to immediately return an empty
 * trace list.
 *
 * @module
 */

import type { VMValue, Instruction, CodeObject, VMTrace } from "./vm.js";
import { VMError, StackUnderflowError, InvalidOpcodeError } from "./vm.js";

// =========================================================================
// Typed Value Support
// =========================================================================

/**
 * A value with explicit type metadata — for typed VMs.
 *
 * ==========================================================================
 * Why Typed Values?
 * ==========================================================================
 *
 * The base GenericVM is dynamically typed — its stack holds plain {@link VMValue}
 * items (numbers, strings, bigints, etc.) with no type tags. This works great for
 * languages like Python or Starlark where types are discovered at runtime.
 *
 * But some VMs have **typed operand stacks**:
 *
 * - **WebAssembly**: Every stack value is explicitly typed as i32, i64, f32, or f64.
 *   The ``select`` instruction needs to know types; ``call_indirect`` does runtime
 *   type checking. Without type tags, the VM can't distinguish an i32 from an f32
 *   (both are JavaScript ``number``).
 *
 * - **JVM**: The operand stack tracks "computational types" — int, long, float,
 *   double, and reference. ``iadd`` expects two ints; ``dadd`` expects two doubles.
 *   Long and double values occupy two stack slots.
 *
 * - **CLR (.NET)**: The evaluation stack is typed with CIL types (int32, int64,
 *   native int, F, O). The verifier ensures type safety at load time.
 *
 * {@link TypedVMValue} adds a numeric ``type`` tag alongside the raw value. The
 * tag's meaning is language-specific — for WASM, it's ValueType (0x7F=i32, 0x7E=i64,
 * 0x7D=f32, 0x7C=f64). For the JVM, it could be T_INT=0, T_LONG=1, etc.
 *
 * The GenericVM provides a parallel ``typedStack`` alongside the untyped ``stack``.
 * Languages that need typing use the typed stack; languages that don't (Starlark,
 * Lisp) continue using the untyped stack. Both coexist without interference.
 */
export interface TypedVMValue {
  readonly type: number;
  readonly value: VMValue;
}

// =========================================================================
// Instruction Hooks
// =========================================================================

/**
 * A hook that runs **before** each instruction is dispatched to its handler.
 *
 * ==========================================================================
 * Why Pre-Instruction Hooks?
 * ==========================================================================
 *
 * Different bytecode formats have different instruction encodings:
 *
 * - Our generic CodeObject uses **fixed-format** instructions — each instruction
 *   is an object with an opcode and an optional operand. The PC is an instruction
 *   index. This works for languages that compile to our bytecode format.
 *
 * - **WebAssembly** uses **variable-length** bytecodes — opcodes are 1 byte, but
 *   immediates are LEB128-encoded (variable length). The PC is a byte offset.
 *   A ``block`` instruction has a 1-byte blocktype immediate; ``br_table`` has
 *   a variable-length vector of label indices.
 *
 * - **JVM** bytecodes are similar — most instructions are 1-3 bytes, but
 *   ``tableswitch`` and ``lookupswitch`` have variable-length padding and tables.
 *
 * The pre-instruction hook bridges this gap: it receives the raw instruction from
 * the CodeObject and can **transform** it before the opcode handler sees it.
 * For WASM, the hook decodes variable-length bytecodes from a raw byte array and
 * returns a fixed-format Instruction with decoded operands.
 *
 * This keeps the GenericVM's eval loop simple (fetch → hook → dispatch → trace)
 * while supporting any instruction encoding.
 *
 * @param vm          - The GenericVM instance.
 * @param instruction - The raw instruction fetched from the CodeObject.
 * @param code        - The full CodeObject.
 * @returns           A (possibly transformed) instruction to dispatch.
 */
export type PreInstructionHook = (
  vm: GenericVM,
  instruction: Instruction,
  code: CodeObject,
) => Instruction;

/**
 * A hook that runs **after** each instruction handler completes.
 *
 * Post-hooks are useful for:
 *
 * - **Tracing/profiling**: Log instruction execution for debugging.
 * - **Assertions**: Verify stack height invariants during development.
 * - **Coverage**: Track which instructions were executed.
 * - **Breakpoints**: Check if a breakpoint condition was met and freeze the VM.
 *
 * @param vm          - The GenericVM instance (state already modified by handler).
 * @param instruction - The instruction that was just executed.
 * @param code        - The full CodeObject.
 */
export type PostInstructionHook = (
  vm: GenericVM,
  instruction: Instruction,
  code: CodeObject,
) => void;

// =========================================================================
// Context-Aware Handler
// =========================================================================

/**
 * An opcode handler that receives an additional context object.
 *
 * ==========================================================================
 * Why Context-Aware Handlers?
 * ==========================================================================
 *
 * Some VMs need per-execution state beyond the stack and variables:
 *
 * - **WASM**: Each execution has a linear memory, function tables, typed globals,
 *   and a label stack for structured control flow. These must be accessible from
 *   every instruction handler.
 *
 * - **JVM**: Each frame has a constant pool, exception handler table, and the
 *   class hierarchy for method resolution.
 *
 * - **CLR**: Each method has a metadata token space for type resolution.
 *
 * Rather than storing this state on the GenericVM (which is generic), we pass
 * it through as a typed context parameter. The handler's type parameter ``TContext``
 * specifies what shape the context takes.
 *
 * This follows the same pattern as React's ``useContext`` or Go's ``context.Context``
 * — threading execution-scoped state without global variables.
 *
 * @typeParam TContext - The type of the execution context (e.g., WasmExecutionContext).
 */
export type ContextOpcodeHandler<TContext = unknown> = (
  vm: GenericVM,
  instruction: Instruction,
  code: CodeObject,
  context: TContext,
) => string | null;

// =========================================================================
// Types
// =========================================================================

/**
 * A function that handles one specific opcode.
 *
 * When the VM's eval loop encounters an instruction with a given opcode,
 * it looks up the registered handler and calls it. The handler is
 * responsible for:
 *
 * 1. Reading any operands from the instruction.
 * 2. Manipulating VM state (stack, variables, PC, etc.).
 * 3. Advancing the program counter (via ``vm.advancePc()`` or ``vm.jumpTo()``).
 * 4. Returning a human-readable description of what happened, or ``null``.
 *
 * **Why return a string?**
 * The description is captured in the {@link VMTrace} for that step, which
 * powers debugging and visualization tools. If you return ``null``, a
 * default description like "Executed opcode 0x01" will be used.
 *
 * @param vm          - The GenericVM instance (use its methods to manipulate state).
 * @param instruction - The instruction being executed (includes opcode + operand).
 * @param code        - The full CodeObject (for accessing constants/names pools).
 * @returns           A description string for the trace, or null.
 */
export type OpcodeHandler = (
  vm: GenericVM,
  instruction: Instruction,
  code: CodeObject,
) => string | null;

/**
 * A host-implemented function that bytecode can call.
 *
 * Built-in functions are the bridge between the VM world and the host
 * language. When bytecode calls ``print("hello")``, the CALL opcode
 * handler looks up ``"print"`` in the builtins registry and invokes
 * its implementation.
 *
 * This is how every major VM works:
 *
 * - **CPython**: ``print``, ``len``, ``type`` are C functions registered
 *   in the ``builtins`` module.
 * - **JVM**: ``System.out.println`` delegates to a native method.
 * - **V8 (Node.js)**: ``console.log`` is a C++ function bound into JS.
 *
 * @example
 * ```typescript
 * vm.registerBuiltin("len", (...args: VMValue[]) => {
 *   const value = args[0];
 *   if (typeof value === "string") return value.length;
 *   throw new VMTypeError(`len() requires a string, got ${typeof value}`);
 * });
 * ```
 */
export interface BuiltinFunction {
  /** The name of the built-in (e.g., "print", "len", "type"). */
  readonly name: string;

  /**
   * The TypeScript implementation.
   *
   * Receives zero or more VMValues (popped from the stack by the CALL
   * handler) and returns a VMValue to be pushed back onto the stack.
   */
  readonly implementation: (...args: VMValue[]) => VMValue;
}

// =========================================================================
// Error Classes
// =========================================================================

/**
 * Raised when a recursive call exceeds the configured maximum depth.
 *
 * This is the GenericVM's equivalent of:
 *
 * - Python's ``RecursionError: maximum recursion depth exceeded``
 * - Java's ``StackOverflowError``
 * - Ruby's ``SystemStackError: stack level too deep``
 *
 * Without this limit, a program like:
 *
 *     def f(): f()
 *     f()
 *
 * would consume all available memory, eventually crashing the host
 * process. The limit provides a clean error instead of an OOM kill.
 */
export class MaxRecursionError extends VMError {
  constructor(message: string = "Maximum recursion depth exceeded") {
    super(message);
    this.name = "MaxRecursionError";
  }
}

/**
 * Raised when an operation encounters a type mismatch at runtime.
 *
 * This is the GenericVM's equivalent of:
 *
 * - Python's ``TypeError``
 * - Java's ``ClassCastException``
 * - Ruby's ``TypeError``
 *
 * Since our VM is dynamically typed, type errors are detected at
 * runtime rather than compile time. For example, trying to add
 * a string and a CodeObject would trigger this error.
 */
export class VMTypeError extends VMError {
  constructor(message: string = "Type error") {
    super(message);
    this.name = "VMTypeError";
  }
}

// =========================================================================
// The Generic VM
// =========================================================================

/**
 * A pluggable stack-based bytecode interpreter.
 *
 * Unlike the {@link VirtualMachine} in ``vm.ts`` which has a fixed
 * instruction set, the GenericVM starts with *no* opcodes at all.
 * Languages register their opcodes via {@link registerOpcode}, and the
 * eval loop dispatches to the registered handler.
 *
 * This makes the GenericVM a **framework** rather than a complete VM —
 * it provides the execution infrastructure (stack, PC, call stack,
 * eval loop), and languages provide the instruction semantics.
 *
 * **Analogy:** Think of the GenericVM as an empty game console. The
 * console has a CPU, memory, and a screen, but no games. Each game
 * cartridge (language) plugs in and tells the console how to interpret
 * its instructions. The console's job is just to fetch instructions
 * and dispatch them to whatever cartridge is plugged in.
 *
 * **State Management:**
 *
 * The VM maintains several pieces of state:
 *
 * ```
 * ┌─────────────────────────────────────────────────┐
 * │                    GenericVM                      │
 * ├─────────────┬──────────────────────────────────-─┤
 * │ stack       │ [10, 20, 30]      ← top is right  │
 * │ variables   │ { x: 42, y: "hi" }                │
 * │ locals      │ [100, 200]        ← indexed slots  │
 * │ pc          │ 5                  ← next instr     │
 * │ halted      │ false                               │
 * │ output      │ ["Hello", "World"]                  │
 * │ callStack   │ [{ returnAddr: 3, ... }]            │
 * ├─────────────┼─────────────────────────────────-──┤
 * │ handlers    │ Map { 0x01 → loadConst, ... }      │
 * │ builtins    │ Map { "print" → { ... }, ... }     │
 * │ maxDepth    │ 1000 | null                         │
 * │ frozen      │ false                               │
 * └─────────────┴───────────────────────────────-────┘
 * ```
 *
 * **Usage:**
 *
 * ```typescript
 * const vm = new GenericVM();
 *
 * // Register opcodes for a language:
 * vm.registerOpcode(0x01, (vm, instr, code) => {
 *   const value = code.constants[instr.operand as number];
 *   vm.push(value);
 *   vm.advancePc();
 *   return `Loaded constant ${value}`;
 * });
 *
 * vm.registerOpcode(0xFF, (vm) => {
 *   vm.halted = true;
 *   return "Halted";
 * });
 *
 * // Execute some bytecode:
 * const code: CodeObject = {
 *   instructions: [
 *     { opcode: 0x01, operand: 0 },
 *     { opcode: 0xFF },
 *   ],
 *   constants: [42],
 *   names: [],
 * };
 *
 * const traces = vm.execute(code);
 * console.log(vm.stack);  // [42]
 * ```
 */
export class GenericVM {
  // -- Public state --------------------------------------------------------
  // These are intentionally public so opcode handlers can read and write
  // them directly. This mirrors how the base VirtualMachine exposes its
  // state, and is necessary because handlers need full access to manipulate
  // the VM's internals.

  /**
   * The operand stack — where all computation happens.
   *
   * Values are pushed onto the right end (top) and popped from the right
   * end. ``stack[stack.length - 1]`` is the top of stack.
   *
   * Example after executing ``PUSH 10; PUSH 20; ADD``:
   *     Before ADD: [10, 20]
   *     After ADD:  [30]
   */
  stack: VMValue[] = [];

  /**
   * Named variable storage — the "global scope."
   *
   * Variables are stored by name and can hold any VMValue.
   * STORE_NAME writes here; LOAD_NAME reads from here.
   */
  variables: Record<string, VMValue> = {};

  /**
   * Indexed local variable slots — the "function scope."
   *
   * Locals are faster than named variables because they use array
   * indexing instead of hash table lookups. This is how real VMs
   * handle local variables — the JVM's ``iload`` and ``istore``
   * instructions use slot indices, not names.
   */
  locals: VMValue[] = [];

  /**
   * The program counter — points to the next instruction to execute.
   *
   * Starts at 0 (the first instruction). Advances by 1 after most
   * instructions, or jumps to a specific target for control flow.
   */
  pc: number = 0;

  /**
   * Whether the VM has been halted.
   *
   * Set to ``true`` by a HALT opcode handler. Once halted, the eval
   * loop stops and ``execute()`` returns the collected traces.
   */
  halted: boolean = false;

  /**
   * Captured output from PRINT-like operations.
   *
   * Instead of writing directly to stdout, handlers push strings here.
   * This makes testing and visualization much easier — you can inspect
   * exactly what the program printed without capturing stdout.
   */
  output: string[] = [];

  /**
   * The call stack — saved execution contexts for function calls.
   *
   * Each entry is a record (dictionary) containing whatever state the
   * language's CALL/RETURN handlers need to save and restore. The
   * GenericVM doesn't prescribe a specific frame shape — that's up
   * to the language that registers the opcode handlers.
   *
   * We use ``Record<string, unknown>`` instead of a fixed interface
   * because different languages may need different frame layouts.
   * Python might save ``locals`` and ``globals``; Ruby might save
   * ``self`` and ``binding``.
   */
  callStack: Record<string, unknown>[] = [];

  /**
   * The typed operand stack — for VMs that track value types.
   *
   * This is a parallel stack to ``stack`` above. Languages with typed
   * operand stacks (WASM, JVM, CLR) use ``pushTyped``/``popTyped`` instead
   * of the untyped ``push``/``pop``. Each entry carries both a value and
   * a numeric type tag whose meaning is defined by the language.
   *
   * Languages that don't need typing (Starlark, Lisp) ignore this entirely.
   *
   * ```
   * // WASM example:
   * typedStack: [
   *   { type: 0x7F, value: 42 },    // i32
   *   { type: 0x7E, value: 7n },    // i64 (BigInt)
   *   { type: 0x7D, value: 3.14 },  // f32
   * ]
   * ```
   */
  typedStack: TypedVMValue[] = [];

  /**
   * Optional execution context — language-specific state passed to handlers.
   *
   * This is set via {@link executeWithContext} and accessed by
   * {@link ContextOpcodeHandler} handlers. Contains whatever extra state
   * the language needs (WASM: memory/tables/globals; JVM: constant pool; etc.)
   */
  executionContext: unknown = null;

  // -- Private state -------------------------------------------------------
  // These are internal to the GenericVM and not directly accessible
  // by opcode handlers (they use the public API methods instead).

  /**
   * The opcode dispatch table — maps opcode numbers to handler functions.
   *
   * This is the heart of the pluggable architecture. When the eval loop
   * encounters opcode 0x01, it looks up ``handlers.get(0x01)`` and calls
   * the result. If no handler is registered, it throws InvalidOpcodeError.
   *
   * Using a Map gives us O(1) lookup, which is important because this
   * lookup happens on *every single instruction* — it's the hottest path
   * in the entire VM.
   */
  private handlers: Map<number, OpcodeHandler> = new Map();

  /**
   * Registry of host-implemented functions callable from bytecode.
   *
   * Opcode handlers can look up builtins by name via ``getBuiltin()``.
   * This decouples the "call" mechanism (an opcode) from the "what to
   * call" mechanism (a named builtin).
   */
  private builtins: Map<string, BuiltinFunction> = new Map();

  /**
   * Maximum allowed call stack depth, or ``null`` for unlimited.
   *
   * When set to a number, ``pushFrame()`` will throw MaxRecursionError
   * if the call stack would exceed this depth. This prevents infinite
   * recursion from crashing the host process.
   *
   * Default is ``null`` (unlimited), mirroring how the JVM defaults to
   * a generous stack size. Languages can set this to a sensible limit
   * (Python defaults to 1000, for example).
   */
  private maxRecursionDepth: number | null = null;

  /**
   * Whether the VM is frozen (execution disabled).
   *
   * When frozen, ``execute()`` immediately returns an empty array and
   * ``step()`` returns a no-op trace. This is useful for:
   *
   * - Pausing execution at a breakpoint.
   * - Preventing a runaway VM from consuming more resources.
   * - Testing that the freeze mechanism works correctly.
   */
  private frozen: boolean = false;

  /**
   * Optional hook that transforms instructions before dispatch.
   *
   * When set, the eval loop calls this function after fetching the raw
   * instruction but before looking up the opcode handler. The hook can
   * modify or replace the instruction — for example, decoding WASM's
   * variable-length bytecodes into fixed-format Instruction objects.
   */
  private preHook: PreInstructionHook | null = null;

  /**
   * Optional hook that runs after each instruction handler completes.
   *
   * When set, the eval loop calls this function after the opcode handler
   * returns, before building the trace. Useful for tracing, assertions,
   * or profiling.
   */
  private postHook: PostInstructionHook | null = null;

  /**
   * Map of context-aware opcode handlers.
   *
   * These handlers receive an additional context parameter alongside
   * the standard (vm, instruction, code) arguments. Used by typed VMs
   * that need per-execution state (WASM: memory/tables; JVM: constant pool).
   */
  private contextHandlers: Map<number, ContextOpcodeHandler> = new Map();

  // =======================================================================
  // Opcode & Builtin Registration
  // =======================================================================

  /**
   * Register a handler for a specific opcode.
   *
   * This is the primary extension point for the GenericVM. A language
   * implementation calls this once per opcode during setup, then the
   * eval loop dispatches to the registered handler whenever it encounters
   * that opcode.
   *
   * **Example: Registering a LOAD_CONST handler**
   *
   * ```typescript
   * vm.registerOpcode(0x01, (vm, instr, code) => {
   *   // Read the constant from the pool using the instruction's operand
   *   const index = instr.operand as number;
   *   const value = code.constants[index];
   *
   *   // Push it onto the stack
   *   vm.push(value);
   *
   *   // Advance past this instruction
   *   vm.advancePc();
   *
   *   // Return a description for the trace
   *   return `Loaded constant ${value}`;
   * });
   * ```
   *
   * **Overwriting handlers:** If a handler for the same opcode is already
   * registered, it will be silently replaced. This allows languages to
   * override default handlers with specialized ones.
   *
   * @param opcode  - The numeric opcode value (0x00–0xFF).
   * @param handler - The function to call when this opcode is encountered.
   */
  registerOpcode(opcode: number, handler: OpcodeHandler): void {
    this.handlers.set(opcode, handler);
  }

  /**
   * Register a built-in function callable from bytecode.
   *
   * Built-in functions are host-implemented (TypeScript) functions that
   * bytecode can invoke by name. The language's CALL opcode handler is
   * responsible for looking up the builtin and invoking it.
   *
   * @param name - The function name (e.g., "print", "len").
   * @param impl - The TypeScript implementation.
   */
  registerBuiltin(
    name: string,
    impl: (...args: VMValue[]) => VMValue,
  ): void {
    this.builtins.set(name, { name, implementation: impl });
  }

  /**
   * Look up a registered built-in function by name.
   *
   * Returns ``undefined`` if no builtin with that name is registered.
   * Opcode handlers should check for this and throw an appropriate error.
   *
   * @param name - The function name to look up.
   * @returns    The BuiltinFunction record, or undefined.
   */
  getBuiltin(name: string): BuiltinFunction | undefined {
    return this.builtins.get(name);
  }

  // =======================================================================
  // Global Injection
  // =======================================================================

  /**
   * Pre-seed named variables into the VM's global scope.
   *
   * These variables are available to the program as regular variables
   * but are set before execution begins. Useful for build context,
   * environment info, etc.
   *
   * Injected globals are merged into ``variables`` — they don't replace
   * the object. If a key already exists, the injected value overwrites it.
   *
   * @param globals - A record of variable names to values.
   *
   * @example
   * ```typescript
   * vm.injectGlobals({ _ctx: { os: "darwin", arch: "arm64" } });
   * ```
   */
  injectGlobals(globals: Record<string, VMValue>): void {
    for (const [key, value] of Object.entries(globals)) {
      this.variables[key] = value;
    }
  }

  // =======================================================================
  // Stack Operations
  // =======================================================================

  /**
   * Push a value onto the operand stack.
   *
   * This is the most fundamental VM operation — every computation starts
   * by pushing operands onto the stack.
   *
   * ```
   *   Before push(42):   [10, 20]
   *   After push(42):    [10, 20, 42]
   *                              ↑ top
   * ```
   *
   * @param value - The value to push (number, string, CodeObject, or null).
   */
  push(value: VMValue): void {
    this.stack.push(value);
  }

  /**
   * Pop and return the top value from the operand stack.
   *
   * Throws {@link StackUnderflowError} if the stack is empty — this
   * indicates a compiler bug (the bytecode tried to pop a value that
   * was never pushed).
   *
   * ```
   *   Before pop():   [10, 20, 42]
   *   After pop():    [10, 20]       → returns 42
   * ```
   *
   * @returns The value that was on top of the stack.
   * @throws  {StackUnderflowError} If the stack is empty.
   */
  pop(): VMValue {
    if (this.stack.length === 0) {
      throw new StackUnderflowError(
        "Cannot pop from an empty stack — possible compiler bug",
      );
    }
    return this.stack.pop()!;
  }

  /**
   * Peek at the top value without removing it.
   *
   * Useful when you need to inspect the top of stack without consuming
   * it — for example, to check its type before deciding what to do.
   *
   * ```
   *   Stack: [10, 20, 42]
   *   peek() → 42
   *   Stack: [10, 20, 42]   ← unchanged
   * ```
   *
   * @returns The value on top of the stack.
   * @throws  {StackUnderflowError} If the stack is empty.
   */
  peek(): VMValue {
    if (this.stack.length === 0) {
      throw new StackUnderflowError(
        "Cannot peek at an empty stack — possible compiler bug",
      );
    }
    return this.stack[this.stack.length - 1];
  }

  // =======================================================================
  // Typed Stack Operations
  // =======================================================================

  /**
   * Push a typed value onto the typed operand stack.
   *
   * Used by typed VMs (WASM, JVM, CLR) that need to track value types
   * alongside values. Each entry carries both a type tag and a raw value.
   *
   * ```
   *   Before pushTyped({ type: 0x7F, value: 42 }):
   *     typedStack: [{ type: 0x7E, value: 7n }]
   *   After:
   *     typedStack: [{ type: 0x7E, value: 7n }, { type: 0x7F, value: 42 }]
   * ```
   *
   * @param value - The typed value to push.
   */
  pushTyped(value: TypedVMValue): void {
    this.typedStack.push(value);
  }

  /**
   * Pop and return the top typed value from the typed stack.
   *
   * @returns The typed value that was on top.
   * @throws  {StackUnderflowError} If the typed stack is empty.
   */
  popTyped(): TypedVMValue {
    if (this.typedStack.length === 0) {
      throw new StackUnderflowError(
        "Cannot pop from an empty typed stack — possible compiler bug",
      );
    }
    return this.typedStack.pop()!;
  }

  /**
   * Peek at the top typed value without removing it.
   *
   * @returns The typed value on top.
   * @throws  {StackUnderflowError} If the typed stack is empty.
   */
  peekTyped(): TypedVMValue {
    if (this.typedStack.length === 0) {
      throw new StackUnderflowError(
        "Cannot peek at an empty typed stack — possible compiler bug",
      );
    }
    return this.typedStack[this.typedStack.length - 1];
  }

  // =======================================================================
  // Call Stack Operations
  // =======================================================================

  /**
   * Push a new frame onto the call stack.
   *
   * This is called by CALL opcode handlers to save the current execution
   * context before entering a function. The frame can contain any data
   * the language needs to restore later (return address, saved variables,
   * etc.).
   *
   * If ``maxRecursionDepth`` is set and the call stack would exceed it,
   * this throws {@link MaxRecursionError} — preventing infinite recursion
   * from consuming all memory.
   *
   * ```
   *   callStack before:  [frame0, frame1]
   *   pushFrame(frame2)
   *   callStack after:   [frame0, frame1, frame2]
   * ```
   *
   * @param frame - The execution context to save.
   * @throws {MaxRecursionError} If the call stack depth would exceed maxRecursionDepth.
   */
  pushFrame(frame: Record<string, unknown>): void {
    if (
      this.maxRecursionDepth !== null &&
      this.callStack.length >= this.maxRecursionDepth
    ) {
      throw new MaxRecursionError(
        `Maximum recursion depth of ${this.maxRecursionDepth} exceeded`,
      );
    }
    this.callStack.push(frame);
  }

  /**
   * Pop and return the top frame from the call stack.
   *
   * This is called by RETURN opcode handlers to restore the caller's
   * execution context. Throws VMError if the call stack is empty —
   * this means a RETURN instruction was executed without a matching CALL.
   *
   * @returns The saved execution context.
   * @throws  {VMError} If the call stack is empty.
   */
  popFrame(): Record<string, unknown> {
    if (this.callStack.length === 0) {
      throw new VMError(
        "Cannot pop from an empty call stack — RETURN without matching CALL",
      );
    }
    return this.callStack.pop()!;
  }

  // =======================================================================
  // Program Counter Control
  // =======================================================================

  /**
   * Advance the program counter to the next instruction.
   *
   * This is the "normal" flow — after executing an instruction, move
   * to the one right after it. Most opcode handlers call this at the
   * end unless they're implementing control flow (jumps, calls, returns).
   *
   * ```
   *   pc before:  3
   *   advancePc()
   *   pc after:   4
   * ```
   */
  advancePc(): void {
    this.pc += 1;
  }

  /**
   * Jump the program counter to a specific instruction index.
   *
   * This is used by control flow opcodes (JUMP, JUMP_IF_TRUE, etc.)
   * to redirect execution to a different part of the program.
   *
   * ```
   *   pc before:    3
   *   jumpTo(10)
   *   pc after:     10
   * ```
   *
   * **No bounds checking:** The target is not validated here — if it's
   * out of bounds, the eval loop will naturally stop (pc >= length)
   * or the next fetch will fail. This keeps the hot path fast.
   *
   * @param target - The instruction index to jump to.
   */
  jumpTo(target: number): void {
    this.pc = target;
  }

  // =======================================================================
  // Configuration
  // =======================================================================

  /**
   * Set the maximum allowed call stack depth.
   *
   * Pass a number to limit recursion depth, or ``null`` for unlimited.
   *
   * **Common values:**
   * - Python default: 1000
   * - Ruby default: ~10,000 (varies by implementation)
   * - JVM: depends on ``-Xss`` flag (default ~512 frames for deep stacks)
   *
   * @param depth - The maximum depth, or null for unlimited.
   */
  setMaxRecursionDepth(depth: number | null): void {
    this.maxRecursionDepth = depth;
  }

  /**
   * Get the current maximum recursion depth setting.
   *
   * @returns The maximum depth, or null if unlimited.
   */
  getMaxRecursionDepth(): number | null {
    return this.maxRecursionDepth;
  }

  /**
   * Freeze or unfreeze the VM.
   *
   * A frozen VM's ``execute()`` returns immediately with no traces,
   * and ``step()`` returns a no-op trace without executing anything.
   *
   * @param frozen - ``true`` to freeze, ``false`` to unfreeze.
   */
  setFrozen(frozen: boolean): void {
    this.frozen = frozen;
  }

  /**
   * Check whether the VM is currently frozen.
   *
   * @returns ``true`` if frozen, ``false`` otherwise.
   */
  isFrozen(): boolean {
    return this.frozen;
  }

  // =======================================================================
  // Execution Engine
  // =======================================================================

  /**
   * Execute a CodeObject from start to finish, collecting traces.
   *
   * This is the main entry point for running a program. It implements
   * the classic **fetch-decode-execute** cycle:
   *
   * ```
   *   while not halted and pc < instructions.length:
   *       trace = step(code)     // fetch + decode + execute one instruction
   *       traces.push(trace)     // record what happened
   *   return traces
   * ```
   *
   * The loop terminates when:
   *
   * 1. A HALT handler sets ``this.halted = true``.
   * 2. The PC advances past the last instruction (natural end of program).
   * 3. The VM is frozen (returns immediately with empty traces).
   *
   * **Thread safety:** This VM is single-threaded and not reentrant.
   * Don't call ``execute()`` from within an opcode handler — that would
   * create a confusing nested execution. Use the call stack mechanism
   * instead.
   *
   * @param code - The compiled bytecode to execute.
   * @returns    An array of VMTrace snapshots, one per executed instruction.
   */
  execute(code: CodeObject): VMTrace[] {
    // If the VM is frozen, bail out immediately.
    if (this.frozen) {
      return [];
    }

    const traces: VMTrace[] = [];

    // The fetch-decode-execute loop. This is the beating heart of the VM.
    // It runs until the program halts or we run out of instructions.
    while (!this.halted && this.pc < code.instructions.length) {
      const trace = this.step(code);
      traces.push(trace);
    }

    return traces;
  }

  /**
   * Execute a CodeObject with an execution context.
   *
   * This variant passes a context object to all context-aware opcode
   * handlers (registered via {@link registerContextOpcode}). The context
   * is available for the duration of execution and cleared afterward.
   *
   * This is how typed VMs thread per-execution state through the eval
   * loop without storing it on the GenericVM:
   *
   * ```typescript
   * // WASM execution:
   * const wasmContext = {
   *   memory: linearMemory,
   *   tables: [funcTable],
   *   globals: [{ type: 0x7F, value: 0 }],
   *   labelStack: [],
   * };
   *
   * const traces = vm.executeWithContext(code, wasmContext);
   * ```
   *
   * @typeParam TContext - The type of the execution context.
   * @param code    - The compiled bytecode to execute.
   * @param context - The execution context to pass to context-aware handlers.
   * @returns       An array of VMTrace snapshots.
   */
  executeWithContext<TContext>(code: CodeObject, context: TContext): VMTrace[] {
    const previousContext = this.executionContext;
    this.executionContext = context;
    try {
      return this.execute(code);
    } finally {
      this.executionContext = previousContext;
    }
  }

  /**
   * Execute a single instruction and return its trace.
   *
   * This is the workhorse method that implements one cycle of
   * fetch-decode-execute:
   *
   * 1. **Snapshot** — Capture the stack state *before* execution.
   * 2. **Fetch** — Read the instruction at the current PC.
   * 3. **Decode** — Look up the handler for this opcode.
   * 4. **Execute** — Call the handler, which manipulates VM state.
   * 5. **Record** — Build a VMTrace capturing what happened.
   *
   * If no handler is registered for the opcode, this throws
   * {@link InvalidOpcodeError} — the bytecode contains an instruction
   * the VM doesn't know how to execute.
   *
   * @param code - The CodeObject containing the instruction stream.
   * @returns    A VMTrace snapshot of this execution step.
   * @throws    {InvalidOpcodeError} If the opcode has no registered handler.
   */
  step(code: CodeObject): VMTrace {
    // ── Step 1: Snapshot the "before" state ─────────────────────────
    // We need to capture the stack *before* the handler modifies it,
    // so the trace can show the transformation.
    const pcBefore = this.pc;
    const stackBefore = [...this.stack];

    // ── Step 2: Fetch the instruction ───────────────────────────────
    let instruction = code.instructions[this.pc];

    // ── Step 2.5: Pre-instruction hook ──────────────────────────────
    // If a pre-hook is registered, give it a chance to transform the
    // instruction before dispatch. This is how WASM decodes variable-
    // length bytecodes into fixed-format Instruction objects.
    if (this.preHook) {
      instruction = this.preHook(this, instruction, code);
    }

    // ── Step 3: Decode — look up the handler ────────────────────────
    // Context-aware handlers take priority during context execution.
    // This lets typed VMs register handlers that receive per-execution
    // state (memory, tables, globals) without affecting untyped VMs.
    const contextHandler = this.contextHandlers.get(instruction.opcode);
    const handler = this.handlers.get(instruction.opcode);

    if (!contextHandler && !handler) {
      throw new InvalidOpcodeError(
        `No handler registered for opcode 0x${instruction.opcode.toString(16).padStart(2, "0")}`,
      );
    }

    // ── Step 4: Execute — call the handler ──────────────────────────
    // The handler is responsible for:
    //   - Manipulating the stack (push/pop)
    //   - Updating variables/locals
    //   - Advancing or jumping the PC
    //   - Setting halted = true if it's a HALT instruction
    //   - Returning a description string (or null)
    let description: string | null;

    if (contextHandler && this.executionContext !== null) {
      // Use context-aware handler when a context is present.
      description = contextHandler(this, instruction, code, this.executionContext);
    } else if (handler) {
      // Fall back to regular handler.
      description = handler(this, instruction, code);
    } else {
      // Context handler exists but no context was set — error.
      throw new InvalidOpcodeError(
        `Context handler for opcode 0x${instruction.opcode.toString(16).padStart(2, "0")} requires an execution context`,
      );
    }

    // ── Step 4.5: Post-instruction hook ─────────────────────────────
    // If a post-hook is registered, call it after the handler completes.
    // Useful for tracing, assertions, or profiling.
    if (this.postHook) {
      this.postHook(this, instruction, code);
    }

    // ── Step 5: Record — build the trace ────────────────────────────
    const trace: VMTrace = {
      pc: pcBefore,
      instruction,
      stackBefore,
      stackAfter: [...this.stack],
      variables: { ...this.variables },
      output: null,
      description: description ?? `Executed opcode 0x${instruction.opcode.toString(16).padStart(2, "0")}`,
    };

    return trace;
  }

  // =======================================================================
  // Reset
  // =======================================================================

  /**
   * Reset the VM to its initial state, preserving registered handlers.
   *
   * This clears all execution state (stack, variables, locals, PC,
   * call stack, output, halted flag) but keeps the opcode handlers
   * and built-in functions intact. This lets you reuse the same
   * configured VM to run multiple programs.
   *
   * Think of it like rebooting a computer — the hardware (handlers)
   * stays the same, but all running programs and their data are wiped.
   *
   * **What is preserved:**
   * - Opcode handlers (the ``handlers`` map)
   * - Context handlers (the ``contextHandlers`` map)
   * - Built-in functions (the ``builtins`` map)
   * - Max recursion depth setting
   * - Frozen state
   * - Pre/post instruction hooks
   *
   * **What is cleared:**
   * - Stack → empty
   * - Typed stack → empty
   * - Variables → empty
   * - Locals → empty
   * - PC → 0
   * - Halted → false
   * - Output → empty
   * - Call stack → empty
   * - Execution context → null
   */
  reset(): void {
    this.stack = [];
    this.typedStack = [];
    this.variables = {};
    this.locals = [];
    this.pc = 0;
    this.halted = false;
    this.output = [];
    this.callStack = [];
    this.executionContext = null;
  }
}
