"""simulator.py — Intel 8051 gate-level simulator.

Implements the SIM00 Simulator[I8051State] protocol.  Every data-path
operation routes through gate primitives from the ALU module, which in
turn calls logic_gates and arithmetic.

=============================================================================
Architecture overview
=============================================================================

The 8051 uses a Harvard architecture with three distinct address spaces:

  _code  [64 KB] — program instructions, read-only at runtime
  _iram  [256 B] — internal RAM + SFRs, the unified byte-addressable space
  _xdata [64 KB] — external data memory, accessed only via MOVX

The Program Counter (PC) is a 16-bit register that points into code memory.
It is NOT memory-mapped — it lives in the register file's dedicated flip-flops.

=============================================================================
Gate-level guarantee
=============================================================================

All data-flow through the ALU uses gate primitives:
  - ADD, ADDC, SUBB → alu.add8(), alu.subb8()
  - ANL, ORL, XRL   → alu.anl8(), alu.orl8(), alu.xrl8()
  - INC, DEC        → alu.inc8(), alu.dec8()
  - Rotates         → alu.rl8(), alu.rr8(), alu.rlc8(), alu.rrc8()
  - MUL, DIV        → alu.mul8(), alu.div8()
  - DA A            → alu.da8()
  - PC increment    → register_file.increment_pc() → bits.add_16bit()

Addresses, memory indexing, and bit-field extraction from opcodes are
treated as "wiring" (index math), not arithmetic — they do not route
through the ALU.
"""

from __future__ import annotations

from intel8051_simulator.state import (
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
from simulator_protocol import ExecutionResult, Simulator, StepTrace

from .alu import (
    ALUResult8051,
    add8,
    anl8,
    da8,
    dec8,
    div8,
    inc8,
    mul8,
    orl8,
    rl8,
    rlc8,
    rr8,
    rrc8,
    subb8,
    swap8,
    xrl8,
)
from .bits import add_16bit, compute_parity, int_to_bits
from .register_file import RegisterFile8051


class Intel8051GateLevelSimulator(Simulator[I8051State]):
    """Gate-level Intel 8051 simulator.

    Identical external behavior to I8051Simulator (behavioral) but routes
    all ALU operations through gate primitives from the alu module.

    Public API (SIM00 protocol):
        reset()     — power-on state
        load(prog)  — reset and copy code to memory at origin
        step()      — execute one instruction, return StepTrace
        execute()   — run until HALT or max_steps
        get_state() — return frozen I8051State snapshot

    Extensions:
        set_input_port(port, value)  — write to port latch (P0-P3)
        get_output_port(port)        — read port latch (P0-P3)
        interrupt()                  — trigger external interrupt (no-op here)
        nmi()                        — trigger NMI (no-op here)
    """

    _BLOCK_MAX = 65536  # safety limit for block operations

    def __init__(self) -> None:
        self._rf: RegisterFile8051 = RegisterFile8051()
        self._code: bytearray = bytearray(CODE_SIZE)
        self._xdata: bytearray = bytearray(XDATA_SIZE)
        self._halted: bool = False
        self.reset()

    # ── Protocol: reset ───────────────────────────────────────────────────────

    def reset(self) -> None:
        """Return the CPU to power-on state.

        Per the 8051 datasheet:
          PC = 0x0000
          SP = 0x07 (SFR 0x81)
          P0-P3 = 0xFF (port latches pulled high)
          All other SFRs = 0x00
          IRAM and code/xdata memory: preserved (only changed by load())
        """
        # Reset IRAM — all bits clear
        for addr in range(IRAM_SIZE):
            self._rf.write_iram8(addr, 0)
        # PC = 0
        self._rf.write_pc(0)
        self._halted = False
        # SP reset value = 0x07
        self._rf.write_iram8(SFR_SP, 0x07)
        # Port latches = 0xFF at reset (open-drain, pulled high by pull-ups)
        for port_sfr in (SFR_P0, SFR_P1, SFR_P2, SFR_P3):
            self._rf.write_iram8(port_sfr, 0xFF)

    # ── Protocol: load ────────────────────────────────────────────────────────

    def load(self, program: bytes, origin: int = 0) -> None:
        """Reset and load program bytes into code memory starting at origin.

        Args:
            program: Raw machine code bytes.
            origin:  Starting address in code memory (default 0x0000).

        Raises:
            ValueError: if program would overflow code memory.
        """
        if origin + len(program) > CODE_SIZE:
            msg = f"Program exceeds code memory: origin=0x{origin:04X} + {len(program)} bytes"
            raise ValueError(msg)
        self.reset()
        for i, byte in enumerate(program):
            self._code[origin + i] = byte

    # ── Protocol: get_state ───────────────────────────────────────────────────

    def get_state(self) -> I8051State:
        """Return a frozen snapshot of the current CPU state."""
        return I8051State(
            pc=self._rf.read_pc(),
            iram=tuple(self._rf.dump_iram()),
            xdata=tuple(self._xdata),
            code=tuple(self._code),
            halted=self._halted,
        )

    # ── Protocol: step ────────────────────────────────────────────────────────

    def step(self) -> StepTrace:
        """Execute one instruction and return a StepTrace.

        If the CPU is halted, returns a no-op trace without advancing PC.
        """
        pc_before = self._rf.read_pc()
        if self._halted:
            return StepTrace(
                pc_before=pc_before,
                pc_after=pc_before,
                mnemonic="HALT",
                description="HALT (already halted)",
            )
        mnemonic = self._execute_one()
        return StepTrace(
            pc_before=pc_before,
            pc_after=self._rf.read_pc(),
            mnemonic=mnemonic,
            description=f"{mnemonic} @ 0x{pc_before:04X}",
        )

    # ── Protocol: execute ─────────────────────────────────────────────────────

    def execute(self, program: bytes, origin: int = 0, max_steps: int = 100_000) -> ExecutionResult:
        """Load and run program until HALT or max_steps exceeded."""
        self.load(program, origin)
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
            halted=self._halted,
            steps=steps,
            traces=traces,
            final_state=self.get_state(),
            error=error,
        )

    # ── Extensions ────────────────────────────────────────────────────────────

    def set_input_port(self, port: int, value: int) -> None:
        """Write to a port latch (P0=0, P1=1, P2=2, P3=3).

        On real hardware, writing to a port SFR drives the output pins.
        Here we simply update the IRAM location for the port SFR.
        """
        port_sfrs = {0: SFR_P0, 1: SFR_P1, 2: SFR_P2, 3: SFR_P3}
        if port in port_sfrs:
            self._rf.write_iram8(port_sfrs[port], value & 0xFF)

    def get_output_port(self, port: int) -> int:
        """Read the current port latch value (P0=0, P1=1, P2=2, P3=3)."""
        port_sfrs = {0: SFR_P0, 1: SFR_P1, 2: SFR_P2, 3: SFR_P3}
        if port in port_sfrs:
            return self._rf.read_iram8(port_sfrs[port])
        return 0

    def interrupt(self) -> None:
        """Trigger external interrupt (behavioral stub — not implemented)."""

    def nmi(self) -> None:
        """Trigger NMI (behavioral stub — not implemented)."""

    # =========================================================================
    # Internal helpers — fetch, register access, flags
    # =========================================================================

    def _fetch8(self) -> int:
        """Fetch one byte from code memory at PC, then increment PC."""
        pc = self._rf.read_pc()
        byte = self._code[pc & 0xFFFF]
        self._rf.increment_pc(1)  # gate-level 16-bit add
        return byte

    def _fetch16(self) -> int:
        """Fetch two bytes (big-endian) from code memory, advance PC by 2."""
        hi = self._fetch8()
        lo = self._fetch8()
        return (hi << 8) | lo

    def _rn_addr(self, n: int) -> int:
        """Return IRAM address of Rn in the current register bank.

        Bank selection: PSW bits 4:3 (RS1:RS0) → bank = 0-3
        Each bank occupies 8 consecutive bytes starting at bank*8.
        """
        psw = self._rf.read_iram8(SFR_PSW)
        bank = (psw >> 3) & 0x3
        return bank * 8 + (n & 0x7)

    def _rn(self, n: int) -> int:
        """Read Rn from the current register bank."""
        return self._rf.read_iram8(self._rn_addr(n))

    def _set_rn(self, n: int, val: int) -> None:
        """Write Rn in the current register bank."""
        self._rf.write_iram8(self._rn_addr(n), val & 0xFF)

    def _acc(self) -> int:
        """Read the accumulator."""
        return self._rf.read_iram8(SFR_ACC)

    def _set_acc(self, val: int) -> None:
        """Write the accumulator and update PSW.P (parity bit).

        PSW.P is the even parity of ACC: P=1 when ACC has an odd number of
        set bits (so that ACC bits + P = even parity count).
        """
        self._rf.write_iram8(SFR_ACC, val & 0xFF)
        self._update_parity()

    def _update_parity(self) -> None:
        """Recompute PSW.P from ACC using the gate-level parity function."""
        acc_val = self._rf.read_iram8(SFR_ACC)
        acc_bits = int_to_bits(acc_val, 8)
        p = compute_parity(acc_bits)
        psw = self._rf.read_iram8(SFR_PSW)
        # Gate-level write: set or clear bit 0 of PSW based on parity
        # We use direct bit manipulation for the SFR update (this is "wiring")
        if p:
            self._rf.write_iram8(SFR_PSW, psw | PSW_P)
        else:
            self._rf.write_iram8(SFR_PSW, psw & (0xFF ^ PSW_P))

    def _cy(self) -> int:
        """Return current carry flag (0 or 1)."""
        psw = self._rf.read_iram8(SFR_PSW)
        return (psw >> 7) & 1

    def _apply_alu_result(self, res: ALUResult8051) -> None:
        """Apply ALU result to ACC and update PSW flags (CY, AC, OV, P)."""
        self._rf.write_iram8(SFR_ACC, res.result)
        psw = self._rf.read_iram8(SFR_PSW)
        # Update CY, AC, OV atomically — standard 8051 flag update path
        psw &= 0xFF ^ (PSW_CY | PSW_AC | PSW_OV | PSW_P)
        if res.cy:
            psw |= PSW_CY
        if res.ac:
            psw |= PSW_AC
        if res.ov:
            psw |= PSW_OV
        if res.parity:
            psw |= PSW_P
        self._rf.write_iram8(SFR_PSW, psw)

    def _set_flags_cy_ac_ov(self, cy: int, ac: int, ov: int) -> None:
        """Update CY, AC, OV in PSW without changing ACC or parity."""
        psw = self._rf.read_iram8(SFR_PSW)
        psw &= 0xFF ^ (PSW_CY | PSW_AC | PSW_OV)
        if cy:
            psw |= PSW_CY
        if ac:
            psw |= PSW_AC
        if ov:
            psw |= PSW_OV
        self._rf.write_iram8(SFR_PSW, psw)

    def _set_cy(self, cy: int) -> None:
        """Update only CY in PSW."""
        psw = self._rf.read_iram8(SFR_PSW)
        if cy:
            self._rf.write_iram8(SFR_PSW, psw | PSW_CY)
        else:
            self._rf.write_iram8(SFR_PSW, psw & (0xFF ^ PSW_CY))

    def _dptr(self) -> int:
        """Read 16-bit DPTR = DPH:DPL."""
        dph = self._rf.read_iram8(SFR_DPH)
        dpl = self._rf.read_iram8(SFR_DPL)
        return (dph << 8) | dpl

    def _set_dptr(self, val: int) -> None:
        """Write 16-bit DPTR."""
        self._rf.write_iram8(SFR_DPH, (val >> 8) & 0xFF)
        self._rf.write_iram8(SFR_DPL, val & 0xFF)

    # ── Direct / indirect IRAM access ─────────────────────────────────────────

    def _direct_read(self, addr: int) -> int:
        """Read using direct addressing (0x00-0xFF maps to IRAM)."""
        return self._rf.read_iram8(addr & 0xFF)

    def _direct_write(self, addr: int, val: int) -> None:
        """Write using direct addressing."""
        self._rf.write_iram8(addr & 0xFF, val & 0xFF)
        # If ACC was written directly, recompute parity
        if (addr & 0xFF) == SFR_ACC:
            self._update_parity()

    def _indirect_read(self, ri: int) -> int:
        """Read using register-indirect addressing (@Ri).

        On the base 8051, indirect addressing can only reach IRAM[0x00-0x7F].
        Addresses 0x80+ via indirect are undefined (we raise ValueError).
        """
        addr = self._rf.read_iram8(self._rn_addr(ri & 1))
        if addr > 0x7F:
            msg = f"Indirect address 0x{addr:02X} >= 0x80 (undefined on 8051)"
            raise ValueError(msg)
        return self._rf.read_iram8(addr)

    def _indirect_write(self, ri: int, val: int) -> None:
        """Write using register-indirect addressing (@Ri)."""
        addr = self._rf.read_iram8(self._rn_addr(ri & 1))
        if addr > 0x7F:
            msg = f"Indirect address 0x{addr:02X} >= 0x80 (undefined on 8051)"
            raise ValueError(msg)
        self._rf.write_iram8(addr, val & 0xFF)

    # ── Bit addressing ─────────────────────────────────────────────────────────

    def _read_bit(self, bit_addr: int) -> int:
        """Read one bit from the bit-addressable space (returns 0 or 1)."""
        return self._rf.read_bit(bit_addr)

    def _write_bit(self, bit_addr: int, val: int) -> None:
        """Write one bit to the bit-addressable space."""
        self._rf.write_bit(bit_addr, val & 1)
        # If ACC bit was changed, recompute parity
        if (bit_addr & 0xF8) == SFR_ACC:
            self._update_parity()

    # ── Stack ─────────────────────────────────────────────────────────────────

    def _push8(self, val: int) -> None:
        """Push one byte onto the stack: SP++; iram[SP] = val."""
        sp = self._rf.read_iram8(SFR_SP)
        # SP increment via gate-level add8
        sp_res = inc8(sp)
        new_sp = sp_res.result
        self._rf.write_iram8(SFR_SP, new_sp)
        self._rf.write_iram8(new_sp, val & 0xFF)

    def _pop8(self) -> int:
        """Pop one byte from the stack: val = iram[SP]; SP--."""
        sp = self._rf.read_iram8(SFR_SP)
        val = self._rf.read_iram8(sp)
        # SP decrement via gate-level dec8
        sp_res = dec8(sp)
        self._rf.write_iram8(SFR_SP, sp_res.result)
        return val

    def _push_pc(self) -> None:
        """Push 16-bit PC onto stack (low byte first, then high byte)."""
        pc = self._rf.read_pc()
        self._push8(pc & 0xFF)
        self._push8((pc >> 8) & 0xFF)

    def _pop_pc(self) -> None:
        """Pop 16-bit PC from stack (high byte first, then low byte)."""
        hi = self._pop8()
        lo = self._pop8()
        self._rf.write_pc((hi << 8) | lo)

    # ── Signed relative offset ─────────────────────────────────────────────────

    def _sign_extend_rel8(self, rel: int) -> int:
        """Sign-extend an 8-bit value to a signed Python int.

        On the 8051, relative branches use a signed 8-bit offset:
          0x00-0x7F → +0 to +127 (forward jump)
          0x80-0xFF → -128 to -1 (backward jump)

        The 8051 hardware sign-extends by testing bit 7 and filling higher
        bits with copies of that bit (arithmetic right shift).
        """
        if rel >= 0x80:
            return rel - 0x100
        return rel

    # =========================================================================
    # Instruction execution dispatch
    # =========================================================================

    def _execute_one(self) -> str:  # noqa: C901
        """Decode and execute one instruction.  Returns the mnemonic string."""
        opcode = self._fetch8()

        # ── HALT sentinel (0xA5 — undefined/reserved on real 8051) ───────────
        if opcode == HALT_OPCODE:
            self._halted = True
            return "HALT"

        # ── NOP ──────────────────────────────────────────────────────────────
        if opcode == 0x00:
            return "NOP"

        # =======================================================================
        # MOV family — data transfer instructions
        # =======================================================================

        # MOV A, Rn  (0xE8-0xEF)
        if 0xE8 <= opcode <= 0xEF:
            self._set_acc(self._rn(opcode & 7))
            return f"MOV A,R{opcode & 7}"

        # MOV A, dir  (0xE5)
        if opcode == 0xE5:
            d = self._fetch8()
            self._set_acc(self._direct_read(d))
            return "MOV A,dir"

        # MOV A, @Ri  (0xE6-0xE7)
        if opcode in (0xE6, 0xE7):
            self._set_acc(self._indirect_read(opcode & 1))
            return f"MOV A,@R{opcode & 1}"

        # MOV A, #imm  (0x74)
        if opcode == 0x74:
            self._set_acc(self._fetch8())
            return "MOV A,#imm"

        # MOV Rn, A  (0xF8-0xFF)
        if 0xF8 <= opcode <= 0xFF:
            self._set_rn(opcode & 7, self._acc())
            return f"MOV R{opcode & 7},A"

        # MOV Rn, dir  (0xA8-0xAF)
        if 0xA8 <= opcode <= 0xAF:
            d = self._fetch8()
            self._set_rn(opcode & 7, self._direct_read(d))
            return f"MOV R{opcode & 7},dir"

        # MOV Rn, #imm  (0x78-0x7F)
        if 0x78 <= opcode <= 0x7F:
            self._set_rn(opcode & 7, self._fetch8())
            return f"MOV R{opcode & 7},#imm"

        # MOV dir, A  (0xF5)
        if opcode == 0xF5:
            d = self._fetch8()
            self._direct_write(d, self._acc())
            return "MOV dir,A"

        # MOV dir, Rn  (0x88-0x8F)
        if 0x88 <= opcode <= 0x8F:
            d = self._fetch8()
            self._direct_write(d, self._rn(opcode & 7))
            return f"MOV dir,R{opcode & 7}"

        # MOV dir, dir2  (0x85) — note: src byte comes first in encoding
        if opcode == 0x85:
            src = self._fetch8()
            dst = self._fetch8()
            self._direct_write(dst, self._direct_read(src))
            return "MOV dir,dir"

        # MOV dir, @Ri  (0x86-0x87)
        if opcode in (0x86, 0x87):
            d = self._fetch8()
            self._direct_write(d, self._indirect_read(opcode & 1))
            return f"MOV dir,@R{opcode & 1}"

        # MOV dir, #imm  (0x75)
        if opcode == 0x75:
            d = self._fetch8()
            imm = self._fetch8()
            self._direct_write(d, imm)
            return "MOV dir,#imm"

        # MOV @Ri, A  (0xF6-0xF7)
        if opcode in (0xF6, 0xF7):
            self._indirect_write(opcode & 1, self._acc())
            return f"MOV @R{opcode & 1},A"

        # MOV @Ri, dir  (0xA6-0xA7)
        if opcode in (0xA6, 0xA7):
            d = self._fetch8()
            self._indirect_write(opcode & 1, self._direct_read(d))
            return f"MOV @R{opcode & 1},dir"

        # MOV @Ri, #imm  (0x76-0x77)
        if opcode in (0x76, 0x77):
            self._indirect_write(opcode & 1, self._fetch8())
            return f"MOV @R{opcode & 1},#imm"

        # MOV DPTR, #imm16  (0x90) — 16-bit immediate load
        if opcode == 0x90:
            self._set_dptr(self._fetch16())
            return "MOV DPTR,#imm16"

        # MOVC A, @A+DPTR  (0x93) — code memory table lookup via DPTR
        if opcode == 0x93:
            # Effective address = ACC + DPTR (address arithmetic, not data arithmetic)
            acc = self._acc()
            dptr = self._dptr()
            ea, _ = add_16bit(acc, dptr, 0)  # gate-level 16-bit add for address
            self._set_acc(self._code[ea & 0xFFFF])
            return "MOVC A,@A+DPTR"

        # MOVC A, @A+PC  (0x83) — code memory table lookup via PC
        if opcode == 0x83:
            # PC is already incremented past the MOVC opcode byte
            acc = self._acc()
            pc = self._rf.read_pc()
            ea, _ = add_16bit(acc, pc, 0)
            self._set_acc(self._code[ea & 0xFFFF])
            return "MOVC A,@A+PC"

        # MOVX A, @Ri  (0xE2-0xE3) — external data read via R0/R1 (8-bit addr)
        if opcode in (0xE2, 0xE3):
            addr = self._rn(opcode & 1)
            self._set_acc(self._xdata[addr])
            return f"MOVX A,@R{opcode & 1}"

        # MOVX A, @DPTR  (0xE0) — external data read via 16-bit DPTR
        if opcode == 0xE0:
            self._set_acc(self._xdata[self._dptr()])
            return "MOVX A,@DPTR"

        # MOVX @Ri, A  (0xF2-0xF3) — external data write via R0/R1
        if opcode in (0xF2, 0xF3):
            self._xdata[self._rn(opcode & 1)] = self._acc()
            return f"MOVX @R{opcode & 1},A"

        # MOVX @DPTR, A  (0xF0) — external data write via 16-bit DPTR
        if opcode == 0xF0:
            self._xdata[self._dptr()] = self._acc()
            return "MOVX @DPTR,A"

        # =======================================================================
        # Stack operations
        # =======================================================================

        # PUSH dir  (0xC0) — push direct-addressed byte onto stack
        if opcode == 0xC0:
            d = self._fetch8()
            self._push8(self._direct_read(d))
            return "PUSH"

        # POP dir  (0xD0) — pop stack into direct-addressed byte
        if opcode == 0xD0:
            d = self._fetch8()
            self._direct_write(d, self._pop8())
            return "POP"

        # =======================================================================
        # Exchange instructions
        # =======================================================================

        # XCH A, Rn  (0xC8-0xCF) — swap ACC with register
        if 0xC8 <= opcode <= 0xCF:
            n = opcode & 7
            a = self._acc()
            rn = self._rn(n)
            self._set_acc(rn)
            self._set_rn(n, a)
            return f"XCH A,R{n}"

        # XCH A, dir  (0xC5) — swap ACC with direct byte
        if opcode == 0xC5:
            d = self._fetch8()
            a = self._acc()
            mem = self._direct_read(d)
            self._set_acc(mem)
            self._direct_write(d, a)
            return "XCH A,dir"

        # XCH A, @Ri  (0xC6-0xC7) — swap ACC with indirect byte
        if opcode in (0xC6, 0xC7):
            i = opcode & 1
            a = self._acc()
            mem = self._indirect_read(i)
            self._set_acc(mem)
            self._indirect_write(i, a)
            return f"XCH A,@R{i}"

        # XCHD A, @Ri  (0xD6-0xD7) — swap LOWER nibble of ACC with @Ri
        if opcode in (0xD6, 0xD7):
            i = opcode & 1
            a = self._acc()
            mem = self._indirect_read(i)
            # Lower nibble swap — address/bit wiring, not ALU arithmetic
            # ANL/ORL gates to isolate and recombine nibbles
            res_a = anl8(a, 0xF0)    # upper nibble of A, lower = 0
            low_m = anl8(mem, 0x0F)  # lower nibble of mem, upper = 0
            new_a = orl8(res_a.result, low_m.result)

            res_m = anl8(mem, 0xF0)  # upper nibble of mem, lower = 0
            low_a = anl8(a, 0x0F)    # lower nibble of A, upper = 0
            new_m = orl8(res_m.result, low_a.result)

            self._set_acc(new_a.result)
            self._indirect_write(i, new_m.result)
            return f"XCHD A,@R{i}"

        # =======================================================================
        # Arithmetic — gate-level ADD/ADDC
        # =======================================================================

        # ADD A, Rn  (0x28-0x2F)
        if 0x28 <= opcode <= 0x2F:
            res = add8(self._acc(), self._rn(opcode & 7), 0)
            self._apply_alu_result(res)
            return f"ADD A,R{opcode & 7}"

        # ADD A, dir  (0x25)
        if opcode == 0x25:
            d = self._fetch8()
            res = add8(self._acc(), self._direct_read(d), 0)
            self._apply_alu_result(res)
            return "ADD A,dir"

        # ADD A, @Ri  (0x26-0x27)
        if opcode in (0x26, 0x27):
            res = add8(self._acc(), self._indirect_read(opcode & 1), 0)
            self._apply_alu_result(res)
            return f"ADD A,@R{opcode & 1}"

        # ADD A, #imm  (0x24)
        if opcode == 0x24:
            res = add8(self._acc(), self._fetch8(), 0)
            self._apply_alu_result(res)
            return "ADD A,#imm"

        # ADDC A, Rn  (0x38-0x3F)
        if 0x38 <= opcode <= 0x3F:
            res = add8(self._acc(), self._rn(opcode & 7), self._cy())
            self._apply_alu_result(res)
            return f"ADDC A,R{opcode & 7}"

        # ADDC A, dir  (0x35)
        if opcode == 0x35:
            d = self._fetch8()
            res = add8(self._acc(), self._direct_read(d), self._cy())
            self._apply_alu_result(res)
            return "ADDC A,dir"

        # ADDC A, @Ri  (0x36-0x37)
        if opcode in (0x36, 0x37):
            res = add8(self._acc(), self._indirect_read(opcode & 1), self._cy())
            self._apply_alu_result(res)
            return f"ADDC A,@R{opcode & 1}"

        # ADDC A, #imm  (0x34)
        if opcode == 0x34:
            res = add8(self._acc(), self._fetch8(), self._cy())
            self._apply_alu_result(res)
            return "ADDC A,#imm"

        # =======================================================================
        # Arithmetic — gate-level SUBB
        # =======================================================================

        # SUBB A, Rn  (0x98-0x9F)
        if 0x98 <= opcode <= 0x9F:
            res = subb8(self._acc(), self._rn(opcode & 7), self._cy())
            self._apply_alu_result(res)
            return f"SUBB A,R{opcode & 7}"

        # SUBB A, dir  (0x95)
        if opcode == 0x95:
            d = self._fetch8()
            res = subb8(self._acc(), self._direct_read(d), self._cy())
            self._apply_alu_result(res)
            return "SUBB A,dir"

        # SUBB A, @Ri  (0x96-0x97)
        if opcode in (0x96, 0x97):
            res = subb8(self._acc(), self._indirect_read(opcode & 1), self._cy())
            self._apply_alu_result(res)
            return f"SUBB A,@R{opcode & 1}"

        # SUBB A, #imm  (0x94)
        if opcode == 0x94:
            res = subb8(self._acc(), self._fetch8(), self._cy())
            self._apply_alu_result(res)
            return "SUBB A,#imm"

        # =======================================================================
        # Increment / Decrement
        # =======================================================================

        # INC A  (0x04) — does NOT update CY
        if opcode == 0x04:
            res = inc8(self._acc())
            # INC only updates the value and parity, not CY/AC/OV
            self._rf.write_iram8(SFR_ACC, res.result)
            self._update_parity()
            return "INC A"

        # INC Rn  (0x08-0x0F)
        if 0x08 <= opcode <= 0x0F:
            n = opcode & 7
            res = inc8(self._rn(n))
            self._set_rn(n, res.result)
            return f"INC R{n}"

        # INC dir  (0x05)
        if opcode == 0x05:
            d = self._fetch8()
            res = inc8(self._direct_read(d))
            self._direct_write(d, res.result)
            return "INC dir"

        # INC @Ri  (0x06-0x07)
        if opcode in (0x06, 0x07):
            i = opcode & 1
            res = inc8(self._indirect_read(i))
            self._indirect_write(i, res.result)
            return f"INC @R{i}"

        # INC DPTR  (0xA3) — 16-bit DPTR increment, gate-level add_16bit
        if opcode == 0xA3:
            new_dptr, _ = add_16bit(self._dptr(), 1, 0)
            self._set_dptr(new_dptr & 0xFFFF)
            return "INC DPTR"

        # DEC A  (0x14) — does NOT update CY
        if opcode == 0x14:
            res = dec8(self._acc())
            self._rf.write_iram8(SFR_ACC, res.result)
            self._update_parity()
            return "DEC A"

        # DEC Rn  (0x18-0x1F)
        if 0x18 <= opcode <= 0x1F:
            n = opcode & 7
            res = dec8(self._rn(n))
            self._set_rn(n, res.result)
            return f"DEC R{n}"

        # DEC dir  (0x15)
        if opcode == 0x15:
            d = self._fetch8()
            res = dec8(self._direct_read(d))
            self._direct_write(d, res.result)
            return "DEC dir"

        # DEC @Ri  (0x16-0x17)
        if opcode in (0x16, 0x17):
            i = opcode & 1
            res = dec8(self._indirect_read(i))
            self._indirect_write(i, res.result)
            return f"DEC @R{i}"

        # MUL AB  (0xA4) — unsigned 8×8 multiply via gate-level repeated add
        if opcode == 0xA4:
            a = self._acc()
            b = self._rf.read_iram8(SFR_B)
            hi, lo, ov = mul8(a, b)
            # Result: A = low byte, B = high byte
            self._rf.write_iram8(SFR_ACC, lo)
            self._rf.write_iram8(SFR_B, hi)
            # CY = 0 always; OV = 1 if result > 255
            self._set_flags_cy_ac_ov(0, 0, ov)
            self._update_parity()
            return "MUL AB"

        # DIV AB  (0x84) — unsigned 8-bit divide via gate-level repeated subtract
        if opcode == 0x84:
            a = self._acc()
            b = self._rf.read_iram8(SFR_B)
            q, r, ov = div8(a, b)
            self._rf.write_iram8(SFR_ACC, q)
            self._rf.write_iram8(SFR_B, r)
            # CY = 0 always; OV = 1 for divide-by-zero
            self._set_flags_cy_ac_ov(0, 0, ov)
            self._update_parity()
            return "DIV AB"

        # DA A  (0xD4) — BCD decimal adjust after binary addition
        if opcode == 0xD4:
            psw = self._rf.read_iram8(SFR_PSW)
            cy_in = (psw >> 7) & 1
            ac_in = (psw >> 6) & 1
            res = da8(self._acc(), cy_in, ac_in)
            # DA A updates: value, CY, P; does not change AC or OV
            self._rf.write_iram8(SFR_ACC, res.result)
            self._set_cy(res.cy)
            self._update_parity()
            return "DA A"

        # =======================================================================
        # Logical operations — gate-level AND/OR/XOR
        # =======================================================================

        # ANL A, Rn  (0x58-0x5F)
        if 0x58 <= opcode <= 0x5F:
            res = anl8(self._acc(), self._rn(opcode & 7))
            self._set_acc(res.result)
            return f"ANL A,R{opcode & 7}"

        # ANL A, dir  (0x55)
        if opcode == 0x55:
            d = self._fetch8()
            res = anl8(self._acc(), self._direct_read(d))
            self._set_acc(res.result)
            return "ANL A,dir"

        # ANL A, @Ri  (0x56-0x57)
        if opcode in (0x56, 0x57):
            res = anl8(self._acc(), self._indirect_read(opcode & 1))
            self._set_acc(res.result)
            return f"ANL A,@R{opcode & 1}"

        # ANL A, #imm  (0x54)
        if opcode == 0x54:
            res = anl8(self._acc(), self._fetch8())
            self._set_acc(res.result)
            return "ANL A,#imm"

        # ANL dir, A  (0x52)
        if opcode == 0x52:
            d = self._fetch8()
            res = anl8(self._direct_read(d), self._acc())
            self._direct_write(d, res.result)
            return "ANL dir,A"

        # ANL dir, #imm  (0x53)
        if opcode == 0x53:
            d = self._fetch8()
            imm = self._fetch8()
            res = anl8(self._direct_read(d), imm)
            self._direct_write(d, res.result)
            return "ANL dir,#imm"

        # ORL A, Rn  (0x48-0x4F)
        if 0x48 <= opcode <= 0x4F:
            res = orl8(self._acc(), self._rn(opcode & 7))
            self._set_acc(res.result)
            return f"ORL A,R{opcode & 7}"

        # ORL A, dir  (0x45)
        if opcode == 0x45:
            d = self._fetch8()
            res = orl8(self._acc(), self._direct_read(d))
            self._set_acc(res.result)
            return "ORL A,dir"

        # ORL A, @Ri  (0x46-0x47)
        if opcode in (0x46, 0x47):
            res = orl8(self._acc(), self._indirect_read(opcode & 1))
            self._set_acc(res.result)
            return f"ORL A,@R{opcode & 1}"

        # ORL A, #imm  (0x44)
        if opcode == 0x44:
            res = orl8(self._acc(), self._fetch8())
            self._set_acc(res.result)
            return "ORL A,#imm"

        # ORL dir, A  (0x42)
        if opcode == 0x42:
            d = self._fetch8()
            res = orl8(self._direct_read(d), self._acc())
            self._direct_write(d, res.result)
            return "ORL dir,A"

        # ORL dir, #imm  (0x43)
        if opcode == 0x43:
            d = self._fetch8()
            imm = self._fetch8()
            res = orl8(self._direct_read(d), imm)
            self._direct_write(d, res.result)
            return "ORL dir,#imm"

        # XRL A, Rn  (0x68-0x6F)
        if 0x68 <= opcode <= 0x6F:
            res = xrl8(self._acc(), self._rn(opcode & 7))
            self._set_acc(res.result)
            return f"XRL A,R{opcode & 7}"

        # XRL A, dir  (0x65)
        if opcode == 0x65:
            d = self._fetch8()
            res = xrl8(self._acc(), self._direct_read(d))
            self._set_acc(res.result)
            return "XRL A,dir"

        # XRL A, @Ri  (0x66-0x67)
        if opcode in (0x66, 0x67):
            res = xrl8(self._acc(), self._indirect_read(opcode & 1))
            self._set_acc(res.result)
            return f"XRL A,@R{opcode & 1}"

        # XRL A, #imm  (0x64)
        if opcode == 0x64:
            res = xrl8(self._acc(), self._fetch8())
            self._set_acc(res.result)
            return "XRL A,#imm"

        # XRL dir, A  (0x62)
        if opcode == 0x62:
            d = self._fetch8()
            res = xrl8(self._direct_read(d), self._acc())
            self._direct_write(d, res.result)
            return "XRL dir,A"

        # XRL dir, #imm  (0x63)
        if opcode == 0x63:
            d = self._fetch8()
            imm = self._fetch8()
            res = xrl8(self._direct_read(d), imm)
            self._direct_write(d, res.result)
            return "XRL dir,#imm"

        # CLR A  (0xE4) — zero the accumulator
        if opcode == 0xE4:
            self._set_acc(0)
            return "CLR A"

        # CPL A  (0xF4) — bitwise complement using XRL with 0xFF (8 XOR gates)
        if opcode == 0xF4:
            res = xrl8(self._acc(), 0xFF)
            self._set_acc(res.result)
            return "CPL A"

        # RL A  (0x23) — rotate left, no carry
        if opcode == 0x23:
            res = rl8(self._acc())
            self._rf.write_iram8(SFR_ACC, res.result)
            self._set_cy(res.cy)
            self._update_parity()
            return "RL A"

        # RLC A  (0x33) — rotate left through carry
        if opcode == 0x33:
            res = rlc8(self._acc(), self._cy())
            self._rf.write_iram8(SFR_ACC, res.result)
            self._set_cy(res.cy)
            self._update_parity()
            return "RLC A"

        # RR A  (0x03) — rotate right, no carry
        if opcode == 0x03:
            res = rr8(self._acc())
            self._rf.write_iram8(SFR_ACC, res.result)
            self._set_cy(res.cy)
            self._update_parity()
            return "RR A"

        # RRC A  (0x13) — rotate right through carry
        if opcode == 0x13:
            res = rrc8(self._acc(), self._cy())
            self._rf.write_iram8(SFR_ACC, res.result)
            self._set_cy(res.cy)
            self._update_parity()
            return "RRC A"

        # SWAP A  (0xC4) — swap nibbles, no flag update
        if opcode == 0xC4:
            res = swap8(self._acc())
            # SWAP does NOT update parity
            self._rf.write_iram8(SFR_ACC, res.result)
            return "SWAP A"

        # =======================================================================
        # Bit operations
        # =======================================================================

        # CLR C  (0xC3) — clear carry flag
        if opcode == 0xC3:
            self._set_cy(0)
            return "CLR C"

        # CLR bit  (0xC2) — clear a bit-addressable bit
        if opcode == 0xC2:
            self._write_bit(self._fetch8(), 0)
            return "CLR bit"

        # SETB C  (0xD3) — set carry flag
        if opcode == 0xD3:
            self._set_cy(1)
            return "SETB C"

        # SETB bit  (0xD2) — set a bit-addressable bit
        if opcode == 0xD2:
            self._write_bit(self._fetch8(), 1)
            return "SETB bit"

        # CPL C  (0xB3) — complement carry
        if opcode == 0xB3:
            cy = self._cy()
            # Complement via NOT gate
            from logic_gates import NOT as _NOT
            self._set_cy(_NOT(cy))
            return "CPL C"

        # CPL bit  (0xB2) — complement a bit-addressable bit
        if opcode == 0xB2:
            bit = self._fetch8()
            from logic_gates import NOT as _NOT
            self._write_bit(bit, _NOT(self._read_bit(bit)))
            return "CPL bit"

        # ANL C, bit  (0x82) — C = C AND bit
        if opcode == 0x82:
            bit = self._fetch8()
            from logic_gates import AND as _AND
            self._set_cy(_AND(self._cy(), self._read_bit(bit)))
            return "ANL C,bit"

        # ANL C, /bit  (0xB0) — C = C AND NOT(bit)
        if opcode == 0xB0:
            bit = self._fetch8()
            from logic_gates import AND as _AND
            from logic_gates import NOT as _NOT
            self._set_cy(_AND(self._cy(), _NOT(self._read_bit(bit))))
            return "ANL C,/bit"

        # ORL C, bit  (0x72) — C = C OR bit
        if opcode == 0x72:
            bit = self._fetch8()
            from logic_gates import OR as _OR
            self._set_cy(_OR(self._cy(), self._read_bit(bit)))
            return "ORL C,bit"

        # ORL C, /bit  (0xA0) — C = C OR NOT(bit)
        if opcode == 0xA0:
            bit = self._fetch8()
            from logic_gates import NOT as _NOT
            from logic_gates import OR as _OR
            self._set_cy(_OR(self._cy(), _NOT(self._read_bit(bit))))
            return "ORL C,/bit"

        # MOV C, bit  (0xA2) — copy bit to carry
        if opcode == 0xA2:
            bit = self._fetch8()
            self._set_cy(self._read_bit(bit))
            return "MOV C,bit"

        # MOV bit, C  (0x92) — copy carry to bit
        if opcode == 0x92:
            self._write_bit(self._fetch8(), self._cy())
            return "MOV bit,C"

        # =======================================================================
        # Branch / Jump instructions
        # =======================================================================

        # LJMP addr16  (0x02) — unconditional 16-bit jump
        if opcode == 0x02:
            self._rf.write_pc(self._fetch16())
            return "LJMP"

        # SJMP rel  (0x80) — short relative jump (signed 8-bit offset)
        if opcode == 0x80:
            rel = self._sign_extend_rel8(self._fetch8())
            pc = self._rf.read_pc()
            if rel >= 0:
                new_pc, _ = add_16bit(pc, rel, 0)
            else:
                new_pc, _ = add_16bit(pc, 0x10000 + rel, 0)
            self._rf.write_pc(new_pc & 0xFFFF)
            return "SJMP"

        # JMP @A+DPTR  (0x73) — indirect jump through accumulator + DPTR
        if opcode == 0x73:
            ea, _ = add_16bit(self._acc(), self._dptr(), 0)
            self._rf.write_pc(ea & 0xFFFF)
            return "JMP @A+DPTR"

        # AJMP  — 11-bit absolute jump (bits [7:5] = page, byte2 = addr[7:0])
        if (opcode & 0x1F) == 0x01:
            addr11_hi = (opcode >> 5) & 0x7
            addr11_lo = self._fetch8()
            pc = self._rf.read_pc()
            # Keep upper 5 bits of PC, replace lower 11 bits
            new_pc = (pc & 0xF800) | (addr11_hi << 8) | addr11_lo
            self._rf.write_pc(new_pc)
            return "AJMP"

        # JZ rel  (0x60) — jump if ACC == 0
        if opcode == 0x60:
            rel = self._sign_extend_rel8(self._fetch8())
            acc_bits = int_to_bits(self._acc(), 8)
            from .bits import compute_zero
            if compute_zero(acc_bits):
                pc = self._rf.read_pc()
                if rel >= 0:
                    new_pc, _ = add_16bit(pc, rel, 0)
                else:
                    new_pc, _ = add_16bit(pc, 0x10000 + rel, 0)
                self._rf.write_pc(new_pc & 0xFFFF)
            return "JZ"

        # JNZ rel  (0x70) — jump if ACC != 0
        if opcode == 0x70:
            rel = self._sign_extend_rel8(self._fetch8())
            acc_bits = int_to_bits(self._acc(), 8)
            from logic_gates import NOT as _NOT

            from .bits import compute_zero
            if _NOT(compute_zero(acc_bits)):
                pc = self._rf.read_pc()
                if rel >= 0:
                    new_pc, _ = add_16bit(pc, rel, 0)
                else:
                    new_pc, _ = add_16bit(pc, 0x10000 + rel, 0)
                self._rf.write_pc(new_pc & 0xFFFF)
            return "JNZ"

        # JC rel  (0x40) — jump if carry set
        if opcode == 0x40:
            rel = self._sign_extend_rel8(self._fetch8())
            if self._cy():
                pc = self._rf.read_pc()
                if rel >= 0:
                    new_pc, _ = add_16bit(pc, rel, 0)
                else:
                    new_pc, _ = add_16bit(pc, 0x10000 + rel, 0)
                self._rf.write_pc(new_pc & 0xFFFF)
            return "JC"

        # JNC rel  (0x50) — jump if carry clear
        if opcode == 0x50:
            rel = self._sign_extend_rel8(self._fetch8())
            from logic_gates import NOT as _NOT
            if _NOT(self._cy()):
                pc = self._rf.read_pc()
                if rel >= 0:
                    new_pc, _ = add_16bit(pc, rel, 0)
                else:
                    new_pc, _ = add_16bit(pc, 0x10000 + rel, 0)
                self._rf.write_pc(new_pc & 0xFFFF)
            return "JNC"

        # JB bit, rel  (0x20) — jump if bit is set
        if opcode == 0x20:
            bit = self._fetch8()
            rel = self._sign_extend_rel8(self._fetch8())
            if self._read_bit(bit):
                pc = self._rf.read_pc()
                if rel >= 0:
                    new_pc, _ = add_16bit(pc, rel, 0)
                else:
                    new_pc, _ = add_16bit(pc, 0x10000 + rel, 0)
                self._rf.write_pc(new_pc & 0xFFFF)
            return "JB"

        # JNB bit, rel  (0x30) — jump if bit is clear
        if opcode == 0x30:
            bit = self._fetch8()
            rel = self._sign_extend_rel8(self._fetch8())
            from logic_gates import NOT as _NOT
            if _NOT(self._read_bit(bit)):
                pc = self._rf.read_pc()
                if rel >= 0:
                    new_pc, _ = add_16bit(pc, rel, 0)
                else:
                    new_pc, _ = add_16bit(pc, 0x10000 + rel, 0)
                self._rf.write_pc(new_pc & 0xFFFF)
            return "JNB"

        # JBC bit, rel  (0x10) — jump if bit set, then clear the bit
        if opcode == 0x10:
            bit = self._fetch8()
            rel = self._sign_extend_rel8(self._fetch8())
            if self._read_bit(bit):
                self._write_bit(bit, 0)
                pc = self._rf.read_pc()
                if rel >= 0:
                    new_pc, _ = add_16bit(pc, rel, 0)
                else:
                    new_pc, _ = add_16bit(pc, 0x10000 + rel, 0)
                self._rf.write_pc(new_pc & 0xFFFF)
            return "JBC"

        # CJNE A, dir, rel  (0xB5)
        if opcode == 0xB5:
            d = self._fetch8()
            rel = self._sign_extend_rel8(self._fetch8())
            val = self._direct_read(d)
            a = self._acc()
            # CY = 1 if A < val (unsigned borrow)
            cmp_res = subb8(a, val, 0)
            self._set_cy(cmp_res.cy)
            if a != val:
                pc = self._rf.read_pc()
                if rel >= 0:
                    new_pc, _ = add_16bit(pc, rel, 0)
                else:
                    new_pc, _ = add_16bit(pc, 0x10000 + rel, 0)
                self._rf.write_pc(new_pc & 0xFFFF)
            return "CJNE A,dir"

        # CJNE A, #imm, rel  (0xB4)
        if opcode == 0xB4:
            imm = self._fetch8()
            rel = self._sign_extend_rel8(self._fetch8())
            a = self._acc()
            cmp_res = subb8(a, imm, 0)
            self._set_cy(cmp_res.cy)
            if a != imm:
                pc = self._rf.read_pc()
                if rel >= 0:
                    new_pc, _ = add_16bit(pc, rel, 0)
                else:
                    new_pc, _ = add_16bit(pc, 0x10000 + rel, 0)
                self._rf.write_pc(new_pc & 0xFFFF)
            return "CJNE A,#imm"

        # CJNE Rn, #imm, rel  (0xB8-0xBF)
        if 0xB8 <= opcode <= 0xBF:
            n = opcode & 7
            imm = self._fetch8()
            rel = self._sign_extend_rel8(self._fetch8())
            rn = self._rn(n)
            cmp_res = subb8(rn, imm, 0)
            self._set_cy(cmp_res.cy)
            if rn != imm:
                pc = self._rf.read_pc()
                if rel >= 0:
                    new_pc, _ = add_16bit(pc, rel, 0)
                else:
                    new_pc, _ = add_16bit(pc, 0x10000 + rel, 0)
                self._rf.write_pc(new_pc & 0xFFFF)
            return f"CJNE R{n},#imm"

        # CJNE @Ri, #imm, rel  (0xB6-0xB7)
        if opcode in (0xB6, 0xB7):
            i = opcode & 1
            imm = self._fetch8()
            rel = self._sign_extend_rel8(self._fetch8())
            mem = self._indirect_read(i)
            cmp_res = subb8(mem, imm, 0)
            self._set_cy(cmp_res.cy)
            if mem != imm:
                pc = self._rf.read_pc()
                if rel >= 0:
                    new_pc, _ = add_16bit(pc, rel, 0)
                else:
                    new_pc, _ = add_16bit(pc, 0x10000 + rel, 0)
                self._rf.write_pc(new_pc & 0xFFFF)
            return f"CJNE @R{i},#imm"

        # DJNZ Rn, rel  (0xD8-0xDF) — decrement and jump if not zero
        if 0xD8 <= opcode <= 0xDF:
            n = opcode & 7
            rel = self._sign_extend_rel8(self._fetch8())
            res = dec8(self._rn(n))  # gate-level decrement
            self._set_rn(n, res.result)
            if res.result != 0:
                pc = self._rf.read_pc()
                if rel >= 0:
                    new_pc, _ = add_16bit(pc, rel, 0)
                else:
                    new_pc, _ = add_16bit(pc, 0x10000 + rel, 0)
                self._rf.write_pc(new_pc & 0xFFFF)
            return f"DJNZ R{n}"

        # DJNZ dir, rel  (0xD5)
        if opcode == 0xD5:
            d = self._fetch8()
            rel = self._sign_extend_rel8(self._fetch8())
            res = dec8(self._direct_read(d))
            self._direct_write(d, res.result)
            if res.result != 0:
                pc = self._rf.read_pc()
                if rel >= 0:
                    new_pc, _ = add_16bit(pc, rel, 0)
                else:
                    new_pc, _ = add_16bit(pc, 0x10000 + rel, 0)
                self._rf.write_pc(new_pc & 0xFFFF)
            return "DJNZ dir"

        # =======================================================================
        # Subroutine calls and returns
        # =======================================================================

        # LCALL addr16  (0x12) — long call (16-bit address)
        if opcode == 0x12:
            addr = self._fetch16()
            self._push_pc()
            self._rf.write_pc(addr)
            return "LCALL"

        # ACALL  — 11-bit page call (bits [7:5] = page, byte2 = addr[7:0])
        if (opcode & 0x1F) == 0x11:
            addr11_hi = (opcode >> 5) & 0x7
            addr11_lo = self._fetch8()
            self._push_pc()
            pc = self._rf.read_pc()
            new_pc = (pc & 0xF800) | (addr11_hi << 8) | addr11_lo
            self._rf.write_pc(new_pc)
            return "ACALL"

        # RET  (0x22) — return from subroutine
        if opcode == 0x22:
            self._pop_pc()
            return "RET"

        # RETI  (0x32) — return from interrupt (same as RET for behavioral sim)
        if opcode == 0x32:
            self._pop_pc()
            return "RETI"

        raise ValueError(f"Unknown opcode: 0x{opcode:02X} at PC=0x{(self._rf.read_pc() - 1) & 0xFFFF:04X}")
