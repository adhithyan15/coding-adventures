"""Zilog Z80 behavioral simulator.

The Z80 (1976) is a superset of the Intel 8080. Every valid 8080 opcode is a
valid Z80 opcode with identical semantics. The Z80 adds:

  - Alternate register bank: A', F', B', C', D', E', H', L'  (swapped via
    EX AF,AF' and EXX — only one bank is live at a time)
  - Index registers IX and IY with signed 8-bit displacement addressing
  - CB-prefix: bit manipulation on all registers (BIT/SET/RES) and extended
    rotate/shift variants (RLC, RRC, RL, RR, SLA, SRA, SRL)
  - ED-prefix: 16-bit arithmetic (ADC HL,rp / SBC HL,rp), block operations
    (LDIR/LDDR/CPIR/CPDR), I/O block ops (INIR/OTIR), register-load variants
  - DD/FD prefix: replaces HL with IX or IY throughout most instructions
  - Three interrupt modes (IM 0 / IM 1 / IM 2) and NMI
  - DJNZ (decrement B and jump if not zero) for tight loops
  - JR (relative jump) and conditional JR NZ/Z/NC/C

Halt condition
--------------
HALT sets halted=True (same convention as HLT on 8080, BRK on 6502).
Real Z80 HALT repeatedly executes NOP internally until an interrupt arrives;
we simply stop the execution loop.

Memory-mapped I/O
-----------------
The Z80 has a separate 8-bit I/O address space (ports 0x00–0xFF) accessed
via IN/OUT instructions. This simulator maps ports to input_ports /
output_ports arrays (256 ports each) per the SIM00 convention.
"""

from __future__ import annotations

from simulator_protocol import ExecutionResult, Simulator, StepTrace

from z80_simulator.flags import (
    compute_half_carry_add,
    compute_half_carry_sub,
    compute_overflow_add,
    compute_overflow_sub,
    compute_parity,
    compute_sz,
    daa,
    pack_f,
    unpack_f,
)
from z80_simulator.state import Z80State

# ── Reset defaults ────────────────────────────────────────────────────────────
_RESET_F = 0xFF    # All flags set at power-on
_NUM_PORTS = 256

# ── Register encoding helpers ─────────────────────────────────────────────────
# Z80 uses a 3-bit field to select one of 8 "r" registers:
#   0=B 1=C 2=D 3=E 4=H 5=L 6=(HL) 7=A
_REG_B, _REG_C, _REG_D, _REG_E, _REG_H, _REG_L, _REG_MEM, _REG_A = range(8)

# 16-bit register pair encoding (for most instructions):
#   0=BC 1=DE 2=HL 3=SP
_RP_BC, _RP_DE, _RP_HL, _RP_SP = range(4)

# 16-bit register pair encoding (for PUSH/POP):
#   0=BC 1=DE 2=HL 3=AF
_RP_AF = 3


class Z80Simulator(Simulator[Z80State]):
    """Behavioral simulator for the Zilog Z80 (NMOS, 1976).

    Implements the full SIM00 Simulator[Z80State] protocol.

    I/O:
      Reads from port n  → input_ports[n]
      Writes to port n   → output_ports[n]

    Example::

        sim = Z80Simulator()
        result = sim.execute(bytes([
            0x3E, 0x0A,   # LD A, 10
            0xC6, 0x05,   # ADD A, 5
            0x76,         # HALT
        ]))
        assert result.final_state.a == 15
    """

    # ── Construction & reset ─────────────────────────────────────────────────

    def __init__(self) -> None:
        self._memory = bytearray(65536)
        # Main registers
        self._a = 0
        self._f = _RESET_F
        self._b = 0
        self._c = 0
        self._d = 0
        self._e = 0
        self._h = 0
        self._l = 0
        # Alternate registers
        self._a2 = 0
        self._f2 = _RESET_F
        self._b2 = 0
        self._c2 = 0
        self._d2 = 0
        self._e2 = 0
        self._h2 = 0
        self._l2 = 0
        # Special registers
        self._ix = 0
        self._iy = 0
        self._sp = 0
        self._pc = 0
        self._i = 0
        self._r = 0
        # Interrupt state
        self._iff1 = False
        self._iff2 = False
        self._im = 0
        # Flags (unpacked from _f for fast access)
        self._flag_s  = True
        self._flag_z  = True
        self._flag_h  = True
        self._flag_pv = True
        self._flag_n  = True
        self._flag_c  = True
        self._halted = False
        # I/O
        self._input_ports:  list[int] = [0] * _NUM_PORTS
        self._output_ports: list[int] = [0] * _NUM_PORTS

    # ── SIM00 protocol ────────────────────────────────────────────────────────

    def reset(self) -> None:
        """Reset to power-on state.

        Registers: all 0
        F = F' = 0xFF
        IX=IY=SP=PC=I=R=0
        IFF1=IFF2=False
        IM=0
        memory zeroed.
        """
        self._memory = bytearray(65536)
        self._a = 0
        self._f = _RESET_F
        self._b = 0
        self._c = 0
        self._d = 0
        self._e = 0
        self._h = 0
        self._l = 0
        self._a2 = 0
        self._f2 = _RESET_F
        self._b2 = 0
        self._c2 = 0
        self._d2 = 0
        self._e2 = 0
        self._h2 = 0
        self._l2 = 0
        self._ix = 0
        self._iy = 0
        self._sp = 0
        self._pc = 0
        self._i = 0
        self._r = 0
        self._iff1 = False
        self._iff2 = False
        self._im = 0
        self._flag_s  = True
        self._flag_z  = True
        self._flag_h  = True
        self._flag_pv = True
        self._flag_n  = True
        self._flag_c  = True
        self._halted = False

    def load(self, program: bytes, origin: int = 0x0000) -> None:
        """Write program bytes into memory at origin and set PC.

        Args:
            program: Machine code bytes.
            origin:  Load address (default 0x0000).

        Raises:
            ValueError: If origin is out of range.
        """
        if not (0 <= origin <= 0xFFFF):
            raise ValueError(f"origin {origin:#06x} out of range 0x0000–0xFFFF")
        for i, byte in enumerate(program):
            self._memory[(origin + i) & 0xFFFF] = byte & 0xFF
        self._pc = origin
        self._halted = False

    def step(self) -> StepTrace:
        """Execute one instruction and return a StepTrace.

        Raises:
            RuntimeError: If already halted.
        """
        if self._halted:
            raise RuntimeError("CPU is halted — call reset() or load() first")
        pc_before = self._pc
        desc = self._fetch_and_execute()
        return StepTrace(
            pc_before=pc_before,
            pc_after=self._pc,
            mnemonic=desc.split()[0] if desc else "?",
            description=desc,
        )

    def execute(
        self,
        program: bytes,
        origin: int = 0x0000,
        max_steps: int = 100_000,
    ) -> ExecutionResult[Z80State]:
        """Load and run until HALT or max_steps.

        I/O port values set before execute() are preserved across the
        internal reset() call.
        """
        saved_in  = list(self._input_ports)
        saved_out = list(self._output_ports)
        self.reset()
        self._input_ports  = saved_in
        self._output_ports = saved_out
        self.load(program, origin)

        traces: list[StepTrace] = []
        steps = 0
        while not self._halted and steps < max_steps:
            traces.append(self.step())
            steps += 1

        return ExecutionResult(
            halted=self._halted,
            steps=steps,
            final_state=self.get_state(),
            error=None,
            traces=traces,
        )

    def get_state(self) -> Z80State:
        """Return an immutable snapshot of the current CPU state."""
        return Z80State(
            a=self._a, b=self._b, c=self._c,
            d=self._d, e=self._e, h=self._h, l=self._l,
            a_prime=self._a2, f_prime=self._f2,
            b_prime=self._b2, c_prime=self._c2,
            d_prime=self._d2, e_prime=self._e2,
            h_prime=self._h2, l_prime=self._l2,
            ix=self._ix, iy=self._iy,
            sp=self._sp, pc=self._pc,
            i=self._i, r=self._r,
            flag_s=self._flag_s, flag_z=self._flag_z,
            flag_h=self._flag_h, flag_pv=self._flag_pv,
            flag_n=self._flag_n, flag_c=self._flag_c,
            iff1=self._iff1, iff2=self._iff2, im=self._im,
            halted=self._halted,
            memory=tuple(self._memory),
        )

    def set_input_port(self, port: int, value: int) -> None:
        """Set the value returned when reading from port `port` (0–255)."""
        if not (0 <= port < _NUM_PORTS):
            raise ValueError(f"port {port} out of range 0–{_NUM_PORTS - 1}")
        if not (0 <= value <= 255):
            raise ValueError(f"value {value} out of range 0–255")
        self._input_ports[port] = value

    def get_output_port(self, port: int) -> int:
        """Return the last value written to output port `port` (0–255)."""
        if not (0 <= port < _NUM_PORTS):
            raise ValueError(f"port {port} out of range 0–{_NUM_PORTS - 1}")
        return self._output_ports[port]

    def interrupt(self, data: int = 0xFF) -> None:
        """Fire a maskable interrupt (INT).

        Only has effect if IFF1 is True.
        - IM 0: treat `data` as RST opcode on bus
        execute RST p
        - IM 1: push PC
        jump to 0x0038
        - IM 2: push PC
        jump to address at mem[I*256 + data]
        """
        if not self._iff1:
            return
        self._iff1 = False
        self._iff2 = False
        self._halted = False
        self._push16(self._pc)
        if self._im == 0:
            # RST p: p = data & 0x38
            self._pc = data & 0x38
        elif self._im == 1:
            self._pc = 0x0038
        else:  # IM 2
            vec_addr = ((self._i << 8) | (data & 0xFE)) & 0xFFFF
            lo = self._memory[vec_addr]
            hi = self._memory[(vec_addr + 1) & 0xFFFF]
            self._pc = (hi << 8) | lo

    def nmi(self) -> None:
        """Fire a non-maskable interrupt (NMI).

        Always accepted regardless of IFF1. Jumps to 0x0066.
        Saves IFF1 into IFF2, then clears IFF1.
        """
        self._iff2 = self._iff1
        self._iff1 = False
        self._halted = False
        self._push16(self._pc)
        self._pc = 0x0066

    # ── Memory helpers ────────────────────────────────────────────────────────

    def _read(self, addr: int) -> int:
        return self._memory[addr & 0xFFFF]

    def _write(self, addr: int, value: int) -> None:
        self._memory[addr & 0xFFFF] = value & 0xFF

    def _read16(self, addr: int) -> int:
        lo = self._memory[addr & 0xFFFF]
        hi = self._memory[(addr + 1) & 0xFFFF]
        return (hi << 8) | lo

    def _write16(self, addr: int, value: int) -> None:
        self._memory[addr & 0xFFFF] = value & 0xFF
        self._memory[(addr + 1) & 0xFFFF] = (value >> 8) & 0xFF

    def _fetch(self) -> int:
        """Read byte at PC and advance PC."""
        val = self._memory[self._pc]
        self._pc = (self._pc + 1) & 0xFFFF
        self._r = ((self._r + 1) & 0x7F) | (self._r & 0x80)
        return val

    def _fetch_signed(self) -> int:
        """Read signed 8-bit byte at PC and advance PC."""
        b = self._fetch()
        return b - 256 if b >= 0x80 else b

    def _fetch16(self) -> int:
        lo = self._fetch()
        hi = self._fetch()
        return (hi << 8) | lo

    # ── Stack helpers ─────────────────────────────────────────────────────────

    def _push16(self, value: int) -> None:
        self._sp = (self._sp - 1) & 0xFFFF
        self._memory[self._sp] = (value >> 8) & 0xFF
        self._sp = (self._sp - 1) & 0xFFFF
        self._memory[self._sp] = value & 0xFF

    def _pop16(self) -> int:
        lo = self._memory[self._sp]
        self._sp = (self._sp + 1) & 0xFFFF
        hi = self._memory[self._sp]
        self._sp = (self._sp + 1) & 0xFFFF
        return (hi << 8) | lo

    # ── Register read/write by 3-bit code ────────────────────────────────────

    def _get_r(self, code: int, idx: int | None = None) -> int:
        """Get 8-bit register by 3-bit code (0=B…7=A).

        When idx is an integer (IX/IY displacement base), code 6 reads
        (idx+d) instead of (HL).
        """
        if code == _REG_B:
            return self._b
        if code == _REG_C:
            return self._c
        if code == _REG_D:
            return self._d
        if code == _REG_E:
            return self._e
        if code == _REG_H:
            return self._h if idx is None else (idx >> 8) & 0xFF
        if code == _REG_L:
            return self._l if idx is None else idx & 0xFF
        if code == _REG_A:
            return self._a
        # code == 6 : (HL) or (IX+d) / (IY+d)
        if idx is None:
            return self._read((self._h << 8) | self._l)
        d = self._fetch_signed()
        return self._read((idx + d) & 0xFFFF)

    def _set_r(self, code: int, value: int, idx: int | None = None) -> None:
        """Set 8-bit register by 3-bit code."""
        v = value & 0xFF
        if code == _REG_B:
            self._b = v
        elif code == _REG_C:
            self._c = v
        elif code == _REG_D:
            self._d = v
        elif code == _REG_E:
            self._e = v
        elif code == _REG_H:
            if idx is None:
                self._h = v
            else:
                if idx is self._ix:
                    self._ix = (v << 8) | (self._ix & 0xFF)
                else:
                    self._iy = (v << 8) | (self._iy & 0xFF)
        elif code == _REG_L:
            if idx is None:
                self._l = v
            else:
                if idx is self._ix:
                    self._ix = (self._ix & 0xFF00) | v
                else:
                    self._iy = (self._iy & 0xFF00) | v
        elif code == _REG_A:
            self._a = v
        else:  # code 6 = (HL) or (idx+d)
            if idx is None:
                self._write((self._h << 8) | self._l, v)
            else:
                d = self._fetch_signed()
                self._write((idx + d) & 0xFFFF, v)

    def _get_rp(self, code: int) -> int:
        """Get 16-bit register pair (0=BC, 1=DE, 2=HL, 3=SP)."""
        if code == _RP_BC:
            return (self._b << 8) | self._c
        if code == _RP_DE:
            return (self._d << 8) | self._e
        if code == _RP_HL:
            return (self._h << 8) | self._l
        return self._sp  # _RP_SP = 3

    def _set_rp(self, code: int, value: int) -> None:
        """Set 16-bit register pair (0=BC, 1=DE, 2=HL, 3=SP)."""
        v = value & 0xFFFF
        if code == _RP_BC:
            self._b, self._c = (v >> 8), v & 0xFF
        elif code == _RP_DE:
            self._d, self._e = (v >> 8), v & 0xFF
        elif code == _RP_HL:
            self._h, self._l = (v >> 8), v & 0xFF
        else:
            self._sp = v

    def _get_rp_af(self, code: int) -> int:
        """Get 16-bit register pair for PUSH/POP (0=BC, 1=DE, 2=HL, 3=AF)."""
        if code == 3:
            return (self._a << 8) | self._f_byte()
        return self._get_rp(code)

    def _set_rp_af(self, code: int, value: int) -> None:
        """Set 16-bit register pair for PUSH/POP (0=BC, 1=DE, 2=HL, 3=AF)."""
        if code == 3:
            self._a = (value >> 8) & 0xFF
            self._f = value & 0xFF
            (
                self._flag_s, self._flag_z, self._flag_h,
                self._flag_pv, self._flag_n, self._flag_c,
            ) = unpack_f(self._f)
        else:
            self._set_rp(code, value)

    # ── Flag helpers ──────────────────────────────────────────────────────────

    def _f_byte(self) -> int:
        return pack_f(
            self._flag_s, self._flag_z, self._flag_h,
            self._flag_pv, self._flag_n, self._flag_c,
        )

    def _set_sz(self, result: int) -> None:
        self._flag_s, self._flag_z = compute_sz(result)

    def _set_szpn(self, result: int, n: bool = False) -> None:
        """Set S, Z, PV (parity), N from result."""
        self._flag_s, self._flag_z = compute_sz(result)
        self._flag_pv = compute_parity(result)
        self._flag_n = n

    def _cond(self, cc: int) -> bool:
        """Evaluate condition code (3-bit field from opcode)."""
        # cc: 0=NZ 1=Z 2=NC 3=C 4=PO 5=PE 6=P(sign+) 7=M(sign-)
        if cc == 0:
            return not self._flag_z
        if cc == 1:
            return self._flag_z
        if cc == 2:
            return not self._flag_c
        if cc == 3:
            return self._flag_c
        if cc == 4:
            return not self._flag_pv
        if cc == 5:
            return self._flag_pv
        if cc == 6:
            return not self._flag_s
        return self._flag_s  # cc == 7

    # ── 8-bit ALU operations ──────────────────────────────────────────────────

    def _alu8(self, op: int, operand: int) -> None:
        """Execute 8-bit ALU operation on A.

        op codes match Z80 opcode bit pattern:
          0=ADD 1=ADC 2=SUB 3=SBC 4=AND 5=XOR 6=OR 7=CP
        """
        a = self._a
        m = operand & 0xFF

        if op == 0:  # ADD A, m
            total = a + m
            r = total & 0xFF
            self._flag_h  = compute_half_carry_add(a, m)
            self._flag_pv = compute_overflow_add(a, m, r)
            self._flag_n  = False
            self._flag_c  = total > 0xFF
            self._a = r
            self._set_sz(r)

        elif op == 1:  # ADC A, m
            c = int(self._flag_c)
            total = a + m + c
            r = total & 0xFF
            self._flag_h  = compute_half_carry_add(a, m, c)
            self._flag_pv = compute_overflow_add(a, m, r)
            self._flag_n  = False
            self._flag_c  = total > 0xFF
            self._a = r
            self._set_sz(r)

        elif op == 2:  # SUB m
            total = a - m
            r = total & 0xFF
            self._flag_h  = compute_half_carry_sub(a, m)
            self._flag_pv = compute_overflow_sub(a, m, r)
            self._flag_n  = True
            self._flag_c  = total < 0
            self._a = r
            self._set_sz(r)

        elif op == 3:  # SBC A, m
            borrow = int(self._flag_c)
            total = a - m - borrow
            r = total & 0xFF
            self._flag_h  = compute_half_carry_sub(a, m, borrow)
            self._flag_pv = compute_overflow_sub(a, m, r)
            self._flag_n  = True
            self._flag_c  = total < 0
            self._a = r
            self._set_sz(r)

        elif op == 4:  # AND m
            r = a & m
            self._flag_h  = True
            self._flag_n  = False
            self._flag_c  = False
            self._a = r
            self._set_szpn(r)

        elif op == 5:  # XOR m
            r = a ^ m
            self._flag_h  = False
            self._flag_n  = False
            self._flag_c  = False
            self._a = r
            self._set_szpn(r)

        elif op == 6:  # OR m
            r = a | m
            self._flag_h  = False
            self._flag_n  = False
            self._flag_c  = False
            self._a = r
            self._set_szpn(r)

        else:  # CP m  (compare: like SUB but don't store result)
            total = a - m
            r = total & 0xFF
            self._flag_h  = compute_half_carry_sub(a, m)
            self._flag_pv = compute_overflow_sub(a, m, r)
            self._flag_n  = True
            self._flag_c  = total < 0
            self._set_sz(r)
            # A unchanged

    # ── Main dispatch ─────────────────────────────────────────────────────────

    def _fetch_and_execute(self) -> str:
        """Fetch and execute one instruction. Returns description string."""
        b = self._fetch()

        if b == 0xCB:
            return self._exec_cb()
        if b == 0xED:
            return self._exec_ed()
        if b == 0xDD:
            return self._exec_ddfd(use_ix=True)
        if b == 0xFD:
            return self._exec_ddfd(use_ix=False)

        return self._exec_main(b)

    # ── Main (unprefixed) instruction set ─────────────────────────────────────

    def _exec_main(self, op: int) -> str:  # noqa: PLR0912, PLR0915
        """Execute an unprefixed opcode."""

        # ── NOP ──────────────────────────────────────────────────────────────
        if op == 0x00:
            return "NOP"

        # ── HALT ─────────────────────────────────────────────────────────────
        if op == 0x76:
            self._halted = True
            return "HALT"

        # ── 8-bit load r, r' (LD r, r') ──────────────────────────────────────
        # Opcodes 0x40–0x7F (excluding 0x76 = HALT)
        if 0x40 <= op <= 0x7F:
            dst = (op >> 3) & 0x07
            src = op & 0x07
            val = self._get_r(src)
            self._set_r(dst, val)
            return "LD r,r'"

        # ── LD r, n (8-bit immediate into register) ───────────────────────────
        if op & 0xC7 == 0x06:
            dst = (op >> 3) & 0x07
            n = self._fetch()
            self._set_r(dst, n)
            return f"LD r,{n:#04x}"

        # ── 8-bit ALU with register operand (op 0x80–0xBF) ────────────────────
        if 0x80 <= op <= 0xBF:
            alu_op = (op >> 3) & 0x07
            src = op & 0x07
            operand = self._get_r(src)
            self._alu8(alu_op, operand)
            return f"ALU{alu_op} r"

        # ── 8-bit ALU with immediate operand (0xC6/0xCE/0xD6/0xDE/0xE6/0xEE/0xF6/0xFE)
        if op & 0xC7 == 0xC6:
            alu_op = (op >> 3) & 0x07
            n = self._fetch()
            self._alu8(alu_op, n)
            return f"ALU{alu_op} {n:#04x}"

        # ── INC / DEC register (0x04/0x0C/0x14/0x1C/0x24/0x2C/0x34/0x3C
        #                       0x05/0x0D/0x15/0x1D/0x25/0x2D/0x35/0x3D) ───────
        if op & 0xC7 == 0x04:   # INC r
            r_code = (op >> 3) & 0x07
            v = self._get_r(r_code)
            r = (v + 1) & 0xFF
            self._set_r(r_code, r)
            self._flag_h  = compute_half_carry_add(v, 1)
            self._flag_pv = (v == 0x7F)
            self._flag_n  = False
            self._set_sz(r)
            return "INC r"

        if op & 0xC7 == 0x05:   # DEC r
            r_code = (op >> 3) & 0x07
            v = self._get_r(r_code)
            r = (v - 1) & 0xFF
            self._set_r(r_code, r)
            self._flag_h  = compute_half_carry_sub(v, 1)
            self._flag_pv = (v == 0x80)
            self._flag_n  = True
            self._set_sz(r)
            return "DEC r"

        # ── 16-bit load ───────────────────────────────────────────────────────

        if op & 0xCF == 0x01:   # LD rp, nn
            rp = (op >> 4) & 0x03
            nn = self._fetch16()
            self._set_rp(rp, nn)
            return f"LD rp,{nn:#06x}"

        if op == 0xF9:          # LD SP, HL
            self._sp = (self._h << 8) | self._l
            return "LD SP,HL"

        if op == 0x2A:          # LD HL, (nn)
            nn = self._fetch16()
            self._l = self._read(nn)
            self._h = self._read((nn + 1) & 0xFFFF)
            return f"LD HL,({nn:#06x})"

        if op == 0x22:          # LD (nn), HL
            nn = self._fetch16()
            self._write(nn, self._l)
            self._write((nn + 1) & 0xFFFF, self._h)
            return f"LD ({nn:#06x}),HL"

        if op == 0x3A:          # LD A, (nn)
            nn = self._fetch16()
            self._a = self._read(nn)
            return f"LD A,({nn:#06x})"

        if op == 0x32:          # LD (nn), A
            nn = self._fetch16()
            self._write(nn, self._a)
            return f"LD ({nn:#06x}),A"

        if op == 0x0A:          # LD A, (BC)
            self._a = self._read((self._b << 8) | self._c)
            return "LD A,(BC)"

        if op == 0x1A:          # LD A, (DE)
            self._a = self._read((self._d << 8) | self._e)
            return "LD A,(DE)"

        if op == 0x02:          # LD (BC), A
            self._write((self._b << 8) | self._c, self._a)
            return "LD (BC),A"

        if op == 0x12:          # LD (DE), A
            self._write((self._d << 8) | self._e, self._a)
            return "LD (DE),A"

        # ── PUSH / POP ────────────────────────────────────────────────────────

        if op & 0xCF == 0xC5:   # PUSH rp
            rp = (op >> 4) & 0x03
            self._push16(self._get_rp_af(rp))
            return "PUSH rp"

        if op & 0xCF == 0xC1:   # POP rp
            rp = (op >> 4) & 0x03
            self._set_rp_af(rp, self._pop16())
            return "POP rp"

        # ── Exchange ──────────────────────────────────────────────────────────

        if op == 0xEB:   # EX DE, HL
            self._d, self._h = self._h, self._d
            self._e, self._l = self._l, self._e
            return "EX DE,HL"

        if op == 0x08:   # EX AF, AF'
            self._a, self._a2 = self._a2, self._a
            f_cur = self._f_byte()
            s2, z2, h2, pv2, n2, c2 = unpack_f(self._f2)
            self._f2 = f_cur
            self._flag_s, self._flag_z, self._flag_h = s2, z2, h2
            self._flag_pv, self._flag_n, self._flag_c = pv2, n2, c2
            return "EX AF,AF'"

        if op == 0xD9:   # EXX  (swap BC/DE/HL with BC'/DE'/HL')
            self._b, self._b2 = self._b2, self._b
            self._c, self._c2 = self._c2, self._c
            self._d, self._d2 = self._d2, self._d
            self._e, self._e2 = self._e2, self._e
            self._h, self._h2 = self._h2, self._h
            self._l, self._l2 = self._l2, self._l
            return "EXX"

        if op == 0xE3:   # EX (SP), HL
            lo = self._read(self._sp)
            hi = self._read((self._sp + 1) & 0xFFFF)
            self._write(self._sp, self._l)
            self._write((self._sp + 1) & 0xFFFF, self._h)
            self._h, self._l = hi, lo
            return "EX (SP),HL"

        # ── 16-bit arithmetic ─────────────────────────────────────────────────

        if op & 0xCF == 0x09:   # ADD HL, rp
            rp = (op >> 4) & 0x03
            hl = (self._h << 8) | self._l
            rp_val = self._get_rp(rp)
            result = hl + rp_val
            self._flag_c = result > 0xFFFF
            self._flag_h = ((hl & 0x0FFF) + (rp_val & 0x0FFF)) > 0x0FFF
            self._flag_n = False
            hl_new = result & 0xFFFF
            self._h, self._l = hl_new >> 8, hl_new & 0xFF
            return "ADD HL,rp"

        if op & 0xCF == 0x03:   # INC rp
            rp = (op >> 4) & 0x03
            self._set_rp(rp, (self._get_rp(rp) + 1) & 0xFFFF)
            return "INC rp"

        if op & 0xCF == 0x0B:   # DEC rp
            rp = (op >> 4) & 0x03
            self._set_rp(rp, (self._get_rp(rp) - 1) & 0xFFFF)
            return "DEC rp"

        # ── Rotate accumulator ────────────────────────────────────────────────

        if op == 0x07:   # RLCA
            c = (self._a >> 7) & 1
            self._a = ((self._a << 1) | c) & 0xFF
            self._flag_c = bool(c)
            self._flag_h = False
            self._flag_n = False
            return "RLCA"

        if op == 0x0F:   # RRCA
            c = self._a & 1
            self._a = ((c << 7) | (self._a >> 1)) & 0xFF
            self._flag_c = bool(c)
            self._flag_h = False
            self._flag_n = False
            return "RRCA"

        if op == 0x17:   # RLA
            c_in = int(self._flag_c)
            self._flag_c = bool(self._a & 0x80)
            self._a = ((self._a << 1) | c_in) & 0xFF
            self._flag_h = False
            self._flag_n = False
            return "RLA"

        if op == 0x1F:   # RRA
            c_in = int(self._flag_c)
            self._flag_c = bool(self._a & 0x01)
            self._a = ((c_in << 7) | (self._a >> 1)) & 0xFF
            self._flag_h = False
            self._flag_n = False
            return "RRA"

        # ── DAA ───────────────────────────────────────────────────────────────

        if op == 0x27:   # DAA
            new_a, new_h, new_pv, new_c = daa(
                self._a, self._flag_n, self._flag_h, self._flag_c
            )
            self._a = new_a
            self._flag_h = new_h
            self._flag_pv = new_pv
            self._flag_c = new_c
            self._set_sz(new_a)
            return "DAA"

        # ── Miscellaneous accumulator ─────────────────────────────────────────

        if op == 0x2F:   # CPL (complement A)
            self._a ^= 0xFF
            self._flag_h = True
            self._flag_n = True
            return "CPL"

        if op == 0x3F:   # CCF (complement carry)
            self._flag_h = self._flag_c
            self._flag_c = not self._flag_c
            self._flag_n = False
            return "CCF"

        if op == 0x37:   # SCF (set carry)
            self._flag_c = True
            self._flag_h = False
            self._flag_n = False
            return "SCF"

        # ── Jumps ─────────────────────────────────────────────────────────────

        if op == 0xC3:   # JP nn
            self._pc = self._fetch16()
            return f"JP {self._pc:#06x}"

        if op & 0xC7 == 0xC2:   # JP cc, nn
            cc = (op >> 3) & 0x07
            nn = self._fetch16()
            if self._cond(cc):
                self._pc = nn
            return f"JP cc,{nn:#06x}"

        if op == 0xE9:   # JP (HL)
            self._pc = (self._h << 8) | self._l
            return "JP (HL)"

        if op == 0x18:   # JR e
            e = self._fetch_signed()
            self._pc = (self._pc + e) & 0xFFFF
            return f"JR {e:+d}"

        if op == 0x20:   # JR NZ, e
            e = self._fetch_signed()
            if not self._flag_z:
                self._pc = (self._pc + e) & 0xFFFF
            return f"JR NZ,{e:+d}"

        if op == 0x28:   # JR Z, e
            e = self._fetch_signed()
            if self._flag_z:
                self._pc = (self._pc + e) & 0xFFFF
            return f"JR Z,{e:+d}"

        if op == 0x30:   # JR NC, e
            e = self._fetch_signed()
            if not self._flag_c:
                self._pc = (self._pc + e) & 0xFFFF
            return f"JR NC,{e:+d}"

        if op == 0x38:   # JR C, e
            e = self._fetch_signed()
            if self._flag_c:
                self._pc = (self._pc + e) & 0xFFFF
            return f"JR C,{e:+d}"

        if op == 0x10:   # DJNZ e
            e = self._fetch_signed()
            self._b = (self._b - 1) & 0xFF
            if self._b != 0:
                self._pc = (self._pc + e) & 0xFFFF
            return f"DJNZ {e:+d}"

        # ── Call / Return ─────────────────────────────────────────────────────

        if op == 0xCD:   # CALL nn
            nn = self._fetch16()
            self._push16(self._pc)
            self._pc = nn
            return f"CALL {nn:#06x}"

        if op & 0xC7 == 0xC4:   # CALL cc, nn
            cc = (op >> 3) & 0x07
            nn = self._fetch16()
            if self._cond(cc):
                self._push16(self._pc)
                self._pc = nn
            return f"CALL cc,{nn:#06x}"

        if op == 0xC9:   # RET
            self._pc = self._pop16()
            return "RET"

        if op & 0xC7 == 0xC0:   # RET cc
            cc = (op >> 3) & 0x07
            if self._cond(cc):
                self._pc = self._pop16()
            return "RET cc"

        # ── RST ───────────────────────────────────────────────────────────────

        if op & 0xC7 == 0xC7:   # RST p
            p = op & 0x38
            self._push16(self._pc)
            self._pc = p
            return f"RST {p:#04x}"

        # ── I/O ───────────────────────────────────────────────────────────────

        if op == 0xD3:   # OUT (n), A
            n = self._fetch()
            self._output_ports[n] = self._a
            return f"OUT ({n:#04x}),A"

        if op == 0xDB:   # IN A, (n)
            n = self._fetch()
            self._a = self._input_ports[n]
            return f"IN A,({n:#04x})"

        # ── Interrupt control ─────────────────────────────────────────────────

        if op == 0xF3:   # DI
            self._iff1 = False
            self._iff2 = False
            return "DI"

        if op == 0xFB:   # EI
            self._iff1 = True
            self._iff2 = True
            return "EI"

        return f"??{op:#04x}"

    # ── CB-prefix: bit manipulation and rotate/shift ──────────────────────────

    def _exec_cb(self) -> str:
        op = self._fetch()
        r_code = op & 0x07
        v = self._get_r(r_code)

        rot_op = (op >> 3) & 0x07
        bit = (op >> 3) & 0x07

        if op < 0x40:  # Rotate / shift
            if rot_op == 0:   # RLC
                c = (v >> 7) & 1
                r = ((v << 1) | c) & 0xFF
                self._flag_c = bool(c)
            elif rot_op == 1:  # RRC
                c = v & 1
                r = ((c << 7) | (v >> 1)) & 0xFF
                self._flag_c = bool(c)
            elif rot_op == 2:  # RL
                c = (v >> 7) & 1
                r = ((v << 1) | int(self._flag_c)) & 0xFF
                self._flag_c = bool(c)
            elif rot_op == 3:  # RR
                c = v & 1
                r = ((int(self._flag_c) << 7) | (v >> 1)) & 0xFF
                self._flag_c = bool(c)
            elif rot_op == 4:  # SLA
                c = (v >> 7) & 1
                r = (v << 1) & 0xFF
                self._flag_c = bool(c)
            elif rot_op == 5:  # SRA (arithmetic: bit 7 preserved)
                c = v & 1
                r = ((v & 0x80) | (v >> 1)) & 0xFF
                self._flag_c = bool(c)
            elif rot_op == 6:  # SLL (undocumented: shifts in 1)
                c = (v >> 7) & 1
                r = ((v << 1) | 1) & 0xFF
                self._flag_c = bool(c)
            else:   # SRL (logical: 0 shifted in)
                c = v & 1
                r = (v >> 1) & 0xFF
                self._flag_c = bool(c)
            self._set_r(r_code, r)
            self._flag_h = False
            self._flag_n = False
            self._set_szpn(r)
            return f"ROT{rot_op} r"

        elif op < 0x80:  # BIT b, r
            self._flag_z = not bool(v & (1 << bit))
            self._flag_h = True
            self._flag_n = False
            return f"BIT {bit},r"

        elif op < 0xC0:  # RES b, r
            r = v & ~(1 << bit)
            self._set_r(r_code, r)
            return f"RES {bit},r"

        else:  # SET b, r
            r = v | (1 << bit)
            self._set_r(r_code, r)
            return f"SET {bit},r"

    # ── ED-prefix: extended instructions ─────────────────────────────────────

    def _exec_ed(self) -> str:  # noqa: PLR0912, PLR0915
        op = self._fetch()

        # ── LD A, I / LD A, R / LD I, A / LD R, A ───────────────────────────
        if op == 0x57:   # LD A, I
            self._a = self._i
            self._flag_s, self._flag_z = compute_sz(self._i)
            self._flag_h = False
            self._flag_n = False
            self._flag_pv = self._iff2
            return "LD A,I"

        if op == 0x5F:   # LD A, R
            self._a = self._r
            self._flag_s, self._flag_z = compute_sz(self._r)
            self._flag_h = False
            self._flag_n = False
            self._flag_pv = self._iff2
            return "LD A,R"

        if op == 0x47:   # LD I, A
            self._i = self._a
            return "LD I,A"

        if op == 0x4F:   # LD R, A
            self._r = self._a
            return "LD R,A"

        # ── 16-bit register load ──────────────────────────────────────────────
        if op & 0xCF == 0x4B:   # LD rp, (nn)
            rp = (op >> 4) & 0x03
            nn = self._fetch16()
            val = self._read16(nn)
            self._set_rp(rp, val)
            return f"LD rp,({nn:#06x})"

        if op & 0xCF == 0x43:   # LD (nn), rp
            rp = (op >> 4) & 0x03
            nn = self._fetch16()
            self._write16(nn, self._get_rp(rp))
            return f"LD ({nn:#06x}),rp"

        # ── 16-bit arithmetic with carry ──────────────────────────────────────
        if op & 0xCF == 0x4A:   # ADC HL, rp
            rp = (op >> 4) & 0x03
            hl = (self._h << 8) | self._l
            rp_v = self._get_rp(rp)
            c = int(self._flag_c)
            total = hl + rp_v + c
            r = total & 0xFFFF
            self._flag_c  = total > 0xFFFF
            self._flag_h  = ((hl & 0x0FFF) + (rp_v & 0x0FFF) + c) > 0x0FFF
            self._flag_pv = compute_overflow_add(hl >> 8, rp_v >> 8, r >> 8)
            self._flag_n  = False
            self._flag_s  = bool(r & 0x8000)
            self._flag_z  = r == 0
            self._h, self._l = r >> 8, r & 0xFF
            return "ADC HL,rp"

        if op & 0xCF == 0x42:   # SBC HL, rp
            rp = (op >> 4) & 0x03
            hl = (self._h << 8) | self._l
            rp_v = self._get_rp(rp)
            borrow = int(self._flag_c)
            total = hl - rp_v - borrow
            r = total & 0xFFFF
            self._flag_c  = total < 0
            self._flag_h  = (hl & 0x0FFF) < (rp_v & 0x0FFF) + borrow
            self._flag_pv = compute_overflow_sub(hl >> 8, rp_v >> 8, r >> 8)
            self._flag_n  = True
            self._flag_s  = bool(r & 0x8000)
            self._flag_z  = r == 0
            self._h, self._l = r >> 8, r & 0xFF
            return "SBC HL,rp"

        # ── NEG ───────────────────────────────────────────────────────────────
        if op == 0x44:   # NEG
            a = self._a
            r = (-a) & 0xFF
            self._flag_c  = a != 0
            self._flag_h  = (a & 0x0F) != 0
            self._flag_pv = a == 0x80
            self._flag_n  = True
            self._a = r
            self._set_sz(r)
            return "NEG"

        # ── Interrupt mode ────────────────────────────────────────────────────
        if op == 0x46:
            self._im = 0
            return "IM 0"
        if op == 0x56:
            self._im = 1
            return "IM 1"
        if op == 0x5E:
            self._im = 2
            return "IM 2"

        # ── RETI / RETN ───────────────────────────────────────────────────────
        if op == 0x4D:   # RETI
            self._iff1 = self._iff2
            self._pc = self._pop16()
            return "RETI"

        if op == 0x45:   # RETN
            self._iff1 = self._iff2
            self._pc = self._pop16()
            return "RETN"

        # ── RLD / RRD ─────────────────────────────────────────────────────────
        if op == 0x6F:   # RLD
            hl_addr = (self._h << 8) | self._l
            m = self._read(hl_addr)
            new_m = ((m << 4) | (self._a & 0x0F)) & 0xFF
            self._a = (self._a & 0xF0) | (m >> 4)
            self._write(hl_addr, new_m)
            self._flag_h = False
            self._flag_n = False
            self._set_szpn(self._a)
            return "RLD"

        if op == 0x67:   # RRD
            hl_addr = (self._h << 8) | self._l
            m = self._read(hl_addr)
            new_m = ((self._a & 0x0F) << 4) | (m >> 4)
            self._a = (self._a & 0xF0) | (m & 0x0F)
            self._write(hl_addr, new_m)
            self._flag_h = False
            self._flag_n = False
            self._set_szpn(self._a)
            return "RRD"

        # ── IN r, (C) / OUT (C), r ────────────────────────────────────────────
        if op & 0xC7 == 0x40:   # IN r, (C)
            r_code = (op >> 3) & 0x07
            val = self._input_ports[self._c]
            if r_code != 6:
                self._set_r(r_code, val)
            self._flag_h = False
            self._flag_n = False
            self._set_szpn(val)
            return "IN r,(C)"

        if op & 0xC7 == 0x41:   # OUT (C), r
            r_code = (op >> 3) & 0x07
            val = self._get_r(r_code) if r_code != 6 else 0
            self._output_ports[self._c] = val
            return "OUT (C),r"

        # ── Block operations ──────────────────────────────────────────────────
        if op == 0xA0:
            return self._ldi()
        if op == 0xA8:
            return self._ldd()
        if op == 0xB0:
            return self._ldir()
        if op == 0xB8:
            return self._lddr()
        if op == 0xA1:
            return self._cpi()
        if op == 0xA9:
            return self._cpd()
        if op == 0xB1:
            return self._cpir()
        if op == 0xB9:
            return self._cpdr()
        if op == 0xA2:
            return self._ini()
        if op == 0xAA:
            return self._ind()
        if op == 0xB2:
            return self._inir()
        if op == 0xBA:
            return self._indr()
        if op == 0xA3:
            return self._outi()
        if op == 0xAB:
            return self._outd()
        if op == 0xB3:
            return self._otir()
        if op == 0xBB:
            return self._otdr()

        return f"ED {op:#04x}"

    # ── DD/FD prefix: index register (IX or IY) instructions ─────────────────

    def _exec_ddfd(self, use_ix: bool) -> str:  # noqa: PLR0912
        """Handle DD-prefixed (IX) or FD-prefixed (IY) instructions."""
        idx_val = self._ix if use_ix else self._iy
        prefix = "IX" if use_ix else "IY"

        op = self._fetch()

        # DDCB / FDCB prefix
        if op == 0xCB:
            return self._exec_ddcb(idx_val, prefix)

        # LD (IX+d), n
        if op == 0x36:
            d = self._fetch_signed()
            n = self._fetch()
            self._write((idx_val + d) & 0xFFFF, n)
            return f"LD ({prefix}{d:+d}),{n:#04x}"

        # LD r, (IX+d)
        if op & 0xF8 == 0x46 or (0x40 <= op <= 0x7F and op != 0x76):
            dst = (op >> 3) & 0x07
            src = op & 0x07
            if src == 6:
                d = self._fetch_signed()
                val = self._read((idx_val + d) & 0xFFFF)
                self._set_r(dst, val)
                return f"LD r,({prefix}{d:+d})"
            if dst == 6:
                d = self._fetch_signed()
                val = self._get_r(src)
                self._write((idx_val + d) & 0xFFFF, val)
                return f"LD ({prefix}{d:+d}),r"

        # LD IX, nn
        if op == 0x21:
            nn = self._fetch16()
            if use_ix:
                self._ix = nn
            else:
                self._iy = nn
            return f"LD {prefix},{nn:#06x}"

        # LD IX, (nn)
        if op == 0x2A:
            nn = self._fetch16()
            val = self._read16(nn)
            if use_ix:
                self._ix = val
            else:
                self._iy = val
            return f"LD {prefix},({nn:#06x})"

        # LD (nn), IX
        if op == 0x22:
            nn = self._fetch16()
            self._write16(nn, idx_val)
            return f"LD ({nn:#06x}),{prefix}"

        # LD SP, IX
        if op == 0xF9:
            self._sp = idx_val
            return f"LD SP,{prefix}"

        # PUSH IX
        if op == 0xE5:
            self._push16(idx_val)
            return f"PUSH {prefix}"

        # POP IX
        if op == 0xE1:
            val = self._pop16()
            if use_ix:
                self._ix = val
            else:
                self._iy = val
            return f"POP {prefix}"

        # ADD IX, rp
        if op & 0xCF == 0x09:
            rp = (op >> 4) & 0x03
            rp_val = self._get_rp(rp) if rp != 2 else idx_val
            total = idx_val + rp_val
            self._flag_c = total > 0xFFFF
            self._flag_h = ((idx_val & 0x0FFF) + (rp_val & 0x0FFF)) > 0x0FFF
            self._flag_n = False
            new_val = total & 0xFFFF
            if use_ix:
                self._ix = new_val
            else:
                self._iy = new_val
            return f"ADD {prefix},rp"

        # INC IX
        if op == 0x23:
            if use_ix:
                self._ix = (self._ix + 1) & 0xFFFF
            else:
                self._iy = (self._iy + 1) & 0xFFFF
            return f"INC {prefix}"

        # DEC IX
        if op == 0x2B:
            if use_ix:
                self._ix = (self._ix - 1) & 0xFFFF
            else:
                self._iy = (self._iy - 1) & 0xFFFF
            return f"DEC {prefix}"

        # INC (IX+d)
        if op == 0x34:
            d = self._fetch_signed()
            addr = (idx_val + d) & 0xFFFF
            v = self._read(addr)
            r = (v + 1) & 0xFF
            self._write(addr, r)
            self._flag_h = compute_half_carry_add(v, 1)
            self._flag_pv = v == 0x7F
            self._flag_n = False
            self._set_sz(r)
            return f"INC ({prefix}{d:+d})"

        # DEC (IX+d)
        if op == 0x35:
            d = self._fetch_signed()
            addr = (idx_val + d) & 0xFFFF
            v = self._read(addr)
            r = (v - 1) & 0xFF
            self._write(addr, r)
            self._flag_h = compute_half_carry_sub(v, 1)
            self._flag_pv = v == 0x80
            self._flag_n = True
            self._set_sz(r)
            return f"DEC ({prefix}{d:+d})"

        # ALU ops with (IX+d)
        if 0x86 <= op <= 0xBE and (op & 0x07) == 0x06:
            alu_op = (op >> 3) & 0x07
            d = self._fetch_signed()
            val = self._read((idx_val + d) & 0xFFFF)
            self._alu8(alu_op, val)
            return f"ALU ({prefix}{d:+d})"

        # JP (IX)
        if op == 0xE9:
            self._pc = idx_val
            return f"JP ({prefix})"

        # EX (SP), IX
        if op == 0xE3:
            lo = self._read(self._sp)
            hi = self._read((self._sp + 1) & 0xFFFF)
            self._write(self._sp, idx_val & 0xFF)
            self._write((self._sp + 1) & 0xFFFF, (idx_val >> 8) & 0xFF)
            if use_ix:
                self._ix = (hi << 8) | lo
            else:
                self._iy = (hi << 8) | lo
            return f"EX (SP),{prefix}"

        return f"DD/FD {op:#04x}"

    def _exec_ddcb(self, idx_val: int, prefix: str) -> str:
        """Handle DDCB / FDCB prefixed bit instructions on (IX+d)/(IY+d)."""
        d = self._fetch_signed()
        op = self._fetch()
        addr = (idx_val + d) & 0xFFFF
        v = self._read(addr)
        bit = (op >> 3) & 0x07
        r_code = op & 0x07

        if op < 0x40:  # rotate/shift (IX+d)
            rot_op = (op >> 3) & 0x07
            if rot_op == 0:
                c = (v >> 7) & 1
                r = ((v << 1) | c) & 0xFF
                self._flag_c = bool(c)
            elif rot_op == 1:
                c = v & 1
                r = ((c << 7) | (v >> 1)) & 0xFF
                self._flag_c = bool(c)
            elif rot_op == 2:
                c = (v >> 7) & 1
                r = ((v << 1) | int(self._flag_c)) & 0xFF
                self._flag_c = bool(c)
            elif rot_op == 3:
                c = v & 1
                r = ((int(self._flag_c) << 7) | (v >> 1)) & 0xFF
                self._flag_c = bool(c)
            elif rot_op == 4:
                c = (v >> 7) & 1
                r = (v << 1) & 0xFF
                self._flag_c = bool(c)
            elif rot_op == 5:
                c = v & 1
                r = ((v & 0x80) | (v >> 1)) & 0xFF
                self._flag_c = bool(c)
            elif rot_op == 6:
                c = (v >> 7) & 1
                r = ((v << 1) | 1) & 0xFF
                self._flag_c = bool(c)
            else:
                c = v & 1
                r = (v >> 1) & 0xFF
                self._flag_c = bool(c)
            self._write(addr, r)
            if r_code != 6:
                self._set_r(r_code, r)
            self._flag_h = False
            self._flag_n = False
            self._set_szpn(r)
            return f"ROT {prefix}{d:+d}"

        elif op < 0x80:  # BIT b, (IX+d)
            self._flag_z = not bool(v & (1 << bit))
            self._flag_h = True
            self._flag_n = False
            return f"BIT {bit},({prefix}{d:+d})"

        elif op < 0xC0:  # RES b, (IX+d)
            r = v & ~(1 << bit)
            self._write(addr, r)
            if r_code != 6:
                self._set_r(r_code, r)
            return f"RES {bit},({prefix}{d:+d})"

        else:  # SET b, (IX+d)
            r = v | (1 << bit)
            self._write(addr, r)
            if r_code != 6:
                self._set_r(r_code, r)
            return f"SET {bit},({prefix}{d:+d})"

    # ── Block operations ──────────────────────────────────────────────────────

    def _ldi(self) -> str:
        """LDI: (DE) ← (HL)
        HL++
        DE++
        BC--. P/V set if BC≠0 after."""
        src = (self._h << 8) | self._l
        dst = (self._d << 8) | self._e
        self._write(dst, self._read(src))
        hl = (src + 1) & 0xFFFF
        self._h, self._l = hl >> 8, hl & 0xFF
        de = (dst + 1) & 0xFFFF
        self._d, self._e = de >> 8, de & 0xFF
        bc = ((self._b << 8) | self._c) - 1
        bc &= 0xFFFF
        self._b, self._c = bc >> 8, bc & 0xFF
        self._flag_h = False
        self._flag_n = False
        self._flag_pv = bc != 0
        return "LDI"

    def _ldd(self) -> str:
        """LDD: like LDI but HL--, DE-- instead."""
        src = (self._h << 8) | self._l
        dst = (self._d << 8) | self._e
        self._write(dst, self._read(src))
        hl = (src - 1) & 0xFFFF
        self._h, self._l = hl >> 8, hl & 0xFF
        de = (dst - 1) & 0xFFFF
        self._d, self._e = de >> 8, de & 0xFF
        bc = ((self._b << 8) | self._c) - 1
        bc &= 0xFFFF
        self._b, self._c = bc >> 8, bc & 0xFF
        self._flag_h = False
        self._flag_n = False
        self._flag_pv = bc != 0
        return "LDD"

    def _ldir(self) -> str:
        """LDIR: repeat LDI until BC=0."""
        while True:
            self._ldi()
            if ((self._b << 8) | self._c) == 0:
                break
        return "LDIR"

    def _lddr(self) -> str:
        """LDDR: repeat LDD until BC=0."""
        while True:
            self._ldd()
            if ((self._b << 8) | self._c) == 0:
                break
        return "LDDR"

    def _cpi(self) -> str:
        """CPI: compare A with (HL)
        HL++
        BC--."""
        hl = (self._h << 8) | self._l
        m = self._read(hl)
        result = (self._a - m) & 0xFF
        hl = (hl + 1) & 0xFFFF
        self._h, self._l = hl >> 8, hl & 0xFF
        bc = ((self._b << 8) | self._c) - 1
        bc &= 0xFFFF
        self._b, self._c = bc >> 8, bc & 0xFF
        self._flag_h = compute_half_carry_sub(self._a, m)
        self._flag_n = True
        self._flag_pv = bc != 0
        self._set_sz(result)
        return "CPI"

    def _cpd(self) -> str:
        """CPD: like CPI but HL-- instead."""
        hl = (self._h << 8) | self._l
        m = self._read(hl)
        result = (self._a - m) & 0xFF
        hl = (hl - 1) & 0xFFFF
        self._h, self._l = hl >> 8, hl & 0xFF
        bc = ((self._b << 8) | self._c) - 1
        bc &= 0xFFFF
        self._b, self._c = bc >> 8, bc & 0xFF
        self._flag_h = compute_half_carry_sub(self._a, m)
        self._flag_n = True
        self._flag_pv = bc != 0
        self._set_sz(result)
        return "CPD"

    def _cpir(self) -> str:
        """CPIR: repeat CPI until match (Z=1) or BC=0."""
        while True:
            self._cpi()
            bc = (self._b << 8) | self._c
            if self._flag_z or bc == 0:
                break
        return "CPIR"

    def _cpdr(self) -> str:
        """CPDR: repeat CPD until match or BC=0."""
        while True:
            self._cpd()
            bc = (self._b << 8) | self._c
            if self._flag_z or bc == 0:
                break
        return "CPDR"

    def _ini(self) -> str:
        """INI: (HL) ← port(C)
        HL++
        B--."""
        val = self._input_ports[self._c]
        self._write((self._h << 8) | self._l, val)
        hl = (((self._h << 8) | self._l) + 1) & 0xFFFF
        self._h, self._l = hl >> 8, hl & 0xFF
        self._b = (self._b - 1) & 0xFF
        self._flag_n = True
        self._flag_z = self._b == 0
        return "INI"

    def _ind(self) -> str:
        """IND: (HL) ← port(C)
        HL--
        B--."""
        val = self._input_ports[self._c]
        self._write((self._h << 8) | self._l, val)
        hl = (((self._h << 8) | self._l) - 1) & 0xFFFF
        self._h, self._l = hl >> 8, hl & 0xFF
        self._b = (self._b - 1) & 0xFF
        self._flag_n = True
        self._flag_z = self._b == 0
        return "IND"

    def _inir(self) -> str:
        """INIR: repeat INI until B=0."""
        while True:
            self._ini()
            if self._b == 0:
                break
        return "INIR"

    def _indr(self) -> str:
        """INDR: repeat IND until B=0."""
        while True:
            self._ind()
            if self._b == 0:
                break
        return "INDR"

    def _outi(self) -> str:
        """OUTI: port(C) ← (HL)
        HL++
        B--."""
        val = self._read((self._h << 8) | self._l)
        self._output_ports[self._c] = val
        hl = (((self._h << 8) | self._l) + 1) & 0xFFFF
        self._h, self._l = hl >> 8, hl & 0xFF
        self._b = (self._b - 1) & 0xFF
        self._flag_n = True
        self._flag_z = self._b == 0
        return "OUTI"

    def _outd(self) -> str:
        """OUTD: port(C) ← (HL)
        HL--
        B--."""
        val = self._read((self._h << 8) | self._l)
        self._output_ports[self._c] = val
        hl = (((self._h << 8) | self._l) - 1) & 0xFFFF
        self._h, self._l = hl >> 8, hl & 0xFF
        self._b = (self._b - 1) & 0xFF
        self._flag_n = True
        self._flag_z = self._b == 0
        return "OUTD"

    def _otir(self) -> str:
        """OTIR: repeat OUTI until B=0."""
        while True:
            self._outi()
            if self._b == 0:
                break
        return "OTIR"

    def _otdr(self) -> str:
        """OTDR: repeat OUTD until B=0."""
        while True:
            self._outd()
            if self._b == 0:
                break
        return "OTDR"
