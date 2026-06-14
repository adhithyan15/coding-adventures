"""ControlUnit — FSM-based control unit for the Intel 8080 gate-level simulator.

=== Architecture ===

The control unit orchestrates the fetch-decode-execute-writeback pipeline.
In hardware, it is a finite-state machine: a set of D flip-flops holding the
current state, plus combinational logic computing the next state and outputs.

=== Pipeline states ===

    FETCH_1   → Read opcode from memory[PC]; increment PC by 1
    FETCH_2   → Read second byte (if instruction is 2 or 3 bytes)
    FETCH_3   → Read third byte (if instruction is 3 bytes)
    EXECUTE   → Route decoded instruction to ALU / register file / memory
    WRITEBACK → Write ALU result back to destination register / flags
    HALT      → Stopped (HLT executed)

=== One tick per instruction ===

For the gate-level simulator, each call to `step()` runs one complete
instruction cycle: FETCH → DECODE → EXECUTE → WRITEBACK. This matches
the behavioral simulator's step() behavior. The internal FSM states are
stepped through automatically within a single Python method call.

=== Instruction execution model ===

The control unit dispatches instructions through:
1. Decoder8080 → DecodedInstruction (combinational)
2. RegisterFile reads → operands
3. ALU8080.execute() → ALUResult8080 (for arithmetic/logical instructions)
4. Memory reads/writes (for M-register, LDA, STA, PUSH, POP, CALL, RET)
5. RegisterFile writes → store result
6. Flag updates

=== Memory interface ===

Memory is passed in as a bytearray for mutation (Python doesn't have actual
shared memory hardware, so we pass a reference).

=== I/O ports ===

The 8080 has 256 input ports and 256 output ports. The control unit
reads input ports and writes output ports during IN/OUT instruction execution.
"""

from __future__ import annotations

from enum import Enum, auto

from simulator_protocol import StepTrace

from intel8080_gatelevel.alu import ALU8080
from intel8080_gatelevel.bits import add_16bit
from intel8080_gatelevel.decoder import Decoder8080
from intel8080_gatelevel.register_file import (
    PAIR_HL,
    REG_A,
    REG_D,
    REG_E,
    REG_H,
    REG_L,
    REG_M,
    Register16,
    RegisterFile,
)


class CPUState(Enum):
    """Internal FSM states of the control unit."""

    FETCH_1 = auto()
    FETCH_2 = auto()
    FETCH_3 = auto()
    EXECUTE = auto()
    WRITEBACK = auto()
    HALT = auto()


# ── ALU operation codes from group-10 opcode bits 5–3 ────────────────────────
_ALU_OP_ADD = 0
_ALU_OP_ADC = 1
_ALU_OP_SUB = 2
_ALU_OP_SBB = 3
_ALU_OP_ANA = 4
_ALU_OP_XRA = 5
_ALU_OP_ORA = 6
_ALU_OP_CMP = 7
_ALU_OP_INR = 8
_ALU_OP_DCR = 9
_ALU_OP_RLC = 10
_ALU_OP_RRC = 11
_ALU_OP_RAL = 12
_ALU_OP_RAR = 13
_ALU_OP_CMA = 14
_ALU_OP_DAA = 15


class FlagRegister:
    """5-bit flag register: S, Z, AC, P, CY.

    Stores the five 8080 condition flags as individual bits.
    Each flag is one D flip-flop in hardware.
    """

    def __init__(self) -> None:
        """Initialize all flags to False (power-on state)."""
        self.s: bool = False    # Sign
        self.z: bool = False    # Zero
        self.ac: bool = False   # Auxiliary carry
        self.p: bool = False    # Parity
        self.cy: bool = False   # Carry

    def to_byte(self) -> int:
        """Pack flags into the 8080 flags byte.

        Layout: S=bit7, Z=bit6, 0=bit5, AC=bit4, 0=bit3, P=bit2, 1=bit1, CY=bit0
        """
        return (
            (int(self.s) << 7)
            | (int(self.z) << 6)
            | (0 << 5)
            | (int(self.ac) << 4)
            | (0 << 3)
            | (int(self.p) << 2)
            | (1 << 1)
            | int(self.cy)
        )

    def from_byte(self, byte: int) -> None:
        """Unpack a flags byte (from PUSH PSW / POP PSW stack operations)."""
        self.s = bool(byte & 0x80)
        self.z = bool(byte & 0x40)
        self.ac = bool(byte & 0x10)
        self.p = bool(byte & 0x04)
        self.cy = bool(byte & 0x01)

    def condition_met(self, cond: int) -> bool:
        """Test a 3-bit condition code for conditional jump/call/return.

        Condition codes (from bits 5–3 of opcode):
            000 = NZ  (Z == 0)
            001 = Z   (Z == 1)
            010 = NC  (CY == 0)
            011 = C   (CY == 1)
            100 = PO  (P == 0, parity odd)
            101 = PE  (P == 1, parity even)
            110 = P   (S == 0, positive / sign clear)
            111 = M   (S == 1, minus / sign set)
        """
        match cond:
            case 0:
                return not self.z
            case 1:
                return self.z
            case 2:
                return not self.cy
            case 3:
                return self.cy
            case 4:
                return not self.p
            case 5:
                return self.p
            case 6:
                return not self.s
            case 7:
                return self.s
            case _:
                msg = f"Invalid condition code: {cond}"
                raise ValueError(msg)


class ControlUnit:
    """FSM-based control unit for the Intel 8080 gate-level simulator.

    Orchestrates the fetch-decode-execute pipeline for all 244 Intel 8080
    instructions. Each step() call executes one complete instruction.

    The control unit owns the register file, flag register, ALU, decoder,
    PC, and SP. The memory array is shared with the simulator.

    Usage:
        >>> cu = ControlUnit()
        >>> cu.memory = bytearray([0x3E, 0x0A, 0x76])  # MVI A,10; HLT
        >>> cu.step()   # executes MVI A, 10
        StepTrace(mnemonic='MVI A', ...)
        >>> cu.step()   # executes HLT
        StepTrace(mnemonic='HLT', ...)
        >>> cu.halted
        True
    """

    def __init__(self) -> None:
        """Initialize the control unit with all components at reset state."""
        self._rf = RegisterFile()
        self._flags = FlagRegister()
        self._alu = ALU8080()
        self._dec = Decoder8080()
        self._pc = Register16()
        self._sp = Register16()
        self._state = CPUState.FETCH_1
        self._memory: bytearray = bytearray(65536)
        self._input_ports: list[int] = [0] * 256
        self._output_ports: list[int] = [0] * 256
        self._inte: bool = False   # interrupt enable flip-flop

    # ─── Public interface ─────────────────────────────────────────────────

    @property
    def halted(self) -> bool:
        """True after a HLT instruction has been executed."""
        return self._state == CPUState.HALT

    def reset(self) -> None:
        """Reset all registers, flags, and state to power-on values."""
        self._rf = RegisterFile()
        self._flags = FlagRegister()
        self._pc = Register16()
        self._sp = Register16()
        self._state = CPUState.FETCH_1
        self._inte = False

    def step(self) -> StepTrace | None:
        """Execute one complete instruction (fetch-decode-execute-writeback).

        Returns:
            StepTrace with pc_before, pc_after, mnemonic, description.
            Returns None if already halted.
        """
        if self._state == CPUState.HALT:
            return None

        pc_before = self._pc.read()

        # ── FETCH_1: read opcode ──────────────────────────────────────────
        opcode = self._memory[self._pc.read()]
        self._pc.inc(1)

        # ── DECODE ────────────────────────────────────────────────────────
        decoded = self._dec.decode(opcode)

        # ── FETCH_2 / FETCH_3: read immediate bytes ───────────────────────
        imm1 = 0
        imm2 = 0
        if decoded.extra_bytes >= 1:
            imm1 = self._memory[self._pc.read()]
            self._pc.inc(1)
        if decoded.extra_bytes >= 2:
            imm2 = self._memory[self._pc.read()]
            self._pc.inc(1)
        imm16 = (imm2 << 8) | imm1  # little-endian 16-bit immediate

        # ── EXECUTE + WRITEBACK ────────────────────────────────────────────
        mnemonic, description = self._execute(opcode, decoded, imm1, imm2, imm16)

        pc_after = self._pc.read()
        return StepTrace(
            pc_before=pc_before,
            pc_after=pc_after,
            mnemonic=mnemonic,
            description=description,
        )

    # ─── Instruction execution ─────────────────────────────────────────────

    def _execute(  # noqa: PLR0911,PLR0912,PLR0915
        self,
        opcode: int,
        decoded: DecodedInstruction,  # noqa: F821 — forward ref
        imm1: int,
        imm2: int,
        imm16: int,
    ) -> tuple[str, str]:
        """Execute the decoded instruction. Returns (mnemonic, description)."""

        # ── HLT ─────────────────────────────────────────────────────────
        if decoded.is_halt:
            self._state = CPUState.HALT
            return "HLT", "Halt — CPU stopped"

        group = decoded.op_group

        # ── Group 01: MOV ────────────────────────────────────────────────
        if group == 1:
            return self._exec_mov(decoded)

        # ── Group 10: ALU register ───────────────────────────────────────
        if group == 2:
            return self._exec_alu_reg(decoded)

        # ── Group 00: misc ────────────────────────────────────────────────
        if group == 0:
            return self._exec_group00(opcode, decoded, imm1, imm2, imm16)

        # ── Group 11: branches, stack, control ───────────────────────────
        return self._exec_group11(opcode, decoded, imm1, imm2, imm16)

    # ─── Group 01: MOV ───────────────────────────────────────────────────

    def _exec_mov(self, decoded: DecodedInstruction) -> tuple[str, str]:  # noqa: F821
        """MOV dst,src — move register to register (or memory via M)."""
        dst = decoded.dst
        src = decoded.src
        reg_names = ["B", "C", "D", "E", "H", "L", "M", "A"]

        # Source read
        if src == REG_M:
            addr = (self._rf.read(REG_H) << 8) | self._rf.read(REG_L)
            value = self._memory[addr]
        else:
            value = self._rf.read(src)

        # Destination write
        if dst == REG_M:
            addr = (self._rf.read(REG_H) << 8) | self._rf.read(REG_L)
            self._memory[addr] = value
        else:
            self._rf.write(dst, value)

        mn = f"MOV {reg_names[dst]},{reg_names[src]}"
        return mn, f"Move {reg_names[src]} → {reg_names[dst]}"

    # ─── Group 10: ALU register ──────────────────────────────────────────

    def _exec_alu_reg(self, decoded: DecodedInstruction) -> tuple[str, str]:  # noqa: F821
        """ALU operations with register operand."""
        alu_op = decoded.alu_op   # 0–7 matching ALU8080 op codes
        src = decoded.src
        reg_names = ["B", "C", "D", "E", "H", "L", "M", "A"]
        alu_names = ["ADD", "ADC", "SUB", "SBB", "ANA", "XRA", "ORA", "CMP"]

        a = self._rf.read(REG_A)

        # Read operand (M = memory[HL])
        if src == REG_M:
            addr = (self._rf.read(REG_H) << 8) | self._rf.read(REG_L)
            b = self._memory[addr]
        else:
            b = self._rf.read(src)

        res = self._alu.execute(alu_op, a, b, self._flags.cy, self._flags.ac)
        self._apply_alu_result(res, alu_op)

        mn = f"{alu_names[alu_op]} {reg_names[src]}"
        return mn, f"{alu_names[alu_op]} A with {reg_names[src]}"

    # ─── Group 00: misc ──────────────────────────────────────────────────

    def _exec_group00(  # noqa: PLR0911,PLR0912,PLR0915
        self,
        opcode: int,
        decoded: DecodedInstruction,  # noqa: F821
        imm1: int,
        imm2: int,
        imm16: int,
    ) -> tuple[str, str]:
        """Group 00: LXI, MVI, INR, DCR, INX, DCX, DAD, DAA, rotates, etc."""
        reg_names = ["B", "C", "D", "E", "H", "L", "M", "A"]
        pair_names = ["B", "D", "H", "SP"]
        rp = decoded.reg_pair

        # ── NOP ──────────────────────────────────────────────────────────
        if opcode == 0x00:
            return "NOP", "No operation"

        # ── LXI rp,d16 — 00rp0001 ────────────────────────────────────────
        if (opcode & 0x0F) == 0x01:
            self._rf.write_pair(rp, imm16, self._sp)
            return f"LXI {pair_names[rp]}", f"Load immediate {imm16:#06x} into {pair_names[rp]}"  # noqa: E501

        # ── INX rp — 00rp0011 ─────────────────────────────────────────────
        if (opcode & 0x0F) == 0x03:
            val = self._rf.read_pair(rp, self._sp)
            new_val, _ = add_16bit(val, 1, 0)
            self._rf.write_pair(rp, new_val & 0xFFFF, self._sp)
            return f"INX {pair_names[rp]}", f"Increment register pair {pair_names[rp]}"

        # ── INR r — 00ddd100 ──────────────────────────────────────────────
        if (opcode & 0x07) == 0x04 and (opcode & 0xC0) == 0x00:
            reg_code = (opcode >> 3) & 0x07
            cy, ac = self._flags.cy, self._flags.ac
            if reg_code == REG_M:
                addr = (self._rf.read(REG_H) << 8) | self._rf.read(REG_L)
                val = self._memory[addr]
                res = self._alu.execute(_ALU_OP_INR, val, 0, cy, ac)
                self._apply_alu_result(res, _ALU_OP_INR)
                self._memory[addr] = res.result
            else:
                val = self._rf.read(reg_code)
                res = self._alu.execute(_ALU_OP_INR, val, 0, cy, ac)
                self._apply_alu_result(res, _ALU_OP_INR)
                self._rf.write(reg_code, res.result)
            return f"INR {reg_names[reg_code]}", f"Increment {reg_names[reg_code]}"

        # ── DCR r — 00ddd101 ──────────────────────────────────────────────
        if (opcode & 0x07) == 0x05 and (opcode & 0xC0) == 0x00:
            reg_code = (opcode >> 3) & 0x07
            cy, ac = self._flags.cy, self._flags.ac
            if reg_code == REG_M:
                addr = (self._rf.read(REG_H) << 8) | self._rf.read(REG_L)
                val = self._memory[addr]
                res = self._alu.execute(_ALU_OP_DCR, val, 0, cy, ac)
                self._apply_alu_result(res, _ALU_OP_DCR)
                self._memory[addr] = res.result
            else:
                val = self._rf.read(reg_code)
                res = self._alu.execute(_ALU_OP_DCR, val, 0, cy, ac)
                self._apply_alu_result(res, _ALU_OP_DCR)
                self._rf.write(reg_code, res.result)
            return f"DCR {reg_names[reg_code]}", f"Decrement {reg_names[reg_code]}"

        # ── MVI r,d8 — 00ddd110 ───────────────────────────────────────────
        if (opcode & 0x07) == 0x06:
            reg_code = (opcode >> 3) & 0x07
            if reg_code == REG_M:
                addr = (self._rf.read(REG_H) << 8) | self._rf.read(REG_L)
                self._memory[addr] = imm1
            else:
                self._rf.write(reg_code, imm1)
            return f"MVI {reg_names[reg_code]}", f"Move immediate {imm1:#04x} to {reg_names[reg_code]}"  # noqa: E501

        # ── RLC — 0x07 ────────────────────────────────────────────────────
        if opcode == 0x07:
            a = self._rf.read(REG_A)
            res = self._alu.execute(_ALU_OP_RLC, a, 0, self._flags.cy, self._flags.ac)
            self._rf.write(REG_A, res.result)
            self._flags.cy = res.cy
            return "RLC", "Rotate A left circular"

        # ── DAD rp — 00rp1001 ─────────────────────────────────────────────
        if (opcode & 0x0F) == 0x09:
            hl = self._rf.read_pair(PAIR_HL, self._sp)
            rp_val = self._rf.read_pair(rp, self._sp)
            new_hl, cy = add_16bit(hl, rp_val, 0)
            self._rf.write_pair(PAIR_HL, new_hl & 0xFFFF, self._sp)
            self._flags.cy = bool(cy)
            return f"DAD {pair_names[rp]}", f"Add {pair_names[rp]} to HL"

        # ── DCX rp — 00rp1011 ─────────────────────────────────────────────
        if (opcode & 0x0F) == 0x0B:
            val = self._rf.read_pair(rp, self._sp)
            new_val, _ = add_16bit(val, 0xFFFF, 0)   # +0xFFFF = -1 mod 2^16
            self._rf.write_pair(rp, new_val & 0xFFFF, self._sp)
            return f"DCX {pair_names[rp]}", f"Decrement register pair {pair_names[rp]}"

        # ── RRC — 0x0F ────────────────────────────────────────────────────
        if opcode == 0x0F:
            a = self._rf.read(REG_A)
            res = self._alu.execute(_ALU_OP_RRC, a, 0, self._flags.cy, self._flags.ac)
            self._rf.write(REG_A, res.result)
            self._flags.cy = res.cy
            return "RRC", "Rotate A right circular"

        # ── RAL — 0x17 ────────────────────────────────────────────────────
        if opcode == 0x17:
            a = self._rf.read(REG_A)
            res = self._alu.execute(_ALU_OP_RAL, a, 0, self._flags.cy, self._flags.ac)
            self._rf.write(REG_A, res.result)
            self._flags.cy = res.cy
            return "RAL", "Rotate A left through carry"

        # ── RAR — 0x1F ────────────────────────────────────────────────────
        if opcode == 0x1F:
            a = self._rf.read(REG_A)
            res = self._alu.execute(_ALU_OP_RAR, a, 0, self._flags.cy, self._flags.ac)
            self._rf.write(REG_A, res.result)
            self._flags.cy = res.cy
            return "RAR", "Rotate A right through carry"

        # ── SHLD addr — 0x22 ──────────────────────────────────────────────
        if opcode == 0x22:
            addr = imm16
            self._memory[addr] = self._rf.read(REG_L)
            self._memory[(addr + 1) & 0xFFFF] = self._rf.read(REG_H)
            return "SHLD", f"Store HL to memory {addr:#06x}"

        # ── DAA — 0x27 ────────────────────────────────────────────────────
        if opcode == 0x27:
            a = self._rf.read(REG_A)
            res = self._alu.execute(_ALU_OP_DAA, a, 0, self._flags.cy, self._flags.ac)
            self._apply_alu_result(res, _ALU_OP_DAA)
            self._rf.write(REG_A, res.result)
            return "DAA", "Decimal adjust accumulator"

        # ── LHLD addr — 0x2A ──────────────────────────────────────────────
        if opcode == 0x2A:
            addr = imm16
            lo = self._memory[addr]
            hi = self._memory[(addr + 1) & 0xFFFF]
            self._rf.write(REG_L, lo)
            self._rf.write(REG_H, hi)
            return "LHLD", f"Load HL from memory {addr:#06x}"

        # ── CMA — 0x2F ────────────────────────────────────────────────────
        if opcode == 0x2F:
            a = self._rf.read(REG_A)
            res = self._alu.execute(_ALU_OP_CMA, a, 0, self._flags.cy, self._flags.ac)
            self._rf.write(REG_A, res.result)
            # CMA does not change flags — don't call _apply_alu_result
            return "CMA", "Complement accumulator"

        # ── STA addr — 0x32 ───────────────────────────────────────────────
        if opcode == 0x32:
            self._memory[imm16] = self._rf.read(REG_A)
            return "STA", f"Store A to memory {imm16:#06x}"

        # ── STC — 0x37 ────────────────────────────────────────────────────
        if opcode == 0x37:
            self._flags.cy = True
            return "STC", "Set carry flag"

        # ── LDA addr — 0x3A ───────────────────────────────────────────────
        if opcode == 0x3A:
            self._rf.write(REG_A, self._memory[imm16])
            return "LDA", f"Load A from memory {imm16:#06x}"

        # ── CMC — 0x3F ────────────────────────────────────────────────────
        if opcode == 0x3F:
            self._flags.cy = not self._flags.cy
            return "CMC", "Complement carry flag"

        # ── LDAX rp — 00rp1010 ────────────────────────────────────────────
        if (opcode & 0x0F) == 0x0A and rp in (0, 1):
            addr = self._rf.read_pair(rp, self._sp)
            self._rf.write(REG_A, self._memory[addr])
            return f"LDAX {pair_names[rp]}", f"Load A indirect from {pair_names[rp]}"

        # ── STAX rp — 00rp0010 ────────────────────────────────────────────
        if (opcode & 0x0F) == 0x02 and rp in (0, 1):
            addr = self._rf.read_pair(rp, self._sp)
            self._memory[addr] = self._rf.read(REG_A)
            return f"STAX {pair_names[rp]}", f"Store A indirect to {pair_names[rp]}"

        return f"UNK_{opcode:02X}", f"Unknown group-00 opcode {opcode:#04x}"

    # ─── Group 11: branches, stack, control ──────────────────────────────

    def _exec_group11(  # noqa: PLR0911,PLR0912,PLR0915
        self,
        opcode: int,
        decoded: DecodedInstruction,  # noqa: F821
        imm1: int,
        imm2: int,
        imm16: int,
    ) -> tuple[str, str]:
        """Group 11: JMP, CALL, RET, PUSH, POP, ALU immediate, etc."""
        pair_names_stack = ["B", "D", "H", "PSW"]  # for PUSH/POP
        cond_names = ["NZ", "Z", "NC", "C", "PO", "PE", "P", "M"]
        alu_names = ["ADD", "ADC", "SUB", "SBB", "ANA", "XRA", "ORA", "CMP"]
        imm_names = ["ADI", "ACI", "SUI", "SBI", "ANI", "XRI", "ORI", "CPI"]

        # ── Conditional RET — 0bCC000 (opcode & 0xC7 == 0xC0) ────────────
        if (opcode & 0xC7) == 0xC0:
            cond = (opcode >> 3) & 0x07
            if self._flags.condition_met(cond):
                lo = self._memory[self._sp.read()]
                hi = self._memory[(self._sp.read() + 1) & 0xFFFF]
                self._sp.inc(2)
                self._pc.write((hi << 8) | lo)
                return f"R{cond_names[cond]}", f"Return if {cond_names[cond]}"
            return f"R{cond_names[cond]}", f"Return if {cond_names[cond]} (not taken)"

        # ── POP rp — 0bCC001 (opcode & 0xCF == 0xC1) ─────────────────────
        if (opcode & 0xCF) == 0xC1:
            stack_pair = (opcode >> 4) & 0x03
            lo = self._memory[self._sp.read()]
            hi = self._memory[(self._sp.read() + 1) & 0xFFFF]
            self._sp.inc(2)
            val16 = (hi << 8) | lo
            if stack_pair == 3:   # PSW: A and flags
                self._rf.write(REG_A, hi)
                self._flags.from_byte(lo)
            else:
                self._rf.write_pair(stack_pair, val16, self._sp)
            return f"POP {pair_names_stack[stack_pair]}", f"Pop {pair_names_stack[stack_pair]} from stack"  # noqa: E501

        # ── Conditional JMP — 0bCC010 (opcode & 0xC7 == 0xC2) ────────────
        if (opcode & 0xC7) == 0xC2:
            cond = (opcode >> 3) & 0x07
            if self._flags.condition_met(cond):
                self._pc.write(imm16)
                desc = f"Jump if {cond_names[cond]} to {imm16:#06x}"
                return f"J{cond_names[cond]}", desc
            return f"J{cond_names[cond]}", f"Jump if {cond_names[cond]} (not taken)"

        # ── JMP addr — 0xC3 ───────────────────────────────────────────────
        if opcode == 0xC3:
            self._pc.write(imm16)
            return "JMP", f"Unconditional jump to {imm16:#06x}"

        # ── Conditional CALL — 0bCC100 (opcode & 0xC7 == 0xC4) ───────────
        if (opcode & 0xC7) == 0xC4:
            cond = (opcode >> 3) & 0x07
            if self._flags.condition_met(cond):
                return_addr = self._pc.read()
                self._sp.dec(2)
                self._memory[(self._sp.read() + 1) & 0xFFFF] = (return_addr >> 8) & 0xFF
                self._memory[self._sp.read()] = return_addr & 0xFF
                self._pc.write(imm16)
                desc = f"Call if {cond_names[cond]} to {imm16:#06x}"
                return f"C{cond_names[cond]}", desc
            return f"C{cond_names[cond]}", f"Call if {cond_names[cond]} (not taken)"

        # ── PUSH rp — 0bCC101 (opcode & 0xCF == 0xC5) ────────────────────
        if (opcode & 0xCF) == 0xC5:
            stack_pair = (opcode >> 4) & 0x03
            if stack_pair == 3:   # PUSH PSW
                hi = self._rf.read(REG_A)
                lo = self._flags.to_byte()
            else:
                val16 = self._rf.read_pair(stack_pair, self._sp)
                hi = (val16 >> 8) & 0xFF
                lo = val16 & 0xFF
            self._sp.dec(2)
            self._memory[(self._sp.read() + 1) & 0xFFFF] = hi
            self._memory[self._sp.read()] = lo
            return f"PUSH {pair_names_stack[stack_pair]}", f"Push {pair_names_stack[stack_pair]} to stack"  # noqa: E501

        # ── ALU immediate — 0bAAA110 (opcode & 0xC7 == 0xC6) ─────────────
        if (opcode & 0xC7) == 0xC6:
            alu_op = (opcode >> 3) & 0x07
            a = self._rf.read(REG_A)
            res = self._alu.execute(alu_op, a, imm1, self._flags.cy, self._flags.ac)
            self._apply_alu_result(res, alu_op)
            return imm_names[alu_op], f"{alu_names[alu_op]} immediate {imm1:#04x}"

        # ── RST n — 0bNNN111 (opcode & 0xC7 == 0xC7) ─────────────────────
        if (opcode & 0xC7) == 0xC7:
            rst_n = (opcode >> 3) & 0x07
            return_addr = self._pc.read()
            self._sp.dec(2)
            self._memory[(self._sp.read() + 1) & 0xFFFF] = (return_addr >> 8) & 0xFF
            self._memory[self._sp.read()] = return_addr & 0xFF
            self._pc.write(rst_n * 8)
            return f"RST {rst_n}", f"Restart to address {rst_n * 8:#06x}"

        # ── RET — 0xC9 ────────────────────────────────────────────────────
        if opcode == 0xC9:
            lo = self._memory[self._sp.read()]
            hi = self._memory[(self._sp.read() + 1) & 0xFFFF]
            self._sp.inc(2)
            self._pc.write((hi << 8) | lo)
            return "RET", "Return from subroutine"

        # ── CALL addr — 0xCD ──────────────────────────────────────────────
        if opcode == 0xCD:
            return_addr = self._pc.read()
            self._sp.dec(2)
            self._memory[(self._sp.read() + 1) & 0xFFFF] = (return_addr >> 8) & 0xFF
            self._memory[self._sp.read()] = return_addr & 0xFF
            self._pc.write(imm16)
            return "CALL", f"Call subroutine at {imm16:#06x}"

        # ── Individual group-11 opcodes ───────────────────────────────────

        if opcode == 0xD3:   # OUT port
            self._output_ports[imm1] = self._rf.read(REG_A)
            return "OUT", f"Output A to port {imm1}"

        if opcode == 0xD9:   # EXX (undocumented, treated as NOP)
            return "NOP", "No operation (undocumented)"

        if opcode == 0xDB:   # IN port
            self._rf.write(REG_A, self._input_ports[imm1])
            return "IN", f"Input from port {imm1} to A"

        if opcode == 0xE3:   # XTHL
            sp_addr = self._sp.read()
            mem_lo = self._memory[sp_addr]
            mem_hi = self._memory[(sp_addr + 1) & 0xFFFF]
            l_val = self._rf.read(REG_L)
            h_val = self._rf.read(REG_H)
            self._memory[sp_addr] = l_val
            self._memory[(sp_addr + 1) & 0xFFFF] = h_val
            self._rf.write(REG_L, mem_lo)
            self._rf.write(REG_H, mem_hi)
            return "XTHL", "Exchange HL with top of stack"

        if opcode == 0xE9:   # PCHL
            h = self._rf.read(REG_H)
            lo = self._rf.read(REG_L)  # noqa: E741
            self._pc.write((h << 8) | lo)
            return "PCHL", "Load PC from HL"

        if opcode == 0xEB:   # XCHG
            h = self._rf.read(REG_H)
            lo = self._rf.read(REG_L)  # noqa: E741
            d = self._rf.read(REG_D)
            e = self._rf.read(REG_E)
            self._rf.write(REG_H, d)
            self._rf.write(REG_L, e)
            self._rf.write(REG_D, h)
            self._rf.write(REG_E, lo)
            return "XCHG", "Exchange DE and HL"

        if opcode == 0xF3:   # DI
            self._inte = False
            return "DI", "Disable interrupts"

        if opcode == 0xF9:   # SPHL
            hl = self._rf.read_pair(PAIR_HL, self._sp)
            self._sp.write(hl)
            return "SPHL", "Load SP from HL"

        if opcode == 0xFB:   # EI
            self._inte = True
            return "EI", "Enable interrupts"

        return f"UNK_{opcode:02X}", f"Unknown group-11 opcode {opcode:#04x}"

    # ─── ALU result application ───────────────────────────────────────────

    def _apply_alu_result(self, res: ALUResult8080, alu_op: int) -> None:  # noqa: F821
        """Update flags (and A for ops that write to accumulator).

        Which ops write to A:
          ADD, ADC, SUB, SBB, ANA, XRA, ORA: write result to A
          CMP: flags only (A unchanged)
          INR, DCR: write result to the specific target register (handled by
                    caller); only flags are updated here
          CMA: caller writes A; no flags changed
          DAA: caller writes A; flags updated here

        CY is only updated by ops that set update_cy=True (add/sub/logical).
        INR/DCR set update_cy=False to preserve CY.
        """
        # CMA: no register write, no flag changes
        if alu_op == _ALU_OP_CMA:
            return

        # Write result to A only for "accumulator-result" ops
        # INR/DCR write to a specific register (the caller does it); skip here
        _writes_to_a = alu_op not in (_ALU_OP_CMP, _ALU_OP_INR, _ALU_OP_DCR)
        if _writes_to_a:
            self._rf.write(REG_A, res.result)

        # Update S, Z, P, AC for all ops that affect flags
        self._flags.s = res.s
        self._flags.z = res.z
        self._flags.p = res.p
        self._flags.ac = res.ac

        # Update CY only if this operation updates it (ADD/SUB/logical do;
        # INR/DCR do not)
        if res.update_cy:
            self._flags.cy = res.cy
