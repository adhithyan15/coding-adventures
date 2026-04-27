"""ARM Simulator — the architecture that powers your phone.

=== What is ARM? ===

ARM (originally Acorn RISC Machine) was designed in 1985 by Sophie Wilson and
Steve Furber at Acorn Computers in Cambridge, England. It was one of the first
commercial RISC processors — inspired by the Berkeley RISC project that also
influenced MIPS and eventually RISC-V.

ARM's big insight was power efficiency. While Intel focused on raw speed, ARM
optimized for low power consumption. This bet paid off spectacularly: today ARM
processors are in virtually every smartphone, tablet, and embedded device on
Earth. Apple's M-series chips are ARM. Your phone is ARM. Most of the world's
CPUs are ARM.

=== ARM vs RISC-V ===

    ARM:      16 registers (R0-R15). Condition codes on every instruction.
              More complex encoding. Commercial (licensed by ARM Ltd).
              Designed 1985. Mature, battle-tested, ubiquitous.

    RISC-V:   32 registers (x0-x31). No condition codes. Clean, regular
              encoding. Open-source. Designed 2010. The "clean slate" ISA.

The biggest architectural difference is conditional execution. In ARM, EVERY
instruction has a 4-bit condition field. This means you can write:

    CMP R0, R1           ; compare R0 and R1, set flags
    ADDGT R2, R0, R1     ; add ONLY IF R0 > R1 (Greater Than)
    SUBLE R2, R0, R1     ; subtract ONLY IF R0 <= R1 (Less or Equal)

RISC-V doesn't have this — it uses separate branch instructions instead.
ARM's approach reduces branch instructions (good for pipelines) but makes
the encoding more complex.

=== Register conventions ===

ARM has 16 registers, each 32 bits wide:

    R0-R3   = function arguments and return values
    R4-R11  = general purpose (callee-saved)
    R12     = IP (intra-procedure scratch register)
    R13     = SP (stack pointer)
    R14     = LR (link register — return address)
    R15     = PC (program counter — yes, it's a visible register!)

Unlike RISC-V, ARM has no hardwired-zero register. R15 being the PC is
a quirk that allows some clever tricks (and some nasty bugs).

=== Instruction encoding ===

Every ARM instruction is exactly 32 bits. The condition code is ALWAYS in
bits [31:28] — this is what makes conditional execution possible.

Data processing format:
    [cond(4) | 00 | I(1) | opcode(4) | S(1) | Rn(4) | Rd(4) | operand2(12)]
     31   28  27 26  25    24     21   20     19  16   15  12   11         0

    cond:     Condition code (0b1110 = AL = always execute)
    I:        Immediate flag (1 = operand2 is an immediate, 0 = register)
    opcode:   Which operation (0b1101=MOV, 0b0100=ADD, 0b0010=SUB)
    S:        Set condition flags (we use 0 for now)
    Rn:       First source register
    Rd:       Destination register
    operand2: Either an 8-bit immediate with 4-bit rotation, or a register

When I=1, operand2 encodes an immediate as:
    [rotate(4) | imm8(8)]
    The actual value is: imm8 rotated right by (rotate * 2) positions

When I=0, operand2's lowest 4 bits are the register number (Rm).

=== MVP instruction set (just enough for x = 1 + 2) ===

    MOV R0, #1         -> R0 = 1          (data processing, I=1, opcode=MOV)
    MOV R1, #2         -> R1 = 2          (data processing, I=1, opcode=MOV)
    ADD R2, R0, R1     -> R2 = R0 + R1    (data processing, I=0, opcode=ADD)
    HLT                -> halt             (custom encoding)
"""

from __future__ import annotations

from dataclasses import dataclass

from cpu_simulator.cpu import CPU
from cpu_simulator.memory import Memory
from cpu_simulator.pipeline import DecodeResult, ExecuteResult, PipelineTrace
from cpu_simulator.registers import RegisterFile


# ---------------------------------------------------------------------------
# Instruction encoding constants
# ---------------------------------------------------------------------------
# ARM data processing opcodes (bits [24:21] of the instruction word).
# The condition code in bits [31:28] is always 0b1110 (AL = always).

COND_AL = 0b1110  # Always execute — the most common condition code

OPCODE_MOV = 0b1101  # MOV Rd, operand2 (Rd = operand2, ignores Rn)
OPCODE_ADD = 0b0100  # ADD Rd, Rn, operand2 (Rd = Rn + operand2)
OPCODE_SUB = 0b0010  # SUB Rd, Rn, operand2 (Rd = Rn - operand2)

# We encode HLT as a special sentinel: all condition bits set to 0b1111
# (which is "never execute" in ARMv4, repurposed here as halt).
HLT_INSTRUCTION = 0xFFFFFFFF


# ---------------------------------------------------------------------------
# Decoder
# ---------------------------------------------------------------------------


class ARMDecoder:
    """Decodes ARM data processing instructions from 32-bit binary to structured fields.

    The decoder extracts the condition code, opcode, register numbers, and
    immediate values from the raw instruction bits. It doesn't execute
    anything — it just figures out what the instruction means.

    ARM's encoding is more complex than RISC-V's because of the condition
    field and the flexible operand2 encoding. But the data processing format
    is regular: the opcode is always in bits [24:21], Rd in [15:12], and
    Rn in [19:16].

    Example: decoding MOV R0, #1 (binary: 0xE3A00001)

        Bits: 1110 00 1 1101 0 0000 0000 000000000001
              ^^^^ ^^ ^ ^^^^ ^ ^^^^ ^^^^ ^^^^^^^^^^^^
              cond    I  MOV S  Rn   Rd    operand2

        Result: DecodeResult(mnemonic="mov", fields={"rd": 0, "imm": 1})
    """

    def decode(self, raw: int, pc: int) -> DecodeResult:
        """Decode a 32-bit ARM instruction.

        Checks for the HLT sentinel first, then extracts the condition
        code and dispatches to the data processing decoder.
        """
        # Check for our custom halt instruction
        if raw == HLT_INSTRUCTION:
            return DecodeResult(
                mnemonic="hlt", fields={}, raw_instruction=raw
            )

        return self._decode_data_processing(raw)

    def _decode_data_processing(self, raw: int) -> DecodeResult:
        """Decode an ARM data processing instruction.

        Data processing format:
            [cond(4) | 00 | I(1) | opcode(4) | S(1) | Rn(4) | Rd(4) | operand2(12)]
             31   28  27 26  25    24     21   20     19  16   15  12   11         0

        The I bit determines how operand2 is interpreted:
            I=1: immediate — [rotate(4) | imm8(8)]
            I=0: register  — lowest 4 bits are Rm

        Example: ADD R2, R0, R1 (register form)
            cond=1110, I=0, opcode=0100, S=0, Rn=0, Rd=2, operand2=...0001
            → Rm=1
        """
        cond = (raw >> 28) & 0xF
        i_bit = (raw >> 25) & 0x1
        opcode = (raw >> 21) & 0xF
        s_bit = (raw >> 20) & 0x1
        rn = (raw >> 16) & 0xF
        rd = (raw >> 12) & 0xF
        operand2 = raw & 0xFFF

        # Determine the mnemonic from the opcode
        if opcode == OPCODE_MOV:
            mnemonic = "mov"
        elif opcode == OPCODE_ADD:
            mnemonic = "add"
        elif opcode == OPCODE_SUB:
            mnemonic = "sub"
        else:
            mnemonic = f"dp_op({opcode:#06b})"

        # Decode operand2 based on the I bit
        if i_bit == 1:
            # Immediate: operand2 = [rotate(4) | imm8(8)]
            # Actual value = imm8 rotated right by (rotate * 2)
            rotate = (operand2 >> 8) & 0xF
            imm8 = operand2 & 0xFF
            # Rotate right by (rotate * 2) positions in a 32-bit field
            shift = rotate * 2
            if shift > 0:
                imm_value = ((imm8 >> shift) | (imm8 << (32 - shift))) & 0xFFFFFFFF
            else:
                imm_value = imm8

            fields: dict[str, int] = {
                "cond": cond,
                "i_bit": i_bit,
                "opcode": opcode,
                "s_bit": s_bit,
                "rn": rn,
                "rd": rd,
                "imm": imm_value,
            }
        else:
            # Register: lowest 4 bits of operand2 are Rm
            rm = operand2 & 0xF

            fields = {
                "cond": cond,
                "i_bit": i_bit,
                "opcode": opcode,
                "s_bit": s_bit,
                "rn": rn,
                "rd": rd,
                "rm": rm,
            }

        return DecodeResult(
            mnemonic=mnemonic,
            fields=fields,
            raw_instruction=raw,
        )


# ---------------------------------------------------------------------------
# Executor
# ---------------------------------------------------------------------------


class ARMExecutor:
    """Executes decoded ARM instructions.

    The executor reads register values, performs the operation, writes the
    result back, and determines the next PC.

    Unlike RISC-V, ARM has no hardwired-zero register. All 16 registers
    (R0-R15) are writable. R15 is the PC, but in our simplified simulator
    we manage the PC separately and don't allow direct writes to it.
    """

    def execute(
        self,
        decoded: DecodeResult,
        registers: RegisterFile,
        memory: Memory,
        pc: int,
    ) -> ExecuteResult:
        """Execute one decoded ARM instruction."""
        mnemonic = decoded.mnemonic

        if mnemonic == "mov":
            return self._exec_mov(decoded, registers, pc)
        elif mnemonic == "add":
            return self._exec_add(decoded, registers, pc)
        elif mnemonic == "sub":
            return self._exec_sub(decoded, registers, pc)
        elif mnemonic == "hlt":
            return ExecuteResult(
                description="Halt",
                registers_changed={},
                memory_changed={},
                next_pc=pc,
                halted=True,
            )
        else:
            return ExecuteResult(
                description=f"Unknown instruction: {mnemonic}",
                registers_changed={},
                memory_changed={},
                next_pc=pc + 4,
            )

    def _exec_mov(
        self, decoded: DecodeResult, registers: RegisterFile, pc: int
    ) -> ExecuteResult:
        """Execute: MOV Rd, #imm -> Rd = imm

        MOV is special among data processing instructions: it ignores Rn
        and writes operand2 directly into Rd.

        Example: MOV R0, #1
            imm = 1
            Write 1 to R0
        """
        rd = decoded.fields["rd"]
        imm = decoded.fields["imm"]

        result = imm & 0xFFFFFFFF
        registers.write(rd, result)
        changes: dict[str, int] = {f"R{rd}": result}

        return ExecuteResult(
            description=f"R{rd} = {result}",
            registers_changed=changes,
            memory_changed={},
            next_pc=pc + 4,
        )

    def _exec_add(
        self, decoded: DecodeResult, registers: RegisterFile, pc: int
    ) -> ExecuteResult:
        """Execute: ADD Rd, Rn, Rm -> Rd = Rn + Rm

        Example: ADD R2, R0, R1  (where R0=1, R1=2)
            rn_val = 1, rm_val = 2
            result = 1 + 2 = 3
            Write 3 to R2
        """
        rd = decoded.fields["rd"]
        rn = decoded.fields["rn"]
        rm = decoded.fields["rm"]

        rn_val = registers.read(rn)
        rm_val = registers.read(rm)
        result = (rn_val + rm_val) & 0xFFFFFFFF

        registers.write(rd, result)
        changes: dict[str, int] = {f"R{rd}": result}

        return ExecuteResult(
            description=f"R{rd} = R{rn}({rn_val}) + R{rm}({rm_val}) = {result}",
            registers_changed=changes,
            memory_changed={},
            next_pc=pc + 4,
        )

    def _exec_sub(
        self, decoded: DecodeResult, registers: RegisterFile, pc: int
    ) -> ExecuteResult:
        """Execute: SUB Rd, Rn, Rm -> Rd = Rn - Rm"""
        rd = decoded.fields["rd"]
        rn = decoded.fields["rn"]
        rm = decoded.fields["rm"]

        rn_val = registers.read(rn)
        rm_val = registers.read(rm)
        result = (rn_val - rm_val) & 0xFFFFFFFF

        registers.write(rd, result)
        changes: dict[str, int] = {f"R{rd}": result}

        return ExecuteResult(
            description=f"R{rd} = R{rn}({rn_val}) - R{rm}({rm_val}) = {result}",
            registers_changed=changes,
            memory_changed={},
            next_pc=pc + 4,
        )


# ---------------------------------------------------------------------------
# Assembler helpers
# ---------------------------------------------------------------------------
# These functions encode ARM instructions from human-readable form
# to binary. This is a tiny assembler — just enough to create test programs.


def encode_mov_imm(rd: int, imm: int) -> int:
    """Encode: MOV Rd, #imm -> 32-bit instruction.

    Data processing format with I=1, opcode=MOV(1101):
        [cond=1110 | 00 | I=1 | 1101 | S=0 | Rn=0000 | Rd | rotate=0000 | imm8]

    The immediate must fit in 8 bits (0-255) with no rotation for this
    simple encoder.

    Example:
        >>> hex(encode_mov_imm(0, 1))  # MOV R0, #1
        '0xe3a00001'
    """
    cond = COND_AL
    i_bit = 1
    opcode = OPCODE_MOV
    s_bit = 0
    rn = 0  # MOV ignores Rn, conventionally set to 0
    imm8 = imm & 0xFF
    rotate = 0

    return (
        (cond << 28)
        | (0b00 << 26)
        | (i_bit << 25)
        | (opcode << 21)
        | (s_bit << 20)
        | (rn << 16)
        | (rd << 12)
        | (rotate << 8)
        | imm8
    )


def encode_add(rd: int, rn: int, rm: int) -> int:
    """Encode: ADD Rd, Rn, Rm -> 32-bit instruction.

    Data processing format with I=0, opcode=ADD(0100):
        [cond=1110 | 00 | I=0 | 0100 | S=0 | Rn | Rd | 00000000 | Rm]

    Example:
        >>> hex(encode_add(2, 0, 1))  # ADD R2, R0, R1
        '0xe0802001'
    """
    cond = COND_AL
    i_bit = 0
    opcode = OPCODE_ADD
    s_bit = 0

    return (
        (cond << 28)
        | (0b00 << 26)
        | (i_bit << 25)
        | (opcode << 21)
        | (s_bit << 20)
        | (rn << 16)
        | (rd << 12)
        | rm
    )


def encode_sub(rd: int, rn: int, rm: int) -> int:
    """Encode: SUB Rd, Rn, Rm -> 32-bit instruction.

    Data processing format with I=0, opcode=SUB(0010):
        [cond=1110 | 00 | I=0 | 0010 | S=0 | Rn | Rd | 00000000 | Rm]

    Example:
        >>> hex(encode_sub(2, 0, 1))  # SUB R2, R0, R1
        '0xe0402001'
    """
    cond = COND_AL
    i_bit = 0
    opcode = OPCODE_SUB
    s_bit = 0

    return (
        (cond << 28)
        | (0b00 << 26)
        | (i_bit << 25)
        | (opcode << 21)
        | (s_bit << 20)
        | (rn << 16)
        | (rd << 12)
        | rm
    )


def encode_hlt() -> int:
    """Encode: HLT -> 32-bit instruction.

    We use 0xFFFFFFFF as a custom halt sentinel. In real ARM, this would
    be an unconditional instruction with condition 0b1111. We repurpose
    it as a clean way to stop the simulator.

    Example:
        >>> hex(encode_hlt())
        '0xffffffff'
    """
    return HLT_INSTRUCTION


# ---------------------------------------------------------------------------
# High-level simulator
# ---------------------------------------------------------------------------


class ARMSimulator:
    """Complete ARM simulator — ISA + CPU in one convenient class.

    This wraps the CPU simulator with the ARM decoder and executor,
    providing a simple interface for running ARM programs.

    Example: running x = 1 + 2

        >>> sim = ARMSimulator()
        >>> program = assemble([
        ...     encode_mov_imm(0, 1),     # R0 = 1
        ...     encode_mov_imm(1, 2),     # R1 = 2
        ...     encode_add(2, 0, 1),      # R2 = R0 + R1 = 3
        ...     encode_hlt(),              # halt
        ... ])
        >>> traces = sim.run(program)
        >>> sim.cpu.registers.read(2)
        3

        The pipeline trace for each instruction shows:
        --- Cycle 0 ---
          FETCH              | DECODE             | EXECUTE
          PC: 0x0000         | mov                | R0 = 1
          -> 0xE3A00001      | rd=0 imm=1         | PC -> 4
    """

    def __init__(self, memory_size: int = 65536) -> None:
        self.decoder = ARMDecoder()
        self.executor = ARMExecutor()
        self.cpu = CPU(
            decoder=self.decoder,
            executor=self.executor,
            num_registers=16,  # ARM has 16 registers (R0-R15)
            bit_width=32,
            memory_size=memory_size,
        )

    def run(self, program: bytes) -> list[PipelineTrace]:
        """Load and run an ARM program, returning the pipeline trace."""
        self.cpu.load_program(program)
        return self.cpu.run()

    def step(self) -> PipelineTrace:
        """Execute one instruction and return its pipeline trace."""
        return self.cpu.step()

    def reset(self) -> None:
        """Reset the simulator to power-on state (simulator-protocol).

        Because the ``CPU`` class does not expose a ``reset()`` method, we
        recreate it from scratch with the same parameters.  This clears all
        registers, zeroes memory, sets PC to 0, and clears the halted flag.

        After ``reset()``:
          - All 16 registers (R0–R15) are 0.
          - PC is 0.
          - Memory is zeroed.
          - ``cpu.halted`` is False.
          - ``cpu.cycle`` counter is 0.
        """
        memory_size = self.cpu.memory.size
        self.cpu = CPU(
            decoder=self.decoder,
            executor=self.executor,
            num_registers=16,
            bit_width=32,
            memory_size=memory_size,
        )

    def load(self, program: bytes) -> None:
        """Load binary program into memory at address 0 (simulator-protocol).

        Writes the program bytes into the CPU's memory starting at offset 0
        and resets the program counter to 0.  Does NOT reset other CPU state
        (registers, halted flag) — call ``reset()`` first if needed.

        Args:
            program: Raw 32-bit little-endian ARM instruction bytes.
        """
        self.cpu.load_program(program)

    def get_state(self) -> "ARMState":
        """Return a frozen snapshot of the current ARM CPU state.

        Conforms to the ``Simulator[ARMState]`` protocol.

        Captures the full CPU state as immutable data:
          - Register values are copied into a tuple (R0–R15).
          - PC is ``registers[15]`` — also stored as a convenience field.
          - Condition flags are ``(False, False, False, False)`` in the
            current MVP (no S-bit instructions implemented).
          - Memory is copied into a ``bytes`` object (immutable snapshot).
          - ``halted`` reflects ``cpu.halted``.

        Returns:
            A frozen ``ARMState`` snapshot.

        Examples
        --------
        >>> sim = ARMSimulator()
        >>> state = sim.get_state()
        >>> state.pc
        0
        >>> state.halted
        False
        """
        from arm_simulator.state import ARMState

        num_regs = self.cpu.registers.num_registers
        register_values = tuple(self.cpu.registers.read(i) for i in range(num_regs))

        # Condition flags: our MVP instructions all use S=0, so no flags
        # are updated. We expose them as a 4-tuple (N, Z, C, V) of False.
        # When S-bit instructions are added, this should read from a CPSR.
        flags: tuple[bool, bool, bool, bool] = (False, False, False, False)

        return ARMState(
            registers=register_values,
            pc=self.cpu.pc,
            flags=flags,
            memory=bytes(self.cpu.memory._data),
            halted=self.cpu.halted,
        )

    def execute(
        self,
        program: bytes,
        max_steps: int = 100_000,
    ) -> "ExecutionResult[ARMState]":
        """Load program, run to HLT or max_steps, return ExecutionResult.

        Conforms to the ``Simulator[ARMState]`` protocol.  This is the
        recommended entry point for end-to-end testing:

            result = sim.execute(machine_code)
            assert result.ok
            assert result.final_state.registers[2] == 3  # R2 = 3

        The method:
          1. Resets the simulator (all registers zeroed, PC = 0).
          2. Loads the program bytes at address 0.
          3. Steps until the HLT sentinel (0xFFFFFFFF) or ``max_steps``.
          4. Returns a full ``ExecutionResult`` with trace and final state.

        Note: existing ``run()`` is unchanged and continues to return
        ``list[PipelineTrace]`` as before.

        Args:
            program:   Raw 32-bit little-endian ARM machine-code bytes.
            max_steps: Safety limit to prevent infinite loops (default 100,000).

        Returns:
            ``ExecutionResult[ARMState]`` with:
            - ``halted``:      True if the HLT sentinel was reached.
            - ``steps``:       Number of instructions executed.
            - ``final_state``: Frozen ``ARMState`` at termination.
            - ``error``:       None on clean halt; error string otherwise.
            - ``traces``:      List of ``StepTrace`` (one per instruction).

        Examples
        --------
        >>> sim = ARMSimulator()
        >>> from arm_simulator.simulator import assemble, encode_hlt
        >>> result = sim.execute(assemble([encode_hlt()]))
        >>> result.ok
        True
        >>> result.steps
        1
        """
        from simulator_protocol import ExecutionResult, StepTrace
        from arm_simulator.state import ARMState  # noqa: F401

        self.reset()
        self.load(program)

        protocol_traces: list[StepTrace] = []
        steps = 0

        while not self.cpu.halted and steps < max_steps:
            pc_before = self.cpu.pc
            pipeline_trace = self.cpu.step()
            protocol_traces.append(
                StepTrace(
                    pc_before=pc_before,
                    pc_after=self.cpu.pc,
                    mnemonic=pipeline_trace.decode.mnemonic,
                    description=(
                        f"{pipeline_trace.decode.mnemonic} @ 0x{pc_before:08X}"
                    ),
                )
            )
            steps += 1

        return ExecutionResult(
            halted=self.cpu.halted,
            steps=steps,
            final_state=self.get_state(),
            error=(
                None
                if self.cpu.halted
                else f"max_steps ({max_steps}) exceeded"
            ),
            traces=protocol_traces,
        )


def assemble(instructions: list[int]) -> bytes:
    """Convert a list of 32-bit instruction words to bytes (little-endian).

    ARM uses little-endian byte order (in its default configuration).
    Each instruction is 4 bytes.

    This is a convenience function for creating test programs:

        >>> program = assemble([
        ...     encode_mov_imm(0, 1),     # R0 = 1
        ...     encode_mov_imm(1, 2),     # R1 = 2
        ...     encode_add(2, 0, 1),      # R2 = R0 + R1
        ...     encode_hlt(),              # halt
        ... ])
    """
    result = b""
    for instr in instructions:
        result += (instr & 0xFFFFFFFF).to_bytes(4, byteorder="little")
    return result
