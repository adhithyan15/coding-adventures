"""Intel 8080 behavioral simulator.

This module implements the complete Intel 8080A instruction set as a pure-Python
behavioral simulator.  Every instruction is executed by direct manipulation of
register variables — no gate-level modeling occurs here.

Architecture recap
------------------
- 7 × 8-bit registers: A (accumulator), B, C, D, E, H, L
- 3 × 16-bit registers: SP (stack pointer), PC (program counter), plus virtual HL
- 5 condition flags: S, Z, AC, P, CY
- 64 KiB RAM-based address space (16-bit addressing)
- 256 input ports + 256 output ports
- Two's complement arithmetic (unlike the IBM 704's sign-magnitude)

Instruction encoding
---------------------
The 8080 uses a very regular 8-bit opcode space:
  bits 7–6: group (00/01/10/11)
  bits 5–3: destination register (or sub-operation)
  bits 2–0: source register (or sub-operation)

Group 01 (0x40–0x7F) is MOV r1,r2 (64 opcodes; 0x76 = HLT).
Group 10 (0x80–0xBF) is ALU register: ADD/ADC/SUB/SBB/ANA/XRA/ORA/CMP r.
Group 00 (0x00–0x3F) is data movement, immediate loads, 16-bit ops.
Group 11 (0xC0–0xFF) is branches, stack, I/O, and control.

SIM00 protocol
--------------
``Intel8080Simulator`` structurally satisfies ``Simulator[Intel8080State]``
from ``coding-adventures-simulator-protocol``.  The ``execute`` method
resets state, loads the program, and runs until HLT (or cycle limit).
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from simulator_protocol import ExecutionResult, StepTrace

from intel8080_simulator.flags import (
    compute_ac_add,
    compute_ac_ana,
    compute_ac_sub,
    compute_cy_add,
    compute_cy_sub,
    flags_from_byte,
    szp_flags,
)
from intel8080_simulator.state import Intel8080State

if TYPE_CHECKING:
    pass

__all__ = ["Intel8080Simulator"]

# ── Constants ──────────────────────────────────────────────────────────────────

MEMORY_SIZE = 65536  # 64 KiB
PORT_COUNT = 256  # I/O ports per direction

# Register code constants (3-bit encoding in instruction bytes)
REG_B = 0
REG_C = 1
REG_D = 2
REG_E = 3
REG_H = 4
REG_L = 5
REG_M = 6  # pseudo-register: memory[HL]
REG_A = 7

# Register pair codes (2-bit encoding in certain instructions)
PAIR_B = 0   # BC
PAIR_D = 1   # DE
PAIR_H = 2   # HL
PAIR_SP = 3  # SP (or PSW for PUSH/POP)

# ALU operation codes (bits 5–3 in group-10 instructions)
ALU_ADD = 0
ALU_ADC = 1
ALU_SUB = 2
ALU_SBB = 3
ALU_ANA = 4
ALU_XRA = 5
ALU_ORA = 6
ALU_CMP = 7

# Condition codes (bits 5–3 in conditional branch instructions)
COND_NZ = 0  # Z=0
COND_Z  = 1  # Z=1
COND_NC = 2  # CY=0
COND_C  = 3  # CY=1
COND_PO = 4  # P=0 (parity odd)
COND_PE = 5  # P=1 (parity even)
COND_P  = 6  # S=0 (positive)
COND_M  = 7  # S=1 (minus)

# Register and pair names for trace output
_REG_NAMES = {
    REG_B: "B", REG_C: "C", REG_D: "D", REG_E: "E",
    REG_H: "H", REG_L: "L", REG_M: "M", REG_A: "A",
}
_PAIR_NAMES = {PAIR_B: "B", PAIR_D: "D", PAIR_H: "H", PAIR_SP: "SP"}
_ALU_MNEMONICS = {
    ALU_ADD: "ADD", ALU_ADC: "ADC", ALU_SUB: "SUB", ALU_SBB: "SBB",
    ALU_ANA: "ANA", ALU_XRA: "XRA", ALU_ORA: "ORA", ALU_CMP: "CMP",
}
_COND_NAMES = {
    COND_NZ: "NZ", COND_Z: "Z", COND_NC: "NC", COND_C: "C",
    COND_PO: "PO", COND_PE: "PE", COND_P: "P", COND_M: "M",
}

_MAX_STEPS = 1_000_000  # safety limit to prevent infinite loops


# ── Intel8080Simulator ────────────────────────────────────────────────────────

class Intel8080Simulator:
    """Behavioral simulator for the Intel 8080A microprocessor.

    Usage::

        sim = Intel8080Simulator()
        program = bytes([0x3E, 0x05, 0x76])   # MVI A,5; HLT
        result = sim.execute(program)
        assert result.final_state.a == 5
        assert result.halted is True
    """

    def __init__(self) -> None:
        self._memory: list[int] = [0] * MEMORY_SIZE
        self._input_ports: list[int] = [0] * PORT_COUNT
        self._output_ports: list[int] = [0] * PORT_COUNT
        self._reset_registers()

    # ── Public SIM00 Protocol Methods ─────────────────────────────────────────

    def load(self, program: bytes) -> None:
        """Write program bytes into memory starting at address 0x0000."""
        self._memory[:len(program)] = list(program)

    def reset(self) -> None:
        """Clear all registers, flags, memory, and I/O ports."""
        self._memory = [0] * MEMORY_SIZE
        self._input_ports = [0] * PORT_COUNT
        self._output_ports = [0] * PORT_COUNT
        self._reset_registers()

    def get_state(self) -> Intel8080State:
        """Return a frozen snapshot of the current CPU state."""
        return Intel8080State(
            a=self._a,
            b=self._b,
            c=self._c,
            d=self._d,
            e=self._e,
            h=self._h,
            l=self._l,
            sp=self._sp,
            pc=self._pc,
            flag_s=self._flag_s,
            flag_z=self._flag_z,
            flag_ac=self._flag_ac,
            flag_p=self._flag_p,
            flag_cy=self._flag_cy,
            interrupts_enabled=self._inte,
            halted=self._halted,
            memory=tuple(self._memory),
            input_ports=tuple(self._input_ports),
            output_ports=tuple(self._output_ports),
        )

    def step(self) -> StepTrace:
        """Execute one instruction and return a StepTrace.

        If the CPU is halted, this is a no-op that returns a HLT trace.
        """
        pc_before = self._pc

        if self._halted:
            return StepTrace(
                pc_before=pc_before,
                pc_after=pc_before,
                mnemonic="HLT",
                description="CPU is halted",
            )

        mnemonic, description = self._execute_one()

        return StepTrace(
            pc_before=pc_before,
            pc_after=self._pc,
            mnemonic=mnemonic,
            description=description,
        )

    def execute(self, program: bytes) -> ExecutionResult:
        """Reset, load program, run until HLT or cycle limit.

        Pre-loaded input port values are preserved across ``execute()`` calls
        so that callers can set them up before calling ``execute``.

        Returns an ExecutionResult with full trace list.
        """
        # Save input ports — reset() would clear them, but callers may have
        # pre-loaded them via set_input_port() before calling execute().
        saved_input_ports = list(self._input_ports)
        self.reset()
        self._input_ports = saved_input_ports
        self.load(program)

        traces: list[StepTrace] = []
        error: str | None = None

        for _ in range(_MAX_STEPS):
            try:
                trace = self.step()
            except (ValueError, IndexError) as exc:
                error = str(exc)
                break

            traces.append(trace)

            if self._halted:
                break
        else:
            error = f"Exceeded maximum step limit ({_MAX_STEPS})"

        final_state = self.get_state()
        halted = self._halted

        return ExecutionResult(
            halted=halted,
            error=error,
            steps=len(traces),
            traces=traces,
            final_state=final_state,
        )

    # ── I/O Port Access ───────────────────────────────────────────────────────

    def set_input_port(self, port: int, value: int) -> None:
        """Pre-load a value into an input port (0–255 each)."""
        if not (0 <= port <= 255):
            raise ValueError(f"Port number must be 0–255, got {port}")
        if not (0 <= value <= 255):
            raise ValueError(f"Port value must be 0–255, got {value}")
        self._input_ports[port] = value

    def get_output_port(self, port: int) -> int:
        """Read the current value of an output port."""
        if not (0 <= port <= 255):
            raise ValueError(f"Port number must be 0–255, got {port}")
        return self._output_ports[port]

    # ── Private helpers ───────────────────────────────────────────────────────

    def _reset_registers(self) -> None:
        """Reset all CPU registers and flags to zero."""
        self._a = 0
        self._b = 0
        self._c = 0
        self._d = 0
        self._e = 0
        self._h = 0
        self._l = 0
        self._sp = 0
        self._pc = 0
        self._flag_s = False
        self._flag_z = False
        self._flag_ac = False
        self._flag_p = False
        self._flag_cy = False
        self._inte = False
        self._halted = False

    def _read_mem(self, addr: int) -> int:
        """Read one byte from memory, wrapping address to 16 bits."""
        return self._memory[addr & 0xFFFF]

    def _write_mem(self, addr: int, value: int) -> None:
        """Write one byte to memory, wrapping address to 16 bits."""
        self._memory[addr & 0xFFFF] = value & 0xFF

    def _fetch_byte(self) -> int:
        """Read byte at PC and advance PC by 1."""
        byte = self._read_mem(self._pc)
        self._pc = (self._pc + 1) & 0xFFFF
        return byte

    def _fetch_word(self) -> int:
        """Read 16-bit word (little-endian) at PC and advance PC by 2."""
        lo = self._fetch_byte()
        hi = self._fetch_byte()
        return (hi << 8) | lo

    def _read_reg(self, code: int) -> int:
        """Read register by 3-bit code; code 6 (M) reads memory[HL]."""
        match code:
            case 0:
                return self._b
            case 1:
                return self._c
            case 2:
                return self._d
            case 3:
                return self._e
            case 4:
                return self._h
            case 5:
                return self._l
            case 6:
                return self._read_mem((self._h << 8) | self._l)
            case 7:
                return self._a
            case _:
                raise ValueError(f"Invalid register code: {code}")

    def _write_reg(self, code: int, value: int) -> None:
        """Write register by 3-bit code; code 6 (M) writes memory[HL]."""
        v = value & 0xFF
        match code:
            case 0:
                self._b = v
            case 1:
                self._c = v
            case 2:
                self._d = v
            case 3:
                self._e = v
            case 4:
                self._h = v
            case 5:
                self._l = v
            case 6:
                self._write_mem((self._h << 8) | self._l, v)
            case 7:
                self._a = v
            case _:
                raise ValueError(f"Invalid register code: {code}")

    def _read_pair(self, code: int) -> int:
        """Read 16-bit register pair by 2-bit code."""
        match code:
            case 0:
                return (self._b << 8) | self._c
            case 1:
                return (self._d << 8) | self._e
            case 2:
                return (self._h << 8) | self._l
            case 3:
                return self._sp
            case _:
                raise ValueError(f"Invalid pair code: {code}")

    def _write_pair(self, code: int, value: int) -> None:
        """Write 16-bit register pair by 2-bit code."""
        v = value & 0xFFFF
        match code:
            case 0:
                self._b, self._c = (v >> 8) & 0xFF, v & 0xFF
            case 1:
                self._d, self._e = (v >> 8) & 0xFF, v & 0xFF
            case 2:
                self._h, self._l = (v >> 8) & 0xFF, v & 0xFF
            case 3:
                self._sp = v
            case _:
                raise ValueError(f"Invalid pair code: {code}")

    def _check_condition(self, cond: int) -> bool:
        """Evaluate a 3-bit condition code against current flags."""
        match cond:
            case 0:
                return not self._flag_z   # NZ
            case 1:
                return self._flag_z        # Z
            case 2:
                return not self._flag_cy   # NC
            case 3:
                return self._flag_cy       # C
            case 4:
                return not self._flag_p    # PO
            case 5:
                return self._flag_p        # PE
            case 6:
                return not self._flag_s    # P (positive)
            case 7:
                return self._flag_s        # M (minus)
            case _:
                raise ValueError(f"Invalid condition code: {cond}")

    def _push16(self, value: int) -> None:
        """Push a 16-bit value onto the stack (SP grows downward)."""
        self._sp = (self._sp - 2) & 0xFFFF
        self._write_mem(self._sp + 1, (value >> 8) & 0xFF)
        self._write_mem(self._sp, value & 0xFF)

    def _pop16(self) -> int:
        """Pop a 16-bit value from the stack."""
        lo = self._read_mem(self._sp)
        hi = self._read_mem(self._sp + 1)
        self._sp = (self._sp + 2) & 0xFFFF
        return (hi << 8) | lo

    # ── ALU Operations ────────────────────────────────────────────────────────

    def _alu_add(self, b: int, carry: int = 0) -> None:
        """A ← A + b + carry; update all flags."""
        a = self._a
        result = a + b + carry
        s, z, p = szp_flags(result)
        self._a = result & 0xFF
        self._flag_s = s
        self._flag_z = z
        self._flag_p = p
        self._flag_cy = compute_cy_add(result)
        self._flag_ac = compute_ac_add(a, b, carry)

    def _alu_sub(self, b: int, borrow: int = 0) -> None:
        """A ← A - b - borrow; update all flags."""
        a = self._a
        result = a - b - borrow
        s, z, p = szp_flags(result)
        self._a = result & 0xFF
        self._flag_s = s
        self._flag_z = z
        self._flag_p = p
        self._flag_cy = compute_cy_sub(a, b, borrow)
        self._flag_ac = compute_ac_sub(a, b, borrow)

    def _alu_and(self, b: int) -> None:
        """A ← A AND b; CY=0; AC = OR of bit 3 of operands (8080 spec)."""
        result = self._a & b
        s, z, p = szp_flags(result)
        self._a = result
        self._flag_s = s
        self._flag_z = z
        self._flag_p = p
        self._flag_cy = False
        self._flag_ac = compute_ac_ana(self._a | b, 0)  # pre-AND AC

    def _alu_xra(self, b: int) -> None:
        """A ← A XOR b; CY=0, AC=0."""
        result = (self._a ^ b) & 0xFF
        s, z, p = szp_flags(result)
        self._a = result
        self._flag_s = s
        self._flag_z = z
        self._flag_p = p
        self._flag_cy = False
        self._flag_ac = False

    def _alu_ora(self, b: int) -> None:
        """A ← A OR b; CY=0, AC=0."""
        result = (self._a | b) & 0xFF
        s, z, p = szp_flags(result)
        self._a = result
        self._flag_s = s
        self._flag_z = z
        self._flag_p = p
        self._flag_cy = False
        self._flag_ac = False

    def _alu_cmp(self, b: int) -> None:
        """Set flags as if A - b; leave A unchanged."""
        a = self._a
        result = a - b
        s, z, p = szp_flags(result)
        self._flag_s = s
        self._flag_z = z
        self._flag_p = p
        self._flag_cy = compute_cy_sub(a, b)
        self._flag_ac = compute_ac_sub(a, b)

    def _alu_dispatch(self, op: int, operand: int) -> None:
        """Dispatch ALU op (0=ADD,1=ADC,2=SUB,3=SBB,4=ANA,5=XRA,6=ORA,7=CMP)."""
        match op:
            case 0:
                self._alu_add(operand)
            case 1:
                self._alu_add(operand, int(self._flag_cy))
            case 2:
                self._alu_sub(operand)
            case 3:
                self._alu_sub(operand, int(self._flag_cy))
            case 4:
                self._alu_and(operand)
            case 5:
                self._alu_xra(operand)
            case 6:
                self._alu_ora(operand)
            case 7:
                self._alu_cmp(operand)

    # ── Instruction Execution ─────────────────────────────────────────────────

    def _execute_one(self) -> tuple[str, str]:
        """Fetch and execute one instruction.  Returns (mnemonic, description)."""
        opcode = self._fetch_byte()

        # Extract bit fields
        bits_76 = (opcode >> 6) & 0x3   # group
        bits_53 = (opcode >> 3) & 0x7   # dst / sub-op
        bits_20 = opcode & 0x7           # src / sub-op

        # ──────────────────────────────────────────────────────────────────────
        # GROUP 01: MOV r1, r2  (0x40–0x7F)
        # Special case: 0x76 is HLT, not MOV M,M
        # ──────────────────────────────────────────────────────────────────────
        if bits_76 == 0b01:
            if opcode == 0x76:
                self._halted = True
                return "HLT", "Halt — CPU stopped"
            dst, src = bits_53, bits_20
            value = self._read_reg(src)
            self._write_reg(dst, value)
            d, s = _REG_NAMES[dst], _REG_NAMES[src]
            return f"MOV {d},{s}", f"{d} ← {s} (0x{value:02X})"

        # ──────────────────────────────────────────────────────────────────────
        # GROUP 10: ALU register  (0x80–0xBF)
        # ──────────────────────────────────────────────────────────────────────
        if bits_76 == 0b10:
            alu_op, src = bits_53, bits_20
            operand = self._read_reg(src)
            self._alu_dispatch(alu_op, operand)
            mnem = _ALU_MNEMONICS[alu_op]
            return (
                f"{mnem} {_REG_NAMES[src]}",
                f"A ← A {mnem} {_REG_NAMES[src]} (0x{operand:02X})"
            )

        # ──────────────────────────────────────────────────────────────────────
        # GROUP 00: Data movement, immediate, 16-bit ops  (0x00–0x3F)
        # ──────────────────────────────────────────────────────────────────────
        if bits_76 == 0b00:
            return self._exec_group00(opcode, bits_53, bits_20)

        # ──────────────────────────────────────────────────────────────────────
        # GROUP 11: Branches, stack, I/O, control  (0xC0–0xFF)
        # ──────────────────────────────────────────────────────────────────────
        return self._exec_group11(opcode, bits_53, bits_20)

    def _exec_group00(self, opcode: int, dst: int, src: int) -> tuple[str, str]:
        """Execute a group-00 instruction (0x00–0x3F)."""
        # NOP
        if opcode == 0x00:
            return "NOP", "No operation"

        # LXI rp, d16  — bits: 00pp0001
        if src == 0b001 and (dst & 1) == 0:
            pair = dst >> 1
            word = self._fetch_word()
            self._write_pair(pair, word)
            return (
                f"LXI {_PAIR_NAMES[pair]},0x{word:04X}",
                f"{_PAIR_NAMES[pair]} ← 0x{word:04X}"
            )

        # INX rp  — bits: 00pp0011
        if src == 0b011 and (dst & 1) == 0:
            pair = dst >> 1
            self._write_pair(pair, (self._read_pair(pair) + 1) & 0xFFFF)
            return f"INX {_PAIR_NAMES[pair]}", f"{_PAIR_NAMES[pair]} ← {_PAIR_NAMES[pair]} + 1"  # noqa: E501

        # DCX rp  — bits: 00pp1011
        if src == 0b011 and (dst & 1) == 1:
            pair = dst >> 1
            self._write_pair(pair, (self._read_pair(pair) - 1) & 0xFFFF)
            return f"DCX {_PAIR_NAMES[pair]}", f"{_PAIR_NAMES[pair]} ← {_PAIR_NAMES[pair]} - 1"  # noqa: E501

        # DAD rp  — bits: 00pp1001
        if src == 0b001 and (dst & 1) == 1:
            pair = dst >> 1
            hl = (self._h << 8) | self._l
            rp = self._read_pair(pair)
            result = hl + rp
            self._h = (result >> 8) & 0xFF
            self._l = result & 0xFF
            self._flag_cy = result > 0xFFFF
            return f"DAD {_PAIR_NAMES[pair]}", f"HL ← HL + {_PAIR_NAMES[pair]} = 0x{result & 0xFFFF:04X}"  # noqa: E501

        # MVI r, d8  — bits: 00rrr110
        if src == 0b110:
            imm = self._fetch_byte()
            self._write_reg(dst, imm)
            return f"MVI {_REG_NAMES[dst]},0x{imm:02X}", f"{_REG_NAMES[dst]} ← 0x{imm:02X}"  # noqa: E501

        # INR r  — bits: 00rrr100
        if src == 0b100:
            reg = dst
            old = self._read_reg(reg)
            result = (old + 1) & 0xFF
            self._write_reg(reg, result)
            s, z, p = szp_flags(result)
            self._flag_s = s
            self._flag_z = z
            self._flag_p = p
            self._flag_ac = compute_ac_add(old, 1)
            return f"INR {_REG_NAMES[reg]}", f"{_REG_NAMES[reg]} ← {_REG_NAMES[reg]} + 1 = 0x{result:02X}"  # noqa: E501

        # DCR r  — bits: 00rrr101
        if src == 0b101:
            reg = dst
            old = self._read_reg(reg)
            result = (old - 1) & 0xFF
            self._write_reg(reg, result)
            s, z, p = szp_flags(result)
            self._flag_s = s
            self._flag_z = z
            self._flag_p = p
            self._flag_ac = compute_ac_sub(old, 1)
            return f"DCR {_REG_NAMES[reg]}", f"{_REG_NAMES[reg]} ← {_REG_NAMES[reg]} - 1 = 0x{result:02X}"  # noqa: E501

        # ── Remaining group-00 opcodes are individually addressed ─────────────

        match opcode:
            # STAX B (0x02), STAX D (0x12)
            case 0x02:
                self._write_mem((self._b << 8) | self._c, self._a)
                return "STAX B", f"memory[BC] ← A (0x{self._a:02X})"
            case 0x12:
                self._write_mem((self._d << 8) | self._e, self._a)
                return "STAX D", f"memory[DE] ← A (0x{self._a:02X})"

            # LDAX B (0x0A), LDAX D (0x1A)
            case 0x0A:
                self._a = self._read_mem((self._b << 8) | self._c)
                return "LDAX B", f"A ← memory[BC] = 0x{self._a:02X}"
            case 0x1A:
                self._a = self._read_mem((self._d << 8) | self._e)
                return "LDAX D", f"A ← memory[DE] = 0x{self._a:02X}"

            # SHLD addr (0x22)
            case 0x22:
                addr = self._fetch_word()
                self._write_mem(addr, self._l)
                self._write_mem(addr + 1, self._h)
                return f"SHLD 0x{addr:04X}", f"memory[0x{addr:04X}] ← L; memory[0x{addr+1:04X}] ← H"  # noqa: E501

            # LHLD addr (0x2A)
            case 0x2A:
                addr = self._fetch_word()
                self._l = self._read_mem(addr)
                self._h = self._read_mem(addr + 1)
                return f"LHLD 0x{addr:04X}", f"L ← memory[0x{addr:04X}]; H ← memory[0x{addr+1:04X}]"  # noqa: E501

            # STA addr (0x32)
            case 0x32:
                addr = self._fetch_word()
                self._write_mem(addr, self._a)
                return f"STA 0x{addr:04X}", f"memory[0x{addr:04X}] ← A (0x{self._a:02X})"  # noqa: E501

            # LDA addr (0x3A)
            case 0x3A:
                addr = self._fetch_word()
                self._a = self._read_mem(addr)
                return f"LDA 0x{addr:04X}", f"A ← memory[0x{addr:04X}] = 0x{self._a:02X}"  # noqa: E501

            # Rotate instructions
            case 0x07:  # RLC
                cy = (self._a >> 7) & 1
                self._a = ((self._a << 1) | cy) & 0xFF
                self._flag_cy = bool(cy)
                return "RLC", f"A ← A<<1 | A[7]; CY={cy}"
            case 0x0F:  # RRC
                cy = self._a & 1
                self._a = ((cy << 7) | (self._a >> 1)) & 0xFF
                self._flag_cy = bool(cy)
                return "RRC", f"A ← A[0]<<7 | A>>1; CY={cy}"
            case 0x17:  # RAL
                cy_in = int(self._flag_cy)
                new_cy = (self._a >> 7) & 1
                self._a = ((self._a << 1) | cy_in) & 0xFF
                self._flag_cy = bool(new_cy)
                return "RAL", "A ← A<<1 | CY; CY=A[7]"
            case 0x1F:  # RAR
                cy_in = int(self._flag_cy)
                new_cy = self._a & 1
                self._a = ((cy_in << 7) | (self._a >> 1)) & 0xFF
                self._flag_cy = bool(new_cy)
                return "RAR", "A ← CY<<7 | A>>1; CY=A[0]"

            # DAA (0x27) — Decimal Adjust Accumulator
            case 0x27:
                return self._exec_daa()

            # CMA (0x2F) — Complement Accumulator
            case 0x2F:
                self._a = (~self._a) & 0xFF
                return "CMA", f"A ← ~A = 0x{self._a:02X}"

            # STC (0x37) — Set Carry
            case 0x37:
                self._flag_cy = True
                return "STC", "CY ← 1"

            # CMC (0x3F) — Complement Carry
            case 0x3F:
                self._flag_cy = not self._flag_cy
                return "CMC", f"CY ← ~CY = {int(self._flag_cy)}"

            case _:
                raise ValueError(f"Undefined opcode 0x{opcode:02X} at PC 0x{self._pc - 1:04X}")  # noqa: E501

    def _exec_daa(self) -> tuple[str, str]:
        """Execute DAA — Decimal Adjust Accumulator.

        After a BCD addition, DAA adjusts A so each nibble holds a valid
        decimal digit (0–9).  It operates in two steps:

        Step 1: If the low nibble > 9 or AC=1, add 6 to A.
        Step 2: If the high nibble > 9 or CY=1, add 0x60 to A and set CY=1.

        This is the most complex single-instruction operation on the 8080.
        """
        a = self._a
        low_nibble = a & 0x0F
        correction = 0
        new_cy = self._flag_cy

        # Step 1: correct low nibble
        if low_nibble > 9 or self._flag_ac:
            correction |= 0x06

        # Step 2: correct high nibble
        if ((a + correction) >> 4) > 9 or self._flag_cy:
            correction |= 0x60
            new_cy = True

        result = (a + correction) & 0xFF
        self._flag_ac = compute_ac_add(a, correction)
        self._flag_cy = new_cy
        self._a = result
        s, z, p = szp_flags(result)
        self._flag_s = s
        self._flag_z = z
        self._flag_p = p

        return "DAA", f"Decimal adjust: A 0x{a:02X} → 0x{result:02X}"

    def _exec_group11(self, opcode: int, dst: int, src: int) -> tuple[str, str]:
        """Execute a group-11 instruction (0xC0–0xFF)."""

        # ── Conditional return: 11CCC000 ──────────────────────────────────────
        if src == 0b000:
            cond = dst
            name = _COND_NAMES[cond]
            if self._check_condition(cond):
                target = self._pop16()
                self._pc = target
                return f"R{name}", f"RET (condition {name} true) → 0x{target:04X}"
            return f"R{name}", f"R{name} not taken (condition {name} false)"

        # ── POP rp: 11pp0001 ──────────────────────────────────────────────────
        if src == 0b001 and (dst & 1) == 0:
            pair = dst >> 1
            if pair == 3:  # POP PSW
                word = self._pop16()
                flags_byte = word & 0xFF
                self._a = (word >> 8) & 0xFF
                s, z, ac, p, cy = flags_from_byte(flags_byte)
                self._flag_s = s
                self._flag_z = z
                self._flag_ac = ac
                self._flag_p = p
                self._flag_cy = cy
                return "POP PSW", f"PSW ← stack; A=0x{self._a:02X}"
            value = self._pop16()
            self._write_pair(pair, value)
            return f"POP {_PAIR_NAMES[pair]}", f"{_PAIR_NAMES[pair]} ← 0x{value:04X}"

        # ── Conditional jump: 11CCC010 ────────────────────────────────────────
        if src == 0b010:
            cond = dst
            addr = self._fetch_word()
            name = _COND_NAMES[cond]
            if self._check_condition(cond):
                self._pc = addr
                return f"J{name} 0x{addr:04X}", f"Jump to 0x{addr:04X} (condition {name} true)"  # noqa: E501
            return f"J{name} 0x{addr:04X}", f"J{name} not taken (condition {name} false)"  # noqa: E501

        # ── Conditional call: 11CCC100 ────────────────────────────────────────
        if src == 0b100:
            cond = dst
            addr = self._fetch_word()
            name = _COND_NAMES[cond]
            if self._check_condition(cond):
                self._push16(self._pc)
                self._pc = addr
                return f"C{name} 0x{addr:04X}", f"CALL 0x{addr:04X} (condition {name} true)"  # noqa: E501
            return f"C{name} 0x{addr:04X}", f"C{name} not taken (condition {name} false)"  # noqa: E501

        # ── PUSH rp: 11pp0101 ─────────────────────────────────────────────────
        if src == 0b101 and (dst & 1) == 0:
            pair = dst >> 1
            if pair == 3:  # PUSH PSW
                flags_byte = self.get_state().flags_byte
                self._push16((self._a << 8) | flags_byte)
                return "PUSH PSW", f"stack ← A=0x{self._a:02X}, flags=0x{flags_byte:02X}"  # noqa: E501
            value = self._read_pair(pair)
            self._push16(value)
            return f"PUSH {_PAIR_NAMES[pair]}", f"stack ← 0x{value:04X}"

        # ── RET: 11001001 (0xC9) ──────────────────────────────────────────────
        if opcode == 0xC9:
            target = self._pop16()
            self._pc = target
            return "RET", f"Return to 0x{target:04X}"

        # ── JMP addr: 11000011 (0xC3) ─────────────────────────────────────────
        if opcode == 0xC3:
            addr = self._fetch_word()
            self._pc = addr
            return f"JMP 0x{addr:04X}", f"Jump to 0x{addr:04X}"

        # ── CALL addr: 11001101 (0xCD) ────────────────────────────────────────
        if opcode == 0xCD:
            addr = self._fetch_word()
            self._push16(self._pc)
            self._pc = addr
            return f"CALL 0x{addr:04X}", f"Call subroutine at 0x{addr:04X}"

        # ── ALU immediate: 11AAA110 ───────────────────────────────────────────
        if src == 0b110:
            alu_op = dst
            imm = self._fetch_byte()
            self._alu_dispatch(alu_op, imm)
            mnem_map = {
                0: "ADI", 1: "ACI", 2: "SUI", 3: "SBI",
                4: "ANI", 5: "XRI", 6: "ORI", 7: "CPI",
            }
            mnem = mnem_map[alu_op]
            return f"{mnem} 0x{imm:02X}", f"A ← A {_ALU_MNEMONICS[alu_op]} 0x{imm:02X}"

        # ── RST n: 11NNN111 ───────────────────────────────────────────────────
        if src == 0b111:
            n = dst
            self._push16(self._pc)
            self._pc = 8 * n
            return f"RST {n}", f"Restart: push PC, jump to 0x{8*n:04X}"

        # ── Individually addressed group-11 opcodes ───────────────────────────
        match opcode:
            case 0xE3:  # XTHL
                l_mem = self._read_mem(self._sp)
                h_mem = self._read_mem(self._sp + 1)
                self._write_mem(self._sp, self._l)
                self._write_mem(self._sp + 1, self._h)
                self._l = l_mem
                self._h = h_mem
                return "XTHL", "L↔memory[SP]; H↔memory[SP+1]"

            case 0xF9:  # SPHL
                self._sp = (self._h << 8) | self._l
                return "SPHL", f"SP ← HL = 0x{self._sp:04X}"

            case 0xEB:  # XCHG
                self._h, self._d = self._d, self._h
                self._l, self._e = self._e, self._l
                return "XCHG", "HL ↔ DE"

            case 0xE9:  # PCHL
                self._pc = (self._h << 8) | self._l
                return "PCHL", f"PC ← HL = 0x{self._pc:04X}"

            case 0xDB:  # IN port
                port = self._fetch_byte()
                self._a = self._input_ports[port]
                return f"IN 0x{port:02X}", f"A ← input_port[{port}] = 0x{self._a:02X}"

            case 0xD3:  # OUT port
                port = self._fetch_byte()
                self._output_ports[port] = self._a
                return f"OUT 0x{port:02X}", f"output_port[{port}] ← A (0x{self._a:02X})"

            case 0xFB:  # EI
                self._inte = True
                return "EI", "Enable interrupts"

            case 0xF3:  # DI
                self._inte = False
                return "DI", "Disable interrupts"

            case _:
                raise ValueError(f"Undefined opcode 0x{opcode:02X} at PC 0x{self._pc - 1:04X}")  # noqa: E501
