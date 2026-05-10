"""Intel 8051 (MCS-51) behavioral simulator — Layer 07p.

The 8051 is a Harvard-architecture microcontroller introduced by Intel in 1980.
It is the most-manufactured CPU architecture in history, with over 20 billion
units produced.  Its key innovations:
  - Single-chip integration of CPU + RAM + ROM + I/O + timers + serial port
  - Harvard architecture: separate code and data buses for simultaneous fetch
  - Bit-addressable memory area: individual bits in RAM can be set/cleared
  - Four register banks switchable via the PSW, enabling fast interrupt context

This simulator implements the SIM00 protocol: Simulator[I8051State].

=============================================================================
Architecture recap (see spec for full details)
=============================================================================

Memory spaces
─────────────
  _code   : 64 KB Harvard code memory (read-only by CPU instructions)
  _iram   : 256 bytes internal RAM, split as:
                0x00–0x7F  general RAM (register banks, bit area, scratch)
                0x80–0xFF  SFRs (Special Function Registers)
  _xdata  : 64 KB external data memory (accessed via MOVX)

Registers (all stored in _iram as SFRs or implicitly in _pc)
─────────────────────────────────────────────────────────────
  PC       16-bit, stored as self._pc (NOT in iram)
  ACC      iram[0xE0]
  B        iram[0xF0]
  SP       iram[0x81]     reset value: 0x07
  DPH:DPL  iram[0x83:0x82]
  PSW      iram[0xD0]     bits: CY AC F0 RS1 RS0 OV - P

Register bank selection (PSW bits 4:3)
  Bank 0 → iram[0x00–0x07]
  Bank 1 → iram[0x08–0x0F]
  Bank 2 → iram[0x10–0x17]
  Bank 3 → iram[0x18–0x1F]

=============================================================================
Encoding notes — how to read the instruction tables below
=============================================================================

The 8051 opcode byte fully encodes the instruction form.  Many instructions
come in "families" that share 5 high bits and use the 3 low bits to select
the operand:

    0x28 + n  → ADD A, Rn      (n = 0–7)
    0xE8 + n  → MOV A, Rn
    0xF8 + n  → MOV Rn, A
    etc.

Two-register indirect instructions use a 1-bit field for the pointer (Ri
can only be R0 or R1):

    0xE6 + i  → MOV A, @Ri    (i = 0 or 1)
    0xE2 + i  → MOVX A, @Ri

Branch instructions carry a signed 8-bit offset byte; the target is
PC_after_fetch + rel8  (where PC_after_fetch = PC of instruction + instr_len).

=============================================================================
HALT convention
=============================================================================

Opcode 0xA5 is "undefined" on real 8051 hardware.  This simulator uses it
as a HALT sentinel (as specified in spec 07p).  Executing 0xA5 sets _halted=True
and returns a StepTrace with mnemonic="HALT".
"""

from __future__ import annotations

from simulator_protocol import ExecutionResult, Simulator, StepTrace

from .flags import add8_flags, da_flags, sub8_flags
from .state import (
    CODE_SIZE,
    HALT_OPCODE,
    IRAM_SIZE,
    PSW_AC,
    PSW_CY,
    PSW_OV,
    PSW_P,
    SFR_ACC,
    SFR_B,
    SFR_DPH,
    SFR_DPL,
    SFR_P0,
    SFR_P1,
    SFR_P2,
    SFR_P3,
    SFR_PSW,
    SFR_SP,
    XDATA_SIZE,
    I8051State,
)

# ── Type alias ─────────────────────────────────────────────────────────────────

_Mem = bytearray


class I8051Simulator(Simulator[I8051State]):
    """Behavioral simulator for the Intel 8051 microcontroller.

    Public API (SIM00 protocol):
        reset()            — return to power-on state
        load(program)      — reset and copy bytes to code memory at 0x0000
        step()             — execute one instruction, return StepTrace
        execute(program)   — run until HALT or max_steps, return ExecutionResult
        get_state()        — return frozen I8051State snapshot
    """

    def __init__(self) -> None:
        self._code:  _Mem = bytearray(CODE_SIZE)
        self._iram:  _Mem = bytearray(IRAM_SIZE)
        self._xdata: _Mem = bytearray(XDATA_SIZE)
        self._pc:    int  = 0
        self._halted: bool = False
        self.reset()

    # ── Protocol: reset ───────────────────────────────────────────────────────

    def reset(self) -> None:
        """Return the CPU to power-on state.

        Hardware reset state (8051 datasheet):
          PC = 0x0000
          SP = 0x07
          P0–P3 = 0xFF  (all port latches high)
          All other SFRs = 0x00
          IRAM content: undefined (zeroed in simulator)
          Code/xdata memory: preserved (only changed by load())
        """
        self._iram[:] = bytearray(IRAM_SIZE)
        self._pc      = 0x0000
        self._halted  = False
        # SP = 0x07 at reset
        self._iram[SFR_SP] = 0x07
        # Port latches = 0xFF at reset
        for port_sfr in (SFR_P0, SFR_P1, SFR_P2, SFR_P3):
            self._iram[port_sfr] = 0xFF

    # ── Protocol: load ────────────────────────────────────────────────────────

    def load(self, program: bytes) -> None:
        """Reset and load program bytes into code memory starting at 0x0000.

        Args:
            program: raw machine code bytes

        Raises:
            ValueError: if len(program) > 65536
        """
        if len(program) > CODE_SIZE:
            msg = f"Program too large: {len(program)} bytes > {CODE_SIZE}"
            raise ValueError(msg)
        self.reset()
        self._code[:len(program)] = program

    # ── Protocol: get_state ───────────────────────────────────────────────────

    def get_state(self) -> I8051State:
        """Return a frozen snapshot of the current CPU state."""
        return I8051State(
            pc     = self._pc,
            iram   = tuple(self._iram),
            xdata  = tuple(self._xdata),
            code   = tuple(self._code),
            halted = self._halted,
        )

    # ── Protocol: step ────────────────────────────────────────────────────────

    def step(self) -> StepTrace:
        """Execute one instruction and return a StepTrace.

        If the CPU is halted, returns a no-op trace without advancing PC.
        """
        pc_before = self._pc
        if self._halted:
            return StepTrace(
                pc_before   = pc_before,
                pc_after    = pc_before,
                mnemonic    = "HALT",
                description = "HALT (already halted)",
            )
        mnemonic = self._execute_one()
        return StepTrace(
            pc_before   = pc_before,
            pc_after    = self._pc,
            mnemonic    = mnemonic,
            description = f"{mnemonic} @ 0x{pc_before:04X}",
        )

    # ── Protocol: execute ─────────────────────────────────────────────────────

    def execute(self, program: bytes, max_steps: int = 100_000) -> ExecutionResult:
        """Load and run program until HALT or max_steps exceeded.

        Args:
            program:   raw machine code
            max_steps: guard against infinite loops

        Returns:
            ExecutionResult with halted, steps, traces, final_state, error
        """
        self.load(program)
        traces: list[StepTrace] = []
        error: str | None = None
        steps = 0
        while not self._halted and steps < max_steps:
            try:
                trace = self.step()
            except Exception as exc:  # noqa: BLE001
                error = str(exc)
                break
            traces.append(trace)
            steps += 1
        if not self._halted and error is None:
            error = f"max_steps ({max_steps}) exceeded"
        return ExecutionResult(
            halted      = self._halted,
            steps       = steps,
            traces      = traces,
            final_state = self.get_state(),
            error       = error,
        )

    # =========================================================================
    # Internal helpers
    # =========================================================================

    def _fetch8(self) -> int:
        """Fetch one byte from code memory at PC and advance PC."""
        b = self._code[self._pc & 0xFFFF]
        self._pc = (self._pc + 1) & 0xFFFF
        return b

    def _fetch16(self) -> int:
        """Fetch two bytes (big-endian) from code memory and advance PC by 2."""
        hi = self._fetch8()
        lo = self._fetch8()
        return (hi << 8) | lo

    def _rn_addr(self, n: int) -> int:
        """Return the IRAM address of Rn in the current register bank."""
        bank = (self._iram[SFR_PSW] >> 3) & 0x3
        return bank * 8 + (n & 0x7)

    def _rn(self, n: int) -> int:
        """Read register Rn from the current register bank."""
        return self._iram[self._rn_addr(n)]

    def _set_rn(self, n: int, val: int) -> None:
        """Write register Rn in the current register bank."""
        self._iram[self._rn_addr(n)] = val & 0xFF

    def _dptr(self) -> int:
        """Read 16-bit DPTR = DPH:DPL."""
        return (self._iram[SFR_DPH] << 8) | self._iram[SFR_DPL]

    def _set_dptr(self, val: int) -> None:
        """Write 16-bit DPTR."""
        self._iram[SFR_DPH] = (val >> 8) & 0xFF
        self._iram[SFR_DPL] = val & 0xFF

    def _acc(self) -> int:
        """Read ACC."""
        return self._iram[SFR_ACC]

    def _set_acc(self, val: int) -> None:
        """Write ACC and update parity bit in PSW."""
        self._iram[SFR_ACC] = val & 0xFF
        self._update_parity()

    def _update_parity(self) -> None:
        """Recompute PSW.P from ACC (even parity: P=1 if odd popcount)."""
        a = self._iram[SFR_ACC]
        a ^= a >> 4; a ^= a >> 2; a ^= a >> 1
        if a & 1:
            self._iram[SFR_PSW] |= PSW_P
        else:
            self._iram[SFR_PSW] &= ~PSW_P & 0xFF

    def _cy(self) -> int:
        """Return carry flag (0 or 1)."""
        return 1 if self._iram[SFR_PSW] & PSW_CY else 0

    def _set_flags(self, cy: int, ac: int, ov: int) -> None:
        """Update CY, AC, OV in PSW.  P is updated separately via _set_acc."""
        psw = self._iram[SFR_PSW]
        psw &= ~(PSW_CY | PSW_AC | PSW_OV) & 0xFF
        if cy: psw |= PSW_CY
        if ac: psw |= PSW_AC
        if ov: psw |= PSW_OV
        self._iram[SFR_PSW] = psw

    # ── Direct / indirect IRAM access ─────────────────────────────────────────

    def _direct_read(self, addr: int) -> int:
        """Read a byte using direct addressing.

        0x00–0x7F → internal lower RAM
        0x80–0xFF → SFR space (same iram array)
        """
        return self._iram[addr & 0xFF]

    def _direct_write(self, addr: int, val: int) -> None:
        """Write a byte using direct addressing."""
        self._iram[addr & 0xFF] = val & 0xFF
        # If ACC was written directly, update parity
        if (addr & 0xFF) == SFR_ACC:
            self._update_parity()

    def _indirect_read(self, ri: int) -> int:
        """Read a byte using register-indirect addressing (@Ri).

        On the 8051 base model, @Ri can only address 0x00–0x7F.
        Values 0x80–0xFF are undefined; we raise ValueError.
        """
        addr = self._iram[self._rn_addr(ri & 1)]
        if addr > 0x7F:
            msg = f"Indirect address 0x{addr:02X} ≥ 0x80 (undefined on 8051)"
            raise ValueError(msg)
        return self._iram[addr]

    def _indirect_write(self, ri: int, val: int) -> None:
        """Write a byte using register-indirect addressing (@Ri)."""
        addr = self._iram[self._rn_addr(ri & 1)]
        if addr > 0x7F:
            msg = f"Indirect address 0x{addr:02X} ≥ 0x80 (undefined on 8051)"
            raise ValueError(msg)
        self._iram[addr] = val & 0xFF

    # ── Bit addressing ─────────────────────────────────────────────────────────

    # Bit-addressable SFRs: only SFRs whose address is a multiple of 8 within
    # the range 0x80–0xFF.  The standard set:
    _BIT_ADDRESSABLE_SFRS = frozenset({
        0x80, 0x88, 0x90, 0x98, 0xA0, 0xA8, 0xB0, 0xB8, 0xC0, 0xC8,
        0xD0, 0xD8, 0xE0, 0xE8, 0xF0, 0xF8,
    })

    def _bit_addr(self, bit: int) -> tuple[int, int]:
        """Resolve a bit address to (iram_byte_addr, bit_position).

        Bit addresses 0x00–0x7F → iram bytes 0x20–0x2F (RAM bit area)
        Bit addresses 0x80–0xFF → SFR bits (byte = bit & 0xF8, pos = bit & 7)

        Returns (byte_addr, bit_pos) where byte_addr is an index into _iram
        and bit_pos is 0–7 (0 = LSB).
        """
        bit &= 0xFF
        if bit < 0x80:
            byte_addr = 0x20 + (bit >> 3)
            bit_pos   = bit & 0x7
        else:
            byte_addr = bit & 0xF8
            bit_pos   = bit & 0x7
        return byte_addr, bit_pos

    def _read_bit(self, bit: int) -> int:
        """Read one bit from the bit-addressable space. Returns 0 or 1."""
        addr, pos = self._bit_addr(bit)
        return (self._iram[addr] >> pos) & 1

    def _write_bit(self, bit: int, val: int) -> None:
        """Write one bit into the bit-addressable space."""
        addr, pos = self._bit_addr(bit)
        if val:
            self._iram[addr] |= (1 << pos)
        else:
            self._iram[addr] &= ~(1 << pos) & 0xFF
        # If ACC bit was changed, update parity
        if addr == SFR_ACC:
            self._update_parity()

    # ── Stack ─────────────────────────────────────────────────────────────────

    def _push8(self, val: int) -> None:
        """Push one byte onto the stack (SP++; iram[SP] = val)."""
        sp = (self._iram[SFR_SP] + 1) & 0xFF
        self._iram[SFR_SP] = sp
        self._iram[sp] = val & 0xFF

    def _pop8(self) -> int:
        """Pop one byte from the stack (val = iram[SP]; SP--)."""
        sp = self._iram[SFR_SP]
        val = self._iram[sp]
        self._iram[SFR_SP] = (sp - 1) & 0xFF
        return val

    def _push_pc(self) -> None:
        """Push 16-bit PC onto stack (low byte first, then high byte)."""
        self._push8(self._pc & 0xFF)
        self._push8((self._pc >> 8) & 0xFF)

    def _pop_pc(self) -> None:
        """Pop 16-bit PC from stack (high byte first, then low byte)."""
        hi = self._pop8()
        lo = self._pop8()
        self._pc = (hi << 8) | lo

    # =========================================================================
    # Instruction execution
    # =========================================================================

    def _execute_one(self) -> str:  # noqa: C901 (complex but table-driven)
        """Decode and execute one instruction.  Returns mnemonic string."""
        opcode = self._fetch8()

        # ── HALT sentinel ────────────────────────────────────────────────────
        if opcode == HALT_OPCODE:  # 0xA5 reserved/undefined
            self._halted = True
            return "HALT"

        # ── NOP ──────────────────────────────────────────────────────────────
        if opcode == 0x00:
            return "NOP"

        # ── Data transfer ─────────────────────────────────────────────────────

        # MOV A, Rn  (0xE8–0xEF)
        if 0xE8 <= opcode <= 0xEF:
            self._set_acc(self._rn(opcode & 7))
            return f"MOV A,R{opcode & 7}"

        # MOV A, dir  (0xE5)
        if opcode == 0xE5:
            d = self._fetch8()
            self._set_acc(self._direct_read(d))
            return "MOV A,dir"

        # MOV A, @Ri  (0xE6–0xE7)
        if opcode in (0xE6, 0xE7):
            self._set_acc(self._indirect_read(opcode & 1))
            return f"MOV A,@R{opcode & 1}"

        # MOV A, #imm  (0x74)
        if opcode == 0x74:
            self._set_acc(self._fetch8())
            return "MOV A,#imm"

        # MOV Rn, A  (0xF8–0xFF)
        if 0xF8 <= opcode <= 0xFF:
            self._set_rn(opcode & 7, self._acc())
            return f"MOV R{opcode & 7},A"

        # MOV Rn, dir  (0xA8–0xAF)
        if 0xA8 <= opcode <= 0xAF:
            d = self._fetch8()
            self._set_rn(opcode & 7, self._direct_read(d))
            return f"MOV R{opcode & 7},dir"

        # MOV Rn, #imm  (0x78–0x7F)
        if 0x78 <= opcode <= 0x7F:
            self._set_rn(opcode & 7, self._fetch8())
            return f"MOV R{opcode & 7},#imm"

        # MOV dir, A  (0xF5)
        if opcode == 0xF5:
            d = self._fetch8()
            self._direct_write(d, self._acc())
            return "MOV dir,A"

        # MOV dir, Rn  (0x88–0x8F)
        if 0x88 <= opcode <= 0x8F:
            d = self._fetch8()
            self._direct_write(d, self._rn(opcode & 7))
            return f"MOV dir,R{opcode & 7}"

        # MOV dir, dir2  (0x85) — note: src=byte2, dst=byte3
        if opcode == 0x85:
            src = self._fetch8()
            dst = self._fetch8()
            self._direct_write(dst, self._direct_read(src))
            return "MOV dir,dir"

        # MOV dir, @Ri  (0x86–0x87)
        if opcode in (0x86, 0x87):
            d = self._fetch8()
            self._direct_write(d, self._indirect_read(opcode & 1))
            return f"MOV dir,@R{opcode & 1}"

        # MOV dir, #imm  (0x75)
        if opcode == 0x75:
            d   = self._fetch8()
            imm = self._fetch8()
            self._direct_write(d, imm)
            return "MOV dir,#imm"

        # MOV @Ri, A  (0xF6–0xF7)
        if opcode in (0xF6, 0xF7):
            self._indirect_write(opcode & 1, self._acc())
            return f"MOV @R{opcode & 1},A"

        # MOV @Ri, dir  (0xA6–0xA7)
        if opcode in (0xA6, 0xA7):
            d = self._fetch8()
            self._indirect_write(opcode & 1, self._direct_read(d))
            return f"MOV @R{opcode & 1},dir"

        # MOV @Ri, #imm  (0x76–0x77)
        if opcode in (0x76, 0x77):
            self._indirect_write(opcode & 1, self._fetch8())
            return f"MOV @R{opcode & 1},#imm"

        # MOV DPTR, #imm16  (0x90)
        if opcode == 0x90:
            self._set_dptr(self._fetch16())
            return "MOV DPTR,#imm16"

        # MOVC A, @A+DPTR  (0x93)
        if opcode == 0x93:
            ea = (self._acc() + self._dptr()) & 0xFFFF
            self._set_acc(self._code[ea])
            return "MOVC A,@A+DPTR"

        # MOVC A, @A+PC  (0x83)
        if opcode == 0x83:
            ea = (self._acc() + self._pc) & 0xFFFF  # _pc already advanced past 0x83
            self._set_acc(self._code[ea])
            return "MOVC A,@A+PC"

        # MOVX A, @Ri  (0xE2–0xE3)
        if opcode in (0xE2, 0xE3):
            addr = self._rn(opcode & 1)
            self._set_acc(self._xdata[addr])
            return f"MOVX A,@R{opcode & 1}"

        # MOVX A, @DPTR  (0xE0)
        if opcode == 0xE0:
            self._set_acc(self._xdata[self._dptr()])
            return "MOVX A,@DPTR"

        # MOVX @Ri, A  (0xF2–0xF3)
        if opcode in (0xF2, 0xF3):
            self._xdata[self._rn(opcode & 1)] = self._acc()
            return f"MOVX @R{opcode & 1},A"

        # MOVX @DPTR, A  (0xF0)
        if opcode == 0xF0:
            self._xdata[self._dptr()] = self._acc()
            return "MOVX @DPTR,A"

        # PUSH dir  (0xC0)
        if opcode == 0xC0:
            d = self._fetch8()
            self._push8(self._direct_read(d))
            return "PUSH"

        # POP dir  (0xD0)
        if opcode == 0xD0:
            d = self._fetch8()
            self._direct_write(d, self._pop8())
            return "POP"

        # XCH A, Rn  (0xC8–0xCF)
        if 0xC8 <= opcode <= 0xCF:
            n  = opcode & 7
            a  = self._acc()
            rn = self._rn(n)
            self._set_acc(rn)
            self._set_rn(n, a)
            return f"XCH A,R{n}"

        # XCH A, dir  (0xC5)
        if opcode == 0xC5:
            d   = self._fetch8()
            a   = self._acc()
            mem = self._direct_read(d)
            self._set_acc(mem)
            self._direct_write(d, a)
            return "XCH A,dir"

        # XCH A, @Ri  (0xC6–0xC7)
        if opcode in (0xC6, 0xC7):
            i   = opcode & 1
            a   = self._acc()
            mem = self._indirect_read(i)
            self._set_acc(mem)
            self._indirect_write(i, a)
            return f"XCH A,@R{i}"

        # XCHD A, @Ri  (0xD6–0xD7)
        if opcode in (0xD6, 0xD7):
            i   = opcode & 1
            a   = self._acc()
            mem = self._indirect_read(i)
            swapped_a   = (a & 0xF0) | (mem & 0x0F)
            swapped_mem = (mem & 0xF0) | (a & 0x0F)
            self._set_acc(swapped_a)
            self._indirect_write(i, swapped_mem)
            return f"XCHD A,@R{i}"

        # ── Arithmetic ────────────────────────────────────────────────────────

        # ADD A, Rn  (0x28–0x2F)
        if 0x28 <= opcode <= 0x2F:
            r, cy, ac, ov, p = add8_flags(self._acc(), self._rn(opcode & 7))
            self._iram[SFR_ACC] = r
            self._set_flags(cy, ac, ov)
            self._update_parity()
            return f"ADD A,R{opcode & 7}"

        # ADD A, dir  (0x25)
        if opcode == 0x25:
            d = self._fetch8()
            r, cy, ac, ov, p = add8_flags(self._acc(), self._direct_read(d))
            self._iram[SFR_ACC] = r
            self._set_flags(cy, ac, ov)
            self._update_parity()
            return "ADD A,dir"

        # ADD A, @Ri  (0x26–0x27)
        if opcode in (0x26, 0x27):
            r, cy, ac, ov, p = add8_flags(self._acc(), self._indirect_read(opcode & 1))
            self._iram[SFR_ACC] = r
            self._set_flags(cy, ac, ov)
            self._update_parity()
            return f"ADD A,@R{opcode & 1}"

        # ADD A, #imm  (0x24)
        if opcode == 0x24:
            r, cy, ac, ov, p = add8_flags(self._acc(), self._fetch8())
            self._iram[SFR_ACC] = r
            self._set_flags(cy, ac, ov)
            self._update_parity()
            return "ADD A,#imm"

        # ADDC A, Rn  (0x38–0x3F)
        if 0x38 <= opcode <= 0x3F:
            r, cy, ac, ov, p = add8_flags(self._acc(), self._rn(opcode & 7), self._cy())
            self._iram[SFR_ACC] = r
            self._set_flags(cy, ac, ov)
            self._update_parity()
            return f"ADDC A,R{opcode & 7}"

        # ADDC A, dir  (0x35)
        if opcode == 0x35:
            d = self._fetch8()
            r, cy, ac, ov, p = add8_flags(self._acc(), self._direct_read(d), self._cy())
            self._iram[SFR_ACC] = r
            self._set_flags(cy, ac, ov)
            self._update_parity()
            return "ADDC A,dir"

        # ADDC A, @Ri  (0x36–0x37)
        if opcode in (0x36, 0x37):
            r, cy, ac, ov, p = add8_flags(self._acc(), self._indirect_read(opcode & 1), self._cy())
            self._iram[SFR_ACC] = r
            self._set_flags(cy, ac, ov)
            self._update_parity()
            return f"ADDC A,@R{opcode & 1}"

        # ADDC A, #imm  (0x34)
        if opcode == 0x34:
            r, cy, ac, ov, p = add8_flags(self._acc(), self._fetch8(), self._cy())
            self._iram[SFR_ACC] = r
            self._set_flags(cy, ac, ov)
            self._update_parity()
            return "ADDC A,#imm"

        # SUBB A, Rn  (0x98–0x9F)
        if 0x98 <= opcode <= 0x9F:
            r, cy, ac, ov, p = sub8_flags(self._acc(), self._rn(opcode & 7), self._cy())
            self._iram[SFR_ACC] = r
            self._set_flags(cy, ac, ov)
            self._update_parity()
            return f"SUBB A,R{opcode & 7}"

        # SUBB A, dir  (0x95)
        if opcode == 0x95:
            d = self._fetch8()
            r, cy, ac, ov, p = sub8_flags(self._acc(), self._direct_read(d), self._cy())
            self._iram[SFR_ACC] = r
            self._set_flags(cy, ac, ov)
            self._update_parity()
            return "SUBB A,dir"

        # SUBB A, @Ri  (0x96–0x97)
        if opcode in (0x96, 0x97):
            r, cy, ac, ov, p = sub8_flags(self._acc(), self._indirect_read(opcode & 1), self._cy())
            self._iram[SFR_ACC] = r
            self._set_flags(cy, ac, ov)
            self._update_parity()
            return f"SUBB A,@R{opcode & 1}"

        # SUBB A, #imm  (0x94)
        if opcode == 0x94:
            r, cy, ac, ov, p = sub8_flags(self._acc(), self._fetch8(), self._cy())
            self._iram[SFR_ACC] = r
            self._set_flags(cy, ac, ov)
            self._update_parity()
            return "SUBB A,#imm"

        # INC A  (0x04)
        if opcode == 0x04:
            self._iram[SFR_ACC] = (self._acc() + 1) & 0xFF
            self._update_parity()
            return "INC A"

        # INC Rn  (0x08–0x0F)
        if 0x08 <= opcode <= 0x0F:
            n = opcode & 7
            self._set_rn(n, (self._rn(n) + 1) & 0xFF)
            return f"INC R{n}"

        # INC dir  (0x05)
        if opcode == 0x05:
            d = self._fetch8()
            self._direct_write(d, (self._direct_read(d) + 1) & 0xFF)
            return "INC dir"

        # INC @Ri  (0x06–0x07)
        if opcode in (0x06, 0x07):
            i = opcode & 1
            self._indirect_write(i, (self._indirect_read(i) + 1) & 0xFF)
            return f"INC @R{i}"

        # INC DPTR  (0xA3)
        if opcode == 0xA3:
            self._set_dptr((self._dptr() + 1) & 0xFFFF)
            return "INC DPTR"

        # DEC A  (0x14)
        if opcode == 0x14:
            self._iram[SFR_ACC] = (self._acc() - 1) & 0xFF
            self._update_parity()
            return "DEC A"

        # DEC Rn  (0x18–0x1F)
        if 0x18 <= opcode <= 0x1F:
            n = opcode & 7
            self._set_rn(n, (self._rn(n) - 1) & 0xFF)
            return f"DEC R{n}"

        # DEC dir  (0x15)
        if opcode == 0x15:
            d = self._fetch8()
            self._direct_write(d, (self._direct_read(d) - 1) & 0xFF)
            return "DEC dir"

        # DEC @Ri  (0x16–0x17)
        if opcode in (0x16, 0x17):
            i = opcode & 1
            self._indirect_write(i, (self._indirect_read(i) - 1) & 0xFF)
            return f"DEC @R{i}"

        # MUL AB  (0xA4)  — B:A = A × B (unsigned 8×8→16)
        if opcode == 0xA4:
            product = self._acc() * self._iram[SFR_B]
            self._iram[SFR_ACC] = product & 0xFF
            self._iram[SFR_B]   = (product >> 8) & 0xFF
            # CY = 0 always; OV = 1 if product > 0xFF (i.e. B ≠ 0 after)
            ov = 1 if (product >> 8) != 0 else 0
            self._set_flags(0, 0, ov)
            self._update_parity()
            return "MUL AB"

        # DIV AB  (0x84)  — A = quotient, B = remainder; OV if B=0 before
        if opcode == 0x84:
            divisor = self._iram[SFR_B]
            if divisor == 0:
                self._set_flags(0, 0, 1)  # OV = 1, CY = 0
                # ACC and B are undefined; leave unchanged per some implementations
            else:
                q = self._acc() // divisor
                r = self._acc() % divisor
                self._iram[SFR_ACC] = q & 0xFF
                self._iram[SFR_B]   = r & 0xFF
                self._set_flags(0, 0, 0)
            self._update_parity()
            return "DIV AB"

        # DA A  (0xD4)  — decimal adjust after BCD addition
        if opcode == 0xD4:
            cy_in = self._cy()
            ac_in = 1 if self._iram[SFR_PSW] & PSW_AC else 0
            result, new_cy, new_p = da_flags(self._acc(), cy_in, ac_in)
            self._iram[SFR_ACC] = result
            psw = self._iram[SFR_PSW]
            if new_cy:
                psw |= PSW_CY
            else:
                psw &= ~PSW_CY & 0xFF
            if new_p:
                psw |= PSW_P
            else:
                psw &= ~PSW_P & 0xFF
            self._iram[SFR_PSW] = psw
            return "DA A"

        # ── Logic ─────────────────────────────────────────────────────────────

        # ANL A, Rn  (0x58–0x5F)
        if 0x58 <= opcode <= 0x5F:
            self._set_acc(self._acc() & self._rn(opcode & 7))
            return f"ANL A,R{opcode & 7}"

        # ANL A, dir  (0x55)
        if opcode == 0x55:
            d = self._fetch8()
            self._set_acc(self._acc() & self._direct_read(d))
            return "ANL A,dir"

        # ANL A, @Ri  (0x56–0x57)
        if opcode in (0x56, 0x57):
            self._set_acc(self._acc() & self._indirect_read(opcode & 1))
            return f"ANL A,@R{opcode & 1}"

        # ANL A, #imm  (0x54)
        if opcode == 0x54:
            self._set_acc(self._acc() & self._fetch8())
            return "ANL A,#imm"

        # ANL dir, A  (0x52)
        if opcode == 0x52:
            d = self._fetch8()
            self._direct_write(d, self._direct_read(d) & self._acc())
            return "ANL dir,A"

        # ANL dir, #imm  (0x53)
        if opcode == 0x53:
            d   = self._fetch8()
            imm = self._fetch8()
            self._direct_write(d, self._direct_read(d) & imm)
            return "ANL dir,#imm"

        # ORL A, Rn  (0x48–0x4F)
        if 0x48 <= opcode <= 0x4F:
            self._set_acc(self._acc() | self._rn(opcode & 7))
            return f"ORL A,R{opcode & 7}"

        # ORL A, dir  (0x45)
        if opcode == 0x45:
            d = self._fetch8()
            self._set_acc(self._acc() | self._direct_read(d))
            return "ORL A,dir"

        # ORL A, @Ri  (0x46–0x47)
        if opcode in (0x46, 0x47):
            self._set_acc(self._acc() | self._indirect_read(opcode & 1))
            return f"ORL A,@R{opcode & 1}"

        # ORL A, #imm  (0x44)
        if opcode == 0x44:
            self._set_acc(self._acc() | self._fetch8())
            return "ORL A,#imm"

        # ORL dir, A  (0x42)
        if opcode == 0x42:
            d = self._fetch8()
            self._direct_write(d, self._direct_read(d) | self._acc())
            return "ORL dir,A"

        # ORL dir, #imm  (0x43)
        if opcode == 0x43:
            d   = self._fetch8()
            imm = self._fetch8()
            self._direct_write(d, self._direct_read(d) | imm)
            return "ORL dir,#imm"

        # XRL A, Rn  (0x68–0x6F)
        if 0x68 <= opcode <= 0x6F:
            self._set_acc(self._acc() ^ self._rn(opcode & 7))
            return f"XRL A,R{opcode & 7}"

        # XRL A, dir  (0x65)
        if opcode == 0x65:
            d = self._fetch8()
            self._set_acc(self._acc() ^ self._direct_read(d))
            return "XRL A,dir"

        # XRL A, @Ri  (0x66–0x67)
        if opcode in (0x66, 0x67):
            self._set_acc(self._acc() ^ self._indirect_read(opcode & 1))
            return f"XRL A,@R{opcode & 1}"

        # XRL A, #imm  (0x64)
        if opcode == 0x64:
            self._set_acc(self._acc() ^ self._fetch8())
            return "XRL A,#imm"

        # XRL dir, A  (0x62)
        if opcode == 0x62:
            d = self._fetch8()
            self._direct_write(d, self._direct_read(d) ^ self._acc())
            return "XRL dir,A"

        # XRL dir, #imm  (0x63)
        if opcode == 0x63:
            d   = self._fetch8()
            imm = self._fetch8()
            self._direct_write(d, self._direct_read(d) ^ imm)
            return "XRL dir,#imm"

        # CLR A  (0xE4)
        if opcode == 0xE4:
            self._set_acc(0)
            return "CLR A"

        # CPL A  (0xF4)
        if opcode == 0xF4:
            self._set_acc(~self._acc() & 0xFF)
            return "CPL A"

        # RL A  (0x23) — rotate left, no carry
        if opcode == 0x23:
            a = self._acc()
            self._set_acc(((a << 1) | (a >> 7)) & 0xFF)
            return "RL A"

        # RLC A  (0x33) — rotate left through carry
        if opcode == 0x33:
            a   = self._acc()
            new_cy = a >> 7
            self._set_acc(((a << 1) | self._cy()) & 0xFF)
            if new_cy:
                self._iram[SFR_PSW] |= PSW_CY
            else:
                self._iram[SFR_PSW] &= ~PSW_CY & 0xFF
            return "RLC A"

        # RR A  (0x03) — rotate right, no carry
        if opcode == 0x03:
            a = self._acc()
            self._set_acc(((a >> 1) | (a << 7)) & 0xFF)
            return "RR A"

        # RRC A  (0x13) — rotate right through carry
        if opcode == 0x13:
            a   = self._acc()
            new_cy = a & 1
            self._set_acc(((a >> 1) | (self._cy() << 7)) & 0xFF)
            if new_cy:
                self._iram[SFR_PSW] |= PSW_CY
            else:
                self._iram[SFR_PSW] &= ~PSW_CY & 0xFF
            return "RRC A"

        # SWAP A  (0xC4) — swap nibbles, no flag changes
        if opcode == 0xC4:
            a = self._acc()
            self._iram[SFR_ACC] = ((a << 4) | (a >> 4)) & 0xFF
            # SWAP does NOT update parity (no logical change, just nibble reorder)
            return "SWAP A"

        # ── Bit operations ────────────────────────────────────────────────────

        # CLR C  (0xC3)
        if opcode == 0xC3:
            self._iram[SFR_PSW] &= ~PSW_CY & 0xFF
            return "CLR C"

        # CLR bit  (0xC2)
        if opcode == 0xC2:
            self._write_bit(self._fetch8(), 0)
            return "CLR bit"

        # SETB C  (0xD3)
        if opcode == 0xD3:
            self._iram[SFR_PSW] |= PSW_CY
            return "SETB C"

        # SETB bit  (0xD2)
        if opcode == 0xD2:
            self._write_bit(self._fetch8(), 1)
            return "SETB bit"

        # CPL C  (0xB3)
        if opcode == 0xB3:
            self._iram[SFR_PSW] ^= PSW_CY
            return "CPL C"

        # CPL bit  (0xB2)
        if opcode == 0xB2:
            bit = self._fetch8()
            self._write_bit(bit, 1 - self._read_bit(bit))
            return "CPL bit"

        # ANL C, bit  (0x82)
        if opcode == 0x82:
            bit = self._fetch8()
            if not self._read_bit(bit):
                self._iram[SFR_PSW] &= ~PSW_CY & 0xFF
            return "ANL C,bit"

        # ANL C, /bit  (0xB0)
        if opcode == 0xB0:
            bit = self._fetch8()
            if self._read_bit(bit):    # /bit = complement
                self._iram[SFR_PSW] &= ~PSW_CY & 0xFF
            return "ANL C,/bit"

        # ORL C, bit  (0x72)
        if opcode == 0x72:
            bit = self._fetch8()
            if self._read_bit(bit):
                self._iram[SFR_PSW] |= PSW_CY
            return "ORL C,bit"

        # ORL C, /bit  (0xA0)
        if opcode == 0xA0:
            bit = self._fetch8()
            if not self._read_bit(bit):
                self._iram[SFR_PSW] |= PSW_CY
            return "ORL C,/bit"

        # MOV C, bit  (0xA2)
        if opcode == 0xA2:
            bit = self._fetch8()
            if self._read_bit(bit):
                self._iram[SFR_PSW] |= PSW_CY
            else:
                self._iram[SFR_PSW] &= ~PSW_CY & 0xFF
            return "MOV C,bit"

        # MOV bit, C  (0x92)
        if opcode == 0x92:
            self._write_bit(self._fetch8(), self._cy())
            return "MOV bit,C"

        # ── Jumps ─────────────────────────────────────────────────────────────

        # LJMP addr16  (0x02)
        if opcode == 0x02:
            self._pc = self._fetch16()
            return "LJMP"

        # SJMP rel  (0x80)
        if opcode == 0x80:
            rel = self._fetch8()
            if rel >= 0x80: rel -= 0x100   # sign-extend
            self._pc = (self._pc + rel) & 0xFFFF
            return "SJMP"

        # JMP @A+DPTR  (0x73)
        if opcode == 0x73:
            self._pc = (self._acc() + self._dptr()) & 0xFFFF
            return "JMP @A+DPTR"

        # AJMP  — opcode pattern: a10:a9:a8:0:0:0:0:1  (bits 7:5 = addr[10:8], bits 4:0 = 00001)
        if (opcode & 0x1F) == 0x01:
            addr11_hi = (opcode >> 5) & 0x7
            addr11_lo = self._fetch8()
            self._pc = (self._pc & 0xF800) | (addr11_hi << 8) | addr11_lo
            return "AJMP"

        # JZ rel  (0x60)
        if opcode == 0x60:
            rel = self._fetch8()
            if rel >= 0x80: rel -= 0x100
            if self._acc() == 0:
                self._pc = (self._pc + rel) & 0xFFFF
            return "JZ"

        # JNZ rel  (0x70)
        if opcode == 0x70:
            rel = self._fetch8()
            if rel >= 0x80: rel -= 0x100
            if self._acc() != 0:
                self._pc = (self._pc + rel) & 0xFFFF
            return "JNZ"

        # JC rel  (0x40)
        if opcode == 0x40:
            rel = self._fetch8()
            if rel >= 0x80: rel -= 0x100
            if self._cy():
                self._pc = (self._pc + rel) & 0xFFFF
            return "JC"

        # JNC rel  (0x50)
        if opcode == 0x50:
            rel = self._fetch8()
            if rel >= 0x80: rel -= 0x100
            if not self._cy():
                self._pc = (self._pc + rel) & 0xFFFF
            return "JNC"

        # JB bit, rel  (0x20)
        if opcode == 0x20:
            bit = self._fetch8()
            rel = self._fetch8()
            if rel >= 0x80: rel -= 0x100
            if self._read_bit(bit):
                self._pc = (self._pc + rel) & 0xFFFF
            return "JB"

        # JNB bit, rel  (0x30)
        if opcode == 0x30:
            bit = self._fetch8()
            rel = self._fetch8()
            if rel >= 0x80: rel -= 0x100
            if not self._read_bit(bit):
                self._pc = (self._pc + rel) & 0xFFFF
            return "JNB"

        # JBC bit, rel  (0x10)
        if opcode == 0x10:
            bit = self._fetch8()
            rel = self._fetch8()
            if rel >= 0x80: rel -= 0x100
            if self._read_bit(bit):
                self._write_bit(bit, 0)
                self._pc = (self._pc + rel) & 0xFFFF
            return "JBC"

        # CJNE A, dir, rel  (0xB5)
        if opcode == 0xB5:
            d   = self._fetch8()
            rel = self._fetch8()
            if rel >= 0x80: rel -= 0x100
            val = self._direct_read(d)
            if self._acc() < val:
                self._iram[SFR_PSW] |= PSW_CY
            else:
                self._iram[SFR_PSW] &= ~PSW_CY & 0xFF
            if self._acc() != val:
                self._pc = (self._pc + rel) & 0xFFFF
            return "CJNE A,dir"

        # CJNE A, #imm, rel  (0xB4)
        if opcode == 0xB4:
            imm = self._fetch8()
            rel = self._fetch8()
            if rel >= 0x80: rel -= 0x100
            if self._acc() < imm:
                self._iram[SFR_PSW] |= PSW_CY
            else:
                self._iram[SFR_PSW] &= ~PSW_CY & 0xFF
            if self._acc() != imm:
                self._pc = (self._pc + rel) & 0xFFFF
            return "CJNE A,#imm"

        # CJNE Rn, #imm, rel  (0xB8–0xBF)
        if 0xB8 <= opcode <= 0xBF:
            n   = opcode & 7
            imm = self._fetch8()
            rel = self._fetch8()
            if rel >= 0x80: rel -= 0x100
            rn = self._rn(n)
            if rn < imm:
                self._iram[SFR_PSW] |= PSW_CY
            else:
                self._iram[SFR_PSW] &= ~PSW_CY & 0xFF
            if rn != imm:
                self._pc = (self._pc + rel) & 0xFFFF
            return f"CJNE R{n},#imm"

        # CJNE @Ri, #imm, rel  (0xB6–0xB7)
        if opcode in (0xB6, 0xB7):
            i   = opcode & 1
            imm = self._fetch8()
            rel = self._fetch8()
            if rel >= 0x80: rel -= 0x100
            mem = self._indirect_read(i)
            if mem < imm:
                self._iram[SFR_PSW] |= PSW_CY
            else:
                self._iram[SFR_PSW] &= ~PSW_CY & 0xFF
            if mem != imm:
                self._pc = (self._pc + rel) & 0xFFFF
            return f"CJNE @R{i},#imm"

        # DJNZ Rn, rel  (0xD8–0xDF)
        if 0xD8 <= opcode <= 0xDF:
            n   = opcode & 7
            rel = self._fetch8()
            if rel >= 0x80: rel -= 0x100
            val = (self._rn(n) - 1) & 0xFF
            self._set_rn(n, val)
            if val != 0:
                self._pc = (self._pc + rel) & 0xFFFF
            return f"DJNZ R{n}"

        # DJNZ dir, rel  (0xD5)
        if opcode == 0xD5:
            d   = self._fetch8()
            rel = self._fetch8()
            if rel >= 0x80: rel -= 0x100
            val = (self._direct_read(d) - 1) & 0xFF
            self._direct_write(d, val)
            if val != 0:
                self._pc = (self._pc + rel) & 0xFFFF
            return "DJNZ dir"

        # ── Subroutines ───────────────────────────────────────────────────────

        # LCALL addr16  (0x12)
        if opcode == 0x12:
            addr = self._fetch16()
            self._push_pc()
            self._pc = addr
            return "LCALL"

        # ACALL  — pattern: a10:a9:a8:1:0:0:1:0  (bits 7:5 = addr[10:8], bits 4:0 = 10010)
        if (opcode & 0x1F) == 0x11:
            addr11_hi = (opcode >> 5) & 0x7
            addr11_lo = self._fetch8()
            self._push_pc()
            self._pc = (self._pc & 0xF800) | (addr11_hi << 8) | addr11_lo
            return "ACALL"

        # RET  (0x22)
        if opcode == 0x22:
            self._pop_pc()
            return "RET"

        # RETI  (0x32) — same as RET for behavioral sim (no interrupt controller)
        if opcode == 0x32:
            self._pop_pc()
            return "RETI"

        raise ValueError(f"Unknown opcode: 0x{opcode:02X} at PC=0x{(self._pc - 1) & 0xFFFF:04X}")
