"""WebAssembly (WASM) Simulator — a modern stack-based virtual machine.

=== What is WebAssembly? ===

WebAssembly (WASM) is a binary instruction format designed as a portable
compilation target for the web. It was standardized by the W3C in 2017 and
is supported by all major browsers. Languages like Rust, C++, Go, and
AssemblyScript compile down to WASM, letting you run near-native-speed code
inside a browser sandbox.

WASM is interesting because it bridges the gap between high-level languages
and the browser runtime. Instead of writing JavaScript, you write Rust —
and the compiler produces WASM bytecode that the browser executes directly.

=== Stack machines vs register machines ===

Our RISC-V simulator is a *register machine*: instructions name specific
registers as operands (e.g., "add x3, x1, x2" — read x1 and x2, write x3).
The CPU has a fixed set of registers, and the instruction encoding must
specify which registers to use.

WASM is a *stack machine*: instructions don't name their operands. Instead,
operands live on an implicit *operand stack*. Push values onto the stack,
then invoke an operation — it pops its inputs and pushes the result.

    Register machine (RISC-V):        Stack machine (WASM):
        addi x1, x0, 1                   i32.const 1
        addi x2, x0, 2                   i32.const 2
        add  x3, x1, x2                  i32.add
                                          local.set 0

Both compute "x = 1 + 2", but the stack machine never names a destination
register for the add. It pops 2 and 1 from the stack, pushes 3, and then
local.set stores it.

Stack machines have a simpler instruction encoding (no register fields!) but
the CPU must manage the stack. Register machines have wider instructions but
can access any register in one cycle.

Our bytecode VM (cpu-simulator) is also a stack-based design internally,
so WASM feels like a natural next step — a real, production stack machine.

=== WASM instruction encoding ===

Unlike RISC-V (where every instruction is exactly 32 bits), WASM instructions
are variable-width. Some are 1 byte (i32.add = 0x6A), others are 2 bytes
(local.get N = 0x20 N), and i32.const is 5 bytes (0x41 + 4-byte LE value).

In real WASM, integer immediates use LEB128 variable-length encoding. For our
MVP, we use a simplified fixed-width encoding:

    Instruction      Encoding              Width
    ─────────────    ────────────────────   ─────
    i32.const V      0x41 V[0] V[1] V[2] V[3]   5 bytes (V as little-endian i32)
    i32.add          0x6A                  1 byte
    i32.sub          0x6B                  1 byte
    local.get N      0x20 N               2 bytes
    local.set N      0x21 N               2 bytes
    end              0x0B                  1 byte

=== The x = 1 + 2 program ===

    i32.const 1    →  stack: [1]           push 1
    i32.const 2    →  stack: [1, 2]        push 2
    i32.add        →  stack: [3]           pop 2 and 1, push 3
    local.set 0    →  stack: [], x=3       pop 3, store in local 0
    end            →  halt

=== Why standalone (not wrapping the CPU class)? ===

The generic CPU class uses a fixed-width fetch cycle: it reads a 32-bit word
at PC, decodes it, and advances PC by 4. WASM instructions are variable-width
bytes, so the fetch cycle is fundamentally different — we read one byte at PC
to determine the opcode, then read additional bytes depending on the opcode.

Rather than forcing the CPU class to support variable-width fetch, we build
a standalone simulator that directly implements the WASM execution model.
"""

from __future__ import annotations

from dataclasses import dataclass, field


# ---------------------------------------------------------------------------
# Opcode constants
# ---------------------------------------------------------------------------
# These are the byte values that identify each WASM instruction.
# In real WASM, these are defined by the spec in the "Binary Format" section.

OP_END = 0x0B  # End of function / halt
OP_LOCAL_GET = 0x20  # Push a local variable onto the stack
OP_LOCAL_SET = 0x21  # Pop the stack into a local variable
OP_I32_CONST = 0x41  # Push a 32-bit integer constant
OP_I32_ADD = 0x6A  # Pop two i32s, push their sum
OP_I32_SUB = 0x6B  # Pop two i32s, push their difference


# ---------------------------------------------------------------------------
# Decoder
# ---------------------------------------------------------------------------


@dataclass
class WasmInstruction:
    """A decoded WASM instruction with its operands and size.

    Unlike RISC-V's fixed 32-bit instructions, WASM instructions vary in
    size. The `size` field tells the simulator how far to advance PC.

    Examples:
        WasmInstruction(opcode=0x41, mnemonic="i32.const", operand=42, size=5)
        WasmInstruction(opcode=0x6A, mnemonic="i32.add", operand=None, size=1)
        WasmInstruction(opcode=0x20, mnemonic="local.get", operand=0, size=2)
    """

    opcode: int
    mnemonic: str
    operand: int | None  # Some instructions have no operand (add, end)
    size: int  # Number of bytes consumed (for advancing PC)


class WasmDecoder:
    """Decodes WASM bytecodes from raw bytes to structured instructions.

    The decoder reads bytes from a bytecode buffer starting at a given PC,
    determines the instruction and its operands, and returns a WasmInstruction.

    Variable-width decoding:
        - Read 1 byte at PC to get the opcode
        - Depending on the opcode, read 0, 1, or 4 more bytes for the operand
        - Return the total size so the simulator knows how to advance PC

    Example: decoding i32.const 42 from bytes [0x41, 0x2A, 0x00, 0x00, 0x00]

        Byte 0: 0x41 → opcode = i32.const (expects 4 more bytes)
        Bytes 1-4: 0x2A 0x00 0x00 0x00 → value = 42 (little-endian)
        Result: WasmInstruction(opcode=0x41, mnemonic="i32.const", operand=42, size=5)
    """

    def decode(self, bytecode: bytes, pc: int) -> WasmInstruction:
        """Decode one instruction starting at `pc` in the bytecode buffer.

        Reads the opcode byte, then dispatches to the appropriate handler
        based on the instruction's operand format.
        """
        opcode = bytecode[pc]

        if opcode == OP_I32_CONST:
            # 5 bytes: opcode + 4-byte little-endian signed integer
            value = int.from_bytes(bytecode[pc + 1 : pc + 5], byteorder="little", signed=True)
            return WasmInstruction(
                opcode=opcode, mnemonic="i32.const", operand=value, size=5
            )

        elif opcode == OP_I32_ADD:
            # 1 byte: just the opcode
            return WasmInstruction(
                opcode=opcode, mnemonic="i32.add", operand=None, size=1
            )

        elif opcode == OP_I32_SUB:
            # 1 byte: just the opcode
            return WasmInstruction(
                opcode=opcode, mnemonic="i32.sub", operand=None, size=1
            )

        elif opcode == OP_LOCAL_GET:
            # 2 bytes: opcode + 1-byte local index
            index = bytecode[pc + 1]
            return WasmInstruction(
                opcode=opcode, mnemonic="local.get", operand=index, size=2
            )

        elif opcode == OP_LOCAL_SET:
            # 2 bytes: opcode + 1-byte local index
            index = bytecode[pc + 1]
            return WasmInstruction(
                opcode=opcode, mnemonic="local.set", operand=index, size=2
            )

        elif opcode == OP_END:
            # 1 byte: end marker
            return WasmInstruction(
                opcode=opcode, mnemonic="end", operand=None, size=1
            )

        else:
            raise ValueError(f"Unknown WASM opcode: 0x{opcode:02X} at PC={pc}")


# ---------------------------------------------------------------------------
# Executor
# ---------------------------------------------------------------------------


@dataclass
class WasmStepTrace:
    """A trace of one WASM instruction execution.

    This is the WASM equivalent of PipelineTrace — it captures what happened
    during a single step so you can visualize execution.

    The key difference from RISC-V traces: instead of register changes,
    we show the operand stack before and after execution. This makes the
    stack-based execution model visible.

    Example trace for i32.add when stack was [1, 2]:
        WasmStepTrace(
            pc=10,
            instruction=WasmInstruction(..., mnemonic="i32.add"),
            stack_before=[1, 2],
            stack_after=[3],
            locals_snapshot=[0, 0, 0, 0],
            description="pop 2 and 1, push 3",
        )
    """

    pc: int
    instruction: WasmInstruction
    stack_before: list[int]
    stack_after: list[int]
    locals_snapshot: list[int]
    description: str
    halted: bool = False


class WasmExecutor:
    """Executes decoded WASM instructions against a stack and local variables.

    The executor is the "do it" phase. Given a decoded instruction, it:
      - Reads from the stack or locals (inputs)
      - Performs the operation
      - Writes to the stack or locals (outputs)

    Stack operations follow WASM semantics:
      - i32.const V: push V onto the stack
      - i32.add:     pop b, pop a, push (a + b)
      - i32.sub:     pop b, pop a, push (a - b)
      - local.get N: push locals[N] onto the stack
      - local.set N: pop the stack into locals[N]
      - end:         halt execution

    Note the operand order for i32.add and i32.sub: the *second* operand
    is on top of the stack (popped first). So "push 1; push 2; i32.sub"
    computes 1 - 2 = -1 (not 2 - 1).
    """

    def execute(
        self,
        instruction: WasmInstruction,
        stack: list[int],
        locals_: list[int],
        pc: int,
    ) -> WasmStepTrace:
        """Execute one decoded WASM instruction.

        Modifies `stack` and `locals_` in place. Returns a trace showing
        what happened (stack before/after, description, halt status).
        """
        stack_before = list(stack)  # Snapshot before mutation
        mnemonic = instruction.mnemonic

        if mnemonic == "i32.const":
            return self._exec_i32_const(instruction, stack, locals_, pc, stack_before)
        elif mnemonic == "i32.add":
            return self._exec_i32_add(instruction, stack, locals_, pc, stack_before)
        elif mnemonic == "i32.sub":
            return self._exec_i32_sub(instruction, stack, locals_, pc, stack_before)
        elif mnemonic == "local.get":
            return self._exec_local_get(instruction, stack, locals_, pc, stack_before)
        elif mnemonic == "local.set":
            return self._exec_local_set(instruction, stack, locals_, pc, stack_before)
        elif mnemonic == "end":
            return WasmStepTrace(
                pc=pc,
                instruction=instruction,
                stack_before=stack_before,
                stack_after=list(stack),
                locals_snapshot=list(locals_),
                description="halt",
                halted=True,
            )
        else:
            raise ValueError(f"Cannot execute: {mnemonic}")

    def _exec_i32_const(
        self,
        instruction: WasmInstruction,
        stack: list[int],
        locals_: list[int],
        pc: int,
        stack_before: list[int],
    ) -> WasmStepTrace:
        """Execute: i32.const V → push V onto the stack.

        Example: i32.const 42
            stack before: []
            stack after:  [42]
        """
        value = instruction.operand
        assert value is not None
        stack.append(value)
        return WasmStepTrace(
            pc=pc,
            instruction=instruction,
            stack_before=stack_before,
            stack_after=list(stack),
            locals_snapshot=list(locals_),
            description=f"push {value}",
        )

    def _exec_i32_add(
        self,
        instruction: WasmInstruction,
        stack: list[int],
        locals_: list[int],
        pc: int,
        stack_before: list[int],
    ) -> WasmStepTrace:
        """Execute: i32.add → pop b, pop a, push (a + b).

        The second-to-top value is the left operand (a), and the top
        value is the right operand (b). This matches WASM spec semantics.

        Example: stack [1, 2] → i32.add → stack [3]
            b = pop() → 2
            a = pop() → 1
            push(1 + 2) → push(3)
        """
        b = stack.pop()
        a = stack.pop()
        result = (a + b) & 0xFFFFFFFF  # Mask to 32-bit unsigned
        stack.append(result)
        return WasmStepTrace(
            pc=pc,
            instruction=instruction,
            stack_before=stack_before,
            stack_after=list(stack),
            locals_snapshot=list(locals_),
            description=f"pop {b} and {a}, push {result}",
        )

    def _exec_i32_sub(
        self,
        instruction: WasmInstruction,
        stack: list[int],
        locals_: list[int],
        pc: int,
        stack_before: list[int],
    ) -> WasmStepTrace:
        """Execute: i32.sub → pop b, pop a, push (a - b).

        Same operand order as i32.add: second-to-top minus top.

        Example: stack [5, 3] → i32.sub → stack [2]
            b = pop() → 3
            a = pop() → 5
            push(5 - 3) → push(2)
        """
        b = stack.pop()
        a = stack.pop()
        result = (a - b) & 0xFFFFFFFF  # Mask to 32-bit unsigned
        stack.append(result)
        return WasmStepTrace(
            pc=pc,
            instruction=instruction,
            stack_before=stack_before,
            stack_after=list(stack),
            locals_snapshot=list(locals_),
            description=f"pop {b} and {a}, push {result}",
        )

    def _exec_local_get(
        self,
        instruction: WasmInstruction,
        stack: list[int],
        locals_: list[int],
        pc: int,
        stack_before: list[int],
    ) -> WasmStepTrace:
        """Execute: local.get N → push locals[N] onto the stack.

        Local variables are like WASM's version of registers — a fixed set
        of named storage slots. But unlike registers, you access them through
        the stack: local.get pushes the value, local.set pops it.

        Example: local.get 0 (where locals[0] = 42)
            stack before: []
            stack after:  [42]
        """
        index = instruction.operand
        assert index is not None
        value = locals_[index]
        stack.append(value)
        return WasmStepTrace(
            pc=pc,
            instruction=instruction,
            stack_before=stack_before,
            stack_after=list(stack),
            locals_snapshot=list(locals_),
            description=f"push locals[{index}] = {value}",
        )

    def _exec_local_set(
        self,
        instruction: WasmInstruction,
        stack: list[int],
        locals_: list[int],
        pc: int,
        stack_before: list[int],
    ) -> WasmStepTrace:
        """Execute: local.set N → pop the stack into locals[N].

        Example: local.set 0 (stack has [3])
            value = pop() → 3
            locals[0] = 3
            stack after: []
        """
        index = instruction.operand
        assert index is not None
        value = stack.pop()
        locals_[index] = value
        return WasmStepTrace(
            pc=pc,
            instruction=instruction,
            stack_before=stack_before,
            stack_after=list(stack),
            locals_snapshot=list(locals_),
            description=f"pop {value}, store in locals[{index}]",
        )


# ---------------------------------------------------------------------------
# Encoding helpers (mini assembler)
# ---------------------------------------------------------------------------
# These functions produce raw bytes for WASM instructions.
# This is a tiny assembler — just enough to create test programs.


def encode_i32_const(value: int) -> bytes:
    """Encode: i32.const value → 5 bytes (opcode + 4-byte LE signed int).

    Example:
        >>> encode_i32_const(1)
        b'\\x41\\x01\\x00\\x00\\x00'
        >>> encode_i32_const(-1)
        b'\\x41\\xff\\xff\\xff\\xff'
    """
    return bytes([OP_I32_CONST]) + value.to_bytes(4, byteorder="little", signed=True)


def encode_i32_add() -> bytes:
    """Encode: i32.add → 1 byte.

    Example:
        >>> encode_i32_add()
        b'\\x6a'
    """
    return bytes([OP_I32_ADD])


def encode_i32_sub() -> bytes:
    """Encode: i32.sub → 1 byte.

    Example:
        >>> encode_i32_sub()
        b'\\x6b'
    """
    return bytes([OP_I32_SUB])


def encode_local_get(index: int) -> bytes:
    """Encode: local.get index → 2 bytes (opcode + 1-byte index).

    Example:
        >>> encode_local_get(0)
        b'\\x20\\x00'
    """
    return bytes([OP_LOCAL_GET, index])


def encode_local_set(index: int) -> bytes:
    """Encode: local.set index → 2 bytes (opcode + 1-byte index).

    Example:
        >>> encode_local_set(0)
        b'\\x21\\x00'
    """
    return bytes([OP_LOCAL_SET, index])


def encode_end() -> bytes:
    """Encode: end → 1 byte.

    Example:
        >>> encode_end()
        b'\\x0b'
    """
    return bytes([OP_END])


def assemble_wasm(instructions: list[bytes]) -> bytes:
    """Concatenate encoded WASM instructions into a bytecode program.

    Unlike RISC-V's assemble() (which packs fixed-width 32-bit words),
    this just concatenates variable-width byte sequences.

    Example:
        >>> program = assemble_wasm([
        ...     encode_i32_const(1),    # push 1
        ...     encode_i32_const(2),    # push 2
        ...     encode_i32_add(),       # pop 2 and 1, push 3
        ...     encode_local_set(0),    # pop 3, store in local 0
        ...     encode_end(),           # halt
        ... ])
    """
    return b"".join(instructions)


# ---------------------------------------------------------------------------
# Simulator
# ---------------------------------------------------------------------------


class WasmSimulator:
    """Complete WASM simulator — decoder, executor, and execution state.

    This is a standalone simulator (not wrapping the generic CPU class)
    because WASM's variable-width instruction fetch is fundamentally
    different from the CPU's fixed-width 32-bit fetch cycle.

    State:
        - stack:    The operand stack (values pushed/popped by instructions)
        - locals:   Local variables (like registers, but accessed via stack)
        - pc:       Program counter (byte offset into bytecode)
        - bytecode: The raw program bytes
        - halted:   Whether execution has finished

    Example: running x = 1 + 2

        >>> sim = WasmSimulator(num_locals=4)
        >>> program = assemble_wasm([
        ...     encode_i32_const(1),    # push 1
        ...     encode_i32_const(2),    # push 2
        ...     encode_i32_add(),       # pop 2 and 1, push 3
        ...     encode_local_set(0),    # pop 3, store in local 0
        ...     encode_end(),           # halt
        ... ])
        >>> traces = sim.run(program)
        >>> sim.locals[0]
        3

        Step-by-step stack evolution:
            Step 0: i32.const 1    stack: [] → [1]
            Step 1: i32.const 2    stack: [1] → [1, 2]
            Step 2: i32.add        stack: [1, 2] → [3]
            Step 3: local.set 0    stack: [3] → []       locals[0] = 3
            Step 4: end            halt
    """

    def __init__(self, num_locals: int = 4) -> None:
        self.stack: list[int] = []
        self.locals: list[int] = [0] * num_locals
        self.pc: int = 0
        self.bytecode: bytes = b""
        self.halted: bool = False
        self.cycle: int = 0
        self._decoder = WasmDecoder()
        self._executor = WasmExecutor()

    def load(self, bytecode: bytes) -> None:
        """Load a WASM bytecode program.

        Resets the PC to 0 but preserves the stack and locals
        (call __init__ again for a full reset).
        """
        self.bytecode = bytecode
        self.pc = 0
        self.halted = False
        self.cycle = 0
        self.stack.clear()
        # Reset locals to zero
        for i in range(len(self.locals)):
            self.locals[i] = 0

    def step(self) -> WasmStepTrace:
        """Execute one WASM instruction and return a trace.

        The WASM execution cycle:

            1. DECODE: Read bytes at PC → determine opcode and operands
            2. EXECUTE: Perform the operation (push/pop stack, read/write locals)
            3. ADVANCE: Move PC forward by the instruction's byte width

        This is simpler than RISC-V's fetch-decode-execute because there's
        no separate "fetch a fixed-width word" stage — the decoder reads
        exactly the bytes it needs directly from the bytecode buffer.

        Returns:
            WasmStepTrace showing the instruction, stack before/after, etc.

        Raises:
            RuntimeError: If the simulator has halted.
        """
        if self.halted:
            msg = "WASM simulator has halted — no more instructions to execute"
            raise RuntimeError(msg)

        # === DECODE ===
        # Read bytes at PC to determine the instruction and its operands.
        # The decoder returns a WasmInstruction with a `size` field telling
        # us how many bytes were consumed.
        instruction = self._decoder.decode(self.bytecode, self.pc)

        # === EXECUTE ===
        # Perform the operation — modifies stack and locals in place.
        trace = self._executor.execute(
            instruction, self.stack, self.locals, self.pc
        )

        # === ADVANCE PC ===
        # Move the program counter forward by the instruction's byte width.
        # (Unlike RISC-V where PC always advances by 4.)
        self.pc += instruction.size
        self.halted = trace.halted
        self.cycle += 1

        return trace

    def run(self, program: bytes, max_steps: int = 10000) -> list[WasmStepTrace]:
        """Load and run a WASM program, returning the execution trace.

        Returns a list of WasmStepTrace objects — one for each instruction
        executed. This gives you the complete execution history with stack
        snapshots at every step.

        Args:
            program: Raw bytecode to execute.
            max_steps: Safety limit to prevent infinite loops.

        Returns:
            List of WasmStepTrace objects, one per instruction.
        """
        self.load(program)
        traces: list[WasmStepTrace] = []
        for _ in range(max_steps):
            if self.halted:
                break
            traces.append(self.step())
        return traces

    # -------------------------------------------------------------------
    # simulator-protocol conformance methods
    # -------------------------------------------------------------------

    def get_state(self) -> WasmState:
        """Return a frozen snapshot of the current WASM simulator state.

        All mutable lists are converted to tuples so the result is a true
        immutable value.  The snapshot will not change even if the simulator
        continues executing after this call returns.

        This method satisfies the ``Simulator[WasmState]`` protocol from the
        ``simulator-protocol`` package.

        Returns
        -------
        WasmState:
            Frozen dataclass capturing: operand stack, local variables,
            program counter, halted flag, and cycle counter.

        Examples
        --------
        >>> sim = WasmSimulator(num_locals=4)
        >>> sim.load(bytes([0x0B]))  # end
        >>> sim.step()
        ...
        >>> state = sim.get_state()
        >>> state.halted
        True
        """
        from wasm_simulator.state import WasmState

        return WasmState(
            stack=tuple(self.stack),
            locals=tuple(self.locals),
            pc=self.pc,
            halted=self.halted,
            cycle=self.cycle,
        )

    def execute(
        self,
        program: bytes,
        max_steps: int = 100_000,
    ) -> "ExecutionResult[WasmState]":
        """Load program, run to end or max_steps, return ExecutionResult.

        This is the protocol-conforming entry point for the
        ``Simulator[WasmState]`` protocol defined in the ``simulator-protocol``
        package.  It loads the program (resetting state via ``load()``), runs
        the execution loop, and returns a rich result type.

        The existing ``load()`` and ``run()`` methods are unchanged — this
        method calls them internally and adapts the return value.

        Parameters
        ----------
        program:
            Raw WASM bytecode bytes.
        max_steps:
            Maximum instructions to execute before giving up (default 100,000).

        Returns
        -------
        ExecutionResult[WasmState]:
            - ``halted``: True if the end instruction was reached.
            - ``steps``: total instructions executed.
            - ``final_state``: frozen ``WasmState`` snapshot at termination.
            - ``error``: None on clean halt; error string otherwise.
            - ``traces``: one ``StepTrace`` per instruction executed.

        Examples
        --------
        >>> sim = WasmSimulator(num_locals=4)
        >>> from wasm_simulator.simulator import (
        ...     assemble_wasm, encode_i32_const, encode_i32_add,
        ...     encode_local_set, encode_end,
        ... )
        >>> program = assemble_wasm([
        ...     encode_i32_const(1),
        ...     encode_i32_const(2),
        ...     encode_i32_add(),
        ...     encode_local_set(0),
        ...     encode_end(),
        ... ])
        >>> result = sim.execute(program)
        >>> result.ok
        True
        >>> result.final_state.locals[0]
        3
        """
        from simulator_protocol import ExecutionResult, StepTrace

        from wasm_simulator.state import WasmState

        # load() resets all execution state (stack, locals, pc, halted, cycle)
        self.load(program)

        step_traces: list[StepTrace] = []
        steps = 0
        error: str | None = None

        try:
            while not self.halted and steps < max_steps:
                pc_before = self.pc
                wasm_trace = self.step()
                step_traces.append(
                    StepTrace(
                        pc_before=pc_before,
                        pc_after=self.pc,
                        mnemonic=wasm_trace.instruction.mnemonic,
                        description=wasm_trace.description,
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

        Clears the operand stack, resets all locals to 0, resets the program
        counter and cycle counter, and clears the halted flag and bytecode
        buffer.

        After ``reset()``, the simulator is in the same state as a freshly
        constructed ``WasmSimulator(num_locals=N)`` where N is the current
        number of locals.

        This method satisfies the ``reset()`` requirement of the
        ``Simulator[WasmState]`` protocol.

        Examples
        --------
        >>> sim = WasmSimulator(num_locals=4)
        >>> sim.load(bytes([0x0B]))  # end
        >>> sim.step()
        ...
        >>> sim.halted
        True
        >>> sim.reset()
        >>> sim.halted
        False
        >>> sim.stack
        []
        """
        num_locals = len(self.locals)
        self.stack = []
        self.locals = [0] * num_locals
        self.pc = 0
        self.bytecode = b""
        self.halted = False
        self.cycle = 0
