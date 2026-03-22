"""Instruction executor for all RV32I + M-mode instructions.

=== How execution works ===

After the decoder extracts instruction fields, the executor performs the
actual computation: arithmetic, memory access, branching, or privilege
operations. Each instruction produces an ExecuteResult describing what
changed (registers, memory, next PC).

=== The x0 invariant ===

Every instruction that writes to a register must check: is the destination
register x0? If so, the write is silently discarded.

=== Signed vs unsigned arithmetic ===

RISC-V registers hold 32-bit values. Some instructions interpret these as
signed (two's complement) and others as unsigned. We use helper functions
to convert between representations.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from cpu_simulator.pipeline import DecodeResult, ExecuteResult

if TYPE_CHECKING:
    from cpu_simulator.memory import Memory
    from cpu_simulator.registers import RegisterFile

    from riscv_simulator.csr import CSRFile


def _to_signed(val: int) -> int:
    """Interpret a 32-bit unsigned value as signed."""
    val = val & 0xFFFFFFFF
    if val & 0x80000000:
        return val - 0x100000000
    return val


def _to_unsigned(val: int) -> int:
    """Mask a value to 32-bit unsigned."""
    return val & 0xFFFFFFFF


def _write_rd(
    registers: RegisterFile, rd: int, value: int
) -> dict[str, int]:
    """Write to destination register, respecting the x0 invariant."""
    changes: dict[str, int] = {}
    value = _to_unsigned(value)
    if rd != 0:
        registers.write(rd, value)
        changes[f"x{rd}"] = value
    return changes


class RiscVExecutor:
    """Executes decoded RISC-V instructions."""

    def __init__(self, csr: CSRFile | None = None) -> None:
        self.csr = csr

    def execute(
        self,
        decoded: DecodeResult,
        registers: RegisterFile,
        memory: Memory,
        pc: int,
    ) -> ExecuteResult:
        """Execute one decoded RISC-V instruction."""
        m = decoded.mnemonic

        # I-type arithmetic
        if m == "addi":
            return self._exec_imm_arith(decoded, registers, pc, lambda a, b: _to_unsigned(a + b))
        if m == "slti":
            return self._exec_imm_arith(decoded, registers, pc, lambda a, b: 1 if a < b else 0)
        if m == "sltiu":
            return self._exec_imm_arith(
                decoded, registers, pc,
                lambda a, b: 1 if _to_unsigned(a) < _to_unsigned(b) else 0,
            )
        if m == "xori":
            return self._exec_imm_arith(decoded, registers, pc, lambda a, b: _to_unsigned(a) ^ _to_unsigned(b))
        if m == "ori":
            return self._exec_imm_arith(decoded, registers, pc, lambda a, b: _to_unsigned(a) | _to_unsigned(b))
        if m == "andi":
            return self._exec_imm_arith(decoded, registers, pc, lambda a, b: _to_unsigned(a) & _to_unsigned(b))

        # Shift immediate
        if m == "slli":
            return self._exec_shift_imm(decoded, registers, pc, lambda v, s: (v << s) & 0xFFFFFFFF)
        if m == "srli":
            return self._exec_shift_imm(decoded, registers, pc, lambda v, s: v >> s)
        if m == "srai":
            return self._exec_shift_imm(decoded, registers, pc, lambda v, s: _to_unsigned(_to_signed(v) >> s))

        # R-type arithmetic
        if m == "add":
            return self._exec_reg_arith(decoded, registers, pc, lambda a, b: _to_unsigned(_to_signed(a) + _to_signed(b)))
        if m == "sub":
            return self._exec_reg_arith(decoded, registers, pc, lambda a, b: _to_unsigned(_to_signed(a) - _to_signed(b)))
        if m == "sll":
            return self._exec_reg_arith(decoded, registers, pc, lambda a, b: (a << (b & 0x1F)) & 0xFFFFFFFF)
        if m == "slt":
            return self._exec_reg_arith(decoded, registers, pc, lambda a, b: 1 if _to_signed(a) < _to_signed(b) else 0)
        if m == "sltu":
            return self._exec_reg_arith(decoded, registers, pc, lambda a, b: 1 if a < b else 0)
        if m == "xor":
            return self._exec_reg_arith(decoded, registers, pc, lambda a, b: a ^ b)
        if m == "srl":
            return self._exec_reg_arith(decoded, registers, pc, lambda a, b: a >> (b & 0x1F))
        if m == "sra":
            return self._exec_reg_arith(decoded, registers, pc, lambda a, b: _to_unsigned(_to_signed(a) >> (b & 0x1F)))
        if m == "or":
            return self._exec_reg_arith(decoded, registers, pc, lambda a, b: a | b)
        if m == "and":
            return self._exec_reg_arith(decoded, registers, pc, lambda a, b: a & b)

        # Loads
        if m in ("lb", "lh", "lw", "lbu", "lhu"):
            return self._exec_load(decoded, registers, memory, pc)

        # Stores
        if m in ("sb", "sh", "sw"):
            return self._exec_store(decoded, registers, memory, pc)

        # Branches
        if m == "beq":
            return self._exec_branch(decoded, registers, pc, lambda a, b: a == b)
        if m == "bne":
            return self._exec_branch(decoded, registers, pc, lambda a, b: a != b)
        if m == "blt":
            return self._exec_branch(decoded, registers, pc, lambda a, b: _to_signed(a) < _to_signed(b))
        if m == "bge":
            return self._exec_branch(decoded, registers, pc, lambda a, b: _to_signed(a) >= _to_signed(b))
        if m == "bltu":
            return self._exec_branch(decoded, registers, pc, lambda a, b: a < b)
        if m == "bgeu":
            return self._exec_branch(decoded, registers, pc, lambda a, b: a >= b)

        # Jumps
        if m == "jal":
            return self._exec_jal(decoded, registers, pc)
        if m == "jalr":
            return self._exec_jalr(decoded, registers, pc)

        # Upper immediates
        if m == "lui":
            return self._exec_lui(decoded, registers, pc)
        if m == "auipc":
            return self._exec_auipc(decoded, registers, pc)

        # System / privileged
        if m == "ecall":
            return self._exec_ecall(decoded, registers, pc)
        if m == "mret":
            return self._exec_mret(decoded, registers, pc)
        if m == "csrrw":
            return self._exec_csrrw(decoded, registers, pc)
        if m == "csrrs":
            return self._exec_csrrs(decoded, registers, pc)
        if m == "csrrc":
            return self._exec_csrrc(decoded, registers, pc)

        return ExecuteResult(
            description=f"Unknown instruction: {m}",
            registers_changed={},
            memory_changed={},
            next_pc=pc + 4,
        )

    # === Helpers ===

    def _exec_imm_arith(
        self,
        decoded: DecodeResult,
        registers: RegisterFile,
        pc: int,
        op: object,
    ) -> ExecuteResult:
        rd = decoded.fields["rd"]
        rs1 = decoded.fields["rs1"]
        imm = decoded.fields["imm"]
        rs1_val = _to_signed(registers.read(rs1))
        result = _to_unsigned(op(rs1_val, _to_signed(imm)))  # type: ignore[operator]
        changes = _write_rd(registers, rd, result)
        return ExecuteResult(
            description=f"{decoded.mnemonic}: x{rd} = {result}",
            registers_changed=changes,
            memory_changed={},
            next_pc=pc + 4,
        )

    def _exec_shift_imm(
        self,
        decoded: DecodeResult,
        registers: RegisterFile,
        pc: int,
        op: object,
    ) -> ExecuteResult:
        rd = decoded.fields["rd"]
        rs1 = decoded.fields["rs1"]
        shamt = decoded.fields["imm"] & 0x1F
        rs1_val = registers.read(rs1)
        result = _to_unsigned(op(rs1_val, shamt))  # type: ignore[operator]
        changes = _write_rd(registers, rd, result)
        return ExecuteResult(
            description=f"{decoded.mnemonic}: x{rd} = {result}",
            registers_changed=changes,
            memory_changed={},
            next_pc=pc + 4,
        )

    def _exec_reg_arith(
        self,
        decoded: DecodeResult,
        registers: RegisterFile,
        pc: int,
        op: object,
    ) -> ExecuteResult:
        rd = decoded.fields["rd"]
        rs1 = decoded.fields["rs1"]
        rs2 = decoded.fields["rs2"]
        rs1_val = registers.read(rs1)
        rs2_val = registers.read(rs2)
        result = _to_unsigned(op(rs1_val, rs2_val))  # type: ignore[operator]
        changes = _write_rd(registers, rd, result)
        return ExecuteResult(
            description=f"{decoded.mnemonic}: x{rd} = {result}",
            registers_changed=changes,
            memory_changed={},
            next_pc=pc + 4,
        )

    def _exec_load(
        self,
        decoded: DecodeResult,
        registers: RegisterFile,
        memory: Memory,
        pc: int,
    ) -> ExecuteResult:
        rd = decoded.fields["rd"]
        rs1 = decoded.fields["rs1"]
        imm = decoded.fields["imm"]
        m = decoded.mnemonic

        addr = _to_unsigned(_to_signed(registers.read(rs1)) + _to_signed(imm))

        if m == "lb":
            b = memory.read_byte(addr)
            # Sign-extend byte
            result = _to_unsigned(b - 256 if b & 0x80 else b)
        elif m == "lh":
            lo = memory.read_byte(addr)
            hi = memory.read_byte(addr + 1)
            half = lo | (hi << 8)
            # Sign-extend halfword
            result = _to_unsigned(half - 0x10000 if half & 0x8000 else half)
        elif m == "lw":
            result = memory.read_word(addr)
        elif m == "lbu":
            result = memory.read_byte(addr)
        elif m == "lhu":
            lo = memory.read_byte(addr)
            hi = memory.read_byte(addr + 1)
            result = lo | (hi << 8)
        else:
            result = 0

        changes = _write_rd(registers, rd, result)
        return ExecuteResult(
            description=f"{m}: x{rd} = mem[{addr}] = {result}",
            registers_changed=changes,
            memory_changed={},
            next_pc=pc + 4,
        )

    def _exec_store(
        self,
        decoded: DecodeResult,
        registers: RegisterFile,
        memory: Memory,
        pc: int,
    ) -> ExecuteResult:
        rs1 = decoded.fields["rs1"]
        rs2 = decoded.fields["rs2"]
        imm = decoded.fields["imm"]
        m = decoded.mnemonic

        addr = _to_unsigned(_to_signed(registers.read(rs1)) + _to_signed(imm))
        val = registers.read(rs2)
        mem_changes: dict[int, int] = {}

        if m == "sb":
            b = val & 0xFF
            memory.write_byte(addr, b)
            mem_changes[addr] = b
        elif m == "sh":
            lo = val & 0xFF
            hi = (val >> 8) & 0xFF
            memory.write_byte(addr, lo)
            memory.write_byte(addr + 1, hi)
            mem_changes[addr] = lo
            mem_changes[addr + 1] = hi
        elif m == "sw":
            memory.write_word(addr, val)
            mem_changes[addr] = val & 0xFF
            mem_changes[addr + 1] = (val >> 8) & 0xFF
            mem_changes[addr + 2] = (val >> 16) & 0xFF
            mem_changes[addr + 3] = (val >> 24) & 0xFF

        return ExecuteResult(
            description=f"{m}: mem[{addr}] = {val}",
            registers_changed={},
            memory_changed=mem_changes,
            next_pc=pc + 4,
        )

    def _exec_branch(
        self,
        decoded: DecodeResult,
        registers: RegisterFile,
        pc: int,
        cond: object,
    ) -> ExecuteResult:
        rs1 = decoded.fields["rs1"]
        rs2 = decoded.fields["rs2"]
        imm = decoded.fields["imm"]

        rs1_val = registers.read(rs1)
        rs2_val = registers.read(rs2)
        taken = cond(rs1_val, rs2_val)  # type: ignore[operator]
        next_pc = (pc + imm) if taken else (pc + 4)

        return ExecuteResult(
            description=f"{decoded.mnemonic}: taken={taken}",
            registers_changed={},
            memory_changed={},
            next_pc=next_pc,
        )

    def _exec_jal(
        self, decoded: DecodeResult, registers: RegisterFile, pc: int
    ) -> ExecuteResult:
        rd = decoded.fields["rd"]
        imm = decoded.fields["imm"]
        return_addr = _to_unsigned(pc + 4)
        changes = _write_rd(registers, rd, return_addr)
        return ExecuteResult(
            description=f"jal: x{rd} = {return_addr}, jump to {pc + imm}",
            registers_changed=changes,
            memory_changed={},
            next_pc=pc + imm,
        )

    def _exec_jalr(
        self, decoded: DecodeResult, registers: RegisterFile, pc: int
    ) -> ExecuteResult:
        rd = decoded.fields["rd"]
        rs1 = decoded.fields["rs1"]
        imm = decoded.fields["imm"]
        return_addr = _to_unsigned(pc + 4)
        target = (_to_signed(registers.read(rs1)) + _to_signed(imm)) & ~1
        changes = _write_rd(registers, rd, return_addr)
        return ExecuteResult(
            description=f"jalr: x{rd} = {return_addr}, jump to {target}",
            registers_changed=changes,
            memory_changed={},
            next_pc=target,
        )

    def _exec_lui(
        self, decoded: DecodeResult, registers: RegisterFile, pc: int
    ) -> ExecuteResult:
        rd = decoded.fields["rd"]
        imm = decoded.fields["imm"]
        result = _to_unsigned(imm << 12)
        changes = _write_rd(registers, rd, result)
        return ExecuteResult(
            description=f"lui: x{rd} = {result}",
            registers_changed=changes,
            memory_changed={},
            next_pc=pc + 4,
        )

    def _exec_auipc(
        self, decoded: DecodeResult, registers: RegisterFile, pc: int
    ) -> ExecuteResult:
        rd = decoded.fields["rd"]
        imm = decoded.fields["imm"]
        result = _to_unsigned(pc + (imm << 12))
        changes = _write_rd(registers, rd, result)
        return ExecuteResult(
            description=f"auipc: x{rd} = {result}",
            registers_changed=changes,
            memory_changed={},
            next_pc=pc + 4,
        )

    def _exec_ecall(
        self, decoded: DecodeResult, registers: RegisterFile, pc: int
    ) -> ExecuteResult:
        from riscv_simulator.csr import CAUSE_ECALL_M_MODE, CSR_MCAUSE, CSR_MEPC, CSR_MSTATUS, CSR_MTVEC, MIE

        if self.csr is None:
            return ExecuteResult(
                description="ecall: halt (no CSR file)",
                registers_changed={},
                memory_changed={},
                next_pc=pc,
                halted=True,
            )

        mtvec = self.csr.read(CSR_MTVEC)
        if mtvec == 0:
            return ExecuteResult(
                description="ecall: halt (mtvec=0)",
                registers_changed={},
                memory_changed={},
                next_pc=pc,
                halted=True,
            )

        # Raise trap
        self.csr.write(CSR_MEPC, pc)
        self.csr.write(CSR_MCAUSE, CAUSE_ECALL_M_MODE)
        mstatus = self.csr.read(CSR_MSTATUS)
        self.csr.write(CSR_MSTATUS, mstatus & ~MIE)

        return ExecuteResult(
            description=f"ecall: trap to mtvec=0x{mtvec:x}",
            registers_changed={},
            memory_changed={},
            next_pc=mtvec,
        )

    def _exec_mret(
        self, decoded: DecodeResult, registers: RegisterFile, pc: int
    ) -> ExecuteResult:
        from riscv_simulator.csr import CSR_MEPC, CSR_MSTATUS, MIE

        if self.csr is None:
            return ExecuteResult(
                description="mret: no CSR file",
                registers_changed={},
                memory_changed={},
                next_pc=pc + 4,
            )

        mepc = self.csr.read(CSR_MEPC)
        mstatus = self.csr.read(CSR_MSTATUS)
        self.csr.write(CSR_MSTATUS, mstatus | MIE)

        return ExecuteResult(
            description=f"mret: return to mepc=0x{mepc:x}",
            registers_changed={},
            memory_changed={},
            next_pc=mepc,
        )

    def _exec_csrrw(
        self, decoded: DecodeResult, registers: RegisterFile, pc: int
    ) -> ExecuteResult:
        rd = decoded.fields["rd"]
        rs1 = decoded.fields["rs1"]
        csr_addr = decoded.fields["csr"]
        rs1_val = registers.read(rs1)
        old_csr = self.csr.read_write(csr_addr, rs1_val)  # type: ignore[union-attr]
        changes = _write_rd(registers, rd, old_csr)
        return ExecuteResult(
            description=f"csrrw: x{rd} = CSR[0x{csr_addr:x}]",
            registers_changed=changes,
            memory_changed={},
            next_pc=pc + 4,
        )

    def _exec_csrrs(
        self, decoded: DecodeResult, registers: RegisterFile, pc: int
    ) -> ExecuteResult:
        rd = decoded.fields["rd"]
        rs1 = decoded.fields["rs1"]
        csr_addr = decoded.fields["csr"]
        rs1_val = registers.read(rs1)
        old_csr = self.csr.read_set(csr_addr, rs1_val)  # type: ignore[union-attr]
        changes = _write_rd(registers, rd, old_csr)
        return ExecuteResult(
            description=f"csrrs: x{rd} = CSR[0x{csr_addr:x}]",
            registers_changed=changes,
            memory_changed={},
            next_pc=pc + 4,
        )

    def _exec_csrrc(
        self, decoded: DecodeResult, registers: RegisterFile, pc: int
    ) -> ExecuteResult:
        rd = decoded.fields["rd"]
        rs1 = decoded.fields["rs1"]
        csr_addr = decoded.fields["csr"]
        rs1_val = registers.read(rs1)
        old_csr = self.csr.read_clear(csr_addr, rs1_val)  # type: ignore[union-attr]
        changes = _write_rd(registers, rd, old_csr)
        return ExecuteResult(
            description=f"csrrc: x{rd} = CSR[0x{csr_addr:x}]",
            registers_changed=changes,
            memory_changed={},
            next_pc=pc + 4,
        )
