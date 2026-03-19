/**
 * Virtual Machine — A General-Purpose Stack-Based Bytecode Interpreter.
 *
 * ==========================================================================
 * Chapter 1: What Is a Virtual Machine?
 * ==========================================================================
 *
 * Imagine you've written a program in Python, Ruby, or some custom language you
 * invented. Before your computer can actually *run* that program, it needs to be
 * translated into something a processor understands. But real CPUs are messy —
 * there are dozens of different architectures (x86, ARM, RISC-V), each with its
 * own instruction set.
 *
 * A **virtual machine** (VM) solves this by providing a *fake* processor that
 * runs everywhere. Instead of compiling your language to x86 or ARM, you compile
 * it to the VM's **bytecode** — a simple, portable instruction set. Then the VM
 * interprets that bytecode on whatever real hardware you happen to have.
 *
 * This is exactly how Java works:
 *     Java source -> javac -> .class file (bytecode) -> JVM interprets it
 *
 * And how .NET works:
 *     C# source -> csc -> .dll (CIL bytecode) -> CLR interprets/JITs it
 *
 * Our VM follows the same principle. It is **language-agnostic**: Python, Ruby,
 * or any future language can compile down to our bytecode, and this single VM
 * will run it all.
 *
 * ==========================================================================
 * Chapter 2: Stack-Based vs Register-Based
 * ==========================================================================
 *
 * There are two main VM architectures:
 *
 * **Register-based** (like Lua's VM or most real CPUs):
 *     ADD R1, R2, R3   ->  "Put R2 + R3 into R1"
 *
 * **Stack-based** (like the JVM, .NET CLR, Python's CPython, and *our* VM):
 *     PUSH 3
 *     PUSH 4
 *     ADD             ->  pops 3 and 4, pushes 7
 *
 * Stack-based VMs are simpler to implement and simpler to compile to. You don't
 * need to worry about register allocation — just push values, operate on them,
 * and pop results. The trade-off is that stack-based code is more verbose (more
 * instructions), but that's fine for an educational VM.
 *
 * Think of the stack like a stack of plates in a cafeteria:
 * - You can only put a plate on **top** (push).
 * - You can only take the **top** plate off (pop).
 * - You can peek at the top plate without removing it.
 *
 * ==========================================================================
 * Chapter 3: The Instruction Set
 * ==========================================================================
 *
 * Our instruction set is deliberately minimal but complete enough to run real
 * programs. Every "serious" VM needs these categories:
 *
 * 1. **Stack manipulation** — moving values on/off the stack
 * 2. **Arithmetic** — math operations
 * 3. **Comparison** — testing relationships between values
 * 4. **Variables** — storing and retrieving named data
 * 5. **Control flow** — jumps, branches, loops
 * 6. **Functions** — calling and returning
 * 7. **I/O** — communicating with the outside world
 * 8. **VM control** — halting execution
 *
 * Each opcode is assigned a hexadecimal value, grouped by category. This is
 * exactly how real bytecode formats work — the JVM's ``iconst_0`` is 0x03,
 * ``iadd`` is 0x60, etc. Our numbering is simpler but follows the same idea.
 */

// =========================================================================
// Value Type
// =========================================================================

/**
 * The type of values that can live on the VM's stack or in variables.
 *
 * Our VM is dynamically typed — values can be numbers, strings, or
 * CodeObjects (for function references). This mirrors how interpreters
 * for dynamic languages like Python or Ruby work internally.
 */
export type VMValue = number | string | CodeObject | null;

// =========================================================================
// OpCode Enumeration
// =========================================================================

/**
 * The complete instruction set for our virtual machine.
 *
 * Each opcode is a single byte value (0x00-0xFF), giving us room for up to
 * 256 different instructions. We group them by category using the high nibble:
 *
 *     0x0_ = stack operations
 *     0x1_ = variable operations
 *     0x2_ = arithmetic
 *     0x3_ = comparison
 *     0x4_ = control flow
 *     0x5_ = function operations
 *     0x6_ = I/O
 *     0xF_ = VM control
 *
 * This grouping is a common convention. The JVM does something similar —
 * all its "load" instructions are in one numeric range, all "store"
 * instructions in another. It makes debugging easier because you can tell
 * the *category* of an instruction just by glancing at its hex value.
 *
 * **Why a const object instead of a TypeScript enum?**
 * Using ``as const`` produces cleaner JavaScript output and is easier to
 * iterate over. The values match the Python ``virtual_machine.OpCode``
 * enum exactly.
 */
export const OpCode = {
  // -- Stack Operations (0x0_) ------------------------------------------
  // These move values onto or off of the operand stack.

  /** Push a constant from the constants pool onto the stack.
   *
   *  Operand: index into the CodeObject's ``constants`` list.
   *
   *  Example: If constants = [42, "hello"], then LOAD_CONST 0 pushes 42.
   *
   *  JVM equivalent: ``ldc`` (load constant from constant pool).
   *  CLR equivalent: ``ldc.i4`` (load 32-bit integer constant). */
  LOAD_CONST: 0x01,

  /** Discard the top value on the stack. No operand.
   *
   *  Why would you ever throw away a value? Sometimes a function returns
   *  something you don't need, or an expression is evaluated for its
   *  side effects only (like a function call whose return value is ignored).
   *
   *  JVM equivalent: ``pop``. */
  POP: 0x02,

  /** Duplicate the top value on the stack. No operand.
   *
   *  This is useful when you need to use a value twice without recomputing it.
   *  For example, ``x = x + 1`` might compile to:
   *      LOAD_NAME x -> DUP -> LOAD_CONST 1 -> ADD -> STORE_NAME x
   *
   *  JVM equivalent: ``dup``. */
  DUP: 0x03,

  // -- Variable Operations (0x1_) ----------------------------------------
  // These store values in and retrieve values from variable storage.
  // We support two kinds of variables:
  //   1. Named variables (like global/module-level vars) — stored in a dict
  //   2. Local slots (like function-local vars) — stored in a list by index
  //
  // The distinction matters for performance: dict lookup by name is slower
  // than direct array indexing. Real VMs (JVM, CPython) use numbered local
  // slots inside functions for exactly this reason.

  /** Pop the top of stack and store it in a named variable.
   *
   *  Operand: index into the CodeObject's ``names`` list, which gives the
   *  variable name as a string.
   *
   *  Example: If names = ["x", "y"], then STORE_NAME 0 pops the top value
   *  and stores it as variable "x".
   *
   *  JVM equivalent: ``putstatic`` (for class-level fields).
   *  CPython equivalent: ``STORE_NAME`` (identical!). */
  STORE_NAME: 0x10,

  /** Push the value of a named variable onto the stack.
   *
   *  Operand: index into the CodeObject's ``names`` list.
   *
   *  If the variable hasn't been defined yet, this is a runtime error —
   *  just like getting a NameError in Python or an "undefined variable"
   *  error in Ruby.
   *
   *  JVM equivalent: ``getstatic``.
   *  CPython equivalent: ``LOAD_NAME``. */
  LOAD_NAME: 0x11,

  /** Pop the top of stack and store it in a local variable slot.
   *
   *  Operand: integer index of the local slot.
   *
   *  Local slots are a flat array of values, indexed by number. Inside a
   *  function, the compiler assigns each local variable a slot number
   *  (0, 1, 2, ...). This is faster than dictionary lookup because it's
   *  just an array index operation — O(1) with minimal overhead.
   *
   *  JVM equivalent: ``istore``, ``astore`` (store int/reference to local). */
  STORE_LOCAL: 0x12,

  /** Push the value from a local variable slot onto the stack.
   *
   *  Operand: integer index of the local slot.
   *
   *  JVM equivalent: ``iload``, ``aload`` (load int/reference from local). */
  LOAD_LOCAL: 0x13,

  // -- Arithmetic Operations (0x2_) --------------------------------------
  // These pop two operands, perform a math operation, and push the result.
  //
  // IMPORTANT: The order matters! For non-commutative operations like
  // subtraction and division, the *first* value pushed is the left operand.
  // So to compute 10 - 3:
  //     PUSH 10    <- pushed first, so it's deeper in the stack
  //     PUSH 3     <- pushed second, so it's on top
  //     SUB        <- pops 3 (top=b), pops 10 (second=a), pushes a - b = 7
  //
  // This "a then b then operate" order is the standard convention for
  // stack-based VMs. The JVM, CLR, and CPython all do it this way.

  /** Pop two values, push their sum.
   *
   *  Supports both integers and strings (concatenation), just like how
   *  Python's ``+`` works on both numbers and strings. Real VMs often have
   *  separate opcodes for integer add vs float add vs string concat, but
   *  we keep it simple with dynamic typing. */
  ADD: 0x20,

  /** Pop two values, push their difference (a - b).
   *
   *  'a' is the value pushed first (deeper in stack).
   *  'b' is the value pushed second (top of stack). */
  SUB: 0x21,

  /** Pop two values, push their product. */
  MUL: 0x22,

  /** Pop two values, push their quotient (a / b).
   *
   *  Uses integer division (Math.trunc). Raises VMError if b is zero —
   *  division by zero is a runtime error in virtually every language and VM. */
  DIV: 0x23,

  // -- Comparison Operations (0x3_) --------------------------------------
  // These pop two values, compare them, and push a boolean result.
  // We represent booleans as integers: 1 for true, 0 for false.
  //
  // Why integers instead of JavaScript's true/false? Because this VM is
  // language-agnostic. Not all languages have a boolean type — C uses
  // integers, and even the JVM represents booleans as ints internally.
  // Using 1/0 keeps things uniform.

  /** Pop two values, push 1 if they are equal, 0 otherwise.
   *
   *  JVM equivalent: ``if_icmpeq`` (though JVM branches directly rather
   *  than pushing a boolean). */
  CMP_EQ: 0x30,

  /** Pop two values, push 1 if a < b, 0 otherwise.
   *
   *  Same operand ordering as arithmetic: 'a' is pushed first (deeper),
   *  'b' is pushed second (top). */
  CMP_LT: 0x31,

  /** Pop two values, push 1 if a > b, 0 otherwise. */
  CMP_GT: 0x32,

  // -- Control Flow (0x4_) -----------------------------------------------
  // These change the program counter (PC) to alter which instruction
  // executes next. Without control flow, programs would be purely linear —
  // no loops, no if-statements, no interesting behavior.
  //
  // The "jump" metaphor: normally the PC advances one instruction at a
  // time (like reading a book page by page). A jump says "skip to page X"
  // or "go back to page Y." Conditional jumps say "skip to page X *only if*
  // some condition is true."

  /** Unconditional jump: set PC to the operand value.
   *
   *  Operand: the instruction index to jump to.
   *
   *  This is like a ``goto`` statement. It's the building block for loops:
   *      0: LOAD_CONST ...
   *      1: PRINT
   *      2: JUMP 0       <- infinite loop!
   *
   *  JVM equivalent: ``goto``. */
  JUMP: 0x40,

  /** Conditional jump: pop top of stack, jump if it's falsy (0).
   *
   *  Operand: the instruction index to jump to if the value is falsy.
   *
   *  "Falsy" means 0, null, or empty string — the standard falsy values.
   *  If the value is truthy, execution continues to the next instruction.
   *
   *  This is how if-statements and while-loops are compiled:
   *      if x > 5:         ->  LOAD_NAME x / LOAD_CONST 5 / CMP_GT
   *          do_something   ->  JUMP_IF_FALSE <past the body>
   *                         ->  <body instructions>
   *
   *  JVM equivalent: ``ifeq`` (jump if zero). */
  JUMP_IF_FALSE: 0x41,

  /** Conditional jump: pop top of stack, jump if it's truthy (non-zero).
   *
   *  Operand: the instruction index to jump to if the value is truthy.
   *
   *  Less commonly used than JUMP_IF_FALSE, but handy for short-circuit
   *  evaluation of ``or`` expressions.
   *
   *  JVM equivalent: ``ifne`` (jump if not zero). */
  JUMP_IF_TRUE: 0x42,

  // -- Function Operations (0x5_) ----------------------------------------
  // Functions are the backbone of structured programming. Our VM supports
  // them through a call stack (just like real hardware).
  //
  // When you CALL a function, the VM saves its current state (PC, locals)
  // onto a call stack, then jumps to the function's code. When the function
  // RETURNs, the VM restores the saved state and continues where it left off.

  /** Call a function.
   *
   *  Operand: the name of the function (index into names pool).
   *
   *  The VM looks up the function in its variables dict (where it was stored
   *  via STORE_NAME), saves the current execution state on the call stack,
   *  and begins executing the function's CodeObject.
   *
   *  JVM equivalent: ``invokevirtual``, ``invokestatic``. */
  CALL: 0x50,

  /** Return from a function.
   *
   *  Pops the call stack to restore the caller's state. If there's a value
   *  on top of the current stack, it becomes the return value and is pushed
   *  onto the caller's stack.
   *
   *  JVM equivalent: ``ireturn``, ``areturn``. */
  RETURN: 0x51,

  // -- I/O Operations (0x6_) ---------------------------------------------

  /** Pop the top of stack and print it.
   *
   *  The output is captured in the VM's ``output`` list so it can be
   *  inspected by tests and the pipeline visualizer without actually writing
   *  to stdout. */
  PRINT: 0x60,

  // -- VM Control (0xF_) -------------------------------------------------

  /** Stop execution immediately.
   *
   *  Every program should end with HALT. If the PC runs past the end of
   *  the instruction list without hitting HALT, the VM stops automatically
   *  (rather than crashing), but explicit HALT is good practice — it makes
   *  the program's end point clear.
   *
   *  JVM equivalent: There isn't one — JVM methods just ``return``. But our
   *  VM is simpler and runs a flat list of instructions, so we need an
   *  explicit "stop" signal. */
  HALT: 0xff,
} as const;

/**
 * The type of an individual opcode value.
 *
 * TypeScript's ``typeof OpCode[keyof typeof OpCode]`` extracts the union
 * of all numeric values in the OpCode object. This lets us type-check that
 * instructions only contain valid opcodes.
 */
export type OpCodeValue = (typeof OpCode)[keyof typeof OpCode];

/**
 * Reverse lookup table: maps opcode numeric values back to their names.
 *
 * This is used by the ``_describe`` method and ``Instruction.toString()``
 * to produce human-readable output like "LOAD_CONST" instead of "0x01".
 *
 * We build it dynamically from the OpCode object so it's always in sync.
 */
const opCodeNames: Record<number, string> = {};
for (const [name, value] of Object.entries(OpCode)) {
  opCodeNames[value] = name;
}

// =========================================================================
// Data Structures
// =========================================================================

/**
 * A single VM instruction: an opcode plus an optional operand.
 *
 * Think of this as one line of assembly language:
 *
 *     ADD                -> opcode = ADD, operand = undefined
 *     LOAD_CONST 0       -> opcode = LOAD_CONST, operand = 0
 *     STORE_NAME 1       -> opcode = STORE_NAME, operand = 1
 *
 * Some instructions (like ADD, POP, HALT) don't need an operand — they
 * operate purely on what's already on the stack. Others (like LOAD_CONST,
 * JUMP) need an operand to know *which* constant to load or *where* to jump.
 *
 * In a real bytecode format, this would be encoded as raw bytes:
 *     [opcode_byte] [operand_bytes...]
 *
 * We use a TypeScript interface for clarity, but the concept is identical.
 */
export interface Instruction {
  /** The operation to perform. */
  readonly opcode: OpCodeValue;

  /**
   * Optional data for the operation.
   *
   * - For LOAD_CONST: index into the constants pool (number).
   * - For STORE_NAME/LOAD_NAME: index into the names pool (number).
   * - For STORE_LOCAL/LOAD_LOCAL: local slot index (number).
   * - For JUMP/JUMP_IF_*: target instruction index (number).
   * - For CALL: index into the names pool (number).
   * - For stack/arithmetic ops: undefined (not used).
   */
  readonly operand?: number | string | null;
}

/**
 * Format an Instruction as a human-readable string.
 *
 * Produces output like ``"Instruction(LOAD_CONST, 0)"`` or ``"Instruction(ADD)"``.
 * This mirrors the Python ``__repr__`` method.
 */
export function instructionToString(instr: Instruction): string {
  const name = opCodeNames[instr.opcode] ?? `0x${instr.opcode.toString(16).padStart(2, "0")}`;
  if (instr.operand !== undefined && instr.operand !== null) {
    if (typeof instr.operand === "string") {
      return `Instruction(${name}, '${instr.operand}')`;
    }
    return `Instruction(${name}, ${instr.operand})`;
  }
  return `Instruction(${name})`;
}

/**
 * A compiled unit of code — the bytecode equivalent of a source file.
 *
 * This is our version of Java's ``.class`` file or Python's ``code`` object.
 * It bundles together everything the VM needs to execute a piece of code:
 *
 * 1. **instructions** — The ordered list of operations to perform.
 * 2. **constants** — A pool of literal values (numbers, strings) referenced
 *    by LOAD_CONST instructions. Instead of embedding "42" directly in
 *    the instruction stream, we store it here and reference it by index.
 *    This is more efficient (the constant is stored once even if used many
 *    times) and mirrors how real bytecode formats work.
 * 3. **names** — A pool of identifier strings (variable names, function
 *    names) referenced by STORE_NAME/LOAD_NAME/CALL instructions. Same
 *    idea as the constants pool but for names.
 *
 * **Why pools?**
 * Real bytecode formats use constant pools extensively. The JVM's constant
 * pool stores strings, class names, method signatures, and numeric
 * literals. Our two pools (constants + names) are a simplified version
 * of the same idea.
 *
 * **Example:**
 * To represent ``x = 42``:
 *
 *     constants = [42]
 *     names = ["x"]
 *     instructions = [
 *       { opcode: LOAD_CONST, operand: 0 },   // push constants[0] = 42
 *       { opcode: STORE_NAME, operand: 0 },    // pop into names[0] = "x"
 *       { opcode: HALT },                       // stop
 *     ]
 */
export interface CodeObject {
  /** The sequence of instructions to execute, in order. */
  readonly instructions: readonly Instruction[];

  /**
   * The constants pool — literal values referenced by index.
   *
   * Index 0 is the first constant, index 1 is the second, etc.
   * LOAD_CONST instructions reference this pool by index.
   */
  readonly constants: readonly (number | string)[];

  /**
   * The names pool — variable/function names referenced by index.
   *
   * STORE_NAME, LOAD_NAME, and CALL instructions reference this pool
   * by index to find the actual string name.
   */
  readonly names: readonly string[];
}

/**
 * A snapshot of one execution step — the VM's "black box recorder."
 *
 * Every time the VM executes an instruction, it produces a VMTrace
 * capturing the complete state before and after. This serves two purposes:
 *
 * 1. **Debugging** — You can replay the entire execution step by step,
 *    seeing exactly what happened to the stack, variables, and output
 *    at each point.
 *
 * 2. **Visualization** — The pipeline visualizer (a future component)
 *    can animate the VM's execution, showing values flowing onto and
 *    off of the stack, variables changing, etc.
 *
 * Think of it like a flight recorder (black box) on an airplane — it
 * records everything so you can reconstruct what happened.
 */
export interface VMTrace {
  /**
   * The program counter *before* this instruction executed.
   *
   * This tells you which instruction in the CodeObject was being executed.
   */
  readonly pc: number;

  /** The instruction that was executed in this step. */
  readonly instruction: Instruction;

  /**
   * A snapshot of the stack before the instruction ran.
   *
   * This is a copy, not a reference, so it won't change as execution
   * continues.
   */
  readonly stackBefore: readonly VMValue[];

  /** A snapshot of the stack after the instruction ran. */
  readonly stackAfter: readonly VMValue[];

  /** A snapshot of all named variables after the instruction ran. */
  readonly variables: Readonly<Record<string, VMValue>>;

  /**
   * If this instruction was PRINT, the string that was printed.
   *
   * null for all other instructions.
   */
  readonly output: string | null;

  /**
   * A human-readable explanation of what this step did.
   *
   * Examples:
   *     "Push constant 42 onto the stack"
   *     "Pop 3 and 7, push sum 10"
   *     "Store 42 into variable 'x'"
   */
  readonly description: string;
}

/**
 * A saved execution context for function calls.
 *
 * When you call a function, the VM needs to remember where it was so it
 * can come back after the function returns. A CallFrame saves:
 *
 * - The return address (which instruction to resume at)
 * - The caller's local variables
 * - The caller's stack state
 *
 * This is exactly what real CPUs do with their hardware call stack — the
 * ``call`` instruction pushes a return address, and ``ret`` pops it.
 * Our CallFrame is a richer version that also saves local variable state.
 *
 * The collection of all active CallFrames is the **call stack** — the same
 * call stack you see in debugger backtraces and error stack traces.
 */
export interface CallFrame {
  /** The PC value to restore when the function returns. */
  readonly returnAddress: number;

  /** The caller's named variables, saved for restoration. */
  readonly savedVariables: Record<string, VMValue>;

  /** The caller's local variable slots, saved for restoration. */
  readonly savedLocals: VMValue[];
}

// =========================================================================
// Errors
// =========================================================================

/**
 * Base class for all virtual machine runtime errors.
 *
 * Just as the JVM throws ``java.lang.RuntimeException`` and Python raises
 * ``RuntimeError``, our VM raises VMError when something goes wrong during
 * execution — stack underflow, division by zero, undefined variables, etc.
 */
export class VMError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "VMError";
  }
}

/**
 * Raised when an operation tries to pop from an empty stack.
 *
 * This is the VM equivalent of a segfault — something tried to read data
 * that isn't there. It usually means the bytecode is malformed (a compiler
 * bug) rather than a user program error.
 */
export class StackUnderflowError extends VMError {
  constructor(message: string = "Cannot pop from an empty stack — possible compiler bug") {
    super(message);
    this.name = "StackUnderflowError";
  }
}

/**
 * Raised when code tries to read a variable that hasn't been defined.
 *
 * This is our equivalent of Python's ``NameError`` or JavaScript's
 * ``ReferenceError``.
 */
export class UndefinedNameError extends VMError {
  constructor(message: string) {
    super(message);
    this.name = "UndefinedNameError";
  }
}

/**
 * Raised when code attempts to divide by zero.
 *
 * Every language and VM treats this as an error. The JVM throws
 * ``ArithmeticException``, Python raises ``ZeroDivisionError``,
 * and we raise ``DivisionByZeroError``.
 */
export class DivisionByZeroError extends VMError {
  constructor(message: string = "Division by zero") {
    super(message);
    this.name = "DivisionByZeroError";
  }
}

/**
 * Raised when the VM encounters an opcode it doesn't recognize.
 *
 * This would happen if the bytecode is corrupted or was produced by
 * a buggy compiler.
 */
export class InvalidOpcodeError extends VMError {
  constructor(message: string) {
    super(message);
    this.name = "InvalidOpcodeError";
  }
}

/**
 * Raised when an instruction's operand is out of bounds.
 *
 * For example, LOAD_CONST 99 when the constants pool only has 3 entries.
 */
export class InvalidOperandError extends VMError {
  constructor(message: string) {
    super(message);
    this.name = "InvalidOperandError";
  }
}

// =========================================================================
// The Virtual Machine
// =========================================================================

/**
 * A general-purpose stack-based bytecode interpreter.
 *
 * This is the heart of our computing stack — the component that actually
 * *runs* programs. It takes a CodeObject (compiled bytecode) and executes
 * it instruction by instruction, maintaining:
 *
 * - **stack** — The operand stack, where all computation happens.
 * - **variables** — Named variable storage (like global scope).
 * - **locals** — Indexed local variable slots (like function scope).
 * - **pc** — The program counter, pointing to the current instruction.
 * - **callStack** — Saved contexts for function calls.
 * - **output** — Captured print output for testing and visualization.
 *
 * **The Fetch-Decode-Execute Cycle:**
 * Like every processor (real or virtual), our VM runs in a loop:
 *
 * 1. **Fetch** — Read the instruction at ``pc``.
 * 2. **Decode** — Look at the opcode to determine what to do.
 * 3. **Execute** — Perform the operation (push, pop, add, jump, etc.).
 * 4. **Advance** — Move ``pc`` to the next instruction (unless we jumped).
 * 5. **Repeat** — Go back to step 1.
 *
 * This is the *exact same cycle* that real CPUs use, just implemented in
 * software rather than silicon.
 *
 * **Usage:**
 *
 *     const code: CodeObject = {
 *       instructions: [
 *         { opcode: OpCode.LOAD_CONST, operand: 0 },
 *         { opcode: OpCode.LOAD_CONST, operand: 1 },
 *         { opcode: OpCode.ADD },
 *         { opcode: OpCode.PRINT },
 *         { opcode: OpCode.HALT },
 *       ],
 *       constants: [3, 4],
 *       names: [],
 *     };
 *     const vm = new VirtualMachine();
 *     const traces = vm.execute(code);
 *     console.log(vm.output); // ["7"]
 */
export class VirtualMachine {
  /** The operand stack — where all values live during computation. */
  stack: VMValue[] = [];

  /** Named variable storage — like a global scope dictionary. */
  variables: Record<string, VMValue> = {};

  /**
   * Local variable slots — a flat array indexed by number.
   *
   * Inside functions, local variables are stored here by index rather
   * than by name, for performance (array lookup is faster than dict
   * lookup). The compiler assigns each local variable a slot number.
   */
  locals: VMValue[] = [];

  /**
   * The program counter — index of the next instruction to execute.
   *
   * This is the VM's "read head," pointing to where we are in the
   * instruction stream. It advances by 1 after each instruction,
   * unless a jump changes it.
   */
  pc: number = 0;

  /**
   * Whether the VM has stopped execution.
   *
   * Set to true by the HALT instruction or when the PC runs past the
   * end of the instruction list.
   */
  halted: boolean = false;

  /**
   * Captured print output.
   *
   * Instead of writing directly to stdout, PRINT instructions append
   * their output here. This makes testing easy (just check vm.output)
   * and enables the pipeline visualizer to display output.
   */
  output: string[] = [];

  /**
   * The call stack — saved contexts for function calls.
   *
   * Each time CALL is executed, a CallFrame is pushed here. Each time
   * RETURN is executed, a CallFrame is popped and its state restored.
   */
  callStack: CallFrame[] = [];

  /**
   * Reset the VM to its initial state.
   *
   * Call this between executions if you want to reuse the same VM
   * instance. Equivalent to creating a new VirtualMachine().
   */
  reset(): void {
    this.stack = [];
    this.variables = {};
    this.locals = [];
    this.pc = 0;
    this.halted = false;
    this.output = [];
    this.callStack = [];
  }

  // -----------------------------------------------------------------
  // Public API
  // -----------------------------------------------------------------

  /**
   * Execute a complete CodeObject, returning a trace of every step.
   *
   * This is the main entry point. It runs the fetch-decode-execute cycle
   * until the program HALTs or the PC goes past the last instruction.
   *
   * @param code - The compiled bytecode to execute.
   * @returns A trace entry for every instruction that was executed, in order.
   *          This is the complete execution history — invaluable for debugging
   *          and visualization.
   * @throws VMError if a runtime error occurs (stack underflow, division by
   *         zero, undefined variable, etc.).
   */
  execute(code: CodeObject): VMTrace[] {
    const traces: VMTrace[] = [];

    while (!this.halted && this.pc < code.instructions.length) {
      const trace = this.step(code);
      traces.push(trace);
    }

    return traces;
  }

  /**
   * Execute one instruction and return a trace of what happened.
   *
   * This is the single-step entry point, useful for debuggers and
   * step-through visualization. It performs exactly one iteration of
   * the fetch-decode-execute cycle.
   *
   * @param code - The CodeObject being executed.
   * @returns A snapshot of the VM state before and after this instruction.
   * @throws VMError if the instruction causes a runtime error.
   */
  step(code: CodeObject): VMTrace {
    // -- Fetch --
    const instruction = code.instructions[this.pc];
    const pcBefore = this.pc;
    const stackBefore = [...this.stack]; // snapshot (copy)

    // -- Decode & Execute --
    const outputValue = this._dispatch(instruction, code);

    // -- Build trace --
    const description = this._describe(instruction, code, stackBefore);
    const trace: VMTrace = {
      pc: pcBefore,
      instruction,
      stackBefore,
      stackAfter: [...this.stack], // snapshot after
      variables: { ...this.variables }, // snapshot
      output: outputValue,
      description,
    };

    return trace;
  }

  // -----------------------------------------------------------------
  // The Dispatch Table (Decode + Execute)
  // -----------------------------------------------------------------

  /**
   * Decode and execute a single instruction.
   *
   * This is the classic "big switch" at the heart of every interpreter.
   * The JVM's interpreter loop, CPython's ceval.c, and Ruby's YARV all
   * have one of these — a giant switch statement that handles every
   * possible opcode.
   *
   * @param instruction - The instruction to execute.
   * @param code - The containing CodeObject (needed for constants/names pools).
   * @returns If the instruction was PRINT, returns the printed string.
   *          Otherwise returns null.
   */
  private _dispatch(instruction: Instruction, code: CodeObject): string | null {
    let outputValue: string | null = null;

    switch (instruction.opcode) {
      // == Stack Operations ==========================================

      case OpCode.LOAD_CONST: {
        // Push a constant from the pool onto the stack.
        const index = this._requireOperand(instruction);
        if (typeof index !== "number" || index < 0 || index >= code.constants.length) {
          throw new InvalidOperandError(
            `LOAD_CONST operand ${index} is out of range ` +
            `(constants pool has ${code.constants.length} entries)`,
          );
        }
        const value = code.constants[index];
        this.stack.push(value);
        this.pc += 1;
        break;
      }

      case OpCode.POP: {
        // Discard the top of the stack.
        this._pop();
        this.pc += 1;
        break;
      }

      case OpCode.DUP: {
        // Duplicate the top of the stack.
        if (this.stack.length === 0) {
          throw new StackUnderflowError(
            "DUP requires at least one value on the stack",
          );
        }
        this.stack.push(this.stack[this.stack.length - 1]);
        this.pc += 1;
        break;
      }

      // == Variable Operations =======================================

      case OpCode.STORE_NAME: {
        // Pop the top value and store it in a named variable.
        const index = this._requireOperand(instruction);
        if (typeof index !== "number" || index < 0 || index >= code.names.length) {
          throw new InvalidOperandError(
            `STORE_NAME operand ${index} is out of range ` +
            `(names pool has ${code.names.length} entries)`,
          );
        }
        const name = code.names[index];
        const value = this._pop();
        this.variables[name] = value;
        this.pc += 1;
        break;
      }

      case OpCode.LOAD_NAME: {
        // Push the value of a named variable onto the stack.
        const index = this._requireOperand(instruction);
        if (typeof index !== "number" || index < 0 || index >= code.names.length) {
          throw new InvalidOperandError(
            `LOAD_NAME operand ${index} is out of range ` +
            `(names pool has ${code.names.length} entries)`,
          );
        }
        const name = code.names[index];
        if (!(name in this.variables)) {
          throw new UndefinedNameError(
            `Variable '${name}' is not defined`,
          );
        }
        this.stack.push(this.variables[name]);
        this.pc += 1;
        break;
      }

      case OpCode.STORE_LOCAL: {
        // Pop the top value and store it in a local slot.
        const index = this._requireOperand(instruction);
        if (typeof index !== "number" || index < 0) {
          throw new InvalidOperandError(
            `STORE_LOCAL operand must be a non-negative integer, ` +
            `got ${index}`,
          );
        }
        const value = this._pop();
        // Extend the locals list if needed (auto-grow).
        while (this.locals.length <= index) {
          this.locals.push(null);
        }
        this.locals[index] = value;
        this.pc += 1;
        break;
      }

      case OpCode.LOAD_LOCAL: {
        // Push the value from a local slot onto the stack.
        const index = this._requireOperand(instruction);
        if (typeof index !== "number" || index < 0) {
          throw new InvalidOperandError(
            `LOAD_LOCAL operand must be a non-negative integer, ` +
            `got ${index}`,
          );
        }
        if (index >= this.locals.length) {
          throw new InvalidOperandError(
            `LOAD_LOCAL slot ${index} has not been initialized ` +
            `(only ${this.locals.length} slots exist)`,
          );
        }
        this.stack.push(this.locals[index]);
        this.pc += 1;
        break;
      }

      // == Arithmetic ================================================

      case OpCode.ADD: {
        const b = this._pop();
        const a = this._pop();
        // Support both number addition and string concatenation.
        if (typeof a === "number" && typeof b === "number") {
          this.stack.push(a + b);
        } else if (typeof a === "string" && typeof b === "string") {
          this.stack.push(a + b);
        } else {
          // Dynamic typing: just coerce to add. In practice this handles
          // the same cases Python's ``+`` does on mixed types.
          this.stack.push((a as number) + (b as number));
        }
        this.pc += 1;
        break;
      }

      case OpCode.SUB: {
        const b = this._pop() as number;
        const a = this._pop() as number;
        this.stack.push(a - b);
        this.pc += 1;
        break;
      }

      case OpCode.MUL: {
        const b = this._pop() as number;
        const a = this._pop() as number;
        this.stack.push(a * b);
        this.pc += 1;
        break;
      }

      case OpCode.DIV: {
        const b = this._pop() as number;
        const a = this._pop() as number;
        if (b === 0) {
          throw new DivisionByZeroError("Division by zero");
        }
        this.stack.push(Math.trunc(a / b));
        this.pc += 1;
        break;
      }

      // == Comparison ================================================

      case OpCode.CMP_EQ: {
        const b = this._pop();
        const a = this._pop();
        this.stack.push(a === b ? 1 : 0);
        this.pc += 1;
        break;
      }

      case OpCode.CMP_LT: {
        const b = this._pop() as number;
        const a = this._pop() as number;
        this.stack.push(a < b ? 1 : 0);
        this.pc += 1;
        break;
      }

      case OpCode.CMP_GT: {
        const b = this._pop() as number;
        const a = this._pop() as number;
        this.stack.push(a > b ? 1 : 0);
        this.pc += 1;
        break;
      }

      // == Control Flow ==============================================

      case OpCode.JUMP: {
        const target = this._requireOperand(instruction);
        if (typeof target !== "number") {
          throw new InvalidOperandError(
            `JUMP operand must be an integer, got ${JSON.stringify(target)}`,
          );
        }
        this.pc = target; // Don't increment — we're jumping!
        break;
      }

      case OpCode.JUMP_IF_FALSE: {
        const target = this._requireOperand(instruction);
        if (typeof target !== "number") {
          throw new InvalidOperandError(
            `JUMP_IF_FALSE operand must be an integer, got ${JSON.stringify(target)}`,
          );
        }
        const condition = this._pop();
        if (this._isFalsy(condition)) {
          this.pc = target;
        } else {
          this.pc += 1;
        }
        break;
      }

      case OpCode.JUMP_IF_TRUE: {
        const target = this._requireOperand(instruction);
        if (typeof target !== "number") {
          throw new InvalidOperandError(
            `JUMP_IF_TRUE operand must be an integer, got ${JSON.stringify(target)}`,
          );
        }
        const condition = this._pop();
        if (!this._isFalsy(condition)) {
          this.pc = target;
        } else {
          this.pc += 1;
        }
        break;
      }

      // == Functions =================================================

      case OpCode.CALL: {
        const nameIndex = this._requireOperand(instruction);
        if (typeof nameIndex !== "number" || nameIndex < 0 || nameIndex >= code.names.length) {
          throw new InvalidOperandError(
            `CALL operand ${nameIndex} is out of range ` +
            `(names pool has ${code.names.length} entries)`,
          );
        }
        const funcName = code.names[nameIndex];
        if (!(funcName in this.variables)) {
          throw new UndefinedNameError(
            `Function '${funcName}' is not defined`,
          );
        }
        const funcCode = this.variables[funcName];
        if (
          !funcCode ||
          typeof funcCode !== "object" ||
          !("instructions" in funcCode)
        ) {
          throw new VMError(
            `'${funcName}' is not callable (expected CodeObject, ` +
            `got ${typeof funcCode})`,
          );
        }

        // Save current execution context.
        const frame: CallFrame = {
          returnAddress: this.pc + 1,
          savedVariables: { ...this.variables },
          savedLocals: [...this.locals],
        };
        this.callStack.push(frame);

        // Jump to the function's first instruction.
        // We set pc to 0 because the function has its own CodeObject,
        // but we need to re-enter the execute loop with the new code.
        // For simplicity, we execute the function inline by iteration.
        this.locals = [];
        this.pc = 0;
        while (!this.halted && this.pc < funcCode.instructions.length) {
          const currentInstr = funcCode.instructions[this.pc];
          if (currentInstr.opcode === OpCode.RETURN) {
            break;
          }
          this._dispatch(currentInstr, funcCode);
        }

        // Restore caller context.
        const poppedFrame = this.callStack.pop()!;
        this.pc = poppedFrame.returnAddress;
        this.locals = poppedFrame.savedLocals;
        // Note: variables persist across call/return (they're "global").
        break;
      }

      case OpCode.RETURN: {
        // Return from a function call.
        // If there's a call frame, restore the caller's state.
        if (this.callStack.length > 0) {
          const frame = this.callStack.pop()!;
          this.pc = frame.returnAddress;
          this.locals = frame.savedLocals;
        } else {
          // No call frame — RETURN at the top level acts like HALT.
          this.halted = true;
        }
        break;
      }

      // == I/O =======================================================

      case OpCode.PRINT: {
        const value = this._pop();
        const outputStr = String(value);
        this.output.push(outputStr);
        outputValue = outputStr;
        this.pc += 1;
        break;
      }

      // == VM Control ================================================

      case OpCode.HALT: {
        this.halted = true;
        // Don't advance PC — execution is done.
        break;
      }

      default: {
        throw new InvalidOpcodeError(
          `Unknown opcode: ${instruction.opcode}`,
        );
      }
    }

    return outputValue;
  }

  // -----------------------------------------------------------------
  // Helper Methods
  // -----------------------------------------------------------------

  /**
   * Pop and return the top value from the stack.
   *
   * Raises StackUnderflowError if the stack is empty. This is a safety
   * net — well-compiled bytecode should never underflow, but bugs happen.
   */
  private _pop(): VMValue {
    if (this.stack.length === 0) {
      throw new StackUnderflowError(
        "Cannot pop from an empty stack — possible compiler bug",
      );
    }
    return this.stack.pop()!;
  }

  /**
   * Get the operand from an instruction, raising an error if missing.
   *
   * Some instructions (LOAD_CONST, JUMP, etc.) require an operand.
   * This helper ensures one was provided.
   */
  private _requireOperand(instruction: Instruction): number | string {
    if (instruction.operand === undefined || instruction.operand === null) {
      const name = opCodeNames[instruction.opcode] ?? `${instruction.opcode}`;
      throw new InvalidOperandError(
        `${name} requires an operand but none was provided`,
      );
    }
    return instruction.operand;
  }

  /**
   * Determine whether a value is "falsy" for conditional jumps.
   *
   * Our falsy values are:
   * - 0 (integer zero)
   * - null
   * - "" (empty string)
   *
   * Everything else is truthy. This is a common convention — Python,
   * JavaScript, and Ruby all have similar truthiness rules (though the
   * exact details vary).
   */
  private _isFalsy(value: VMValue): boolean {
    return value === 0 || value === null || value === "";
  }

  /**
   * Generate a human-readable description of what an instruction did.
   *
   * These descriptions are meant for complete beginners — they explain
   * not just *what* happened but *why* in plain English.
   */
  _describe(
    instruction: Instruction,
    code: CodeObject,
    stackBefore: readonly VMValue[],
  ): string {
    const op = instruction.opcode;

    switch (op) {
      case OpCode.LOAD_CONST: {
        const idx = instruction.operand;
        const val =
          typeof idx === "number" && idx >= 0 && idx < code.constants.length
            ? code.constants[idx]
            : "?";
        return `Push constant ${JSON.stringify(val)} onto the stack`;
      }

      case OpCode.POP: {
        const val = stackBefore.length > 0 ? stackBefore[stackBefore.length - 1] : "?";
        return `Discard top of stack (${JSON.stringify(val)})`;
      }

      case OpCode.DUP: {
        const val = stackBefore.length > 0 ? stackBefore[stackBefore.length - 1] : "?";
        return `Duplicate top of stack (${JSON.stringify(val)})`;
      }

      case OpCode.STORE_NAME: {
        const idx = instruction.operand;
        const name =
          typeof idx === "number" && idx >= 0 && idx < code.names.length
            ? code.names[idx]
            : "?";
        const val = stackBefore.length > 0 ? stackBefore[stackBefore.length - 1] : "?";
        return `Store ${JSON.stringify(val)} into variable '${name}'`;
      }

      case OpCode.LOAD_NAME: {
        const idx = instruction.operand;
        const name =
          typeof idx === "number" && idx >= 0 && idx < code.names.length
            ? code.names[idx]
            : "?";
        return `Push variable '${name}' onto the stack`;
      }

      case OpCode.STORE_LOCAL: {
        const idx = instruction.operand;
        const val = stackBefore.length > 0 ? stackBefore[stackBefore.length - 1] : "?";
        return `Store ${JSON.stringify(val)} into local slot ${idx}`;
      }

      case OpCode.LOAD_LOCAL: {
        return `Push local slot ${instruction.operand} onto the stack`;
      }

      case OpCode.ADD: {
        if (stackBefore.length >= 2) {
          const a = stackBefore[stackBefore.length - 2];
          const b = stackBefore[stackBefore.length - 1];
          if (typeof a === "number" && typeof b === "number") {
            return `Pop ${JSON.stringify(b)} and ${JSON.stringify(a)}, push sum ${JSON.stringify(a + b)}`;
          }
          if (typeof a === "string" && typeof b === "string") {
            return `Pop ${JSON.stringify(b)} and ${JSON.stringify(a)}, push sum ${JSON.stringify(a + b)}`;
          }
        }
        return "Add top two stack values";
      }

      case OpCode.SUB: {
        if (stackBefore.length >= 2) {
          const a = stackBefore[stackBefore.length - 2] as number;
          const b = stackBefore[stackBefore.length - 1] as number;
          return `Pop ${JSON.stringify(b)} and ${JSON.stringify(a)}, push difference ${JSON.stringify(a - b)}`;
        }
        return "Subtract top two stack values";
      }

      case OpCode.MUL: {
        if (stackBefore.length >= 2) {
          const a = stackBefore[stackBefore.length - 2] as number;
          const b = stackBefore[stackBefore.length - 1] as number;
          return `Pop ${JSON.stringify(b)} and ${JSON.stringify(a)}, push product ${JSON.stringify(a * b)}`;
        }
        return "Multiply top two stack values";
      }

      case OpCode.DIV: {
        if (stackBefore.length >= 2) {
          const a = stackBefore[stackBefore.length - 2] as number;
          const b = stackBefore[stackBefore.length - 1] as number;
          if (b !== 0) {
            return `Pop ${JSON.stringify(b)} and ${JSON.stringify(a)}, push quotient ${JSON.stringify(Math.trunc(a / b))}`;
          }
          return `Pop ${JSON.stringify(b)} and ${JSON.stringify(a)}, DIVISION BY ZERO`;
        }
        return "Divide top two stack values";
      }

      case OpCode.CMP_EQ: {
        if (stackBefore.length >= 2) {
          const a = stackBefore[stackBefore.length - 2];
          const b = stackBefore[stackBefore.length - 1];
          const result = a === b ? 1 : 0;
          return `Compare ${JSON.stringify(a)} == ${JSON.stringify(b)} -> ${result}`;
        }
        return "Compare top two stack values for equality";
      }

      case OpCode.CMP_LT: {
        if (stackBefore.length >= 2) {
          const a = stackBefore[stackBefore.length - 2] as number;
          const b = stackBefore[stackBefore.length - 1] as number;
          const result = a < b ? 1 : 0;
          return `Compare ${JSON.stringify(a)} < ${JSON.stringify(b)} -> ${result}`;
        }
        return "Compare top two stack values (less than)";
      }

      case OpCode.CMP_GT: {
        if (stackBefore.length >= 2) {
          const a = stackBefore[stackBefore.length - 2] as number;
          const b = stackBefore[stackBefore.length - 1] as number;
          const result = a > b ? 1 : 0;
          return `Compare ${JSON.stringify(a)} > ${JSON.stringify(b)} -> ${result}`;
        }
        return "Compare top two stack values (greater than)";
      }

      case OpCode.JUMP: {
        return `Jump to instruction ${instruction.operand}`;
      }

      case OpCode.JUMP_IF_FALSE: {
        const val = stackBefore.length > 0 ? stackBefore[stackBefore.length - 1] : "?";
        return `Pop ${JSON.stringify(val)}, jump to ${instruction.operand} if falsy`;
      }

      case OpCode.JUMP_IF_TRUE: {
        const val = stackBefore.length > 0 ? stackBefore[stackBefore.length - 1] : "?";
        return `Pop ${JSON.stringify(val)}, jump to ${instruction.operand} if truthy`;
      }

      case OpCode.CALL: {
        const idx = instruction.operand;
        const name =
          typeof idx === "number" && idx >= 0 && idx < code.names.length
            ? code.names[idx]
            : "?";
        return `Call function '${name}'`;
      }

      case OpCode.RETURN: {
        return "Return from function";
      }

      case OpCode.PRINT: {
        const val = stackBefore.length > 0 ? stackBefore[stackBefore.length - 1] : "?";
        return `Print ${JSON.stringify(val)}`;
      }

      case OpCode.HALT: {
        return "Halt execution";
      }

      default: {
        return `Unknown opcode ${op}`;
      }
    }
  }
}

// =========================================================================
// Helper Functions
// =========================================================================

/**
 * Convenience function to build a CodeObject from parts.
 *
 * This is a simple "assembler" — it takes human-readable instructions and
 * packages them into a CodeObject that the VM can execute. In a real system,
 * this would be done by a compiler, but for testing and experimentation,
 * hand-assembling is invaluable.
 *
 * @param instructions - The instruction sequence to execute.
 * @param constants - The constants pool. Defaults to an empty array.
 * @param names - The names pool. Defaults to an empty array.
 * @returns A complete, ready-to-execute code object.
 *
 * @example
 * const code = assembleCode(
 *   [
 *     { opcode: OpCode.LOAD_CONST, operand: 0 },
 *     { opcode: OpCode.PRINT },
 *     { opcode: OpCode.HALT },
 *   ],
 *   [42],
 * );
 * const vm = new VirtualMachine();
 * vm.execute(code);
 * console.log(vm.output); // ["42"]
 */
export function assembleCode(
  instructions: Instruction[],
  constants?: (number | string)[] | null,
  names?: string[] | null,
): CodeObject {
  return {
    instructions,
    constants: constants ?? [],
    names: names ?? [],
  };
}
