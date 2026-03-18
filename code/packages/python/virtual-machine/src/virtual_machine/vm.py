"""Virtual Machine — A General-Purpose Stack-Based Bytecode Interpreter.

==========================================================================
Chapter 1: What Is a Virtual Machine?
==========================================================================

Imagine you've written a program in Python, Ruby, or some custom language you
invented. Before your computer can actually *run* that program, it needs to be
translated into something a processor understands. But real CPUs are messy —
there are dozens of different architectures (x86, ARM, RISC-V), each with its
own instruction set.

A **virtual machine** (VM) solves this by providing a *fake* processor that
runs everywhere. Instead of compiling your language to x86 or ARM, you compile
it to the VM's **bytecode** — a simple, portable instruction set. Then the VM
interprets that bytecode on whatever real hardware you happen to have.

This is exactly how Java works:
    Java source → javac → .class file (bytecode) → JVM interprets it

And how .NET works:
    C# source → csc → .dll (CIL bytecode) → CLR interprets/JITs it

Our VM follows the same principle. It is **language-agnostic**: Python, Ruby,
or any future language can compile down to our bytecode, and this single VM
will run it all.

==========================================================================
Chapter 2: Stack-Based vs Register-Based
==========================================================================

There are two main VM architectures:

**Register-based** (like Lua's VM or most real CPUs):
    ADD R1, R2, R3   →  "Put R2 + R3 into R1"

**Stack-based** (like the JVM, .NET CLR, Python's CPython, and *our* VM):
    PUSH 3
    PUSH 4
    ADD             →  pops 3 and 4, pushes 7

Stack-based VMs are simpler to implement and simpler to compile to. You don't
need to worry about register allocation — just push values, operate on them,
and pop results. The trade-off is that stack-based code is more verbose (more
instructions), but that's fine for an educational VM.

Think of the stack like a stack of plates in a cafeteria:
- You can only put a plate on **top** (push).
- You can only take the **top** plate off (pop).
- You can peek at the top plate without removing it.

==========================================================================
Chapter 3: The Instruction Set
==========================================================================

Our instruction set is deliberately minimal but complete enough to run real
programs. Every "serious" VM needs these categories:

1. **Stack manipulation** — moving values on/off the stack
2. **Arithmetic** — math operations
3. **Comparison** — testing relationships between values
4. **Variables** — storing and retrieving named data
5. **Control flow** — jumps, branches, loops
6. **Functions** — calling and returning
7. **I/O** — communicating with the outside world
8. **VM control** — halting execution

Each opcode is assigned a hexadecimal value, grouped by category. This is
exactly how real bytecode formats work — the JVM's `iconst_0` is 0x03,
`iadd` is 0x60, etc. Our numbering is simpler but follows the same idea.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import IntEnum
from typing import Any


# =========================================================================
# OpCode Enumeration
# =========================================================================


class OpCode(IntEnum):
    """The complete instruction set for our virtual machine.

    Each opcode is a single byte value (0x00–0xFF), giving us room for up to
    256 different instructions. We group them by category using the high nibble:

        0x0_ = stack operations
        0x1_ = variable operations
        0x2_ = arithmetic
        0x3_ = comparison
        0x4_ = control flow
        0x5_ = function operations
        0x6_ = I/O
        0xF_ = VM control

    This grouping is a common convention. The JVM does something similar —
    all its "load" instructions are in one numeric range, all "store"
    instructions in another. It makes debugging easier because you can tell
    the *category* of an instruction just by glancing at its hex value.

    **Why IntEnum?**
    Using IntEnum (rather than plain Enum) means each opcode *is* an integer.
    We can store it directly in a bytestream and compare it with ``==``
    against raw integers. This mirrors how real bytecode works — opcodes are
    literally just numbers in a byte array.
    """

    # -- Stack Operations (0x0_) ------------------------------------------
    # These move values onto or off of the operand stack.

    LOAD_CONST = 0x01
    """Push a constant from the constants pool onto the stack.

    Operand: index into the CodeObject's ``constants`` list.

    Example: If constants = [42, "hello"], then LOAD_CONST 0 pushes 42.

    JVM equivalent: ``ldc`` (load constant from constant pool).
    CLR equivalent: ``ldc.i4`` (load 32-bit integer constant).
    """

    POP = 0x02
    """Discard the top value on the stack.

    No operand. Simply removes the top element.

    Why would you ever throw away a value? Sometimes a function returns
    something you don't need, or an expression is evaluated for its
    side effects only (like a function call whose return value is ignored).

    JVM equivalent: ``pop``.
    """

    DUP = 0x03
    """Duplicate the top value on the stack.

    No operand. Peeks at the top element and pushes a copy of it.

    This is useful when you need to use a value twice without recomputing it.
    For example, ``x = x + 1`` might compile to:
        LOAD_NAME x → DUP → LOAD_CONST 1 → ADD → STORE_NAME x

    JVM equivalent: ``dup``.
    """

    # -- Variable Operations (0x1_) ----------------------------------------
    # These store values in and retrieve values from variable storage.
    # We support two kinds of variables:
    #   1. Named variables (like global/module-level vars) — stored in a dict
    #   2. Local slots (like function-local vars) — stored in a list by index
    #
    # The distinction matters for performance: dict lookup by name is slower
    # than direct array indexing. Real VMs (JVM, CPython) use numbered local
    # slots inside functions for exactly this reason.

    STORE_NAME = 0x10
    """Pop the top of stack and store it in a named variable.

    Operand: index into the CodeObject's ``names`` list, which gives the
    variable name as a string.

    Example: If names = ["x", "y"], then STORE_NAME 0 pops the top value
    and stores it as variable "x".

    JVM equivalent: ``putstatic`` (for class-level fields).
    CPython equivalent: ``STORE_NAME`` (identical!).
    """

    LOAD_NAME = 0x11
    """Push the value of a named variable onto the stack.

    Operand: index into the CodeObject's ``names`` list.

    If the variable hasn't been defined yet, this is a runtime error —
    just like getting a NameError in Python or an "undefined variable"
    error in Ruby.

    JVM equivalent: ``getstatic``.
    CPython equivalent: ``LOAD_NAME``.
    """

    STORE_LOCAL = 0x12
    """Pop the top of stack and store it in a local variable slot.

    Operand: integer index of the local slot.

    Local slots are a flat array of values, indexed by number. Inside a
    function, the compiler assigns each local variable a slot number
    (0, 1, 2, ...). This is faster than dictionary lookup because it's
    just an array index operation — O(1) with minimal overhead.

    JVM equivalent: ``istore``, ``astore`` (store int/reference to local).
    """

    LOAD_LOCAL = 0x13
    """Push the value from a local variable slot onto the stack.

    Operand: integer index of the local slot.

    JVM equivalent: ``iload``, ``aload`` (load int/reference from local).
    """

    # -- Arithmetic Operations (0x2_) --------------------------------------
    # These pop two operands, perform a math operation, and push the result.
    #
    # IMPORTANT: The order matters! For non-commutative operations like
    # subtraction and division, the *first* value pushed is the left operand.
    # So to compute 10 - 3:
    #     PUSH 10    ← pushed first, so it's deeper in the stack
    #     PUSH 3     ← pushed second, so it's on top
    #     SUB        ← pops 3 (top=b), pops 10 (second=a), pushes a - b = 7
    #
    # This "a then b then operate" order is the standard convention for
    # stack-based VMs. The JVM, CLR, and CPython all do it this way.

    ADD = 0x20
    """Pop two values, push their sum.

    Supports both integers and strings (concatenation), just like how
    Python's ``+`` works on both numbers and strings. Real VMs often have
    separate opcodes for integer add vs float add vs string concat, but
    we keep it simple with dynamic typing.
    """

    SUB = 0x21
    """Pop two values, push their difference (a - b).

    'a' is the value pushed first (deeper in stack).
    'b' is the value pushed second (top of stack).
    """

    MUL = 0x22
    """Pop two values, push their product."""

    DIV = 0x23
    """Pop two values, push their quotient (a / b).

    Uses integer division (//). Raises VMError if b is zero — division by
    zero is a runtime error in virtually every language and VM.
    """

    # -- Comparison Operations (0x3_) --------------------------------------
    # These pop two values, compare them, and push a boolean result.
    # We represent booleans as integers: 1 for true, 0 for false.
    #
    # Why integers instead of Python's True/False? Because this VM is
    # language-agnostic. Not all languages have a boolean type — C uses
    # integers, and even the JVM represents booleans as ints internally.
    # Using 1/0 keeps things uniform.

    CMP_EQ = 0x30
    """Pop two values, push 1 if they are equal, 0 otherwise.

    JVM equivalent: ``if_icmpeq`` (though JVM branches directly rather
    than pushing a boolean).
    """

    CMP_LT = 0x31
    """Pop two values, push 1 if a < b, 0 otherwise.

    Same operand ordering as arithmetic: 'a' is pushed first (deeper),
    'b' is pushed second (top).
    """

    CMP_GT = 0x32
    """Pop two values, push 1 if a > b, 0 otherwise."""

    # -- Control Flow (0x4_) -----------------------------------------------
    # These change the program counter (PC) to alter which instruction
    # executes next. Without control flow, programs would be purely linear —
    # no loops, no if-statements, no interesting behavior.
    #
    # The "jump" metaphor: normally the PC advances one instruction at a
    # time (like reading a book page by page). A jump says "skip to page X"
    # or "go back to page Y." Conditional jumps say "skip to page X *only if*
    # some condition is true."

    JUMP = 0x40
    """Unconditional jump: set PC to the operand value.

    Operand: the instruction index to jump to.

    This is like a ``goto`` statement. It's the building block for loops:
        0: LOAD_CONST ...
        1: PRINT
        2: JUMP 0       ← infinite loop!

    JVM equivalent: ``goto``.
    """

    JUMP_IF_FALSE = 0x41
    """Conditional jump: pop top of stack, jump if it's falsy (0).

    Operand: the instruction index to jump to if the value is falsy.

    "Falsy" means 0, None, or empty string — the standard falsy values.
    If the value is truthy, execution continues to the next instruction.

    This is how if-statements and while-loops are compiled:
        if x > 5:         →  LOAD_NAME x / LOAD_CONST 5 / CMP_GT
            do_something   →  JUMP_IF_FALSE <past the body>
                           →  <body instructions>

    JVM equivalent: ``ifeq`` (jump if zero).
    """

    JUMP_IF_TRUE = 0x42
    """Conditional jump: pop top of stack, jump if it's truthy (non-zero).

    Operand: the instruction index to jump to if the value is truthy.

    Less commonly used than JUMP_IF_FALSE, but handy for short-circuit
    evaluation of ``or`` expressions.

    JVM equivalent: ``ifne`` (jump if not zero).
    """

    # -- Function Operations (0x5_) ----------------------------------------
    # Functions are the backbone of structured programming. Our VM supports
    # them through a call stack (just like real hardware).
    #
    # When you CALL a function, the VM saves its current state (PC, locals)
    # onto a call stack, then jumps to the function's code. When the function
    # RETURNs, the VM restores the saved state and continues where it left off.

    CALL = 0x50
    """Call a function.

    Operand: the name of the function (index into names pool).

    The VM looks up the function in its variables dict (where it was stored
    via STORE_NAME), saves the current execution state on the call stack,
    and begins executing the function's CodeObject.

    JVM equivalent: ``invokevirtual``, ``invokestatic``.
    """

    RETURN = 0x51
    """Return from a function.

    Pops the call stack to restore the caller's state. If there's a value
    on top of the current stack, it becomes the return value and is pushed
    onto the caller's stack.

    JVM equivalent: ``ireturn``, ``areturn``.
    """

    # -- I/O Operations (0x6_) ---------------------------------------------

    PRINT = 0x60
    """Pop the top of stack and print it.

    The output is captured in the VM's ``output`` list so it can be
    inspected by tests and the pipeline visualizer without actually writing
    to stdout.
    """

    # -- VM Control (0xF_) -------------------------------------------------

    HALT = 0xFF
    """Stop execution immediately.

    Every program should end with HALT. If the PC runs past the end of
    the instruction list without hitting HALT, the VM stops automatically
    (rather than crashing), but explicit HALT is good practice — it makes
    the program's end point clear.

    JVM equivalent: There isn't one — JVM methods just ``return``. But our
    VM is simpler and runs a flat list of instructions, so we need an
    explicit "stop" signal.
    """


# =========================================================================
# Data Structures
# =========================================================================


@dataclass
class Instruction:
    """A single VM instruction: an opcode plus an optional operand.

    Think of this as one line of assembly language:

        ADD                → opcode=ADD, operand=None
        LOAD_CONST 0       → opcode=LOAD_CONST, operand=0
        STORE_NAME 1       → opcode=STORE_NAME, operand=1

    Some instructions (like ADD, POP, HALT) don't need an operand — they
    operate purely on what's already on the stack. Others (like LOAD_CONST,
    JUMP) need an operand to know *which* constant to load or *where* to jump.

    In a real bytecode format, this would be encoded as raw bytes:
        [opcode_byte] [operand_bytes...]

    We use a Python dataclass for clarity, but the concept is identical.
    """

    opcode: OpCode
    """The operation to perform."""

    operand: int | str | None = None
    """Optional data for the operation.

    - For LOAD_CONST: index into the constants pool (int).
    - For STORE_NAME/LOAD_NAME: index into the names pool (int).
    - For STORE_LOCAL/LOAD_LOCAL: local slot index (int).
    - For JUMP/JUMP_IF_*: target instruction index (int).
    - For CALL: index into the names pool (int).
    - For stack/arithmetic ops: None (not used).
    """

    def __repr__(self) -> str:
        """Produce a human-readable representation like 'LOAD_CONST 0'."""
        if self.operand is not None:
            return f"Instruction({self.opcode.name}, {self.operand!r})"
        return f"Instruction({self.opcode.name})"


@dataclass
class CodeObject:
    """A compiled unit of code — the bytecode equivalent of a source file.

    This is our version of Java's ``.class`` file or Python's ``code`` object.
    It bundles together everything the VM needs to execute a piece of code:

    1. **instructions** — The ordered list of operations to perform.
    2. **constants** — A pool of literal values (numbers, strings) referenced
       by LOAD_CONST instructions. Instead of embedding "42" directly in
       the instruction stream, we store it here and reference it by index.
       This is more efficient (the constant is stored once even if used many
       times) and mirrors how real bytecode formats work.
    3. **names** — A pool of identifier strings (variable names, function
       names) referenced by STORE_NAME/LOAD_NAME/CALL instructions. Same
       idea as the constants pool but for names.

    **Why pools?**
    Real bytecode formats use constant pools extensively. The JVM's constant
    pool stores strings, class names, method signatures, and numeric
    literals. Our two pools (constants + names) are a simplified version
    of the same idea.

    **Example:**
    To represent ``x = 42``:
        constants = [42]
        names = ["x"]
        instructions = [
            Instruction(LOAD_CONST, 0),   # Push constants[0] → 42
            Instruction(STORE_NAME, 0),   # Pop into names[0] → "x"
            Instruction(HALT),
        ]
    """

    instructions: list[Instruction]
    """The sequence of instructions to execute, in order."""

    constants: list[int | float | str] = field(default_factory=list)
    """The constants pool — literal values referenced by index.

    Index 0 is the first constant, index 1 is the second, etc.
    LOAD_CONST instructions reference this pool by index.
    """

    names: list[str] = field(default_factory=list)
    """The names pool — variable/function names referenced by index.

    STORE_NAME, LOAD_NAME, and CALL instructions reference this pool
    by index to find the actual string name.
    """


@dataclass
class VMTrace:
    """A snapshot of one execution step — the VM's "black box recorder."

    Every time the VM executes an instruction, it produces a VMTrace
    capturing the complete state before and after. This serves two purposes:

    1. **Debugging** — You can replay the entire execution step by step,
       seeing exactly what happened to the stack, variables, and output
       at each point.

    2. **Visualization** — The pipeline visualizer (a future component)
       can animate the VM's execution, showing values flowing onto and
       off of the stack, variables changing, etc.

    Think of it like a flight recorder (black box) on an airplane — it
    records everything so you can reconstruct what happened.
    """

    pc: int
    """The program counter *before* this instruction executed.

    This tells you which instruction in the CodeObject was being executed.
    """

    instruction: Instruction
    """The instruction that was executed in this step."""

    stack_before: list[Any]
    """A snapshot of the stack before the instruction ran.

    This is a copy, not a reference, so it won't change as execution
    continues.
    """

    stack_after: list[Any]
    """A snapshot of the stack after the instruction ran."""

    variables: dict[str, Any]
    """A snapshot of all named variables after the instruction ran."""

    output: str | None = None
    """If this instruction was PRINT, the string that was printed.

    None for all other instructions.
    """

    description: str = ""
    """A human-readable explanation of what this step did.

    Examples:
        "Push constant 42 onto the stack"
        "Pop 3 and 7, push sum 10"
        "Store 42 into variable 'x'"
    """


@dataclass
class CallFrame:
    """A saved execution context for function calls.

    When you call a function, the VM needs to remember where it was so it
    can come back after the function returns. A CallFrame saves:

    - The return address (which instruction to resume at)
    - The caller's local variables
    - The caller's stack state

    This is exactly what real CPUs do with their hardware call stack — the
    ``call`` instruction pushes a return address, and ``ret`` pops it.
    Our CallFrame is a richer version that also saves local variable state.

    The collection of all active CallFrames is the **call stack** — the same
    call stack you see in debugger backtraces and error stack traces.
    """

    return_address: int
    """The PC value to restore when the function returns."""

    saved_variables: dict[str, Any]
    """The caller's named variables, saved for restoration."""

    saved_locals: list[Any]
    """The caller's local variable slots, saved for restoration."""


# =========================================================================
# Errors
# =========================================================================


class VMError(Exception):
    """Base class for all virtual machine runtime errors.

    Just as the JVM throws ``java.lang.RuntimeException`` and Python raises
    ``RuntimeError``, our VM raises VMError when something goes wrong during
    execution — stack underflow, division by zero, undefined variables, etc.
    """


class StackUnderflowError(VMError):
    """Raised when an operation tries to pop from an empty stack.

    This is the VM equivalent of a segfault — something tried to read data
    that isn't there. It usually means the bytecode is malformed (a compiler
    bug) rather than a user program error.
    """


class UndefinedNameError(VMError):
    """Raised when code tries to read a variable that hasn't been defined.

    This is our equivalent of Python's ``NameError`` or JavaScript's
    ``ReferenceError``.
    """


class DivisionByZeroError(VMError):
    """Raised when code attempts to divide by zero.

    Every language and VM treats this as an error. The JVM throws
    ``ArithmeticException``, Python raises ``ZeroDivisionError``,
    and we raise ``DivisionByZeroError``.
    """


class InvalidOpcodeError(VMError):
    """Raised when the VM encounters an opcode it doesn't recognize.

    This would happen if the bytecode is corrupted or was produced by
    a buggy compiler.
    """


class InvalidOperandError(VMError):
    """Raised when an instruction's operand is out of bounds.

    For example, LOAD_CONST 99 when the constants pool only has 3 entries.
    """


# =========================================================================
# The Virtual Machine
# =========================================================================


class VirtualMachine:
    """A general-purpose stack-based bytecode interpreter.

    This is the heart of our computing stack — the component that actually
    *runs* programs. It takes a CodeObject (compiled bytecode) and executes
    it instruction by instruction, maintaining:

    - **stack** — The operand stack, where all computation happens.
    - **variables** — Named variable storage (like global scope).
    - **locals** — Indexed local variable slots (like function scope).
    - **pc** — The program counter, pointing to the current instruction.
    - **call_stack** — Saved contexts for function calls.
    - **output** — Captured print output for testing and visualization.

    **The Fetch-Decode-Execute Cycle:**
    Like every processor (real or virtual), our VM runs in a loop:

    1. **Fetch** — Read the instruction at ``pc``.
    2. **Decode** — Look at the opcode to determine what to do.
    3. **Execute** — Perform the operation (push, pop, add, jump, etc.).
    4. **Advance** — Move ``pc`` to the next instruction (unless we jumped).
    5. **Repeat** — Go back to step 1.

    This is the *exact same cycle* that real CPUs use, just implemented in
    software rather than silicon.

    **Usage:**

        >>> code = CodeObject(
        ...     instructions=[
        ...         Instruction(OpCode.LOAD_CONST, 0),
        ...         Instruction(OpCode.LOAD_CONST, 1),
        ...         Instruction(OpCode.ADD),
        ...         Instruction(OpCode.PRINT),
        ...         Instruction(OpCode.HALT),
        ...     ],
        ...     constants=[3, 4],
        ... )
        >>> vm = VirtualMachine()
        >>> traces = vm.execute(code)
        >>> vm.output
        ['7']
    """

    def __init__(self) -> None:
        """Initialize a fresh VM with empty state.

        Every execution should use a fresh VM (or call ``reset()``), just
        as the JVM creates a fresh runtime environment for each program.
        """
        self.stack: list[Any] = []
        """The operand stack — where all values live during computation."""

        self.variables: dict[str, Any] = {}
        """Named variable storage — like a global scope dictionary."""

        self.locals: list[Any] = []
        """Local variable slots — a flat array indexed by number.

        Inside functions, local variables are stored here by index rather
        than by name, for performance (array lookup is faster than dict
        lookup). The compiler assigns each local variable a slot number.
        """

        self.pc: int = 0
        """The program counter — index of the next instruction to execute.

        This is the VM's "read head," pointing to where we are in the
        instruction stream. It advances by 1 after each instruction,
        unless a jump changes it.
        """

        self.halted: bool = False
        """Whether the VM has stopped execution.

        Set to True by the HALT instruction or when the PC runs past the
        end of the instruction list.
        """

        self.output: list[str] = []
        """Captured print output.

        Instead of writing directly to stdout, PRINT instructions append
        their output here. This makes testing easy (just check vm.output)
        and enables the pipeline visualizer to display output.
        """

        self.call_stack: list[CallFrame] = []
        """The call stack — saved contexts for function calls.

        Each time CALL is executed, a CallFrame is pushed here. Each time
        RETURN is executed, a CallFrame is popped and its state restored.
        """

    def reset(self) -> None:
        """Reset the VM to its initial state.

        Call this between executions if you want to reuse the same VM
        instance. Equivalent to creating a new VirtualMachine().
        """
        self.stack = []
        self.variables = {}
        self.locals = []
        self.pc = 0
        self.halted = False
        self.output = []
        self.call_stack = []

    # -----------------------------------------------------------------
    # Public API
    # -----------------------------------------------------------------

    def execute(self, code: CodeObject) -> list[VMTrace]:
        """Execute a complete CodeObject, returning a trace of every step.

        This is the main entry point. It runs the fetch-decode-execute cycle
        until the program HALTs or the PC goes past the last instruction.

        Parameters
        ----------
        code : CodeObject
            The compiled bytecode to execute.

        Returns
        -------
        list[VMTrace]
            A trace entry for every instruction that was executed, in order.
            This is the complete execution history — invaluable for debugging
            and visualization.

        Raises
        ------
        VMError
            If a runtime error occurs (stack underflow, division by zero,
            undefined variable, etc.).

        Example
        -------
        >>> code = assemble_code(
        ...     [Instruction(OpCode.LOAD_CONST, 0), Instruction(OpCode.HALT)],
        ...     constants=[42],
        ... )
        >>> vm = VirtualMachine()
        >>> traces = vm.execute(code)
        >>> traces[0].stack_after
        [42]
        """
        traces: list[VMTrace] = []

        while not self.halted and self.pc < len(code.instructions):
            trace = self.step(code)
            traces.append(trace)

        return traces

    def step(self, code: CodeObject) -> VMTrace:
        """Execute one instruction and return a trace of what happened.

        This is the single-step entry point, useful for debuggers and
        step-through visualization. It performs exactly one iteration of
        the fetch-decode-execute cycle.

        Parameters
        ----------
        code : CodeObject
            The CodeObject being executed.

        Returns
        -------
        VMTrace
            A snapshot of the VM state before and after this instruction.

        Raises
        ------
        VMError
            If the instruction causes a runtime error.
        """
        # -- Fetch --
        instruction = code.instructions[self.pc]
        pc_before = self.pc
        stack_before = list(self.stack)  # snapshot (copy)

        # -- Decode & Execute --
        output_value = self._dispatch(instruction, code)

        # -- Build trace --
        description = self._describe(instruction, code, stack_before)
        trace = VMTrace(
            pc=pc_before,
            instruction=instruction,
            stack_before=stack_before,
            stack_after=list(self.stack),  # snapshot after
            variables=dict(self.variables),  # snapshot
            output=output_value,
            description=description,
        )

        return trace

    # -----------------------------------------------------------------
    # The Dispatch Table (Decode + Execute)
    # -----------------------------------------------------------------

    def _dispatch(self, instruction: Instruction, code: CodeObject) -> str | None:
        """Decode and execute a single instruction.

        This is the classic "big switch" at the heart of every interpreter.
        The JVM's interpreter loop, CPython's ceval.c, and Ruby's YARV all
        have one of these — a giant match/switch statement that handles
        every possible opcode.

        We use Python's ``match`` statement (introduced in 3.10), which is
        the modern Pythonic equivalent of C's ``switch``.

        Parameters
        ----------
        instruction : Instruction
            The instruction to execute.
        code : CodeObject
            The containing CodeObject (needed for constants/names pools).

        Returns
        -------
        str or None
            If the instruction was PRINT, returns the printed string.
            Otherwise returns None.
        """
        output_value: str | None = None

        match instruction.opcode:
            # == Stack Operations ==========================================

            case OpCode.LOAD_CONST:
                # Push a constant from the pool onto the stack.
                index = self._require_operand(instruction)
                if not isinstance(index, int) or index < 0 or index >= len(code.constants):
                    raise InvalidOperandError(
                        f"LOAD_CONST operand {index} is out of range "
                        f"(constants pool has {len(code.constants)} entries)"
                    )
                value = code.constants[index]
                self.stack.append(value)
                self.pc += 1

            case OpCode.POP:
                # Discard the top of the stack.
                self._pop()
                self.pc += 1

            case OpCode.DUP:
                # Duplicate the top of the stack.
                if len(self.stack) == 0:
                    raise StackUnderflowError(
                        "DUP requires at least one value on the stack"
                    )
                self.stack.append(self.stack[-1])
                self.pc += 1

            # == Variable Operations =======================================

            case OpCode.STORE_NAME:
                # Pop the top value and store it in a named variable.
                index = self._require_operand(instruction)
                if not isinstance(index, int) or index < 0 or index >= len(code.names):
                    raise InvalidOperandError(
                        f"STORE_NAME operand {index} is out of range "
                        f"(names pool has {len(code.names)} entries)"
                    )
                name = code.names[index]
                value = self._pop()
                self.variables[name] = value
                self.pc += 1

            case OpCode.LOAD_NAME:
                # Push the value of a named variable onto the stack.
                index = self._require_operand(instruction)
                if not isinstance(index, int) or index < 0 or index >= len(code.names):
                    raise InvalidOperandError(
                        f"LOAD_NAME operand {index} is out of range "
                        f"(names pool has {len(code.names)} entries)"
                    )
                name = code.names[index]
                if name not in self.variables:
                    raise UndefinedNameError(
                        f"Variable '{name}' is not defined"
                    )
                self.stack.append(self.variables[name])
                self.pc += 1

            case OpCode.STORE_LOCAL:
                # Pop the top value and store it in a local slot.
                index = self._require_operand(instruction)
                if not isinstance(index, int) or index < 0:
                    raise InvalidOperandError(
                        f"STORE_LOCAL operand must be a non-negative integer, "
                        f"got {index}"
                    )
                value = self._pop()
                # Extend the locals list if needed (auto-grow).
                while len(self.locals) <= index:
                    self.locals.append(None)
                self.locals[index] = value
                self.pc += 1

            case OpCode.LOAD_LOCAL:
                # Push the value from a local slot onto the stack.
                index = self._require_operand(instruction)
                if not isinstance(index, int) or index < 0:
                    raise InvalidOperandError(
                        f"LOAD_LOCAL operand must be a non-negative integer, "
                        f"got {index}"
                    )
                if index >= len(self.locals):
                    raise InvalidOperandError(
                        f"LOAD_LOCAL slot {index} has not been initialized "
                        f"(only {len(self.locals)} slots exist)"
                    )
                self.stack.append(self.locals[index])
                self.pc += 1

            # == Arithmetic ================================================

            case OpCode.ADD:
                b = self._pop()
                a = self._pop()
                self.stack.append(a + b)
                self.pc += 1

            case OpCode.SUB:
                b = self._pop()
                a = self._pop()
                self.stack.append(a - b)
                self.pc += 1

            case OpCode.MUL:
                b = self._pop()
                a = self._pop()
                self.stack.append(a * b)
                self.pc += 1

            case OpCode.DIV:
                b = self._pop()
                a = self._pop()
                if b == 0:
                    raise DivisionByZeroError("Division by zero")
                self.stack.append(a // b)
                self.pc += 1

            # == Comparison ================================================

            case OpCode.CMP_EQ:
                b = self._pop()
                a = self._pop()
                self.stack.append(1 if a == b else 0)
                self.pc += 1

            case OpCode.CMP_LT:
                b = self._pop()
                a = self._pop()
                self.stack.append(1 if a < b else 0)
                self.pc += 1

            case OpCode.CMP_GT:
                b = self._pop()
                a = self._pop()
                self.stack.append(1 if a > b else 0)
                self.pc += 1

            # == Control Flow ==============================================

            case OpCode.JUMP:
                target = self._require_operand(instruction)
                if not isinstance(target, int):
                    raise InvalidOperandError(
                        f"JUMP operand must be an integer, got {target!r}"
                    )
                self.pc = target  # Don't increment — we're jumping!

            case OpCode.JUMP_IF_FALSE:
                target = self._require_operand(instruction)
                if not isinstance(target, int):
                    raise InvalidOperandError(
                        f"JUMP_IF_FALSE operand must be an integer, got {target!r}"
                    )
                condition = self._pop()
                if self._is_falsy(condition):
                    self.pc = target
                else:
                    self.pc += 1

            case OpCode.JUMP_IF_TRUE:
                target = self._require_operand(instruction)
                if not isinstance(target, int):
                    raise InvalidOperandError(
                        f"JUMP_IF_TRUE operand must be an integer, got {target!r}"
                    )
                condition = self._pop()
                if not self._is_falsy(condition):
                    self.pc = target
                else:
                    self.pc += 1

            # == Functions =================================================

            case OpCode.CALL:
                name_index = self._require_operand(instruction)
                if not isinstance(name_index, int) or name_index < 0 or name_index >= len(code.names):
                    raise InvalidOperandError(
                        f"CALL operand {name_index} is out of range "
                        f"(names pool has {len(code.names)} entries)"
                    )
                func_name = code.names[name_index]
                if func_name not in self.variables:
                    raise UndefinedNameError(
                        f"Function '{func_name}' is not defined"
                    )
                func_code = self.variables[func_name]
                if not isinstance(func_code, CodeObject):
                    raise VMError(
                        f"'{func_name}' is not callable (expected CodeObject, "
                        f"got {type(func_code).__name__})"
                    )

                # Save current execution context.
                frame = CallFrame(
                    return_address=self.pc + 1,
                    saved_variables=dict(self.variables),
                    saved_locals=list(self.locals),
                )
                self.call_stack.append(frame)

                # Jump to the function's first instruction.
                # We set pc to 0 because the function has its own CodeObject,
                # but we need to re-enter the execute loop with the new code.
                # For simplicity, we execute the function inline by recursion.
                self.locals = []
                saved_pc = self.pc
                self.pc = 0
                while not self.halted and self.pc < len(func_code.instructions):
                    current_instr = func_code.instructions[self.pc]
                    if current_instr.opcode == OpCode.RETURN:
                        break
                    self._dispatch(current_instr, func_code)

                # Restore caller context.
                frame = self.call_stack.pop()
                self.pc = frame.return_address
                self.locals = frame.saved_locals
                # Note: variables persist across call/return (they're "global").

            case OpCode.RETURN:
                # Return from a function call.
                # If there's a call frame, restore the caller's state.
                if self.call_stack:
                    frame = self.call_stack.pop()
                    self.pc = frame.return_address
                    self.locals = frame.saved_locals
                else:
                    # No call frame — RETURN at the top level acts like HALT.
                    self.halted = True

            # == I/O =======================================================

            case OpCode.PRINT:
                value = self._pop()
                output_str = str(value)
                self.output.append(output_str)
                output_value = output_str
                self.pc += 1

            # == VM Control ================================================

            case OpCode.HALT:
                self.halted = True
                # Don't advance PC — execution is done.

            case _:
                raise InvalidOpcodeError(
                    f"Unknown opcode: {instruction.opcode!r}"
                )

        return output_value

    # -----------------------------------------------------------------
    # Helper Methods
    # -----------------------------------------------------------------

    def _pop(self) -> Any:
        """Pop and return the top value from the stack.

        Raises StackUnderflowError if the stack is empty. This is a safety
        net — well-compiled bytecode should never underflow, but bugs happen.
        """
        if len(self.stack) == 0:
            raise StackUnderflowError(
                "Cannot pop from an empty stack — possible compiler bug"
            )
        return self.stack.pop()

    def _require_operand(self, instruction: Instruction) -> Any:
        """Get the operand from an instruction, raising an error if missing.

        Some instructions (LOAD_CONST, JUMP, etc.) require an operand.
        This helper ensures one was provided.
        """
        if instruction.operand is None:
            raise InvalidOperandError(
                f"{instruction.opcode.name} requires an operand but none "
                f"was provided"
            )
        return instruction.operand

    @staticmethod
    def _is_falsy(value: Any) -> bool:
        """Determine whether a value is "falsy" for conditional jumps.

        Our falsy values are:
        - 0 (integer zero)
        - None
        - "" (empty string)

        Everything else is truthy. This is a common convention — Python,
        JavaScript, and Ruby all have similar truthiness rules (though the
        exact details vary).
        """
        return value in (0, None, "")

    def _describe(
        self,
        instruction: Instruction,
        code: CodeObject,
        stack_before: list[Any],
    ) -> str:
        """Generate a human-readable description of what an instruction did.

        These descriptions are meant for complete beginners — they explain
        not just *what* happened but *why* in plain English.
        """
        op = instruction.opcode

        match op:
            case OpCode.LOAD_CONST:
                idx = instruction.operand
                val = code.constants[idx] if isinstance(idx, int) and 0 <= idx < len(code.constants) else "?"
                return f"Push constant {val!r} onto the stack"

            case OpCode.POP:
                val = stack_before[-1] if stack_before else "?"
                return f"Discard top of stack ({val!r})"

            case OpCode.DUP:
                val = stack_before[-1] if stack_before else "?"
                return f"Duplicate top of stack ({val!r})"

            case OpCode.STORE_NAME:
                idx = instruction.operand
                name = code.names[idx] if isinstance(idx, int) and 0 <= idx < len(code.names) else "?"
                val = stack_before[-1] if stack_before else "?"
                return f"Store {val!r} into variable '{name}'"

            case OpCode.LOAD_NAME:
                idx = instruction.operand
                name = code.names[idx] if isinstance(idx, int) and 0 <= idx < len(code.names) else "?"
                return f"Push variable '{name}' onto the stack"

            case OpCode.STORE_LOCAL:
                idx = instruction.operand
                val = stack_before[-1] if stack_before else "?"
                return f"Store {val!r} into local slot {idx}"

            case OpCode.LOAD_LOCAL:
                return f"Push local slot {instruction.operand} onto the stack"

            case OpCode.ADD:
                if len(stack_before) >= 2:
                    a, b = stack_before[-2], stack_before[-1]
                    return f"Pop {b!r} and {a!r}, push sum {a + b!r}"
                return "Add top two stack values"

            case OpCode.SUB:
                if len(stack_before) >= 2:
                    a, b = stack_before[-2], stack_before[-1]
                    return f"Pop {b!r} and {a!r}, push difference {a - b!r}"
                return "Subtract top two stack values"

            case OpCode.MUL:
                if len(stack_before) >= 2:
                    a, b = stack_before[-2], stack_before[-1]
                    return f"Pop {b!r} and {a!r}, push product {a * b!r}"
                return "Multiply top two stack values"

            case OpCode.DIV:
                if len(stack_before) >= 2:
                    a, b = stack_before[-2], stack_before[-1]
                    if b != 0:
                        return f"Pop {b!r} and {a!r}, push quotient {a // b!r}"
                    return f"Pop {b!r} and {a!r}, DIVISION BY ZERO"
                return "Divide top two stack values"

            case OpCode.CMP_EQ:
                if len(stack_before) >= 2:
                    a, b = stack_before[-2], stack_before[-1]
                    result = 1 if a == b else 0
                    return f"Compare {a!r} == {b!r} → {result}"
                return "Compare top two stack values for equality"

            case OpCode.CMP_LT:
                if len(stack_before) >= 2:
                    a, b = stack_before[-2], stack_before[-1]
                    result = 1 if a < b else 0
                    return f"Compare {a!r} < {b!r} → {result}"
                return "Compare top two stack values (less than)"

            case OpCode.CMP_GT:
                if len(stack_before) >= 2:
                    a, b = stack_before[-2], stack_before[-1]
                    result = 1 if a > b else 0
                    return f"Compare {a!r} > {b!r} → {result}"
                return "Compare top two stack values (greater than)"

            case OpCode.JUMP:
                return f"Jump to instruction {instruction.operand}"

            case OpCode.JUMP_IF_FALSE:
                val = stack_before[-1] if stack_before else "?"
                return f"Pop {val!r}, jump to {instruction.operand} if falsy"

            case OpCode.JUMP_IF_TRUE:
                val = stack_before[-1] if stack_before else "?"
                return f"Pop {val!r}, jump to {instruction.operand} if truthy"

            case OpCode.CALL:
                idx = instruction.operand
                name = code.names[idx] if isinstance(idx, int) and 0 <= idx < len(code.names) else "?"
                return f"Call function '{name}'"

            case OpCode.RETURN:
                return "Return from function"

            case OpCode.PRINT:
                val = stack_before[-1] if stack_before else "?"
                return f"Print {val!r}"

            case OpCode.HALT:
                return "Halt execution"

            case _:
                return f"Unknown opcode {op!r}"


# =========================================================================
# Helper Functions
# =========================================================================


def assemble_code(
    instructions: list[Instruction],
    constants: list[int | float | str] | None = None,
    names: list[str] | None = None,
) -> CodeObject:
    """Convenience function to build a CodeObject from parts.

    This is a simple "assembler" — it takes human-readable instructions and
    packages them into a CodeObject that the VM can execute. In a real system,
    this would be done by a compiler, but for testing and experimentation,
    hand-assembling is invaluable.

    Parameters
    ----------
    instructions : list[Instruction]
        The instruction sequence to execute.
    constants : list, optional
        The constants pool. Defaults to an empty list.
    names : list[str], optional
        The names pool. Defaults to an empty list.

    Returns
    -------
    CodeObject
        A complete, ready-to-execute code object.

    Example
    -------
    >>> code = assemble_code(
    ...     instructions=[
    ...         Instruction(OpCode.LOAD_CONST, 0),
    ...         Instruction(OpCode.PRINT),
    ...         Instruction(OpCode.HALT),
    ...     ],
    ...     constants=[42],
    ... )
    >>> vm = VirtualMachine()
    >>> vm.execute(code)  # doctest: +SKIP
    >>> vm.output
    ['42']
    """
    return CodeObject(
        instructions=instructions,
        constants=constants if constants is not None else [],
        names=names if names is not None else [],
    )
