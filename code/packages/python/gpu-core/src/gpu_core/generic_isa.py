"""GenericISA — a simplified, educational instruction set.

=== What is this? ===

This is the default InstructionSet implementation — a vendor-neutral ISA
designed for teaching, not for matching any real hardware. It proves that
the pluggable ISA design works: if you can implement GenericISA, you can
implement NVIDIA PTX, AMD GCN, Intel Xe, or ARM Mali the same way.

=== How it works ===

The GenericISA.execute() method is a big match/case statement. For each
opcode, it:
1. Reads source registers
2. Calls the appropriate fp-arithmetic function
3. Writes the result to the destination register
4. Returns an ExecuteResult describing what happened

    FADD R2, R0, R1:
        a = registers.read(R0)          # read 3.14
        b = registers.read(R1)          # read 2.71
        result = fp_add(a, b)           # 3.14 + 2.71 = 5.85
        registers.write(R2, result)     # store in R2
        return ExecuteResult("R2 = R0 + R1 = 3.14 + 2.71 = 5.85", ...)

=== Future ISAs follow the same pattern ===

    class PTXISA:
        def execute(self, instruction, registers, memory):
            match instruction.opcode:
                case PTXOp.ADD_F32:   # same as FADD but with PTX naming
                case PTXOp.FMA_RN_F32: # same as FFMA but with PTX naming

The GPUCore doesn't care which ISA is plugged in — it just calls
isa.execute() and processes the ExecuteResult.
"""

from __future__ import annotations

from fp_arithmetic import (
    bits_to_float,
    fp_abs,
    fp_add,
    fp_compare,
    fp_fma,
    fp_mul,
    fp_neg,
    fp_sub,
)

from gpu_core.memory import LocalMemory
from gpu_core.opcodes import Instruction, Opcode
from gpu_core.protocols import ExecuteResult
from gpu_core.registers import FPRegisterFile


class GenericISA:
    """A simplified, educational instruction set for GPU cores.

    This ISA is not tied to any vendor — it's a teaching tool. It has
    16 opcodes covering arithmetic, memory, data movement, and control
    flow. Any floating-point program can be expressed with these.

    To use a different ISA, create a class with the same execute() method
    signature and pass it to GPUCore(isa=YourISA()).
    """

    @property
    def name(self) -> str:
        """ISA identifier."""
        return "Generic"

    def execute(
        self,
        instruction: Instruction,
        registers: FPRegisterFile,
        memory: LocalMemory,
    ) -> ExecuteResult:
        """Execute a single instruction.

        This is the heart of the ISA — a dispatch table that maps opcodes
        to their implementations. Each case reads operands, performs the
        operation, writes results, and returns a trace description.
        """
        match instruction.opcode:
            # --- Floating-point arithmetic ---
            case Opcode.FADD:
                return self._exec_fadd(instruction, registers)
            case Opcode.FSUB:
                return self._exec_fsub(instruction, registers)
            case Opcode.FMUL:
                return self._exec_fmul(instruction, registers)
            case Opcode.FFMA:
                return self._exec_ffma(instruction, registers)
            case Opcode.FNEG:
                return self._exec_fneg(instruction, registers)
            case Opcode.FABS:
                return self._exec_fabs(instruction, registers)

            # --- Memory ---
            case Opcode.LOAD:
                return self._exec_load(instruction, registers, memory)
            case Opcode.STORE:
                return self._exec_store(instruction, registers, memory)

            # --- Data movement ---
            case Opcode.MOV:
                return self._exec_mov(instruction, registers)
            case Opcode.LIMM:
                return self._exec_limm(instruction, registers)

            # --- Control flow ---
            case Opcode.BEQ:
                return self._exec_beq(instruction, registers)
            case Opcode.BLT:
                return self._exec_blt(instruction, registers)
            case Opcode.BNE:
                return self._exec_bne(instruction, registers)
            case Opcode.JMP:
                return self._exec_jmp(instruction)
            case Opcode.NOP:
                return ExecuteResult(description="No operation")
            case Opcode.HALT:
                return ExecuteResult(description="Halted", halted=True)
            case _:  # pragma: no cover
                msg = f"Unknown opcode: {instruction.opcode}"
                raise ValueError(msg)

    # --- Arithmetic implementations ---

    def _exec_fadd(
        self, inst: Instruction, regs: FPRegisterFile
    ) -> ExecuteResult:
        """FADD Rd, Rs1, Rs2 → Rd = Rs1 + Rs2."""
        a = regs.read(inst.rs1)
        b = regs.read(inst.rs2)
        result = fp_add(a, b)
        regs.write(inst.rd, result)
        a_f, b_f, r_f = (
            bits_to_float(a),
            bits_to_float(b),
            bits_to_float(result),
        )
        return ExecuteResult(
            description=(
                f"R{inst.rd} = R{inst.rs1} + R{inst.rs2}"
                f" = {a_f} + {b_f} = {r_f}"
            ),
            registers_changed={f"R{inst.rd}": r_f},
        )

    def _exec_fsub(
        self, inst: Instruction, regs: FPRegisterFile
    ) -> ExecuteResult:
        """FSUB Rd, Rs1, Rs2 → Rd = Rs1 - Rs2."""
        a = regs.read(inst.rs1)
        b = regs.read(inst.rs2)
        result = fp_sub(a, b)
        regs.write(inst.rd, result)
        a_f, b_f, r_f = (
            bits_to_float(a),
            bits_to_float(b),
            bits_to_float(result),
        )
        return ExecuteResult(
            description=(
                f"R{inst.rd} = R{inst.rs1} - R{inst.rs2}"
                f" = {a_f} - {b_f} = {r_f}"
            ),
            registers_changed={f"R{inst.rd}": r_f},
        )

    def _exec_fmul(
        self, inst: Instruction, regs: FPRegisterFile
    ) -> ExecuteResult:
        """FMUL Rd, Rs1, Rs2 → Rd = Rs1 × Rs2."""
        a = regs.read(inst.rs1)
        b = regs.read(inst.rs2)
        result = fp_mul(a, b)
        regs.write(inst.rd, result)
        a_f, b_f, r_f = (
            bits_to_float(a),
            bits_to_float(b),
            bits_to_float(result),
        )
        return ExecuteResult(
            description=(
                f"R{inst.rd} = R{inst.rs1} * R{inst.rs2}"
                f" = {a_f} * {b_f} = {r_f}"
            ),
            registers_changed={f"R{inst.rd}": r_f},
        )

    def _exec_ffma(
        self, inst: Instruction, regs: FPRegisterFile
    ) -> ExecuteResult:
        """FFMA Rd, Rs1, Rs2, Rs3 → Rd = Rs1 × Rs2 + Rs3."""
        a = regs.read(inst.rs1)
        b = regs.read(inst.rs2)
        c = regs.read(inst.rs3)
        result = fp_fma(a, b, c)
        regs.write(inst.rd, result)
        a_f, b_f, c_f, r_f = (
            bits_to_float(a),
            bits_to_float(b),
            bits_to_float(c),
            bits_to_float(result),
        )
        return ExecuteResult(
            description=(
                f"R{inst.rd} = R{inst.rs1} * R{inst.rs2} + R{inst.rs3}"
                f" = {a_f} * {b_f} + {c_f} = {r_f}"
            ),
            registers_changed={f"R{inst.rd}": r_f},
        )

    def _exec_fneg(
        self, inst: Instruction, regs: FPRegisterFile
    ) -> ExecuteResult:
        """FNEG Rd, Rs1 → Rd = -Rs1."""
        a = regs.read(inst.rs1)
        result = fp_neg(a)
        regs.write(inst.rd, result)
        a_f, r_f = bits_to_float(a), bits_to_float(result)
        return ExecuteResult(
            description=f"R{inst.rd} = -R{inst.rs1} = -{a_f} = {r_f}",
            registers_changed={f"R{inst.rd}": r_f},
        )

    def _exec_fabs(
        self, inst: Instruction, regs: FPRegisterFile
    ) -> ExecuteResult:
        """FABS Rd, Rs1 → Rd = |Rs1|."""
        a = regs.read(inst.rs1)
        result = fp_abs(a)
        regs.write(inst.rd, result)
        a_f, r_f = bits_to_float(a), bits_to_float(result)
        return ExecuteResult(
            description=f"R{inst.rd} = |R{inst.rs1}| = |{a_f}| = {r_f}",
            registers_changed={f"R{inst.rd}": r_f},
        )

    # --- Memory implementations ---

    def _exec_load(
        self,
        inst: Instruction,
        regs: FPRegisterFile,
        memory: LocalMemory,
    ) -> ExecuteResult:
        """LOAD Rd, [Rs1+imm] → Rd = Mem[Rs1 + immediate]."""
        base = bits_to_float(regs.read(inst.rs1))
        address = int(base + inst.immediate)
        value = memory.load_float(address, regs.fmt)
        regs.write(inst.rd, value)
        val_f = bits_to_float(value)
        return ExecuteResult(
            description=(
                f"R{inst.rd} = Mem[R{inst.rs1}+{inst.immediate}]"
                f" = Mem[{address}] = {val_f}"
            ),
            registers_changed={f"R{inst.rd}": val_f},
        )

    def _exec_store(
        self,
        inst: Instruction,
        regs: FPRegisterFile,
        memory: LocalMemory,
    ) -> ExecuteResult:
        """STORE [Rs1+imm], Rs2 → Mem[Rs1 + immediate] = Rs2."""
        base = bits_to_float(regs.read(inst.rs1))
        address = int(base + inst.immediate)
        value = regs.read(inst.rs2)
        memory.store_float(address, value)
        val_f = bits_to_float(value)
        return ExecuteResult(
            description=(
                f"Mem[R{inst.rs1}+{inst.immediate}] = R{inst.rs2}"
                f" → Mem[{address}] = {val_f}"
            ),
            memory_changed={address: val_f},
        )

    # --- Data movement implementations ---

    def _exec_mov(
        self, inst: Instruction, regs: FPRegisterFile
    ) -> ExecuteResult:
        """MOV Rd, Rs1 → Rd = Rs1."""
        value = regs.read(inst.rs1)
        regs.write(inst.rd, value)
        val_f = bits_to_float(value)
        return ExecuteResult(
            description=f"R{inst.rd} = R{inst.rs1} = {val_f}",
            registers_changed={f"R{inst.rd}": val_f},
        )

    def _exec_limm(
        self, inst: Instruction, regs: FPRegisterFile
    ) -> ExecuteResult:
        """LIMM Rd, immediate → Rd = float literal."""
        regs.write_float(inst.rd, inst.immediate)
        return ExecuteResult(
            description=f"R{inst.rd} = {inst.immediate}",
            registers_changed={f"R{inst.rd}": inst.immediate},
        )

    # --- Control flow implementations ---

    def _exec_beq(
        self, inst: Instruction, regs: FPRegisterFile
    ) -> ExecuteResult:
        """BEQ Rs1, Rs2, offset → if Rs1 == Rs2: PC += offset."""
        cmp = fp_compare(regs.read(inst.rs1), regs.read(inst.rs2))
        taken = cmp == 0
        offset = int(inst.immediate) if taken else 1
        a_f = bits_to_float(regs.read(inst.rs1))
        b_f = bits_to_float(regs.read(inst.rs2))
        return ExecuteResult(
            description=(
                f"BEQ R{inst.rs1}({a_f}) == R{inst.rs2}({b_f})? "
                f"{'Yes → branch' if taken else 'No → fall through'}"
            ),
            next_pc_offset=offset,
        )

    def _exec_blt(
        self, inst: Instruction, regs: FPRegisterFile
    ) -> ExecuteResult:
        """BLT Rs1, Rs2, offset → if Rs1 < Rs2: PC += offset."""
        cmp = fp_compare(regs.read(inst.rs1), regs.read(inst.rs2))
        taken = cmp < 0
        offset = int(inst.immediate) if taken else 1
        a_f = bits_to_float(regs.read(inst.rs1))
        b_f = bits_to_float(regs.read(inst.rs2))
        return ExecuteResult(
            description=(
                f"BLT R{inst.rs1}({a_f}) < R{inst.rs2}({b_f})? "
                f"{'Yes → branch' if taken else 'No → fall through'}"
            ),
            next_pc_offset=offset,
        )

    def _exec_bne(
        self, inst: Instruction, regs: FPRegisterFile
    ) -> ExecuteResult:
        """BNE Rs1, Rs2, offset → if Rs1 != Rs2: PC += offset."""
        cmp = fp_compare(regs.read(inst.rs1), regs.read(inst.rs2))
        taken = cmp != 0
        offset = int(inst.immediate) if taken else 1
        a_f = bits_to_float(regs.read(inst.rs1))
        b_f = bits_to_float(regs.read(inst.rs2))
        return ExecuteResult(
            description=(
                f"BNE R{inst.rs1}({a_f}) != R{inst.rs2}({b_f})? "
                f"{'Yes → branch' if taken else 'No → fall through'}"
            ),
            next_pc_offset=offset,
        )

    def _exec_jmp(self, inst: Instruction) -> ExecuteResult:
        """JMP target → PC = target (absolute jump)."""
        target = int(inst.immediate)
        return ExecuteResult(
            description=f"Jump to PC={target}",
            next_pc_offset=target,
            absolute_jump=True,
        )
