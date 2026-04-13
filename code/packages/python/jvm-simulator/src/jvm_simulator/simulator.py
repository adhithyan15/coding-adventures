"""Java Virtual Machine (JVM) Bytecode Simulator -- a typed stack machine.

=== What is the JVM? ===

The Java Virtual Machine was introduced by Sun Microsystems in 1995 alongside
the Java programming language. Its revolutionary promise was "write once, run
anywhere" -- compile your source code to platform-independent bytecode, and any
machine with a JVM can execute it. This was a radical idea at the time: instead
of compiling to x86, SPARC, or PowerPC machine code, you target a *virtual*
instruction set that a software interpreter executes.

Today the JVM is the most widely deployed virtual machine in history. It runs
not just Java but also Kotlin (Android's primary language), Scala, Clojure,
Groovy, and JRuby. The HotSpot JVM uses just-in-time (JIT) compilation to
achieve near-native performance, making the "virtual" overhead nearly invisible.

=== Stack machine with typed opcodes ===

Like our bytecode VM and WASM, the JVM is a stack-based machine. But the JVM
differs in a crucial way: its opcodes are *typed*. Where our VM has a single
ADD instruction that works on whatever is on the stack, the JVM has separate
opcodes for each primitive type:

    Our VM:    ADD           <-- works on whatever's on the stack
    JVM:       iadd          <-- integer add
               ladd          <-- long add
               fadd          <-- float add
               dadd          <-- double add

Why typed opcodes? Because the JVM was designed for *verification*. Before
executing any bytecode, the JVM verifier checks that every operation receives
operands of the correct type. If you try to iadd a float and an int, the
verifier rejects the class file before it ever runs. This is how Java achieves
type safety at the bytecode level -- something our untyped VM cannot do.

The type prefix convention:
    i = int (32-bit signed integer)
    l = long (64-bit signed integer)
    f = float (32-bit IEEE 754)
    d = double (64-bit IEEE 754)
    a = reference (object pointer)

Our MVP implements only the "i" (integer) variants, which is enough to
demonstrate all the core concepts.

=== Variable-width bytecode encoding ===

Like WASM (and unlike RISC-V's fixed 32-bit instructions), JVM bytecode uses
variable-width encoding. Each instruction starts with a 1-byte opcode, followed
by zero or more operand bytes:

    Instruction      Encoding                Width
    ===============  ======================  =====
    iconst_0         0x03                    1 byte  (constant baked into opcode)
    iconst_1         0x04                    1 byte
    bipush 42        0x10 0x2A              2 bytes (opcode + signed byte)
    ldc #3           0x12 0x03              2 bytes (opcode + pool index)
    iload 5          0x15 0x05              2 bytes (opcode + local index)
    iload_0          0x1A                    1 byte  (shortcut for iload 0)
    istore 5         0x36 0x05              2 bytes (opcode + local index)
    istore_0         0x3B                    1 byte  (shortcut for istore 0)
    iadd             0x60                    1 byte
    goto +5          0xA7 0x00 0x05         3 bytes (opcode + 2-byte signed offset)
    if_icmpeq +3     0x9F 0x00 0x03        3 bytes (opcode + 2-byte signed offset)
    return           0xB1                    1 byte

The iconst_N shortcuts are a classic JVM optimization. Pushing small constants
(0 through 5) is so common that each gets its own single-byte opcode, saving
the extra byte that bipush would require. Similarly, iload_0 through iload_3
and istore_0 through istore_3 are shortcuts for the most commonly used local
variable slots (method parameters and the first few locals).

=== Branch offsets ===

JVM branch instructions (goto, if_icmpeq, if_icmpgt) use *relative* offsets
measured from the start of the branch instruction itself. So "goto +0" is an
infinite loop (jumps back to itself), "goto +3" skips over the goto's own
3 bytes to the next instruction, and "goto -5" jumps 5 bytes backward.

This is different from absolute addresses -- relative offsets mean the bytecode
is position-independent and can be loaded at any base address.

=== Comparison with WASM and our VM ===

    Feature        Our VM         WASM              JVM
    ===========    ===========    ==============    ==============
    Architecture   Stack          Stack             Stack
    Types          Untyped        Typed (i32/f64)   Typed (i/l/f/d)
    Variables      Named (dict)   Numbered slots    Numbered slots
    Constants      Pool index     i32.const imm     iconst_N + pool
    Encoding       Object-based   Variable bytes    Variable bytes
    Verification   None           Structured        Type-checking
    Control flow   JUMP/JUMP_IF   block/br          goto/if_icmp*

The JVM's closest relative in our project is WASM -- both use variable-width
bytecode with a stack and numbered local variables. The key difference is that
JVM opcodes encode the type in the mnemonic (iadd vs i32.add) and the JVM has
a richer constant pool mechanism.

=== The x = 1 + 2 program ===

Here is the simplest JVM program, showing how "x = 1 + 2" compiles:

    iconst_1          Push integer constant 1        stack: [1]
    iconst_2          Push integer constant 2        stack: [1, 2]
    iadd              Pop two ints, push sum          stack: [3]
    istore_0          Pop and store in local 0        stack: []  locals[0]=3
    return            Return void from method

This is nearly identical to the WASM version (i32.const 1, i32.const 2,
i32.add, local.set 0, end) -- the stack machine model is the same, only
the opcode names and encoding differ.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import IntEnum


# ---------------------------------------------------------------------------
# JVM Opcode definitions
# ---------------------------------------------------------------------------
# These are the REAL opcode byte values from the JVM specification.
# Each opcode is a single unsigned byte (0x00 to 0xFF). The JVM spec defines
# about 200 opcodes; we implement a minimal subset for integer arithmetic,
# local variables, control flow, and method return.


class JVMOpcode(IntEnum):
    """Real JVM opcode values from the JVM specification.

    Naming convention: the "i" prefix means "integer" (32-bit signed).
    The JVM has parallel opcodes for long (l), float (f), double (d),
    and reference (a) types, but we only implement integer operations.

    The opcode values here match the official JVM spec exactly, so you
    could compare them against `javap -c` output from a real Java compiler.
    """

    # --- Constant-pushing opcodes ---
    # iconst_N opcodes push small integer constants 0-5 directly.
    # These are single-byte instructions -- no operand needed because
    # the value is encoded in the opcode itself.
    ICONST_0 = 0x03  # Push int 0
    ICONST_1 = 0x04  # Push int 1
    ICONST_2 = 0x05  # Push int 2
    ICONST_3 = 0x06  # Push int 3
    ICONST_4 = 0x07  # Push int 4
    ICONST_5 = 0x08  # Push int 5

    # bipush pushes a signed byte value (-128 to 127) as an integer.
    # It is 2 bytes: opcode + signed byte operand.
    BIPUSH = 0x10

    # ldc loads a constant from the constant pool by index.
    # It is 2 bytes: opcode + unsigned byte index into the constant pool.
    LDC = 0x12

    # --- Local variable load opcodes ---
    # iload loads an int from an arbitrary local variable slot.
    # It is 2 bytes: opcode + unsigned byte slot index.
    ILOAD = 0x15

    # iload_N shortcuts for slots 0-3 (single byte, no operand).
    # These exist because the first 4 locals (method parameters + first
    # few local variables) are used far more frequently than others.
    ILOAD_0 = 0x1A
    ILOAD_1 = 0x1B
    ILOAD_2 = 0x1C
    ILOAD_3 = 0x1D

    # --- Local variable store opcodes ---
    # istore stores an int to an arbitrary local variable slot.
    # It is 2 bytes: opcode + unsigned byte slot index.
    ISTORE = 0x36

    # istore_N shortcuts for slots 0-3 (single byte, no operand).
    ISTORE_0 = 0x3B
    ISTORE_1 = 0x3C
    ISTORE_2 = 0x3D
    ISTORE_3 = 0x3E

    # --- Integer arithmetic opcodes ---
    # All arithmetic opcodes are single byte: pop two ints, push result.
    # Operand order: second-to-top is the left operand, top is the right.
    # So "push 5; push 3; isub" computes 5 - 3 = 2 (not 3 - 5).
    IADD = 0x60  # Pop b, pop a, push (a + b)
    ISUB = 0x64  # Pop b, pop a, push (a - b)
    IMUL = 0x68  # Pop b, pop a, push (a * b)
    IDIV = 0x6C  # Pop b, pop a, push (a // b) -- integer division

    # --- Control flow opcodes ---
    # All branch opcodes are 3 bytes: opcode + 2-byte signed offset.
    # The offset is relative to the PC of the branch instruction itself.
    IF_ICMPEQ = 0x9F  # Pop b, pop a; branch if a == b
    IF_ICMPGT = 0xA3  # Pop b, pop a; branch if a > b
    GOTO = 0xA7  # Unconditional branch

    # --- Return opcodes ---
    IRETURN = 0xAC  # Pop int and return it (we store it as return_value)
    RETURN = 0xB1  # Return void (halt execution)


# ---------------------------------------------------------------------------
# Trace dataclass
# ---------------------------------------------------------------------------


@dataclass
class JVMTrace:
    """A trace of one JVM instruction execution.

    Captures the complete state transition for a single step, allowing
    you to visualize execution. This is the JVM equivalent of WASM's
    WasmStepTrace -- it shows what the stack and locals looked like
    before and after the instruction executed.

    Example trace for iadd when stack was [1, 2]:
        JVMTrace(
            pc=4,
            opcode="iadd",
            stack_before=[1, 2],
            stack_after=[3],
            locals_snapshot=[None, None, ...],
            description="pop 2 and 1, push 3",
        )
    """

    pc: int
    opcode: str  # Mnemonic like "iadd", "iconst_1", etc.
    stack_before: list[int]
    stack_after: list[int]
    locals_snapshot: list[int | None]
    description: str


# ---------------------------------------------------------------------------
# JVM Simulator
# ---------------------------------------------------------------------------


class JVMSimulator:
    """Complete JVM bytecode simulator -- decoder, executor, and state.

    This is a standalone simulator (not wrapping the generic CPU class)
    because, like WASM, the JVM uses variable-width bytecode that doesn't
    fit the CPU class's fixed-width fetch cycle.

    State:
        - stack:      The operand stack (values pushed/popped by instructions)
        - locals:     Local variable array (numbered slots 0 through num_locals-1)
        - constants:  Constant pool (values loaded by the ldc instruction)
        - pc:         Program counter (byte offset into bytecode)
        - halted:     Whether execution has finished (RETURN or IRETURN)
        - return_value: Value returned by IRETURN (None if RETURN/void)

    Example: running x = 1 + 2

        >>> sim = JVMSimulator()
        >>> program = assemble_jvm(
        ...     (JVMOpcode.ICONST_1,),   # push 1
        ...     (JVMOpcode.ICONST_2,),   # push 2
        ...     (JVMOpcode.IADD,),       # pop 2 and 1, push 3
        ...     (JVMOpcode.ISTORE_0,),   # pop 3, store in local 0
        ...     (JVMOpcode.RETURN,),     # halt
        ... )
        >>> sim.load(program)
        >>> traces = sim.run()
        >>> sim.locals[0]
        3

    Step-by-step stack evolution:
        Step 0: iconst_1      stack: [] -> [1]
        Step 1: iconst_2      stack: [1] -> [1, 2]
        Step 2: iadd           stack: [1, 2] -> [3]
        Step 3: istore_0      stack: [3] -> []       locals[0] = 3
        Step 4: return         halt
    """

    def __init__(self) -> None:
        self.stack: list[int] = []
        self.locals: list[int | None] = [None] * 16
        self.constants: list[int | str] = []
        self.pc: int = 0
        self.halted: bool = False
        self.return_value: int | None = None
        self._bytecode: bytes = b""
        self._num_locals: int = 16

    def load(
        self,
        bytecode: bytes,
        constants: list[int | str] | None = None,
        num_locals: int = 16,
    ) -> None:
        """Load a JVM bytecode program.

        Resets all simulator state: stack, locals, PC, and halt flag.
        Optionally sets the constant pool and number of local variable slots.

        Args:
            bytecode: Raw JVM bytecode to execute.
            constants: Constant pool entries (used by the ldc instruction).
            num_locals: Number of local variable slots (default 16).
        """
        self._bytecode = bytecode
        self.constants = constants if constants is not None else []
        self._num_locals = num_locals
        self.stack = []
        self.locals = [None] * num_locals
        self.pc = 0
        self.halted = False
        self.return_value = None

    def step(self) -> JVMTrace:
        """Execute one JVM instruction and return a trace.

        The JVM execution cycle (similar to WASM):

            1. FETCH: Read the opcode byte at PC
            2. DECODE: Determine instruction width and read operand bytes
            3. EXECUTE: Perform the operation (push/pop stack, read/write locals)
            4. ADVANCE: Move PC forward by the instruction's byte width
               (unless the instruction is a taken branch, which sets PC directly)

        Returns:
            JVMTrace showing the instruction, stack before/after, etc.

        Raises:
            RuntimeError: If the simulator has halted or encounters an error.
        """
        if self.halted:
            msg = "JVM simulator has halted -- no more instructions to execute"
            raise RuntimeError(msg)

        if self.pc >= len(self._bytecode):
            msg = f"PC ({self.pc}) is past end of bytecode ({len(self._bytecode)} bytes)"
            raise RuntimeError(msg)

        # Snapshot state before execution
        stack_before = list(self.stack)

        # Fetch the opcode byte
        opcode_byte = self._bytecode[self.pc]

        # Decode and execute based on the opcode
        try:
            opcode = JVMOpcode(opcode_byte)
        except ValueError:
            msg = f"Unknown JVM opcode: 0x{opcode_byte:02X} at PC={self.pc}"
            raise RuntimeError(msg) from None

        trace = self._execute(opcode, stack_before)
        return trace

    def run(self, max_steps: int = 10000) -> list[JVMTrace]:
        """Execute until RETURN/IRETURN, returning all traces.

        Args:
            max_steps: Safety limit to prevent infinite loops.

        Returns:
            List of JVMTrace objects, one per instruction executed.
        """
        traces: list[JVMTrace] = []
        for _ in range(max_steps):
            if self.halted:
                break
            traces.append(self.step())
        return traces

    # -------------------------------------------------------------------
    # simulator-protocol conformance methods
    # -------------------------------------------------------------------

    def get_state(self) -> "JVMState":
        """Return a frozen snapshot of the current JVM simulator state.

        All mutable lists are converted to tuples so the result is a true
        immutable value.  The snapshot will not change even if the simulator
        continues executing after this call returns.

        This method satisfies the ``Simulator[JVMState]`` protocol from the
        ``simulator-protocol`` package.

        Returns
        -------
        JVMState:
            Frozen dataclass capturing: operand stack, locals, constants,
            program counter, halted flag, and return value.

        Examples
        --------
        >>> sim = JVMSimulator()
        >>> sim.load(bytes([0x04, 0xB1]))  # iconst_1, return
        >>> sim.run()
        [...]
        >>> state = sim.get_state()
        >>> state.halted
        True
        """
        from jvm_simulator.state import JVMState

        return JVMState(
            stack=tuple(self.stack),
            locals=tuple(self.locals),
            constants=tuple(self.constants),
            pc=self.pc,
            halted=self.halted,
            return_value=self.return_value,
        )

    def execute(
        self,
        program: bytes,
        max_steps: int = 100_000,
    ) -> "ExecutionResult[JVMState]":
        """Load program, run to RETURN/IRETURN or max_steps, return ExecutionResult.

        This is the protocol-conforming entry point for the
        ``Simulator[JVMState]`` protocol defined in the ``simulator-protocol``
        package.  It resets internal state, loads the program, runs the
        execution loop, and returns a rich result type.

        The existing ``load()`` and ``run()`` methods are unchanged — this
        method calls them internally and adapts the return value.

        Parameters
        ----------
        program:
            Raw JVM bytecode bytes.
        max_steps:
            Maximum instructions to execute before giving up (default 100,000).

        Returns
        -------
        ExecutionResult[JVMState]:
            - ``halted``: True if RETURN/IRETURN was reached.
            - ``steps``: total instructions executed.
            - ``final_state``: frozen ``JVMState`` snapshot at termination.
            - ``error``: None on clean halt; error string otherwise.
            - ``traces``: one ``StepTrace`` per instruction executed.

        Examples
        --------
        >>> sim = JVMSimulator()
        >>> from jvm_simulator.simulator import assemble_jvm, JVMOpcode
        >>> program = assemble_jvm(
        ...     (JVMOpcode.ICONST_1,),
        ...     (JVMOpcode.ICONST_2,),
        ...     (JVMOpcode.IADD,),
        ...     (JVMOpcode.ISTORE_0,),
        ...     (JVMOpcode.RETURN,),
        ... )
        >>> result = sim.execute(program)
        >>> result.ok
        True
        >>> result.final_state.locals[0]
        3
        """
        from simulator_protocol import ExecutionResult, StepTrace

        from jvm_simulator.state import JVMState

        # Reset state and load the program
        self.load(program)

        step_traces: list[StepTrace] = []
        steps = 0
        error: str | None = None

        try:
            while not self.halted and steps < max_steps:
                pc_before = self.pc
                jvm_trace = self.step()
                step_traces.append(
                    StepTrace(
                        pc_before=pc_before,
                        pc_after=self.pc,
                        mnemonic=jvm_trace.opcode,
                        description=jvm_trace.description,
                    )
                )
                steps += 1
        except Exception as exc:
            error = str(exc)

        if error is None and not self.halted:
            error = f"max_steps ({max_steps}) exceeded"

        return ExecutionResult(
            halted=self.halted,
            steps=steps,
            final_state=self.get_state(),
            error=error,
            traces=step_traces,
        )

    def reset(self) -> None:
        """Reset all simulator state to initial values.

        Clears the stack, locals, constants, program counter, halted flag,
        return value, and bytecode buffer.  After ``reset()``, the simulator
        is in the same state as a freshly constructed ``JVMSimulator()``.

        This method satisfies the ``reset()`` requirement of the
        ``Simulator[JVMState]`` protocol.

        Examples
        --------
        >>> sim = JVMSimulator()
        >>> sim.load(bytes([0x04, 0xB1]))  # iconst_1, return
        >>> sim.run()
        [...]
        >>> sim.halted
        True
        >>> sim.reset()
        >>> sim.halted
        False
        >>> sim.stack
        []
        """
        self.stack = []
        self.locals = [None] * self._num_locals
        self.constants = []
        self.pc = 0
        self.halted = False
        self.return_value = None
        self._bytecode = b""

    # -------------------------------------------------------------------
    # Private: instruction dispatch and execution
    # -------------------------------------------------------------------

    def _execute(self, opcode: JVMOpcode, stack_before: list[int]) -> JVMTrace:
        """Dispatch to the appropriate handler for the given opcode.

        Each handler modifies self.stack and self.locals in place, advances
        self.pc by the instruction's width, and returns a JVMTrace.
        """
        pc = self.pc

        # --- iconst_N: push small integer constants (1 byte) ---
        if JVMOpcode.ICONST_0 <= opcode <= JVMOpcode.ICONST_5:
            value = opcode - JVMOpcode.ICONST_0
            self.stack.append(value)
            self.pc += 1
            return JVMTrace(
                pc=pc,
                opcode=opcode.name.lower(),
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=f"push {value}",
            )

        # --- bipush: push a signed byte value (2 bytes) ---
        if opcode == JVMOpcode.BIPUSH:
            # The operand is a signed byte (-128 to 127).
            # Python's bytes are unsigned (0-255), so we convert manually.
            raw = self._bytecode[self.pc + 1]
            value = raw if raw < 128 else raw - 256
            self.stack.append(value)
            self.pc += 2
            return JVMTrace(
                pc=pc,
                opcode="bipush",
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=f"push {value}",
            )

        # --- ldc: load from constant pool (2 bytes) ---
        if opcode == JVMOpcode.LDC:
            index = self._bytecode[self.pc + 1]
            if index >= len(self.constants):
                msg = f"Constant pool index {index} out of range (pool size: {len(self.constants)})"
                raise RuntimeError(msg)
            value = self.constants[index]
            if not isinstance(value, int):
                msg = f"ldc: constant pool entry {index} is not an integer: {value!r}"
                raise RuntimeError(msg)
            self.stack.append(value)
            self.pc += 2
            return JVMTrace(
                pc=pc,
                opcode="ldc",
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=f"push constant[{index}] = {value}",
            )

        # --- iload_N: load int from local slot N (1 byte) ---
        if JVMOpcode.ILOAD_0 <= opcode <= JVMOpcode.ILOAD_3:
            slot = opcode - JVMOpcode.ILOAD_0
            return self._do_iload(pc, slot, opcode.name.lower(), stack_before)

        # --- iload: load int from arbitrary local slot (2 bytes) ---
        if opcode == JVMOpcode.ILOAD:
            slot = self._bytecode[self.pc + 1]
            self.pc += 1  # extra byte consumed (handler adds 1 more)
            return self._do_iload(pc, slot, "iload", stack_before)

        # --- istore_N: store int to local slot N (1 byte) ---
        if JVMOpcode.ISTORE_0 <= opcode <= JVMOpcode.ISTORE_3:
            slot = opcode - JVMOpcode.ISTORE_0
            return self._do_istore(pc, slot, opcode.name.lower(), stack_before)

        # --- istore: store int to arbitrary local slot (2 bytes) ---
        if opcode == JVMOpcode.ISTORE:
            slot = self._bytecode[self.pc + 1]
            self.pc += 1  # extra byte consumed
            return self._do_istore(pc, slot, "istore", stack_before)

        # --- iadd: integer addition (1 byte) ---
        if opcode == JVMOpcode.IADD:
            return self._do_binary_op(pc, "iadd", lambda a, b: a + b, stack_before)

        # --- isub: integer subtraction (1 byte) ---
        if opcode == JVMOpcode.ISUB:
            return self._do_binary_op(pc, "isub", lambda a, b: a - b, stack_before)

        # --- imul: integer multiplication (1 byte) ---
        if opcode == JVMOpcode.IMUL:
            return self._do_binary_op(pc, "imul", lambda a, b: a * b, stack_before)

        # --- idiv: integer division (1 byte) ---
        if opcode == JVMOpcode.IDIV:
            # Check for division by zero before executing
            if len(self.stack) < 2:
                msg = "Stack underflow: idiv requires 2 operands"
                raise RuntimeError(msg)
            if self.stack[-1] == 0:
                msg = "ArithmeticException: division by zero"
                raise RuntimeError(msg)
            return self._do_binary_op(pc, "idiv", lambda a, b: int(a / b), stack_before)

        # --- goto: unconditional branch (3 bytes) ---
        if opcode == JVMOpcode.GOTO:
            offset = int.from_bytes(
                self._bytecode[self.pc + 1 : self.pc + 3],
                byteorder="big",
                signed=True,
            )
            target = self.pc + offset
            self.pc = target
            return JVMTrace(
                pc=pc,
                opcode="goto",
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=f"jump to PC={target} (offset {offset:+d})",
            )

        # --- if_icmpeq: branch if two ints are equal (3 bytes) ---
        if opcode == JVMOpcode.IF_ICMPEQ:
            return self._do_if_icmp(
                pc, "if_icmpeq", stack_before, lambda a, b: a == b
            )

        # --- if_icmpgt: branch if first int > second (3 bytes) ---
        if opcode == JVMOpcode.IF_ICMPGT:
            return self._do_if_icmp(
                pc, "if_icmpgt", stack_before, lambda a, b: a > b
            )

        # --- ireturn: return an int value (1 byte) ---
        if opcode == JVMOpcode.IRETURN:
            if len(self.stack) < 1:
                msg = "Stack underflow: ireturn requires 1 operand"
                raise RuntimeError(msg)
            self.return_value = self.stack.pop()
            self.halted = True
            self.pc += 1
            return JVMTrace(
                pc=pc,
                opcode="ireturn",
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=f"return {self.return_value}",
            )

        # --- return: return void (1 byte) ---
        if opcode == JVMOpcode.RETURN:
            self.halted = True
            self.pc += 1
            return JVMTrace(
                pc=pc,
                opcode="return",
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description="return void",
            )

        # Should be unreachable if JVMOpcode enum is complete
        msg = f"Unimplemented opcode: {opcode.name} (0x{opcode:02X})"
        raise RuntimeError(msg)

    def _do_iload(
        self, pc: int, slot: int, mnemonic: str, stack_before: list[int]
    ) -> JVMTrace:
        """Execute an iload instruction: push locals[slot] onto the stack.

        Local variables in the JVM are like registers in a register machine,
        but you access them through the stack: iload pushes the value, and
        istore pops it. The slot number identifies which local to access.
        """
        value = self.locals[slot]
        if value is None:
            msg = f"Local variable {slot} has not been initialized"
            raise RuntimeError(msg)
        self.stack.append(value)
        self.pc += 1
        return JVMTrace(
            pc=pc,
            opcode=mnemonic,
            stack_before=stack_before,
            stack_after=list(self.stack),
            locals_snapshot=list(self.locals),
            description=f"push locals[{slot}] = {value}",
        )

    def _do_istore(
        self, pc: int, slot: int, mnemonic: str, stack_before: list[int]
    ) -> JVMTrace:
        """Execute an istore instruction: pop the stack into locals[slot].

        This is how JVM bytecode implements variable assignment. The compiler
        translates "int x = expr;" into code that evaluates expr (leaving the
        result on the stack), then istore_N to save it to the variable's slot.
        """
        if len(self.stack) < 1:
            msg = f"Stack underflow: {mnemonic} requires 1 operand"
            raise RuntimeError(msg)
        value = self.stack.pop()
        self.locals[slot] = value
        self.pc += 1
        return JVMTrace(
            pc=pc,
            opcode=mnemonic,
            stack_before=stack_before,
            stack_after=list(self.stack),
            locals_snapshot=list(self.locals),
            description=f"pop {value}, store in locals[{slot}]",
        )

    def _do_binary_op(
        self,
        pc: int,
        mnemonic: str,
        op: object,
        stack_before: list[int],
    ) -> JVMTrace:
        """Execute a binary arithmetic instruction: pop b, pop a, push op(a, b).

        All JVM integer arithmetic follows this pattern. The operand order
        matters: the second-to-top value is the left operand (a), and the
        top value is the right operand (b). So "push 5; push 3; isub"
        computes 5 - 3 = 2.

        In the real JVM, integer arithmetic wraps at 32 bits (two's complement).
        We simulate this with a mask to 32 bits and sign extension.
        """
        if len(self.stack) < 2:
            msg = f"Stack underflow: {mnemonic} requires 2 operands"
            raise RuntimeError(msg)
        b = self.stack.pop()
        a = self.stack.pop()
        result = op(a, b)  # type: ignore[operator]
        # Wrap to 32-bit signed integer range (-2^31 to 2^31-1)
        result = self._to_i32(result)
        self.stack.append(result)
        self.pc += 1
        return JVMTrace(
            pc=pc,
            opcode=mnemonic,
            stack_before=stack_before,
            stack_after=list(self.stack),
            locals_snapshot=list(self.locals),
            description=f"pop {b} and {a}, push {result}",
        )

    def _do_if_icmp(
        self,
        pc: int,
        mnemonic: str,
        stack_before: list[int],
        condition: object,
    ) -> JVMTrace:
        """Execute a conditional branch: pop b, pop a, branch if condition(a, b).

        JVM conditional branches pop TWO values from the stack (unlike some
        architectures that compare against zero). The offset is relative to
        the branch instruction's own PC.

        If the condition is true, PC jumps to (pc + offset).
        If false, PC advances past the 3-byte instruction (pc + 3).
        """
        if len(self.stack) < 2:
            msg = f"Stack underflow: {mnemonic} requires 2 operands"
            raise RuntimeError(msg)

        offset = int.from_bytes(
            self._bytecode[self.pc + 1 : self.pc + 3],
            byteorder="big",
            signed=True,
        )

        b = self.stack.pop()
        a = self.stack.pop()
        taken = condition(a, b)  # type: ignore[operator]

        if taken:
            target = pc + offset
            self.pc = target
            desc = f"pop {b} and {a}, {a} {'==' if 'eq' in mnemonic else '>'} {b} is true, jump to PC={target}"
        else:
            self.pc = pc + 3  # skip past the 3-byte instruction
            desc = f"pop {b} and {a}, {a} {'==' if 'eq' in mnemonic else '>'} {b} is false, fall through"

        return JVMTrace(
            pc=pc,
            opcode=mnemonic,
            stack_before=stack_before,
            stack_after=list(self.stack),
            locals_snapshot=list(self.locals),
            description=desc,
        )

    @staticmethod
    def _to_i32(value: int) -> int:
        """Wrap a Python integer to 32-bit signed range.

        The JVM specifies that integer arithmetic wraps at 32 bits using
        two's complement. Python integers are arbitrary precision, so we
        must manually wrap:

            1. Mask to 32 bits (unsigned): value & 0xFFFFFFFF
            2. If bit 31 is set, interpret as negative (subtract 2^32)

        Examples:
            _to_i32(2147483647)  -> 2147483647  (max int)
            _to_i32(2147483648)  -> -2147483648  (overflow wraps)
            _to_i32(-1)          -> -1           (unchanged)
        """
        value = value & 0xFFFFFFFF
        if value >= 0x80000000:
            value -= 0x100000000
        return value


# ---------------------------------------------------------------------------
# Encoding helpers (mini assembler)
# ---------------------------------------------------------------------------
# These functions produce raw bytes for JVM instructions. This is a tiny
# assembler -- just enough to create test programs without manually writing
# hex bytes.


def encode_iconst(n: int) -> bytes:
    """Encode pushing a small integer constant (0-5) using iconst_N opcodes.

    If n is outside 0-5, falls back to bipush (for -128 to 127) or raises
    an error for values outside the signed byte range.

    Examples:
        >>> encode_iconst(0)
        b'\\x03'
        >>> encode_iconst(5)
        b'\\x08'
        >>> encode_iconst(42)
        b'\\x10\\x2a'
    """
    if 0 <= n <= 5:
        return bytes([JVMOpcode.ICONST_0 + n])
    if -128 <= n <= 127:
        # Fall back to bipush for values outside 0-5 but within byte range
        raw = n if n >= 0 else n + 256
        return bytes([JVMOpcode.BIPUSH, raw])
    msg = f"encode_iconst: value {n} is outside signed byte range (-128 to 127). Use bipush or ldc."
    raise ValueError(msg)


def encode_istore(slot: int) -> bytes:
    """Encode storing to a local variable slot.

    Uses the istore_N shortcut for slots 0-3, otherwise the generic
    2-byte istore form.

    Examples:
        >>> encode_istore(0)
        b'\\x3b'
        >>> encode_istore(5)
        b'\\x36\\x05'
    """
    if 0 <= slot <= 3:
        return bytes([JVMOpcode.ISTORE_0 + slot])
    return bytes([JVMOpcode.ISTORE, slot])


def encode_iload(slot: int) -> bytes:
    """Encode loading from a local variable slot.

    Uses the iload_N shortcut for slots 0-3, otherwise the generic
    2-byte iload form.

    Examples:
        >>> encode_iload(0)
        b'\\x1a'
        >>> encode_iload(5)
        b'\\x15\\x05'
    """
    if 0 <= slot <= 3:
        return bytes([JVMOpcode.ILOAD_0 + slot])
    return bytes([JVMOpcode.ILOAD, slot])


def assemble_jvm(*instructions: tuple[JVMOpcode, ...]) -> bytes:
    """Assemble JVM bytecode from instruction tuples.

    Each instruction is a tuple of (opcode,) or (opcode, operand, ...).
    The function encodes each instruction according to its format and
    concatenates the results.

    The encoding rules:
        - iconst_N, iload_N, istore_N, iadd, isub, imul, idiv,
          ireturn, return: 1 byte (just the opcode)
        - bipush, ldc, iload, istore: 2 bytes (opcode + 1-byte operand)
        - goto, if_icmpeq, if_icmpgt: 3 bytes (opcode + 2-byte signed offset)

    Examples:
        >>> assemble_jvm(
        ...     (JVMOpcode.ICONST_1,),
        ...     (JVMOpcode.ICONST_2,),
        ...     (JVMOpcode.IADD,),
        ...     (JVMOpcode.ISTORE_0,),
        ...     (JVMOpcode.RETURN,),
        ... )
        b'\\x04\\x05\\x60\\x3b\\xb1'
    """
    result = bytearray()

    # Single-byte opcodes (no operand needed)
    one_byte_opcodes = {
        JVMOpcode.ICONST_0,
        JVMOpcode.ICONST_1,
        JVMOpcode.ICONST_2,
        JVMOpcode.ICONST_3,
        JVMOpcode.ICONST_4,
        JVMOpcode.ICONST_5,
        JVMOpcode.ILOAD_0,
        JVMOpcode.ILOAD_1,
        JVMOpcode.ILOAD_2,
        JVMOpcode.ILOAD_3,
        JVMOpcode.ISTORE_0,
        JVMOpcode.ISTORE_1,
        JVMOpcode.ISTORE_2,
        JVMOpcode.ISTORE_3,
        JVMOpcode.IADD,
        JVMOpcode.ISUB,
        JVMOpcode.IMUL,
        JVMOpcode.IDIV,
        JVMOpcode.IRETURN,
        JVMOpcode.RETURN,
    }

    # Two-byte opcodes (opcode + 1-byte operand)
    two_byte_opcodes = {
        JVMOpcode.BIPUSH,
        JVMOpcode.LDC,
        JVMOpcode.ILOAD,
        JVMOpcode.ISTORE,
    }

    # Three-byte opcodes (opcode + 2-byte signed offset)
    three_byte_opcodes = {
        JVMOpcode.GOTO,
        JVMOpcode.IF_ICMPEQ,
        JVMOpcode.IF_ICMPGT,
    }

    for instr in instructions:
        op = instr[0]

        if op in one_byte_opcodes:
            result.append(op)

        elif op in two_byte_opcodes:
            if len(instr) < 2:
                msg = f"Opcode {op.name} requires an operand"
                raise ValueError(msg)
            operand = instr[1]
            if op == JVMOpcode.BIPUSH:
                # bipush operand is a signed byte
                raw = operand if operand >= 0 else operand + 256
                result.append(op)
                result.append(raw & 0xFF)
            else:
                # ldc, iload, istore: unsigned byte operand
                result.append(op)
                result.append(operand & 0xFF)

        elif op in three_byte_opcodes:
            if len(instr) < 2:
                msg = f"Opcode {op.name} requires an offset operand"
                raise ValueError(msg)
            offset = instr[1]
            result.append(op)
            result.extend(offset.to_bytes(2, byteorder="big", signed=True))

        else:
            msg = f"Unknown opcode in assemble_jvm: {op}"
            raise ValueError(msg)

    return bytes(result)
