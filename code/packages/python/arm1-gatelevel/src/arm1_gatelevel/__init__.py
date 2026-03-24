"""
arm1_gatelevel — ARM1 Gate-Level Simulator Built from Logic Gates
=========================================================================

Every arithmetic operation routes through actual logic gate functions — AND,
OR, XOR, NOT — chained into adders, then into a 32-bit ALU. The barrel
shifter is built from multiplexer trees. Registers are stored as bit arrays.

This is NOT the same as the behavioral simulator (arm1_simulator package).
Both produce identical results for any program. The difference is the
execution path::

    Behavioral:  opcode -> match statement -> host arithmetic -> result
    Gate-level:  opcode -> decoder gates -> barrel shifter muxes ->
                 ALU gates -> adder gates -> logic gates -> result

Architecture
~~~~~~~~~~~~

The gate-level simulator composes packages from layers below:

- logic_gates: AND, OR, XOR, NOT, mux2
- arithmetic: ripple_carry_adder
- arm1_simulator: types, condition codes, instruction encoding helpers

The execution flow for a single ADD instruction:

1. FETCH:   Read 32-bit instruction from memory
2. DECODE:  Extract bit fields (combinational logic)
3. CONDITION: Evaluate 4-bit condition code (gate tree)
4. BARREL SHIFT: Process Operand2 (5-level mux tree, ~640 gates)
5. ALU:     32-bit ripple-carry add (32 full adders, ~160 gates)
6. FLAGS:   Compute N/Z/C/V from result bits (NOR tree, XOR gates)
7. WRITE:   Store result in register file (32 flip-flops)

Total per instruction: ~1,000-1,500 gate function calls.
"""

from __future__ import annotations

from logic_gates import AND, NOT, OR, XNOR, XOR, mux2
from arithmetic import ripple_carry_adder

from arm1_simulator import (
    ARM1 as BehavioralARM1,
    COND_AL,
    COND_CC,
    COND_CS,
    COND_EQ,
    COND_GE,
    COND_GT,
    COND_HI,
    COND_LE,
    COND_LS,
    COND_LT,
    COND_MI,
    COND_NE,
    COND_NV,
    COND_PL,
    COND_VC,
    COND_VS,
    DecodedInstruction,
    FLAG_F,
    FLAG_I,
    Flags,
    HALT_SWI,
    INST_BLOCK_TRANSFER,
    INST_BRANCH,
    INST_COPROCESSOR,
    INST_DATA_PROCESSING,
    INST_LOAD_STORE,
    INST_SWI,
    INST_UNDEFINED,
    MASK_32,
    MODE_MASK,
    MODE_SVC,
    MemoryAccess,
    OP_MOV,
    OP_MVN,
    PC_MASK,
    Trace,
    cond_string,
    decode,
    disassemble,
    evaluate_condition,
    is_test_op,
)

__version__ = "0.1.0"


# =========================================================================
# Bit Conversion Helpers
# =========================================================================
#
# Converts between integer values and bit lists (LSB-first). The ARM1
# uses 32-bit data paths, so most conversions use width=32.
#
# LSB-first ordering matches how ripple-carry adders process data:
# bit 0 feeds the first full adder, bit 1 feeds the second, etc.
#
#   int_to_bits(5, 32) -> [1, 0, 1, 0, 0, 0, ..., 0]  (32 elements)
#   bits_to_int(...)   -> 5


def int_to_bits(value: int, width: int = 32) -> list[int]:
    """Convert an integer to a list of bits (LSB first).

    This is the bridge between the integer world (test programs, external API)
    and the gate-level world (lists of 0s and 1s flowing through gates).
    """
    value = value & MASK_32
    return [(value >> i) & 1 for i in range(width)]


def bits_to_int(bits: list[int]) -> int:
    """Convert a list of bits (LSB first) to an integer."""
    result = 0
    for i, bit in enumerate(bits):
        if i >= 32:
            break
        result |= bit << i
    return result


# =========================================================================
# Gate-Level ALU
# =========================================================================
#
# This ALU wraps the arithmetic package's ripple-carry adder and uses
# logic gate functions from the logic-gates package. Every ADD instruction
# traverses a chain of 32 full adders, each built from XOR, AND, and OR
# gates. Total: ~160 gate calls per addition.


def _bitwise_gate(
    a: list[int], b: list[int], gate: object
) -> list[int]:
    """Apply a 2-input gate function to each bit pair.

    This is how the real ARM1 does AND, OR, XOR — 32 gate instances in parallel.
    """
    return [gate(a[i], b[i]) for i in range(len(a))]  # type: ignore[operator]


def _bitwise_not(bits: list[int]) -> list[int]:
    """Apply NOT to each bit."""
    return [NOT(b) for b in bits]


def _compute_zero(bits: list[int]) -> int:
    """Check if all 32 bits are zero using NOR gates.

    In hardware, this is a tree of NOR/OR gates reducing 32 bits to 1.
    """
    combined = bits[0]
    for i in range(1, len(bits)):
        combined = OR(combined, bits[i])
    return NOT(combined)


def _compute_overflow(a: list[int], b: list[int], result: list[int]) -> int:
    """Detect signed overflow using XOR gates.

    V = (a[31] XOR result[31]) AND (b[31] XOR result[31])
    """
    xor1 = XOR(a[31], result[31])
    xor2 = XOR(b[31], result[31])
    return AND(xor1, xor2)


def gate_alu_execute(
    opcode: int,
    a: list[int],
    b: list[int],
    carry_in: int,
    shifter_carry: int,
    old_v: int,
) -> dict[str, object]:
    """Perform one of the 16 ALU operations using gate-level logic.

    Every operation routes through actual gate function calls:

    - Arithmetic: ripple_carry_adder (32 full adders -> 160+ gate calls)
    - Logical: AND/OR/XOR/NOT applied to each of 32 bits (32-64 gate calls)

    Returns a dict with keys: result (list[int]), n, z, c, v (all int).
    """
    result: list[int]
    carry: int = 0
    overflow: int = 0

    # -- Logical operations -------------------------------------------------
    if opcode in (0x0, 0x8):  # AND, TST
        result = _bitwise_gate(a, b, AND)
        carry = shifter_carry
        overflow = old_v

    elif opcode in (0x1, 0x9):  # EOR, TEQ
        result = _bitwise_gate(a, b, XOR)
        carry = shifter_carry
        overflow = old_v

    elif opcode == 0xC:  # ORR
        result = _bitwise_gate(a, b, OR)
        carry = shifter_carry
        overflow = old_v

    elif opcode == 0xD:  # MOV
        result = list(b)
        carry = shifter_carry
        overflow = old_v

    elif opcode == 0xE:  # BIC = AND(a, NOT(b))
        not_b = _bitwise_not(b)
        result = _bitwise_gate(a, not_b, AND)
        carry = shifter_carry
        overflow = old_v

    elif opcode == 0xF:  # MVN = NOT(b)
        result = _bitwise_not(b)
        carry = shifter_carry
        overflow = old_v

    # -- Arithmetic operations -----------------------------------------------
    elif opcode in (0x4, 0xB):  # ADD, CMN: A + B
        result, carry = ripple_carry_adder(a, b, 0)
        overflow = _compute_overflow(a, b, result)

    elif opcode == 0x5:  # ADC: A + B + C
        result, carry = ripple_carry_adder(a, b, carry_in)
        overflow = _compute_overflow(a, b, result)

    elif opcode in (0x2, 0xA):  # SUB, CMP: A - B = A + NOT(B) + 1
        not_b = _bitwise_not(b)
        result, carry = ripple_carry_adder(a, not_b, 1)
        overflow = _compute_overflow(a, not_b, result)

    elif opcode == 0x6:  # SBC: A - B - !C = A + NOT(B) + C
        not_b = _bitwise_not(b)
        result, carry = ripple_carry_adder(a, not_b, carry_in)
        overflow = _compute_overflow(a, not_b, result)

    elif opcode == 0x3:  # RSB: B - A = B + NOT(A) + 1
        not_a = _bitwise_not(a)
        result, carry = ripple_carry_adder(b, not_a, 1)
        overflow = _compute_overflow(b, not_a, result)

    elif opcode == 0x7:  # RSC: B - A - !C = B + NOT(A) + C
        not_a = _bitwise_not(a)
        result, carry = ripple_carry_adder(b, not_a, carry_in)
        overflow = _compute_overflow(b, not_a, result)

    else:
        result = [0] * 32

    # Compute N and Z flags from result bits
    n = result[31]
    z = _compute_zero(result)

    return {
        "result": result,
        "n": n,
        "z": z,
        "c": carry,
        "v": overflow,
    }


# =========================================================================
# Gate-Level Barrel Shifter
# =========================================================================
#
# On the real ARM1, the barrel shifter was implemented as a 32x32 crossbar
# network of pass transistors. We model it with a 5-level tree of mux2
# gates from the logic-gates package.
#
# Each level handles one bit of the shift amount:
#   Level 0: shift by 0 or 1   (controlled by amount bit 0)
#   Level 1: shift by 0 or 2   (controlled by amount bit 1)
#   Level 2: shift by 0 or 4   (controlled by amount bit 2)
#   Level 3: shift by 0 or 8   (controlled by amount bit 3)
#   Level 4: shift by 0 or 16  (controlled by amount bit 4)


def _gate_lsl(
    value: list[int], amount: int, carry_in: int, by_register: bool
) -> tuple[list[int], int]:
    """Logical Shift Left using a 5-level multiplexer tree."""
    if amount == 0:
        return list(value), carry_in
    if amount >= 32:
        result = [0] * 32
        if amount == 32:
            return result, value[0]
        return result, 0

    current = list(value)
    for level in range(5):
        shift = 1 << level
        sel = (amount >> level) & 1
        nxt = [0] * 32
        for i in range(32):
            shifted = current[i - shift] if i >= shift else 0
            nxt[i] = mux2(current[i], shifted, sel)
        current = nxt

    carry = value[32 - amount] if 0 < amount <= 32 else carry_in
    return current, carry


def _gate_lsr(
    value: list[int], amount: int, carry_in: int, by_register: bool
) -> tuple[list[int], int]:
    """Logical Shift Right using mux tree."""
    if amount == 0 and not by_register:
        return [0] * 32, value[31]
    if amount == 0:
        return list(value), carry_in
    if amount >= 32:
        result = [0] * 32
        if amount == 32:
            return result, value[31]
        return result, 0

    current = list(value)
    for level in range(5):
        shift = 1 << level
        sel = (amount >> level) & 1
        nxt = [0] * 32
        for i in range(32):
            shifted = current[i + shift] if i + shift < 32 else 0
            nxt[i] = mux2(current[i], shifted, sel)
        current = nxt

    carry = value[amount - 1]
    return current, carry


def _gate_asr(
    value: list[int], amount: int, carry_in: int, by_register: bool
) -> tuple[list[int], int]:
    """Arithmetic Shift Right (sign-extending) using mux tree."""
    sign_bit = value[31]

    if amount == 0 and not by_register:
        return [sign_bit] * 32, sign_bit
    if amount == 0:
        return list(value), carry_in
    if amount >= 32:
        return [sign_bit] * 32, sign_bit

    current = list(value)
    for level in range(5):
        shift = 1 << level
        sel = (amount >> level) & 1
        nxt = [0] * 32
        for i in range(32):
            shifted = current[i + shift] if i + shift < 32 else sign_bit
            nxt[i] = mux2(current[i], shifted, sel)
        current = nxt

    carry = value[amount - 1]
    return current, carry


def _gate_ror(
    value: list[int], amount: int, carry_in: int, by_register: bool
) -> tuple[list[int], int]:
    """Rotate Right using mux tree."""
    if amount == 0 and not by_register:
        # RRX: 33-bit rotate through carry
        result = [0] * 32
        for i in range(31):
            result[i] = value[i + 1]
        result[31] = carry_in  # Old carry becomes MSB
        carry = value[0]       # Old LSB becomes new carry
        return result, carry
    if amount == 0:
        return list(value), carry_in

    # Normalize to 0-31
    amount = amount & 31
    if amount == 0:
        return list(value), value[31]

    current = list(value)
    for level in range(5):
        shift = 1 << level
        sel = (amount >> level) & 1
        nxt = [0] * 32
        for i in range(32):
            shifted = current[(i + shift) % 32]
            nxt[i] = mux2(current[i], shifted, sel)
        current = nxt

    return current, current[31]


def gate_barrel_shift(
    value: list[int],
    shift_type: int,
    amount: int,
    carry_in: int,
    by_register: bool,
) -> tuple[list[int], int]:
    """Perform a shift operation on a 32-bit value using a tree of mux gates.

    Returns (shifted_value, carry_out) where both are bit arrays/ints.
    """
    if by_register and amount == 0:
        return list(value), carry_in

    if shift_type == 0:   # LSL
        return _gate_lsl(value, amount, carry_in, by_register)
    if shift_type == 1:   # LSR
        return _gate_lsr(value, amount, carry_in, by_register)
    if shift_type == 2:   # ASR
        return _gate_asr(value, amount, carry_in, by_register)
    if shift_type == 3:   # ROR
        return _gate_ror(value, amount, carry_in, by_register)
    return list(value), carry_in


def gate_decode_immediate(imm8: int, rotate: int) -> tuple[list[int], int]:
    """Decode a rotated immediate using gate-level rotation."""
    bits = int_to_bits(imm8, 32)
    rotate_amount = int(rotate * 2)
    if rotate_amount == 0:
        return bits, 0
    result, carry = _gate_ror(bits, rotate_amount, 0, False)
    return result, carry


# =========================================================================
# ARM1 Gate-Level CPU
# =========================================================================


class ARM1GateLevel:
    """Gate-level ARM1 simulator.

    Has the same external behavior as the behavioral ARM1, but routes all
    DATA PATH operations (ALU, barrel shifter, register read/write, flag
    computation) through gate-level primitives.

    The register file is stored as 27 arrays of 32 bit values (flip-flop
    states). Memory is NOT gate-level (would need millions of flip-flops).
    """

    def __init__(self, memory_size: int = 1024 * 1024) -> None:
        if memory_size <= 0:
            memory_size = 1024 * 1024
        self._regs: list[list[int]] = [[0] * 32 for _ in range(27)]
        self._memory: bytearray = bytearray(memory_size)
        self._halted: bool = False
        self._gate_ops: int = 0
        self.reset()

    def reset(self) -> None:
        """Restore the CPU to power-on state."""
        self._regs = [[0] * 32 for _ in range(27)]
        r15val = (FLAG_I | FLAG_F | MODE_SVC) & MASK_32
        self._regs[15] = int_to_bits(r15val, 32)
        self._halted = False
        self._gate_ops = 0

    # ── Register access (gate-level) ────────────────────────────────────

    def _read_reg(self, index: int) -> int:
        """Read a register as an integer."""
        phys = self._physical_reg(index)
        return bits_to_int(self._regs[phys])

    def _write_reg(self, index: int, value: int) -> None:
        """Write a register from an integer."""
        phys = self._physical_reg(index)
        self._regs[phys] = int_to_bits(value, 32)

    def _physical_reg(self, index: int) -> int:
        """Map logical register to physical, based on current mode."""
        mode = bits_to_int(self._regs[15]) & MODE_MASK
        if mode == 1 and 8 <= index <= 14:    # FIQ
            return 16 + (index - 8)
        if mode == 2 and 13 <= index <= 14:   # IRQ
            return 23 + (index - 13)
        if mode == 3 and 13 <= index <= 14:   # SVC
            return 25 + (index - 13)
        return index

    def _read_reg_bits(self, index: int) -> list[int]:
        """Read a register as a bit list."""
        phys = self._physical_reg(index)
        return list(self._regs[phys])

    @property
    def pc(self) -> int:
        """Return the current program counter."""
        return bits_to_int(self._regs[15]) & PC_MASK

    @pc.setter
    def pc(self, addr: int) -> None:
        """Set the PC portion of R15."""
        r15 = bits_to_int(self._regs[15])
        r15 = (r15 & ~PC_MASK & MASK_32) | (addr & PC_MASK)
        self._regs[15] = int_to_bits(r15, 32)

    @property
    def flags(self) -> Flags:
        """Return the current condition flags."""
        r15 = self._regs[15]
        return Flags(
            n=r15[31] == 1,
            z=r15[30] == 1,
            c=r15[29] == 1,
            v=r15[28] == 1,
        )

    def _set_flags(self, n: int, z: int, c: int, v: int) -> None:
        """Set condition flag bits directly."""
        self._regs[15][31] = n
        self._regs[15][30] = z
        self._regs[15][29] = c
        self._regs[15][28] = v

    @property
    def mode(self) -> int:
        """Return the current processor mode."""
        return bits_to_int(self._regs[15]) & MODE_MASK

    @property
    def halted(self) -> bool:
        """Return True if the CPU has been halted."""
        return self._halted

    @property
    def gate_ops(self) -> int:
        """Return the total number of gate operations performed."""
        return self._gate_ops

    # ── Memory (same as behavioral) ────────────────────────────────────

    def read_word(self, addr: int) -> int:
        """Read a 32-bit word from memory (little-endian)."""
        addr = addr & PC_MASK
        a = addr & ~3
        if a + 3 >= len(self._memory):
            return 0
        return (
            self._memory[a]
            | (self._memory[a + 1] << 8)
            | (self._memory[a + 2] << 16)
            | (self._memory[a + 3] << 24)
        )

    def write_word(self, addr: int, value: int) -> None:
        """Write a 32-bit word to memory (little-endian)."""
        addr = addr & PC_MASK
        a = addr & ~3
        value = value & MASK_32
        if a + 3 >= len(self._memory):
            return
        self._memory[a] = value & 0xFF
        self._memory[a + 1] = (value >> 8) & 0xFF
        self._memory[a + 2] = (value >> 16) & 0xFF
        self._memory[a + 3] = (value >> 24) & 0xFF

    def read_byte(self, addr: int) -> int:
        """Read a single byte from memory."""
        addr = addr & PC_MASK
        if addr >= len(self._memory):
            return 0
        return self._memory[addr]

    def write_byte(self, addr: int, value: int) -> None:
        """Write a single byte to memory."""
        addr = addr & PC_MASK
        if addr >= len(self._memory):
            return
        self._memory[addr] = value & 0xFF

    def load_program(self, code: bytes | bytearray, start_addr: int = 0) -> None:
        """Load machine code into memory."""
        for i, b in enumerate(code):
            addr = start_addr + i
            if addr < len(self._memory):
                self._memory[addr] = b

    # ── Condition evaluation (gate-level) ────────────────────────────

    def _evaluate_condition(self, cond: int, flags: Flags) -> bool:
        """Evaluate a condition code using gate-level logic."""
        n = 1 if flags.n else 0
        z = 1 if flags.z else 0
        c = 1 if flags.c else 0
        v = 1 if flags.v else 0

        self._gate_ops += 4

        if cond == COND_EQ:
            return z == 1
        if cond == COND_NE:
            return NOT(z) == 1
        if cond == COND_CS:
            return c == 1
        if cond == COND_CC:
            return NOT(c) == 1
        if cond == COND_MI:
            return n == 1
        if cond == COND_PL:
            return NOT(n) == 1
        if cond == COND_VS:
            return v == 1
        if cond == COND_VC:
            return NOT(v) == 1
        if cond == COND_HI:
            return AND(c, NOT(z)) == 1
        if cond == COND_LS:
            return OR(NOT(c), z) == 1
        if cond == COND_GE:
            return XNOR(n, v) == 1
        if cond == COND_LT:
            return XOR(n, v) == 1
        if cond == COND_GT:
            return AND(NOT(z), XNOR(n, v)) == 1
        if cond == COND_LE:
            return OR(z, XOR(n, v)) == 1
        if cond == COND_AL:
            return True
        if cond == COND_NV:
            return False
        return False

    # ── Execution ──────────────────────────────────────────────────────

    def step(self) -> Trace:
        """Execute one instruction and return a trace."""
        cur_pc = self.pc
        regs_before = [self._read_reg(i) for i in range(16)]
        flags_before = self.flags

        instruction = self.read_word(cur_pc)
        decoded = decode(instruction)
        cond_met = self._evaluate_condition(decoded.cond, flags_before)

        trace = Trace(
            address=cur_pc,
            raw=instruction,
            mnemonic=disassemble(decoded),
            condition=cond_string(decoded.cond),
            condition_met=cond_met,
            regs_before=regs_before,
            flags_before=flags_before,
        )

        self.pc = (cur_pc + 4) & PC_MASK

        if cond_met:
            if decoded.inst_type == INST_DATA_PROCESSING:
                self._execute_data_processing(decoded, trace)
            elif decoded.inst_type == INST_LOAD_STORE:
                self._execute_load_store(decoded, trace)
            elif decoded.inst_type == INST_BLOCK_TRANSFER:
                self._execute_block_transfer(decoded, trace)
            elif decoded.inst_type == INST_BRANCH:
                self._execute_branch(decoded, trace)
            elif decoded.inst_type == INST_SWI:
                self._execute_swi(decoded, trace)
            elif decoded.inst_type in (INST_COPROCESSOR, INST_UNDEFINED):
                self._trap_undefined(cur_pc)

        trace.regs_after = [self._read_reg(i) for i in range(16)]
        trace.flags_after = self.flags
        return trace

    def run(self, max_steps: int = 10000) -> list[Trace]:
        """Execute instructions until halted or max_steps reached."""
        traces: list[Trace] = []
        for _ in range(max_steps):
            if self._halted:
                break
            traces.append(self.step())
        return traces

    # ── Data Processing (gate-level) ────────────────────────────────

    def _read_reg_bits_for_exec(self, index: int) -> list[int]:
        """Read register bits, with PC+8 adjustment for R15."""
        if index == 15:
            val = (bits_to_int(self._regs[15]) + 4) & MASK_32
            return int_to_bits(val, 32)
        return self._read_reg_bits(index)

    def _read_reg_for_exec(self, index: int) -> int:
        """Read register value, with PC+8 adjustment for R15."""
        if index == 15:
            return (bits_to_int(self._regs[15]) + 4) & MASK_32
        return self._read_reg(index)

    def _execute_data_processing(
        self, d: DecodedInstruction, trace: Trace
    ) -> None:
        """Execute a data processing instruction through gate-level ALU."""
        # Read Rn as bits
        if d.opcode not in (OP_MOV, OP_MVN):
            a_bits = self._read_reg_bits_for_exec(d.rn)
        else:
            a_bits = [0] * 32

        # Get Operand2 through gate-level barrel shifter
        flags = self.flags
        flag_c = 1 if flags.c else 0
        flag_v = 1 if flags.v else 0

        if d.immediate:
            b_bits, shifter_carry = gate_decode_immediate(d.imm8, d.rotate)
            if d.rotate == 0:
                shifter_carry = flag_c
        else:
            rm_bits = self._read_reg_bits_for_exec(d.rm)
            if d.shift_by_reg:
                shift_amount = self._read_reg(d.rs) & 0xFF
            else:
                shift_amount = d.shift_imm
            b_bits, shifter_carry = gate_barrel_shift(
                rm_bits, d.shift_type, shift_amount, flag_c, d.shift_by_reg
            )

        # Execute ALU through gate-level ALU
        result = gate_alu_execute(
            d.opcode, a_bits, b_bits, flag_c, shifter_carry, flag_v
        )
        self._gate_ops += 200

        result_val = bits_to_int(result["result"])  # type: ignore[arg-type]

        # Write result
        if not is_test_op(d.opcode):
            if d.rd == 15:
                if d.s:
                    self._regs[15] = int_to_bits(result_val, 32)
                else:
                    self.pc = result_val & PC_MASK
            else:
                self._write_reg(d.rd, result_val)

        # Update flags
        if d.s and d.rd != 15:
            self._set_flags(
                result["n"], result["z"], result["c"], result["v"]  # type: ignore[arg-type]
            )
        if is_test_op(d.opcode):
            self._set_flags(
                result["n"], result["z"], result["c"], result["v"]  # type: ignore[arg-type]
            )

    # ── Load/Store, Block Transfer, Branch, SWI ────────────────────

    def _execute_load_store(
        self, d: DecodedInstruction, trace: Trace
    ) -> None:
        """Execute a load/store instruction using gate-level barrel shifter."""
        if d.immediate:
            rm_val = self._read_reg_for_exec(d.rm)
            if d.shift_imm != 0:
                rm_bits = int_to_bits(rm_val, 32)
                flag_c = 1 if self.flags.c else 0
                shifted, _ = gate_barrel_shift(
                    rm_bits, d.shift_type, d.shift_imm, flag_c, False
                )
                rm_val = bits_to_int(shifted)
            offset = rm_val
        else:
            offset = d.offset12

        base = self._read_reg_for_exec(d.rn)
        addr = ((base + offset) if d.up else (base - offset)) & MASK_32
        transfer_addr = addr if d.pre_index else base

        if d.load:
            if d.byte:
                value = self.read_byte(transfer_addr)
            else:
                value = self.read_word(transfer_addr)
                rotation = (transfer_addr & 3) * 8
                if rotation != 0:
                    value = ((value >> rotation) | (value << (32 - rotation))) & MASK_32
            trace.memory_reads.append(
                MemoryAccess(address=transfer_addr, value=value)
            )
            if d.rd == 15:
                self._regs[15] = int_to_bits(value, 32)
            else:
                self._write_reg(d.rd, value)
        else:
            value = self._read_reg_for_exec(d.rd)
            if d.byte:
                self.write_byte(transfer_addr, value & 0xFF)
            else:
                self.write_word(transfer_addr, value)
            trace.memory_writes.append(
                MemoryAccess(address=transfer_addr, value=value)
            )

        if d.write_back or not d.pre_index:
            if d.rn != 15:
                self._write_reg(d.rn, addr)

    def _execute_block_transfer(
        self, d: DecodedInstruction, trace: Trace
    ) -> None:
        """Execute a block data transfer instruction."""
        base = self._read_reg(d.rn)
        count = bin(d.register_list).count("1")
        if count == 0:
            return

        if not d.pre_index and d.up:       # IA
            start_addr = base
        elif d.pre_index and d.up:         # IB
            start_addr = base + 4
        elif not d.pre_index and not d.up: # DA
            start_addr = base - (count * 4) + 4
        else:                              # DB
            start_addr = base - (count * 4)

        start_addr = start_addr & MASK_32
        addr = start_addr

        for i in range(16):
            if (d.register_list >> i) & 1 == 0:
                continue
            if d.load:
                value = self.read_word(addr)
                trace.memory_reads.append(MemoryAccess(address=addr, value=value))
                if i == 15:
                    self._regs[15] = int_to_bits(value, 32)
                else:
                    self._write_reg(i, value)
            else:
                if i == 15:
                    value = (bits_to_int(self._regs[15]) + 4) & MASK_32
                else:
                    value = self._read_reg(i)
                self.write_word(addr, value)
                trace.memory_writes.append(MemoryAccess(address=addr, value=value))
            addr = (addr + 4) & MASK_32

        if d.write_back:
            new_base = (
                (base + count * 4) if d.up else (base - count * 4)
            ) & MASK_32
            self._write_reg(d.rn, new_base)

    def _execute_branch(self, d: DecodedInstruction, trace: Trace) -> None:
        """Execute a branch instruction."""
        branch_base = (self.pc + 4) & MASK_32
        if d.link:
            return_addr = bits_to_int(self._regs[15])
            self._write_reg(14, return_addr)
        target = (branch_base + d.branch_offset) & MASK_32
        self.pc = target & PC_MASK

    def _execute_swi(self, d: DecodedInstruction, trace: Trace) -> None:
        """Execute a software interrupt instruction."""
        if d.swi_comment == HALT_SWI:
            self._halted = True
            return

        r15val = bits_to_int(self._regs[15])
        self._regs[25] = list(self._regs[15])
        self._regs[26] = list(self._regs[15])

        r15val = (r15val & ~MODE_MASK & MASK_32) | MODE_SVC
        r15val |= FLAG_I
        self._regs[15] = int_to_bits(r15val, 32)
        self.pc = 0x08

    def _trap_undefined(self, instr_addr: int) -> None:
        """Handle an undefined instruction trap."""
        self._regs[26] = list(self._regs[15])
        r15val = bits_to_int(self._regs[15])
        r15val = (r15val & ~MODE_MASK & MASK_32) | MODE_SVC
        r15val |= FLAG_I
        self._regs[15] = int_to_bits(r15val, 32)
        self.pc = 0x04
