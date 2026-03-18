"""Intel 4004 Simulator — the world's first commercial microprocessor.

=== What is the Intel 4004? ===

The Intel 4004 was the world's first commercial single-chip microprocessor,
released by Intel in 1971. It was designed by Federico Faggin, Ted Hoff, and
Stanley Mazor for the Busicom 141-PF calculator — a Japanese desktop printing
calculator. Intel negotiated to retain the rights to the chip design, which
turned out to be one of the most consequential business decisions in history.

The entire processor contained just 2,300 transistors. For perspective, a
modern CPU has billions. The 4004 ran at 740 kHz — about a million times
slower than today's processors. Yet it proved that a general-purpose processor
could be built on a single chip, launching the microprocessor revolution.

=== Why 4-bit? ===

The 4004 is a 4-bit processor. Every data value is 4 bits wide (0-15). This
seems tiny, but it was perfect for its intended purpose: calculators. A single
decimal digit (0-9) fits in 4 bits, which is exactly what Binary-Coded Decimal
(BCD) arithmetic needs. The Busicom calculator used BCD throughout, so 4 bits
was the natural data width.

All values in this simulator are masked to 4 bits (& 0xF). This is the
fundamental constraint of the architecture — there are no 8-bit, 16-bit, or
32-bit values anywhere in the data path.

=== Accumulator architecture ===

The 4004 uses an accumulator architecture. This means almost every arithmetic
operation works through a single special register called the Accumulator (A):

    - To add two numbers: load one into A, store it in a register, load the
      other into A, then add the register to A. The result is in A.
    - There is no "add register to register" instruction.

This is very different from other architectures:

    RISC-V (register-register):  add x3, x1, x2     Any register to any register.
    WASM (stack-based):          i32.add              Pops two, pushes result.
    Intel 4004 (accumulator):    ADD R0               A = A + R0. Always uses A.

The accumulator pattern means more instructions to do the same work, but
simpler hardware — which mattered enormously in 1971 when every transistor
was precious.

=== Registers ===

    Accumulator (A):  4 bits. The center of all computation.
    R0-R15:           16 general registers, each 4 bits.
    Carry flag:       1 bit. Set on arithmetic overflow/borrow.
    PC:               Program counter (points to the next instruction in ROM).

=== Instruction encoding ===

Instructions are 8 bits (1 byte). The upper nibble is the opcode, and the
lower nibble is the operand (a register number or immediate value):

    ┌──────────┬──────────┐
    │  opcode  │ operand  │
    │  bits 7-4│ bits 3-0 │
    └──────────┴──────────┘

    LDM N   (0xDN):  Load immediate N into accumulator. A = N.
    XCH RN  (0xBN):  Exchange accumulator with register N. Swap A and RN.
    ADD RN  (0x8N):  Add register N to accumulator with carry. A = A + RN.
    SUB RN  (0x9N):  Subtract register N from accumulator with borrow. A = A - RN.
    HLT     (0x01):  Halt execution. (Simulator-only, not a real 4004 opcode.)

=== The x = 1 + 2 program ===

To compute x = 1 + 2 and store the result in R1:

    LDM 1      A = 1                   -> 0xD1
    XCH R0     R0 = 1, A = 0           -> 0xB0
    LDM 2      A = 2                   -> 0xD2
    ADD R0     A = 2 + 1 = 3           -> 0x80
    XCH R1     R1 = 3, A = 0           -> 0xB1
    HLT        stop                    -> 0x01

Six instructions to add two numbers! RISC-V does it in four (two loads, one
add, one halt). The accumulator bottleneck is the price of simpler hardware.
"""

from dataclasses import dataclass, field


# ---------------------------------------------------------------------------
# Trace — what happened during one instruction
# ---------------------------------------------------------------------------
# Every step() call returns one of these, giving a complete picture of what
# the instruction did. This is the 4004 equivalent of RISC-V's PipelineTrace,
# but simpler — no pipeline stages, just fetch-decode-execute in one cycle.


@dataclass
class Intel4004Trace:
    """Record of a single instruction execution.

    Fields:
        address:            PC where this instruction was fetched from.
        raw:                The raw byte (0x00-0xFF).
        mnemonic:           Human-readable instruction (e.g., "LDM 1", "ADD R0").
        accumulator_before: Value of A before execution.
        accumulator_after:  Value of A after execution.
        carry_before:       Carry flag before execution.
        carry_after:        Carry flag after execution.
    """

    address: int
    raw: int
    mnemonic: str
    accumulator_before: int
    accumulator_after: int
    carry_before: bool
    carry_after: bool


# ---------------------------------------------------------------------------
# The simulator
# ---------------------------------------------------------------------------


class Intel4004Simulator:
    """A simulator for the Intel 4004 microprocessor.

    This is a standalone implementation — the 4004's accumulator architecture
    is too different from register-register machines (like RISC-V) to share
    a generic CPU base class. The 4-bit data width, single accumulator, and
    carry flag are all unique to this style of machine.

    Usage:
        >>> sim = Intel4004Simulator()
        >>> program = bytes([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01])
        >>> traces = sim.run(program)
        >>> sim.registers[1]   # R1 = 3
        3

    State:
        accumulator:  4-bit accumulator (0-15). The heart of computation.
        registers:    16 general-purpose 4-bit registers (R0-R15).
        carry:        Carry/borrow flag from the last arithmetic operation.
        memory:       ROM holding the program bytes.
        pc:           Program counter — index into memory.
        halted:       True after HLT is executed.
    """

    def __init__(self, memory_size: int = 4096) -> None:
        # --- Registers ---
        # The accumulator is where all arithmetic happens.
        # It's 4 bits, so values are always 0-15.
        self.accumulator: int = 0

        # 16 general-purpose registers, each 4 bits.
        # These hold intermediate values — you swap them in and out of A
        # using XCH to do multi-step computations.
        self.registers: list[int] = [0] * 16

        # --- Flags ---
        # The carry flag is set when an ADD overflows past 15, or when a
        # SUB borrows (result would be negative). This is how the 4004
        # handles multi-digit arithmetic — carry propagates between digits.
        self.carry: bool = False

        # --- Memory ---
        # The 4004 had separate ROM (program) and RAM (data) address spaces.
        # We only model ROM here — enough for our instruction set.
        # The original 4004 could address 4096 bytes of ROM.
        self.memory: bytearray = bytearray(memory_size)

        # --- Control ---
        self.pc: int = 0
        self.halted: bool = False

    def load_program(self, program: bytes) -> None:
        """Load a program into ROM starting at address 0.

        Each byte in the program is one instruction. The 4004's instructions
        are 8 bits — much simpler than RISC-V's 32-bit encoding.
        """
        for i, byte in enumerate(program):
            self.memory[i] = byte

    def step(self) -> Intel4004Trace:
        """Fetch, decode, and execute one instruction.

        This is the core of the simulator. The 4004 doesn't have a pipeline —
        it completes each instruction before starting the next. The sequence is:

        1. FETCH:   Read the byte at memory[PC].
        2. DECODE:  Split into opcode (upper nibble) and operand (lower nibble).
        3. EXECUTE: Perform the operation, update state.

        Returns an Intel4004Trace with complete before/after state.
        """
        if self.halted:
            raise RuntimeError("CPU is halted — cannot step further")

        # --- Fetch ---
        address = self.pc
        raw = self.memory[self.pc]
        self.pc += 1

        # --- Snapshot state before execution ---
        acc_before = self.accumulator
        carry_before = self.carry

        # --- Decode ---
        # Upper nibble = opcode, lower nibble = operand (register or immediate)
        opcode = (raw >> 4) & 0xF
        operand = raw & 0xF

        # --- Execute ---
        mnemonic = self._execute(opcode, operand, raw)

        # --- Build trace ---
        return Intel4004Trace(
            address=address,
            raw=raw,
            mnemonic=mnemonic,
            accumulator_before=acc_before,
            accumulator_after=self.accumulator,
            carry_before=carry_before,
            carry_after=self.carry,
        )

    def _execute(self, opcode: int, operand: int, raw: int) -> str:
        """Dispatch and execute a decoded instruction.

        Each case handles one instruction type. The mnemonic string is
        returned for the trace — it's how we make the execution log
        human-readable.
        """

        # --- LDM N (0xDN): Load immediate into accumulator ---
        # The simplest instruction: put a 4-bit constant into A.
        # This is how you get values into the machine — there's no
        # "load from memory" in our minimal set.
        if opcode == 0xD:
            self.accumulator = operand & 0xF
            return f"LDM {operand}"

        # --- XCH RN (0xBN): Exchange accumulator with register ---
        # Swap A and RN. This is the 4004's way of moving data between
        # the accumulator and registers. There's no "move" instruction —
        # you always swap both ways. To "store" A into RN, you XCH (and
        # A gets RN's old value). To "load" RN into A, you also XCH.
        elif opcode == 0xB:
            reg = operand & 0xF
            old_a = self.accumulator
            self.accumulator = self.registers[reg] & 0xF
            self.registers[reg] = old_a & 0xF
            return f"XCH R{reg}"

        # --- ADD RN (0x8N): Add register to accumulator ---
        # A = A + RN. If the result exceeds 15, it wraps around and the
        # carry flag is set. For example: 15 + 1 = 0 with carry=1.
        # The carry flag enables multi-digit BCD addition — the whole
        # reason the 4004 exists.
        elif opcode == 0x8:
            reg = operand & 0xF
            result = self.accumulator + self.registers[reg]
            self.carry = result > 0xF
            self.accumulator = result & 0xF
            return f"ADD R{reg}"

        # --- SUB RN (0x9N): Subtract register from accumulator ---
        # A = A - RN. If the result would be negative, it wraps around
        # (two's complement in 4 bits) and the carry flag is set to
        # indicate a borrow. For example: 0 - 1 = 15 with carry=1.
        elif opcode == 0x9:
            reg = operand & 0xF
            result = self.accumulator - self.registers[reg]
            self.carry = result < 0
            self.accumulator = result & 0xF
            return f"SUB R{reg}"

        # --- HLT (0x01): Halt ---
        # Not a real 4004 instruction — we added it for our simulator.
        # The real 4004 had no halt; it just kept fetching instructions
        # forever (or until power off). We need a way to stop.
        elif raw == 0x01:
            self.halted = True
            return "HLT"

        # --- Unknown instruction ---
        else:
            return f"UNKNOWN(0x{raw:02X})"

    def run(self, program: bytes, max_steps: int = 10000) -> list[Intel4004Trace]:
        """Load and run a program, returning a trace of every instruction.

        Execution continues until HLT is encountered or max_steps is reached.
        The max_steps limit prevents infinite loops from hanging the simulator.

        Example — the x = 1 + 2 program:

            >>> sim = Intel4004Simulator()
            >>> traces = sim.run(bytes([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01]))
            >>> for t in traces:
            ...     print(f"  {t.address:03X}: {t.mnemonic:<10} A={t.accumulator_after}")
              000: LDM 1      A=1
              001: XCH R0     A=0
              002: LDM 2      A=2
              003: ADD R0     A=3
              004: XCH R1     A=0
              005: HLT        A=0
            >>> sim.registers[1]
            3
        """
        self.load_program(program)
        traces: list[Intel4004Trace] = []

        for _ in range(max_steps):
            trace = self.step()
            traces.append(trace)
            if self.halted:
                break

        return traces
