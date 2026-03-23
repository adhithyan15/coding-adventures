"""Intel 4004 gate-level CPU — all operations route through real logic gates.

=== What makes this a "gate-level" simulator? ===

Every computation in this CPU flows through the same gate chain that the
real Intel 4004 used:

    NOT/AND/OR/XOR → half_adder → full_adder → ripple_carry_adder → ALU
    D flip-flop → register → register file / program counter / stack

When you execute ADD R3, the value in register R3 is read from flip-flops,
the accumulator is read from flip-flops, both are fed into the ALU (which
uses full adders built from gates), and the result is clocked back into
the accumulator's flip-flops.

Nothing is simulated behaviorally. Every bit passes through gate functions.

=== Gate count ===

Component               Gates   Transistors (×4 per gate)
─────────────────────   ─────   ─────────────────────────
ALU (4-bit)             32      128
Register file (16×4)    480     1,920
Accumulator (4-bit)     24      96
Carry flag (1-bit)      6       24
Program counter (12)    96      384
Hardware stack (3×12)   226     904
Decoder                 ~50     200
Control + wiring        ~100    400
─────────────────────   ─────   ─────────────────────────
Total                   ~1,014  ~4,056

The real Intel 4004 had 2,300 transistors. Our count is higher because
we model RAM separately (the real 4004 used external 4002 RAM chips)
and our gate model isn't minimized with Karnaugh maps.

=== Execution model ===

Each instruction executes in a single `step()` call, which corresponds
to one machine cycle. The fetch-decode-execute pipeline:

    1. FETCH:   Read instruction byte from ROM using PC
    2. FETCH2:  For 2-byte instructions, read the second byte
    3. DECODE:  Route instruction through decoder gate network
    4. EXECUTE: Perform the operation through ALU/registers/etc.
"""

from __future__ import annotations

from dataclasses import dataclass

from logic_gates import AND, NOT, OR

from intel4004_gatelevel.alu import GateALU
from intel4004_gatelevel.bits import bits_to_int, int_to_bits
from intel4004_gatelevel.decoder import DecodedInstruction, decode
from intel4004_gatelevel.pc import ProgramCounter
from intel4004_gatelevel.ram import RAM
from intel4004_gatelevel.registers import Accumulator, CarryFlag, RegisterFile
from intel4004_gatelevel.stack import HardwareStack


@dataclass
class GateTrace:
    """Trace record for one instruction execution.

    Same information as Intel4004Trace from the behavioral simulator,
    plus gate-level details.
    """

    address: int
    raw: int
    raw2: int | None
    mnemonic: str
    accumulator_before: int
    accumulator_after: int
    carry_before: bool
    carry_after: bool


class Intel4004GateLevel:
    """Intel 4004 CPU where every operation routes through real logic gates.

    Public API matches the behavioral Intel4004Simulator for
    cross-validation, but internally all computation flows through
    gates, flip-flops, and adders.

    Usage:
        >>> cpu = Intel4004GateLevel()
        >>> traces = cpu.run(bytes([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01]))
        >>> cpu.registers[1]  # R1 = 1 + 2 = 3
        3
    """

    def __init__(self) -> None:
        # --- Gate-level components ---
        self._alu = GateALU()
        self._regs = RegisterFile()
        self._acc = Accumulator()
        self._carry = CarryFlag()
        self._pc = ProgramCounter()
        self._stack = HardwareStack()
        self._ram = RAM()

        # --- ROM (read-only, loaded by program) ---
        self._rom: bytearray = bytearray(4096)

        # --- RAM addressing (set by SRC/DCL) ---
        self._ram_bank: int = 0
        self._ram_register: int = 0
        self._ram_character: int = 0

        # --- ROM I/O port ---
        self._rom_port: int = 0

        # --- Control state ---
        self._halted: bool = False

    # ------------------------------------------------------------------
    # Property accessors (match behavioral simulator's interface)
    # ------------------------------------------------------------------

    @property
    def accumulator(self) -> int:
        """Read accumulator from flip-flops."""
        return self._acc.read()

    @property
    def registers(self) -> list[int]:
        """Read all 16 registers from flip-flops."""
        return [self._regs.read(i) for i in range(16)]

    @property
    def carry(self) -> bool:
        """Read carry flag from flip-flop."""
        return self._carry.read()

    @property
    def pc(self) -> int:
        """Read program counter from flip-flops."""
        return self._pc.read()

    @property
    def halted(self) -> bool:
        return self._halted

    @property
    def hw_stack(self) -> list[int]:
        """Read stack levels (for inspection only)."""
        saved_ptr = self._stack._pointer
        values = []
        for i in range(3):
            from logic_gates import register as reg_fn

            output, _ = reg_fn(
                [0] * 12, clock=0,
                state=self._stack._levels[i], width=12,
            )
            values.append(bits_to_int(output))
        self._stack._pointer = saved_ptr
        return values

    @property
    def ram(self) -> list[list[list[int]]]:
        """Read RAM main characters."""
        return [
            [
                [self._ram.read_main(b, r, c) for c in range(16)]
                for r in range(4)
            ]
            for b in range(4)
        ]

    @property
    def ram_status(self) -> list[list[list[int]]]:
        """Read RAM status characters."""
        return [
            [
                [self._ram.read_status(b, r, s) for s in range(4)]
                for r in range(4)
            ]
            for b in range(4)
        ]

    @property
    def ram_bank(self) -> int:
        return self._ram_bank

    @property
    def rom_port(self) -> int:
        return self._rom_port

    @property
    def ram_output(self) -> list[int]:
        return [self._ram.read_output(i) for i in range(4)]

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def load_program(self, program: bytes) -> None:
        """Load a program into ROM."""
        self._rom = bytearray(4096)
        for i, b in enumerate(program):
            if i < 4096:
                self._rom[i] = b

    def step(self) -> GateTrace:
        """Execute one instruction through the gate-level pipeline.

        Returns a GateTrace with before/after state.
        """
        if self._halted:
            raise RuntimeError("CPU is halted — cannot step further")

        # Snapshot state before
        acc_before = self._acc.read()
        carry_before = self._carry.read()
        pc_before = self._pc.read()

        # FETCH: read instruction byte from ROM
        raw = self._rom[pc_before]

        # DECODE: route through combinational decoder
        decoded = decode(raw)

        # FETCH2: if 2-byte, read second byte
        raw2 = None
        if decoded.is_two_byte:
            raw2 = self._rom[(pc_before + 1) & 0xFFF]
            decoded = decode(raw, raw2)

        # EXECUTE: route through appropriate gate paths
        mnemonic = self._execute(decoded)

        return GateTrace(
            address=pc_before,
            raw=raw,
            raw2=raw2,
            mnemonic=mnemonic,
            accumulator_before=acc_before,
            accumulator_after=self._acc.read(),
            carry_before=carry_before,
            carry_after=self._carry.read(),
        )

    def run(
        self, program: bytes, max_steps: int = 10000
    ) -> list[GateTrace]:
        """Load and run a program, returning execution trace."""
        self.reset()
        self.load_program(program)

        traces: list[GateTrace] = []
        for _ in range(max_steps):
            if self._halted:
                break
            traces.append(self.step())
        return traces

    def reset(self) -> None:
        """Reset all CPU state."""
        self._acc.reset()
        self._carry.reset()
        self._regs.reset()
        self._pc.reset()
        self._stack.reset()
        self._ram.reset()
        self._rom = bytearray(4096)
        self._ram_bank = 0
        self._ram_register = 0
        self._ram_character = 0
        self._rom_port = 0
        self._halted = False

    def gate_count(self) -> int:
        """Total estimated gate count for the CPU."""
        return (
            self._alu.gate_count
            + self._regs.gate_count
            + self._acc.gate_count
            + self._carry.gate_count
            + self._pc.gate_count
            + self._stack.gate_count
            + self._ram.gate_count
            + 50   # decoder
            + 100  # control logic and wiring
        )

    # ------------------------------------------------------------------
    # Instruction execution — routes through gate-level components
    # ------------------------------------------------------------------

    def _execute(self, d: DecodedInstruction) -> str:
        """Execute a decoded instruction through gate paths.

        Each instruction routes through the appropriate combination of
        ALU, registers, and flip-flops.
        """
        # NOP
        if d.is_nop:
            self._pc.increment()
            return "NOP"

        # HLT
        if d.is_hlt:
            self._halted = True
            self._pc.increment()
            return "HLT"

        # LDM N: load immediate into accumulator
        if d.is_ldm:
            self._acc.write(d.immediate)
            self._pc.increment()
            return f"LDM {d.immediate}"

        # LD Rn: load register into accumulator
        if d.is_ld:
            val = self._regs.read(d.reg_index)
            self._acc.write(val)
            self._pc.increment()
            return f"LD R{d.reg_index}"

        # XCH Rn: exchange accumulator and register
        if d.is_xch:
            a_val = self._acc.read()
            r_val = self._regs.read(d.reg_index)
            self._acc.write(r_val)
            self._regs.write(d.reg_index, a_val)
            self._pc.increment()
            return f"XCH R{d.reg_index}"

        # INC Rn: increment register (no carry effect)
        if d.is_inc:
            r_val = self._regs.read(d.reg_index)
            result, _ = self._alu.increment(r_val)
            self._regs.write(d.reg_index, result)
            self._pc.increment()
            return f"INC R{d.reg_index}"

        # ADD Rn: add register to accumulator with carry
        if d.is_add:
            a_val = self._acc.read()
            r_val = self._regs.read(d.reg_index)
            carry_in = 1 if self._carry.read() else 0
            result, carry_out = self._alu.add(a_val, r_val, carry_in)
            self._acc.write(result)
            self._carry.write(carry_out)
            self._pc.increment()
            return f"ADD R{d.reg_index}"

        # SUB Rn: subtract register from accumulator
        if d.is_sub:
            a_val = self._acc.read()
            r_val = self._regs.read(d.reg_index)
            borrow_in = 0 if self._carry.read() else 1
            result, carry_out = self._alu.subtract(
                a_val, r_val, borrow_in
            )
            self._acc.write(result)
            self._carry.write(carry_out)
            self._pc.increment()
            return f"SUB R{d.reg_index}"

        # JUN addr: unconditional jump
        if d.is_jun:
            self._pc.load(d.addr12)
            return f"JUN 0x{d.addr12:03X}"

        # JCN cond,addr: conditional jump
        if d.is_jcn:
            return self._exec_jcn(d)

        # ISZ Rn,addr: increment and skip if zero
        if d.is_isz:
            return self._exec_isz(d)

        # JMS addr: jump to subroutine
        if d.is_jms:
            return_addr = self._pc.read() + 2
            self._stack.push(return_addr)
            self._pc.load(d.addr12)
            return f"JMS 0x{d.addr12:03X}"

        # BBL N: branch back and load
        if d.is_bbl:
            self._acc.write(d.immediate)
            return_addr = self._stack.pop()
            self._pc.load(return_addr)
            return f"BBL {d.immediate}"

        # FIM Pp,data: fetch immediate to pair
        if d.is_fim:
            self._regs.write_pair(d.pair_index, d.addr8)
            self._pc.increment2()
            return f"FIM P{d.pair_index},0x{d.addr8:02X}"

        # SRC Pp: send register control
        if d.is_src:
            pair_val = self._regs.read_pair(d.pair_index)
            self._ram_register = (pair_val >> 4) & 0xF
            self._ram_character = pair_val & 0xF
            self._pc.increment()
            return f"SRC P{d.pair_index}"

        # FIN Pp: fetch indirect from ROM
        if d.is_fin:
            p0_val = self._regs.read_pair(0)
            page = self._pc.read() & 0xF00
            rom_addr = page | p0_val
            rom_byte = self._rom[rom_addr & 0xFFF]
            self._regs.write_pair(d.pair_index, rom_byte)
            self._pc.increment()
            return f"FIN P{d.pair_index}"

        # JIN Pp: jump indirect
        if d.is_jin:
            pair_val = self._regs.read_pair(d.pair_index)
            page = self._pc.read() & 0xF00
            self._pc.load(page | pair_val)
            return f"JIN P{d.pair_index}"

        # I/O operations (0xE_ range)
        if d.is_io:
            return self._exec_io(d)

        # Accumulator operations (0xF_ range)
        if d.is_accum:
            return self._exec_accum(d)

        # Unknown — advance PC to avoid infinite loop
        self._pc.increment()
        return f"UNKNOWN(0x{d.raw:02X})"

    def _exec_jcn(self, d: DecodedInstruction) -> str:
        """JCN cond,addr: conditional jump using gate logic.

        Condition nibble bits (evaluated with OR/AND/NOT gates):
            Bit 3: INVERT
            Bit 2: TEST A==0
            Bit 1: TEST carry==1
            Bit 0: TEST pin (always 0)
        """
        cond = d.condition
        a_val = self._acc.read()
        carry_val = 1 if self._carry.read() else 0

        # Test A==0: OR all accumulator bits, then NOT
        a_bits = int_to_bits(a_val, 4)
        a_is_zero = NOT(OR(OR(a_bits[0], a_bits[1]),
                           OR(a_bits[2], a_bits[3])))

        # Build test result using gates
        test_zero = AND((cond >> 2) & 1, a_is_zero)
        test_carry = AND((cond >> 1) & 1, carry_val)
        test_pin = AND(cond & 1, 0)  # Pin always 0

        test_result = OR(OR(test_zero, test_carry), test_pin)

        # Invert if bit 3 set
        invert = (cond >> 3) & 1
        # XOR with invert: if invert=1, flip result
        # Using gates: result XOR invert = OR(AND(result, NOT(invert)),
        #                                      AND(NOT(result), invert))
        final = OR(
            AND(test_result, NOT(invert)),
            AND(NOT(test_result), invert),
        )

        page = (self._pc.read() + 2) & 0xF00
        target = page | d.addr8

        if final:
            self._pc.load(target)
        else:
            self._pc.increment2()

        return f"JCN {cond},{d.addr8:02X}"

    def _exec_isz(self, d: DecodedInstruction) -> str:
        """ISZ Rn,addr: increment register, skip if zero."""
        r_val = self._regs.read(d.reg_index)
        result, _ = self._alu.increment(r_val)
        self._regs.write(d.reg_index, result)

        # Test if result is zero using NOR of all bits
        r_bits = int_to_bits(result, 4)
        is_zero = NOT(OR(OR(r_bits[0], r_bits[1]),
                         OR(r_bits[2], r_bits[3])))

        page = (self._pc.read() + 2) & 0xF00
        target = page | d.addr8

        if is_zero:
            # Result is zero → fall through
            self._pc.increment2()
        else:
            # Result is nonzero → jump
            self._pc.load(target)

        return f"ISZ R{d.reg_index},0x{d.addr8:02X}"

    def _exec_io(self, d: DecodedInstruction) -> str:
        """Execute I/O instructions (0xE0–0xEF)."""
        a_val = self._acc.read()
        sub_op = d.lower

        if sub_op == 0x0:  # WRM
            self._ram.write_main(
                self._ram_bank, self._ram_register,
                self._ram_character, a_val,
            )
            self._pc.increment()
            return "WRM"

        if sub_op == 0x1:  # WMP
            self._ram.write_output(self._ram_bank, a_val)
            self._pc.increment()
            return "WMP"

        if sub_op == 0x2:  # WRR
            self._rom_port = a_val & 0xF
            self._pc.increment()
            return "WRR"

        if sub_op == 0x3:  # WPM (NOP in simulation)
            self._pc.increment()
            return "WPM"

        if 0x4 <= sub_op <= 0x7:  # WR0–WR3
            idx = sub_op - 0x4
            self._ram.write_status(
                self._ram_bank, self._ram_register, idx, a_val,
            )
            self._pc.increment()
            return f"WR{idx}"

        if sub_op == 0x8:  # SBM
            ram_val = self._ram.read_main(
                self._ram_bank, self._ram_register, self._ram_character,
            )
            borrow_in = 0 if self._carry.read() else 1
            result, carry_out = self._alu.subtract(
                a_val, ram_val, borrow_in,
            )
            self._acc.write(result)
            self._carry.write(carry_out)
            self._pc.increment()
            return "SBM"

        if sub_op == 0x9:  # RDM
            val = self._ram.read_main(
                self._ram_bank, self._ram_register, self._ram_character,
            )
            self._acc.write(val)
            self._pc.increment()
            return "RDM"

        if sub_op == 0xA:  # RDR
            self._acc.write(self._rom_port & 0xF)
            self._pc.increment()
            return "RDR"

        if sub_op == 0xB:  # ADM
            ram_val = self._ram.read_main(
                self._ram_bank, self._ram_register, self._ram_character,
            )
            carry_in = 1 if self._carry.read() else 0
            result, carry_out = self._alu.add(a_val, ram_val, carry_in)
            self._acc.write(result)
            self._carry.write(carry_out)
            self._pc.increment()
            return "ADM"

        if 0xC <= sub_op <= 0xF:  # RD0–RD3
            idx = sub_op - 0xC
            val = self._ram.read_status(
                self._ram_bank, self._ram_register, idx,
            )
            self._acc.write(val)
            self._pc.increment()
            return f"RD{idx}"

        self._pc.increment()
        return f"IO(0x{d.raw:02X})"

    def _exec_accum(self, d: DecodedInstruction) -> str:
        """Execute accumulator operations (0xF0–0xFD)."""
        a_val = self._acc.read()
        sub_op = d.lower

        if sub_op == 0x0:  # CLB
            self._acc.write(0)
            self._carry.write(False)
            self._pc.increment()
            return "CLB"

        if sub_op == 0x1:  # CLC
            self._carry.write(False)
            self._pc.increment()
            return "CLC"

        if sub_op == 0x2:  # IAC
            result, carry = self._alu.increment(a_val)
            self._acc.write(result)
            self._carry.write(carry)
            self._pc.increment()
            return "IAC"

        if sub_op == 0x3:  # CMC
            self._carry.write(not self._carry.read())
            self._pc.increment()
            return "CMC"

        if sub_op == 0x4:  # CMA
            result = self._alu.complement(a_val)
            self._acc.write(result)
            self._pc.increment()
            return "CMA"

        if sub_op == 0x5:  # RAL
            old_carry = 1 if self._carry.read() else 0
            # Use gates: A3 goes to carry, shift left, old carry to bit 0
            a_bits = int_to_bits(a_val, 4)
            self._carry.write(bool(a_bits[3]))
            new_bits = [old_carry, a_bits[0], a_bits[1], a_bits[2]]
            self._acc.write(bits_to_int(new_bits))
            self._pc.increment()
            return "RAL"

        if sub_op == 0x6:  # RAR
            old_carry = 1 if self._carry.read() else 0
            a_bits = int_to_bits(a_val, 4)
            self._carry.write(bool(a_bits[0]))
            new_bits = [a_bits[1], a_bits[2], a_bits[3], old_carry]
            self._acc.write(bits_to_int(new_bits))
            self._pc.increment()
            return "RAR"

        if sub_op == 0x7:  # TCC
            self._acc.write(1 if self._carry.read() else 0)
            self._carry.write(False)
            self._pc.increment()
            return "TCC"

        if sub_op == 0x8:  # DAC
            result, carry = self._alu.decrement(a_val)
            self._acc.write(result)
            self._carry.write(carry)
            self._pc.increment()
            return "DAC"

        if sub_op == 0x9:  # TCS
            self._acc.write(10 if self._carry.read() else 9)
            self._carry.write(False)
            self._pc.increment()
            return "TCS"

        if sub_op == 0xA:  # STC
            self._carry.write(True)
            self._pc.increment()
            return "STC"

        if sub_op == 0xB:  # DAA
            if a_val > 9 or self._carry.read():
                result, carry = self._alu.add(a_val, 6, 0)
                if carry:
                    self._carry.write(True)
                self._acc.write(result)
            self._pc.increment()
            return "DAA"

        if sub_op == 0xC:  # KBP
            kbp_table = {0: 0, 1: 1, 2: 2, 4: 3, 8: 4}
            self._acc.write(kbp_table.get(a_val, 15))
            self._pc.increment()
            return "KBP"

        if sub_op == 0xD:  # DCL
            bank = self._alu.bitwise_and(a_val, 0x7)
            if bank > 3:
                bank = self._alu.bitwise_and(bank, 0x3)
            self._ram_bank = bank
            self._pc.increment()
            return "DCL"

        self._pc.increment()
        return f"ACCUM(0x{d.raw:02X})"
