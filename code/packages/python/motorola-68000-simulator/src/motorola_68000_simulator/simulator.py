"""Motorola 68000 (1979) behavioral simulator.

──────────────────────────────────────────────────────────────────────────────
ARCHITECTURE SUMMARY
──────────────────────────────────────────────────────────────────────────────

The 68000 is a 16/32-bit CPU with:
  • 8 data registers  D0–D7  (32-bit; byte/word/long ops)
  • 8 address registers A0–A7 (32-bit; A7 = supervisor stack pointer)
  • 24-bit flat linear address space (16 MB)
  • Big-endian byte order (MSB at lowest address)
  • 14 orthogonal addressing modes
  • 5 condition codes: X N Z V C

Register usage conventions (ABI, relevant for LINK/UNLK tests):
  D0–D1, A0–A1  — scratch / return values
  D2–D7, A2–A6  — callee-saved (subroutine must preserve)
  A7  (SP)      — supervisor stack pointer (grows downward)
  A6  (FP)      — frame pointer by convention (LINK/UNLK)

──────────────────────────────────────────────────────────────────────────────
INSTRUCTION WORD LAYOUT
──────────────────────────────────────────────────────────────────────────────

Every instruction starts with a 16-bit opword.  Bits 15–12 give a rough
category (not a complete opcode):

  0000  immediate / bit ops (ORI, ANDI, SUBI, ADDI, EORI, CMPI, BTST…)
  0001  MOVE.B
  0010  MOVE.L
  0011  MOVE.W
  0100  miscellaneous (NEG, CLR, TST, LEA, PEA, SWAP, EXT, JSR, JMP…)
  0101  ADDQ, SUBQ, DBcc, Scc
  0110  BRA, BSR, Bcc
  0111  MOVEQ
  1000  OR, DIVU, DIVS
  1001  SUB, SUBA, SUBX
  1010  (unimplemented line A trap)
  1011  CMP, CMPA, EOR
  1100  AND, MULU, MULS, EXG
  1101  ADD, ADDA, ADDX
  1110  shift/rotate family
  1111  (co-processor / line F trap)

──────────────────────────────────────────────────────────────────────────────
EFFECTIVE ADDRESS (EA) ENCODING
──────────────────────────────────────────────────────────────────────────────

6-bit EA field = mode[2:0] : reg[2:0]

  mode  reg   Notation        Description
  000   Dn    Dn              Data register direct
  001   An    An              Address register direct
  010   An    (An)            Address register indirect
  011   An    (An)+           Indirect with postincrement
  100   An    -(An)           Indirect with predecrement
  101   An    d16(An)         Indirect + signed 16-bit displacement
  110   An    d8(An,Xn.sz)    Indirect + index register + 8-bit disp
  111   000   (abs).W         Absolute short (sign-extended 16-bit)
  111   001   (abs).L         Absolute long (32-bit)
  111   010   d16(PC)         PC-relative + 16-bit displacement
  111   011   d8(PC,Xn.sz)    PC-relative + index
  111   100   #imm            Immediate data

──────────────────────────────────────────────────────────────────────────────
SIZE CODES
──────────────────────────────────────────────────────────────────────────────

Most instructions (ADD, SUB, AND, OR, EOR, CMP, shifts, etc.):
  00 = byte (8-bit)
  01 = word (16-bit)
  10 = long (32-bit)

MOVE instruction (different encoding):
  01 = byte
  11 = word
  10 = long

──────────────────────────────────────────────────────────────────────────────
STACK OPERATIONS AND BYTE SIZE
──────────────────────────────────────────────────────────────────────────────

On the real 68000, the stack pointer (A7/SSP) is always kept word-aligned.
Even byte pushes/pops bump SP by 2 (not 1).  We model this: for A7, the
predecrement and postincrement amounts are max(size, 2).

──────────────────────────────────────────────────────────────────────────────
"""

from __future__ import annotations

from simulator_protocol import ExecutionResult, StepTrace

from motorola_68000_simulator.flags import (
    compute_c_sub,
    compute_n,
    compute_nz_logic,
    compute_nzvc_add,
    compute_nzvc_neg,
    compute_nzvc_sub,
    compute_v_add,
    compute_v_sub,
    compute_z,
)
from motorola_68000_simulator.state import M68KState

# ── Constants ─────────────────────────────────────────────────────────────────

_LOAD_ADDR  = 0x001000   # programs are loaded starting here
_INIT_SP    = 0x00F000   # initial supervisor stack pointer
_ADDR_MASK  = 0x00FF_FFFF
_LONG_MASK  = 0xFFFF_FFFF
_WORD_MASK  = 0x0000_FFFF
_BYTE_MASK  = 0x0000_00FF
_LONG_MSB   = 0x8000_0000
_WORD_MSB   = 0x0000_8000
_BYTE_MSB   = 0x0000_0080
_MEM_SIZE   = 0x100_0000   # 16 MB

# Size code tables (opword bits 7-6 for most instructions)
_SZ_ARITH   = {0: 1, 1: 2, 2: 4}   # 00=byte, 01=word, 10=long
_SZ_MASK    = {1: _BYTE_MASK, 2: _WORD_MASK, 4: _LONG_MASK}
_SZ_MSB     = {1: _BYTE_MSB,  2: _WORD_MSB,  4: _LONG_MSB}

# MOVE instruction uses a different size encoding
_SZ_MOVE    = {1: 1, 3: 2, 2: 4}   # 01=byte, 11=word, 10=long

# Condition code check functions (for Bcc / DBcc / Scc)
# Each takes (N, Z, V, C) booleans and returns True if branch taken.
# N = (sr >> 3) & 1, Z = (sr >> 2) & 1, V = (sr >> 1) & 1, C = sr & 1
def _cc_t(n, z, v, c):   return True
def _cc_f(n, z, v, c):   return False
def _cc_hi(n, z, v, c):  return (not c) and (not z)
def _cc_ls(n, z, v, c):  return c or z
def _cc_cc(n, z, v, c):  return not c
def _cc_cs(n, z, v, c):  return c
def _cc_ne(n, z, v, c):  return not z
def _cc_eq(n, z, v, c):  return z
def _cc_vc(n, z, v, c):  return not v
def _cc_vs(n, z, v, c):  return v
def _cc_pl(n, z, v, c):  return not n
def _cc_mi(n, z, v, c):  return n
def _cc_ge(n, z, v, c):  return n == v
def _cc_lt(n, z, v, c):  return n != v
def _cc_gt(n, z, v, c):  return (not z) and (n == v)
def _cc_le(n, z, v, c):  return z or (n != v)

_CC_FUNCS = [
    _cc_t,  _cc_f,  _cc_hi, _cc_ls,
    _cc_cc, _cc_cs, _cc_ne, _cc_eq,
    _cc_vc, _cc_vs, _cc_pl, _cc_mi,
    _cc_ge, _cc_lt, _cc_gt, _cc_le,
]
_CC_NAMES = [
    "T", "F", "HI", "LS",
    "CC", "CS", "NE", "EQ",
    "VC", "VS", "PL", "MI",
    "GE", "LT", "GT", "LE",
]


# ── Helper utilities ──────────────────────────────────────────────────────────

def _sign_extend(value: int, bits: int) -> int:
    """Sign-extend `value` from `bits`-bit signed to Python int.

    >>> _sign_extend(0xFF, 8)   # -1
    -1
    >>> _sign_extend(0x7F, 8)   # +127
    127
    >>> _sign_extend(0x8000, 16)  # -32768
    -32768
    """
    sign = 1 << (bits - 1)
    return (value & (sign - 1)) - (value & sign)


def _to_signed(value: int, sz: int) -> int:
    """Convert unsigned value of `sz` bytes to signed Python int."""
    bits = sz * 8
    msb  = 1 << (bits - 1)
    mask = (1 << bits) - 1
    v    = value & mask
    return v - (1 << bits) if v >= msb else v


def _sz_kwargs(sz: int) -> dict:
    """Return dict of word=/long= booleans for flags.py helpers."""
    return {"word": sz == 2, "long": sz == 4}


# ── Main simulator class ──────────────────────────────────────────────────────

class M68KSimulator:
    """Motorola 68000 (1979) behavioral simulator.

    Implements the ``Simulator[M68KState]`` protocol from ``simulator_protocol``.

    The simulator runs in supervisor mode (SR bit 13 always set).
    Programs are loaded at address 0x001000; the supervisor stack pointer
    starts at 0x00F000.  Execution stops on STOP or TRAP #15.

    Examples
    --------
    >>> sim = M68KSimulator()
    >>> prog = bytes([
    ...     0x70, 0x05,              # MOVEQ #5, D0
    ...     0x72, 0x03,              # MOVEQ #3, D1
    ...     0xD0, 0x81,              # ADD.L D1, D0
    ...     0x4E, 0x72, 0x27, 0x00, # STOP #0x2700
    ... ])
    >>> result = sim.execute(prog)
    >>> result.ok
    True
    >>> result.final_state.d0
    8
    """

    def __init__(self) -> None:
        self._mem: bytearray = bytearray(_MEM_SIZE)
        self._d: list[int]   = [0] * 8   # data registers D0–D7
        self._a: list[int]   = [0] * 8   # address registers A0–A7
        self._pc: int        = _LOAD_ADDR
        self._sr: int        = 0x2700    # supervisor mode, IMask=7
        self._halted: bool   = False
        self._traces: list[StepTrace] = []

    # ──────────────────────────────────────────────────────────────────────────
    # SIM00 protocol methods
    # ──────────────────────────────────────────────────────────────────────────

    def reset(self) -> None:
        """Reset to power-on state.

        All data registers → 0.  All address registers → 0 except A7 = 0x00F000.
        PC = 0x001000.  SR = 0x2700.  Memory zeroed.  Halt cleared.
        """
        self._d   = [0] * 8
        self._a   = [0] * 8
        self._a[7] = _INIT_SP
        self._pc  = _LOAD_ADDR
        self._sr  = 0x2700
        self._halted = False
        self._traces = []
        self._mem[:] = b"\x00" * _MEM_SIZE

    def load(self, program: bytes) -> None:
        """Load binary program bytes starting at address 0x001000."""
        end = _LOAD_ADDR + len(program)
        if end > _MEM_SIZE:
            raise ValueError(
                f"Program too large: {len(program)} bytes from 0x{_LOAD_ADDR:06X}"
                f" exceeds 16 MB address space"
            )
        self._mem[_LOAD_ADDR:end] = program

    def step(self) -> StepTrace:
        """Execute one instruction and return a StepTrace."""
        if self._halted:
            raise RuntimeError("CPU is halted — call reset() before stepping")
        pc_before = self._pc
        mnemonic  = self._decode_and_execute()
        pc_after  = self._pc
        trace = StepTrace(
            pc_before   = pc_before,
            pc_after    = pc_after,
            mnemonic    = mnemonic,
            description = f"{mnemonic} @ 0x{pc_before:06X}",
        )
        self._traces.append(trace)
        return trace

    def execute(
        self, program: bytes, max_steps: int = 100_000
    ) -> ExecutionResult[M68KState]:
        """Load program, reset state, run to STOP/TRAP#15 or max_steps."""
        self.reset()
        self.load(program)
        steps = 0
        error: str | None = None
        while not self._halted and steps < max_steps:
            try:
                self.step()
            except RuntimeError as exc:
                error = str(exc)
                break
            steps += 1
        if not self._halted and error is None:
            error = f"max_steps ({max_steps}) exceeded"
        return ExecutionResult(
            halted      = self._halted,
            steps       = steps,
            final_state = self.get_state(),
            error       = error,
            traces      = list(self._traces),
        )

    def get_state(self) -> M68KState:
        """Return a frozen snapshot of the current CPU state."""
        return M68KState(
            d0=self._d[0], d1=self._d[1], d2=self._d[2], d3=self._d[3],
            d4=self._d[4], d5=self._d[5], d6=self._d[6], d7=self._d[7],
            a0=self._a[0], a1=self._a[1], a2=self._a[2], a3=self._a[3],
            a4=self._a[4], a5=self._a[5], a6=self._a[6], a7=self._a[7],
            pc      = self._pc,
            sr      = self._sr,
            halted  = self._halted,
            memory  = tuple(self._mem),
        )

    # ──────────────────────────────────────────────────────────────────────────
    # Memory helpers (big-endian)
    # ──────────────────────────────────────────────────────────────────────────

    def _mem_read_byte(self, addr: int) -> int:
        return self._mem[addr & _ADDR_MASK]

    def _mem_read_word(self, addr: int) -> int:
        a = addr & _ADDR_MASK
        if a & 1:
            raise ValueError(f"Misaligned word read at 0x{a:06X}")
        return (self._mem[a] << 8) | self._mem[a + 1]

    def _mem_read_long(self, addr: int) -> int:
        a = addr & _ADDR_MASK
        if a & 1:
            raise ValueError(f"Misaligned long read at 0x{a:06X}")
        return (
            (self._mem[a    ] << 24)
            | (self._mem[a + 1] << 16)
            | (self._mem[a + 2] << 8)
            |  self._mem[a + 3]
        )

    def _mem_read(self, addr: int, sz: int) -> int:
        """Read sz bytes (1, 2, or 4) from addr (big-endian)."""
        if sz == 1: return self._mem_read_byte(addr)
        if sz == 2: return self._mem_read_word(addr)
        return self._mem_read_long(addr)

    def _mem_write_byte(self, addr: int, val: int) -> None:
        self._mem[addr & _ADDR_MASK] = val & _BYTE_MASK

    def _mem_write_word(self, addr: int, val: int) -> None:
        a = addr & _ADDR_MASK
        if a & 1:
            raise ValueError(f"Misaligned word write at 0x{a:06X}")
        self._mem[a    ] = (val >> 8) & _BYTE_MASK
        self._mem[a + 1] =  val       & _BYTE_MASK

    def _mem_write_long(self, addr: int, val: int) -> None:
        a = addr & _ADDR_MASK
        if a & 1:
            raise ValueError(f"Misaligned long write at 0x{a:06X}")
        self._mem[a    ] = (val >> 24) & _BYTE_MASK
        self._mem[a + 1] = (val >> 16) & _BYTE_MASK
        self._mem[a + 2] = (val >>  8) & _BYTE_MASK
        self._mem[a + 3] =  val        & _BYTE_MASK

    def _mem_write(self, addr: int, sz: int, val: int) -> None:
        """Write sz bytes (1, 2, or 4) to addr (big-endian)."""
        if sz == 1: self._mem_write_byte(addr, val)
        elif sz == 2: self._mem_write_word(addr, val)
        else: self._mem_write_long(addr, val)

    # ──────────────────────────────────────────────────────────────────────────
    # PC-based fetch helpers
    # ──────────────────────────────────────────────────────────────────────────

    def _fetch_word(self) -> int:
        """Fetch a 16-bit word at PC, advance PC by 2."""
        w = self._mem_read_word(self._pc)
        self._pc = (self._pc + 2) & _ADDR_MASK
        return w

    def _fetch_long(self) -> int:
        """Fetch a 32-bit long at PC, advance PC by 4."""
        v = self._mem_read_long(self._pc)
        self._pc = (self._pc + 4) & _ADDR_MASK
        return v

    def _fetch_word_signed(self) -> int:
        """Fetch signed 16-bit word at PC, advance PC by 2."""
        return _sign_extend(self._fetch_word(), 16)

    def _fetch_imm(self, sz: int) -> int:
        """Fetch immediate value of sz bytes; byte imm uses 16-bit extension."""
        if sz == 4:
            return self._fetch_long()
        val = self._fetch_word()       # always 16-bit extension, even for byte
        return val & _SZ_MASK[sz]

    # ──────────────────────────────────────────────────────────────────────────
    # Register helpers
    # ──────────────────────────────────────────────────────────────────────────

    def _set_dn(self, n: int, val: int, sz: int) -> None:
        """Write sz bytes into Dn; upper bytes of Dn are preserved."""
        mask = _SZ_MASK[sz]
        keep = _LONG_MASK ^ mask   # bits above the operation size
        self._d[n] = (self._d[n] & keep) | (val & mask)

    def _get_dn(self, n: int, sz: int) -> int:
        """Read sz bytes (low bits) from Dn."""
        return self._d[n] & _SZ_MASK[sz]

    # ──────────────────────────────────────────────────────────────────────────
    # CCR / SR helpers
    # ──────────────────────────────────────────────────────────────────────────

    def _ccr_bits(self) -> tuple[bool, bool, bool, bool]:
        """Return (N, Z, V, C) from current SR."""
        sr = self._sr
        return (bool(sr & 8), bool(sr & 4), bool(sr & 2), bool(sr & 1))

    def _set_ccr(self, n: bool, z: bool, v: bool, c: bool,
                 x: bool | None = None) -> None:
        """Update CCR bits in SR.  If x is None, X is unchanged."""
        sr = self._sr & 0xFFE0          # keep system byte, clear CCR
        sr |= (int(n) << 3)
        sr |= (int(z) << 2)
        sr |= (int(v) << 1)
        sr |=  int(c)
        if x is not None:
            sr = (sr & ~0x10) | (int(x) << 4)
        else:
            sr |= (self._sr & 0x10)     # preserve X
        self._sr = sr

    def _set_ccr_nzvc_add(
        self, a: int, b: int, raw: int, carry_in: int = 0,
        sz: int = 4,
    ) -> None:
        kw = _sz_kwargs(sz)
        n, z, v, c, x = compute_nzvc_add(a, b, raw, carry_in, **kw)
        self._set_ccr(n, z, v, c, x)

    def _set_ccr_nzvc_sub(
        self, a: int, b: int, raw: int, borrow: int = 0,
        sz: int = 4,
    ) -> None:
        kw = _sz_kwargs(sz)
        n, z, v, c, x = compute_nzvc_sub(a, b, raw, borrow, **kw)
        self._set_ccr(n, z, v, c, x)

    def _set_ccr_logic(self, result: int, sz: int) -> None:
        """Set N/Z; clear V/C; leave X unchanged."""
        n, z = compute_nz_logic(result, **_sz_kwargs(sz))
        self._set_ccr(n, z, False, False, x=None)

    def _set_ccr_cmp(self, a: int, b: int, raw: int, sz: int) -> None:
        """Set flags for CMP (same as SUB but X is not modified)."""
        kw = _sz_kwargs(sz)
        m      = _SZ_MASK[sz]
        result = raw & m
        n = compute_n(result, **kw)
        z = compute_z(result, **kw)
        v = compute_v_sub(a, b, result, **kw)
        c = compute_c_sub(a, b)
        self._set_ccr(n, z, v, c)   # X unchanged

    # ──────────────────────────────────────────────────────────────────────────
    # Stack helpers (push/pop always use A7)
    # ──────────────────────────────────────────────────────────────────────────

    def _push_long(self, val: int) -> None:
        self._a[7] = (self._a[7] - 4) & _ADDR_MASK
        self._mem_write_long(self._a[7], val)

    def _pop_long(self) -> int:
        val = self._mem_read_long(self._a[7])
        self._a[7] = (self._a[7] + 4) & _ADDR_MASK
        return val

    def _push_word(self, val: int) -> None:
        self._a[7] = (self._a[7] - 2) & _ADDR_MASK
        self._mem_write_word(self._a[7], val)

    def _pop_word(self) -> int:
        val = self._mem_read_word(self._a[7])
        self._a[7] = (self._a[7] + 2) & _ADDR_MASK
        return val

    # ──────────────────────────────────────────────────────────────────────────
    # Effective address resolution
    # ──────────────────────────────────────────────────────────────────────────
    #
    # _ea_address(mode, reg, sz) → physical address (memory modes only)
    #   - Handles pre-decrement (updates An first, returns decremented addr).
    #   - Handles post-increment (updates An after, returns pre-increment addr).
    #   - Reads extension words from PC when needed.
    #   - Raises ValueError for register-direct modes (0, 1) and immediate (7,4).
    #
    # _ea_read(mode, reg, sz)  → value  (works for all modes)
    # _ea_write(mode, reg, sz, val)     (works for all modes)
    #
    # For read-modify-write (ADD Dn, <ea>; NOT <ea>; etc.):
    #   addr = _ea_address(mode, reg, sz)   ← computes AND applies pre/postinc
    #   val  = _mem_read(addr, sz)
    #   ...modify...
    #   _mem_write(addr, sz, new_val)
    #
    # ──────────────────────────────────────────────────────────────────────────

    def _ea_address(self, mode: int, reg: int, sz: int) -> int:
        """Compute memory address for EA field.  Updates An for pre/postinc."""
        if mode == 2:  # (An) — indirect
            return self._a[reg] & _ADDR_MASK

        if mode == 3:  # (An)+ — postincrement
            addr = self._a[reg] & _ADDR_MASK
            inc  = max(sz, 2) if reg == 7 else sz   # SP always by ≥ 2
            self._a[reg] = (self._a[reg] + inc) & _ADDR_MASK
            return addr

        if mode == 4:  # -(An) — predecrement
            dec  = max(sz, 2) if reg == 7 else sz
            self._a[reg] = (self._a[reg] - dec) & _ADDR_MASK
            return self._a[reg] & _ADDR_MASK

        if mode == 5:  # d16(An) — 16-bit displacement
            d16  = self._fetch_word_signed()
            return (self._a[reg] + d16) & _ADDR_MASK

        if mode == 6:  # d8(An,Xn) — index + 8-bit displacement
            ext  = self._fetch_word()
            d8   = _sign_extend(ext & 0xFF, 8)
            xn_n = (ext >> 12) & 7
            xn_l = (ext >> 11) & 1       # 0=sign-extend word, 1=full long
            da   = (ext >> 15) & 1       # 0=Dn, 1=An
            xn   = self._a[xn_n] if da else self._d[xn_n]
            xn = _sign_extend(xn & _WORD_MASK, 16) if not xn_l else xn & _LONG_MASK
            return (self._a[reg] + xn + d8) & _ADDR_MASK

        if mode == 7:
            if reg == 0:   # (abs).W — absolute short (sign-extended)
                w = self._fetch_word()
                return _sign_extend(w, 16) & _ADDR_MASK
            if reg == 1:   # (abs).L — absolute long
                return self._fetch_long() & _ADDR_MASK
            if reg == 2:   # d16(PC)
                pc_base = self._pc
                d16     = self._fetch_word_signed()
                return (pc_base + d16) & _ADDR_MASK
            if reg == 3:   # d8(PC,Xn)
                pc_base = self._pc
                ext     = self._fetch_word()
                d8      = _sign_extend(ext & 0xFF, 8)
                xn_n    = (ext >> 12) & 7
                xn_l    = (ext >> 11) & 1
                da      = (ext >> 15) & 1
                xn      = self._a[xn_n] if da else self._d[xn_n]
                xn = _sign_extend(xn & _WORD_MASK, 16) if not xn_l else xn & _LONG_MASK
                return (pc_base + xn + d8) & _ADDR_MASK

        raise ValueError(f"EA mode {mode}/{reg} has no memory address")

    def _ea_read(self, mode: int, reg: int, sz: int) -> int:
        """Read sz-byte value from effective address."""
        if mode == 0:  # Dn — data register direct
            return self._d[reg] & _SZ_MASK[sz]
        if mode == 1:  # An — address register direct (always 32-bit)
            return self._a[reg] & _LONG_MASK
        if mode == 7 and reg == 4:   # immediate
            return self._fetch_imm(sz)
        addr = self._ea_address(mode, reg, sz)
        return self._mem_read(addr, sz)

    def _ea_write(self, mode: int, reg: int, sz: int, val: int) -> None:
        """Write sz-byte value to effective address."""
        if mode == 0:  # Dn
            self._set_dn(reg, val, sz)
            return
        if mode == 1:  # An — word writes are sign-extended to 32 bits
            if sz == 2:
                val = _sign_extend(val & _WORD_MASK, 16)
            self._a[reg] = val & _LONG_MASK
            return
        addr = self._ea_address(mode, reg, sz)
        self._mem_write(addr, sz, val)

    def _ea_read_addr(self, mode: int, reg: int, sz: int) -> tuple[int, int]:
        """Read from memory EA; return (value, address) for RMW operations.

        Pre/postincrement is applied exactly once.
        Only valid for memory-mode EAs (mode 2–7 excluding 7.4).
        """
        addr = self._ea_address(mode, reg, sz)
        return self._mem_read(addr, sz), addr

    # ──────────────────────────────────────────────────────────────────────────
    # EA mnemonic helpers (for StepTrace)
    # ──────────────────────────────────────────────────────────────────────────

    _SZ_SUFFIX = {1: ".B", 2: ".W", 4: ".L"}

    def _ea_str(self, mode: int, reg: int) -> str:
        """Short human-readable string for an EA (for mnemonics).

        Does NOT consume extension words — just generates a representative
        string.  For modes requiring extensions the placeholder is shown.
        """
        if mode == 0:   return f"D{reg}"
        if mode == 1:   return f"A{reg}"
        if mode == 2:   return f"(A{reg})"
        if mode == 3:   return f"(A{reg})+"
        if mode == 4:   return f"-(A{reg})"
        if mode == 5:   return f"d16(A{reg})"
        if mode == 6:   return f"d8(A{reg},Xn)"
        if mode == 7:
            return ["(abs).W", "(abs).L", "d16(PC)", "d8(PC,Xn)", "#imm"][reg]
        return f"?{mode}/{reg}"

    # ──────────────────────────────────────────────────────────────────────────
    # Main decode/execute dispatcher
    # ──────────────────────────────────────────────────────────────────────────

    def _decode_and_execute(self) -> str:
        """Fetch and execute one instruction.  Returns mnemonic string."""
        op = self._fetch_word()
        hi = (op >> 12) & 0xF

        if hi == 0x0: return self._exec_line0(op)
        if hi == 0x1 or hi == 0x2 or hi == 0x3: return self._exec_move(op)
        if hi == 0x4: return self._exec_line4(op)
        if hi == 0x5: return self._exec_line5(op)
        if hi == 0x6: return self._exec_line6(op)
        if hi == 0x7: return self._exec_moveq(op)
        if hi == 0x8: return self._exec_line8(op)
        if hi == 0x9: return self._exec_line9(op)
        if hi == 0xB: return self._exec_lineB(op)
        if hi == 0xC: return self._exec_lineC(op)
        if hi == 0xD: return self._exec_lineD(op)
        if hi == 0xE: return self._exec_lineE(op)

        raise RuntimeError(f"Unimplemented opcode 0x{op:04X} at PC-2")

    # ──────────────────────────────────────────────────────────────────────────
    # Line 0 — immediate group (ORI, ANDI, SUBI, ADDI, EORI, CMPI)
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_line0(self, op: int) -> str:
        sz_code = (op >> 6) & 3
        mode    = (op >> 3) & 7
        reg     = op & 7

        # BTST/BCHG/BCLR/BSET with immediate bit number (0000 1000 ...)
        if (op & 0xFF00) == 0x0800:
            return self._exec_bit_imm(op)
        # BTST/BCHG/BCLR/BSET register bit number (0000 rrr1 00 ea)
        if (op & 0x0138) == 0x0100 and sz_code <= 3:
            return self._exec_bit_reg(op)

        op8 = (op >> 8) & 0xFF

        # ORI #imm, <ea>  — 0000 0000 ss ea
        if op8 == 0x00:
            sz  = _SZ_ARITH.get(sz_code)
            if sz is None: raise RuntimeError(f"ORI bad size 0x{op:04X}")
            imm = self._fetch_imm(sz)
            if mode == 7 and reg == 4:   # ORI #imm, CCR
                self._sr = (self._sr & 0xFFE0) | ((self._sr & 0x1F) | (imm & 0x1F))
                return "ORI #imm, CCR"
            if mode == 7 and reg == 5:   # ORI #imm, SR
                self._sr |= imm & 0xFFFF
                return "ORI #imm, SR"
            val    = self._ea_read(mode, reg, sz)
            result = (val | imm) & _SZ_MASK[sz]
            self._ea_write(mode, reg, sz, result)
            self._set_ccr_logic(result, sz)
            return f"ORI{self._SZ_SUFFIX[sz]} #{imm:#x},{self._ea_str(mode,reg)}"

        # ANDI #imm, <ea>  — 0000 0010 ss ea
        if op8 == 0x02:
            sz  = _SZ_ARITH.get(sz_code)
            if sz is None: raise RuntimeError(f"ANDI bad size 0x{op:04X}")
            imm = self._fetch_imm(sz)
            if mode == 7 and reg == 4:   # ANDI #imm, CCR
                self._sr = (self._sr & 0xFFE0) | ((self._sr & 0x1F) & (imm & 0x1F))
                return "ANDI #imm, CCR"
            if mode == 7 and reg == 5:   # ANDI #imm, SR
                self._sr &= imm | 0xFF00  # keep system byte bits from imm
                self._sr &= 0xFFFF
                return "ANDI #imm, SR"
            val    = self._ea_read(mode, reg, sz)
            result = (val & imm) & _SZ_MASK[sz]
            self._ea_write(mode, reg, sz, result)
            self._set_ccr_logic(result, sz)
            return f"ANDI{self._SZ_SUFFIX[sz]} #{imm:#x},{self._ea_str(mode,reg)}"

        # SUBI #imm, <ea>  — 0000 0100 ss ea
        if op8 == 0x04:
            sz  = _SZ_ARITH.get(sz_code)
            if sz is None: raise RuntimeError(f"SUBI bad size 0x{op:04X}")
            imm = self._fetch_imm(sz)
            a   = self._ea_read(mode, reg, sz)
            raw = a - imm
            result = raw & _SZ_MASK[sz]
            self._ea_write(mode, reg, sz, result)
            self._set_ccr_nzvc_sub(a, imm, raw, sz=sz)
            return f"SUBI{self._SZ_SUFFIX[sz]} #{imm:#x},{self._ea_str(mode,reg)}"

        # ADDI #imm, <ea>  — 0000 0110 ss ea
        if op8 == 0x06:
            sz  = _SZ_ARITH.get(sz_code)
            if sz is None: raise RuntimeError(f"ADDI bad size 0x{op:04X}")
            imm = self._fetch_imm(sz)
            a   = self._ea_read(mode, reg, sz)
            raw = a + imm
            result = raw & _SZ_MASK[sz]
            self._ea_write(mode, reg, sz, result)
            self._set_ccr_nzvc_add(a, imm, raw, sz=sz)
            return f"ADDI{self._SZ_SUFFIX[sz]} #{imm:#x},{self._ea_str(mode,reg)}"

        # EORI #imm, <ea>  — 0000 1010 ss ea
        if op8 == 0x0A:
            sz  = _SZ_ARITH.get(sz_code)
            if sz is None: raise RuntimeError(f"EORI bad size 0x{op:04X}")
            imm = self._fetch_imm(sz)
            if mode == 7 and reg == 4:   # EORI #imm, CCR
                self._sr = (self._sr & 0xFFE0) | ((self._sr & 0x1F) ^ (imm & 0x1F))
                return "EORI #imm, CCR"
            val    = self._ea_read(mode, reg, sz)
            result = (val ^ imm) & _SZ_MASK[sz]
            self._ea_write(mode, reg, sz, result)
            self._set_ccr_logic(result, sz)
            return f"EORI{self._SZ_SUFFIX[sz]} #{imm:#x},{self._ea_str(mode,reg)}"

        # CMPI #imm, <ea>  — 0000 1100 ss ea
        if op8 == 0x0C:
            sz  = _SZ_ARITH.get(sz_code)
            if sz is None: raise RuntimeError(f"CMPI bad size 0x{op:04X}")
            imm = self._fetch_imm(sz)
            a   = self._ea_read(mode, reg, sz)
            raw = a - imm
            self._set_ccr_cmp(a, imm, raw, sz)
            return f"CMPI{self._SZ_SUFFIX[sz]} #{imm:#x},{self._ea_str(mode,reg)}"

        raise RuntimeError(f"Unimplemented line-0 opcode 0x{op:04X}")

    def _exec_bit_imm(self, op: int) -> str:
        """BTST/BCHG/BCLR/BSET with immediate bit number."""
        kind = (op >> 6) & 3   # 0=BTST,1=BCHG,2=BCLR,3=BSET
        mode = (op >> 3) & 7
        reg  = op & 7
        bit_n = self._fetch_word() & 0x1F   # immediate bit number (0-31)
        names = ["BTST", "BCHG", "BCLR", "BSET"]
        if mode == 0:  # register — 32-bit
            bit_n &= 31
            val = self._d[reg]
            z_val = not bool(val & (1 << bit_n))
            if kind == 1:   self._d[reg] = val ^ (1 << bit_n)
            elif kind == 2: self._d[reg] = val & ~(1 << bit_n)
            elif kind == 3: self._d[reg] = val | (1 << bit_n)
        else:          # memory — 8-bit
            bit_n &= 7
            addr = self._ea_address(mode, reg, 1)
            val  = self._mem_read_byte(addr)
            z_val = not bool(val & (1 << bit_n))
            if kind == 1:   self._mem_write_byte(addr, val ^ (1 << bit_n))
            elif kind == 2: self._mem_write_byte(addr, val & ~(1 << bit_n))
            elif kind == 3: self._mem_write_byte(addr, val | (1 << bit_n))
        # Z flag set from tested bit; N/V/C unchanged
        sr = self._sr & ~4
        sr |= (4 if z_val else 0)
        self._sr = sr
        return f"{names[kind]} #{bit_n},{self._ea_str(mode,reg)}"

    def _exec_bit_reg(self, op: int) -> str:
        """BTST/BCHG/BCLR/BSET with register-specified bit number."""
        dn   = (op >> 9) & 7
        kind = (op >> 6) & 3
        mode = (op >> 3) & 7
        reg  = op & 7
        names = ["BTST", "BCHG", "BCLR", "BSET"]
        bit_n = self._d[dn]
        if mode == 0:  # register — 32-bit
            bit_n &= 31
            val = self._d[reg]
            z_val = not bool(val & (1 << bit_n))
            if kind == 1:   self._d[reg] = val ^ (1 << bit_n)
            elif kind == 2: self._d[reg] = val & ~(1 << bit_n)
            elif kind == 3: self._d[reg] = val | (1 << bit_n)
        else:
            bit_n &= 7
            addr = self._ea_address(mode, reg, 1)
            val  = self._mem_read_byte(addr)
            z_val = not bool(val & (1 << bit_n))
            if kind == 1:   self._mem_write_byte(addr, val ^ (1 << bit_n))
            elif kind == 2: self._mem_write_byte(addr, val & ~(1 << bit_n))
            elif kind == 3: self._mem_write_byte(addr, val | (1 << bit_n))
        sr = self._sr & ~4
        sr |= (4 if z_val else 0)
        self._sr = sr
        return f"{names[kind]} D{dn},{self._ea_str(mode,reg)}"

    # ──────────────────────────────────────────────────────────────────────────
    # Lines 1/2/3 — MOVE (byte, long, word)
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_move(self, op: int) -> str:
        """MOVE / MOVEA — the most versatile instruction in the 68000 ISA.

        Encoding: 00ss DDD ddd MMM mmm
          ss       = size code (01=byte, 10=long, 11=word)
          DDD ddd  = destination EA (reg field first, then mode)
          MMM mmm  = source EA (mode then reg, normal order)
        """
        sz_code  = (op >> 12) & 3    # 01=byte, 10=long, 11=word
        sz       = _SZ_MOVE.get(sz_code)
        if sz is None:
            raise RuntimeError(f"MOVE bad size code {sz_code} in 0x{op:04X}")
        dst_reg  = (op >> 9) & 7
        dst_mode = (op >> 6) & 7
        src_mode = (op >> 3) & 7
        src_reg  = op & 7

        val = self._ea_read(src_mode, src_reg, sz)

        if dst_mode == 1:  # MOVEA — move to address register (no flags)
            if sz == 2:
                val = _sign_extend(val & _WORD_MASK, 16) & _LONG_MASK
            self._a[dst_reg] = val & _LONG_MASK
            suf = ".W" if sz == 2 else ".L"
            return (f"MOVEA{suf} {self._ea_str(src_mode,src_reg)},"
                    f"A{dst_reg}")

        # Normal MOVE — write destination, set N/Z, clear V/C
        self._ea_write(dst_mode, dst_reg, sz, val)
        result = val & _SZ_MASK[sz]
        self._set_ccr_logic(result, sz)
        suf = self._SZ_SUFFIX[sz]
        return (f"MOVE{suf} {self._ea_str(src_mode,src_reg)},"
                f"{self._ea_str(dst_mode,dst_reg)}")

    # ──────────────────────────────────────────────────────────────────────────
    # Line 4 — miscellaneous (NEG, CLR, TST, NOT, SWAP, EXT, LEA, PEA, JSR, JMP…)
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_line4(self, op: int) -> str:
        # ── Special encodings identified by full upper byte ──────────────────

        # NOP: 0x4E71
        if op == 0x4E71: return "NOP"

        # RESET: 0x4E70
        if op == 0x4E70: return "RESET"

        # RTS: 0x4E75
        if op == 0x4E75:
            self._pc = self._pop_long() & _ADDR_MASK
            return "RTS"

        # RTR: 0x4E77 — pop CCR then PC
        if op == 0x4E77:
            ccr_word = self._pop_word()
            self._sr = (self._sr & 0xFF00) | (ccr_word & 0x1F)
            self._pc = self._pop_long() & _ADDR_MASK
            return "RTR"

        # STOP #imm: 0x4E72 xxxx — load imm into SR, halt
        if op == 0x4E72:
            imm      = self._fetch_word()
            self._sr = imm & 0xFFFF
            self._halted = True
            return f"STOP #{imm:#06x}"

        # TRAP #n: 0x4E40–0x4E4F
        if 0x4E40 <= op <= 0x4E4F:
            n = op & 0xF
            if n == 15:                  # TRAP #15 = halt
                self._halted = True
            else:
                self._d[7] = n           # stub: record trap number in D7
            return f"TRAP #{n}"

        # LINK An, #d16: 0x4E50–0x4E57
        if 0x4E50 <= op <= 0x4E57:
            n    = op & 7
            disp = self._fetch_word_signed()
            self._push_long(self._a[n])
            self._a[n] = self._a[7]
            self._a[7] = (self._a[7] + disp) & _ADDR_MASK
            return f"LINK A{n},#{disp}"

        # UNLK An: 0x4E58–0x4E5F
        if 0x4E58 <= op <= 0x4E5F:
            n         = op & 7
            self._a[7] = self._a[n]
            self._a[n] = self._pop_long()
            return f"UNLK A{n}"

        # SWAP Dn: 0x4840–0x4847
        if 0x4840 <= op <= 0x4847:
            n        = op & 7
            val      = self._d[n] & _LONG_MASK
            swapped  = ((val >> 16) | ((val & _WORD_MASK) << 16)) & _LONG_MASK
            self._d[n] = swapped
            self._set_ccr_logic(swapped, 4)
            return f"SWAP D{n}"

        # EXT.W Dn: 0x4880–0x4887
        if 0x4880 <= op <= 0x4887:
            n   = op & 7
            b   = _sign_extend(self._d[n] & _BYTE_MASK, 8)
            w   = b & _WORD_MASK
            self._set_dn(n, w, 2)
            self._set_ccr_logic(w, 2)
            return f"EXT.W D{n}"

        # EXT.L Dn: 0x48C0–0x48C7
        if 0x48C0 <= op <= 0x48C7:
            n   = op & 7
            w   = _sign_extend(self._d[n] & _WORD_MASK, 16)
            lw  = w & _LONG_MASK
            self._d[n] = lw
            self._set_ccr_logic(lw, 4)
            return f"EXT.L D{n}"

        # MOVE SR, Dn: 0x40C0–0x40C7
        if 0x40C0 <= op <= 0x40C7:
            n = op & 7
            self._set_dn(n, self._sr, 2)
            return f"MOVE SR,D{n}"

        # MOVE CCR, Dn: 0x42C0–0x42C7
        if 0x42C0 <= op <= 0x42C7:
            n = op & 7
            self._set_dn(n, self._sr & 0x1F, 2)
            return f"MOVE CCR,D{n}"

        # MOVE #imm, CCR: 0x44FC
        if op == 0x44FC:
            imm      = self._fetch_word()
            self._sr = (self._sr & 0xFFE0) | (imm & 0x1F)
            return f"MOVE #{imm & 0x1F:#x},CCR"

        # MOVE #imm, SR: 0x46FC
        if op == 0x46FC:
            imm      = self._fetch_word()
            self._sr = imm & 0xFFFF
            return f"MOVE #{imm:#06x},SR"

        # ── EA-based ops identified by upper 8 bits ──────────────────────────

        sz_code = (op >> 6) & 3
        mode    = (op >> 3) & 7
        reg     = op & 7

        # NEGX.sz <ea>: 0100 0000 ss ea
        if (op & 0xFF00) == 0x4000 and sz_code <= 2:
            sz   = _SZ_ARITH[sz_code]
            a    = self._ea_read(mode, reg, sz)
            x    = int(bool(self._sr & 0x10))
            raw  = 0 - a - x
            result = raw & _SZ_MASK[sz]
            self._ea_write(mode, reg, sz, result)
            # NEGX: Z is only *cleared* if result != 0, never set
            n_f  = compute_n(result, **_sz_kwargs(sz))
            z_f  = bool(self._sr & 4) and (result == 0)
            v_f  = bool(result == _SZ_MSB[sz])
            c_f  = (result != 0)
            self._set_ccr(n_f, z_f, v_f, c_f, x=c_f)
            return f"NEGX{self._SZ_SUFFIX[sz]} {self._ea_str(mode,reg)}"

        # CLR.sz <ea>: 0100 0010 ss ea
        if (op & 0xFF00) == 0x4200 and sz_code <= 2:
            sz = _SZ_ARITH[sz_code]
            self._ea_write(mode, reg, sz, 0)
            self._set_ccr(False, True, False, False)
            return f"CLR{self._SZ_SUFFIX[sz]} {self._ea_str(mode,reg)}"

        # NEG.sz <ea>: 0100 0100 ss ea
        if (op & 0xFF00) == 0x4400 and sz_code <= 2:
            sz     = _SZ_ARITH[sz_code]
            src    = self._ea_read(mode, reg, sz)
            raw    = (0 - src) & _SZ_MASK[sz]
            self._ea_write(mode, reg, sz, raw)
            n_f, z_f, v_f, c_f, x_f = compute_nzvc_neg(src, raw, **_sz_kwargs(sz))
            self._set_ccr(n_f, z_f, v_f, c_f, x=x_f)
            return f"NEG{self._SZ_SUFFIX[sz]} {self._ea_str(mode,reg)}"

        # NOT.sz <ea>: 0100 0110 ss ea
        if (op & 0xFF00) == 0x4600 and sz_code <= 2:
            sz     = _SZ_ARITH[sz_code]
            val    = self._ea_read(mode, reg, sz)
            result = (~val) & _SZ_MASK[sz]
            self._ea_write(mode, reg, sz, result)
            self._set_ccr_logic(result, sz)
            return f"NOT{self._SZ_SUFFIX[sz]} {self._ea_str(mode,reg)}"

        # TST.sz <ea>: 0100 1010 ss ea
        if (op & 0xFF00) == 0x4A00 and sz_code <= 2:
            sz     = _SZ_ARITH[sz_code]
            val    = self._ea_read(mode, reg, sz) & _SZ_MASK[sz]
            self._set_ccr_logic(val, sz)
            return f"TST{self._SZ_SUFFIX[sz]} {self._ea_str(mode,reg)}"

        # PEA <ea>: 0100 1000 01 mm rrr  (mode >= 2, not Dn/An/imm)
        if (op & 0xFFC0) == 0x4840 and mode >= 2:
            addr = self._ea_address(mode, reg, 4)
            self._push_long(addr)
            return f"PEA {self._ea_str(mode,reg)}"

        # LEA <ea>, An: 0100 aaa1 11 mm rrr
        if (op & 0x01C0) == 0x01C0 and (op & 0xF000) == 0x4000:
            an   = (op >> 9) & 7
            if mode >= 2 and not (mode == 7 and reg == 4):
                addr       = self._ea_address(mode, reg, 4)
                self._a[an] = addr & _LONG_MASK
                return f"LEA {self._ea_str(mode,reg)},A{an}"

        # JSR <ea>: 0100 1110 10 mm rrr
        if (op & 0xFFC0) == 0x4E80:
            target = self._ea_address(mode, reg, 4)
            self._push_long(self._pc)
            self._pc = target & _ADDR_MASK
            return f"JSR {self._ea_str(mode,reg)}"

        # JMP <ea>: 0100 1110 11 mm rrr
        if (op & 0xFFC0) == 0x4EC0:
            target   = self._ea_address(mode, reg, 4)
            self._pc = target & _ADDR_MASK
            return f"JMP {self._ea_str(mode,reg)}"

        raise RuntimeError(f"Unimplemented line-4 opcode 0x{op:04X}")

    # ──────────────────────────────────────────────────────────────────────────
    # Line 5 — ADDQ, SUBQ, DBcc, Scc
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_line5(self, op: int) -> str:
        sz_code = (op >> 6) & 3
        mode    = (op >> 3) & 7
        reg     = op & 7
        data    = (op >> 9) & 7   # 3-bit immediate: 0 means 8
        imm     = 8 if data == 0 else data

        # DBcc Dn, #disp: 0101 cccc 1100 1 rrr
        if sz_code == 3 and mode == 1:   # mode=001 = An-direct slot used by DBcc
            cc = (op >> 8) & 0xF
            pc_before_ext = self._pc   # PC points at the extension word
            disp = self._fetch_word_signed()
            # target = pc_before_ext + disp (displacement from extension word addr)
            target = (pc_before_ext + disp) & _ADDR_MASK
            n, z, v, c = self._ccr_bits()
            if not _CC_FUNCS[cc](n, z, v, c):
                # condition false → decrement Dn and branch if ≠ -1
                count  = _sign_extend(self._d[reg] & _WORD_MASK, 16) - 1
                self._set_dn(reg, count & _WORD_MASK, 2)
                if count != -1:
                    self._pc = target
            return f"DB{_CC_NAMES[cc]} D{reg},#{disp}"

        # Scc <ea>: 0101 cccc 11 mm rrr
        if sz_code == 3:
            cc = (op >> 8) & 0xF
            n, z, v, c = self._ccr_bits()
            val = 0xFF if _CC_FUNCS[cc](n, z, v, c) else 0x00
            self._ea_write(mode, reg, 1, val)
            return f"S{_CC_NAMES[cc]} {self._ea_str(mode,reg)}"

        sz = _SZ_ARITH.get(sz_code)
        if sz is None:
            raise RuntimeError(f"ADDQ/SUBQ bad size 0x{op:04X}")

        sub = (op >> 8) & 1   # 0=ADDQ, 1=SUBQ

        if sub == 0:  # ADDQ
            if mode == 1:  # ADDQ An — no flags set
                self._a[reg] = (self._a[reg] + imm) & _LONG_MASK
                return f"ADDQ #{imm},{self._ea_str(mode,reg)}"
            a   = self._ea_read(mode, reg, sz)
            raw = a + imm
            result = raw & _SZ_MASK[sz]
            self._ea_write(mode, reg, sz, result)
            self._set_ccr_nzvc_add(a, imm, raw, sz=sz)
            return f"ADDQ{self._SZ_SUFFIX[sz]} #{imm},{self._ea_str(mode,reg)}"
        else:          # SUBQ
            if mode == 1:  # SUBQ An — no flags set
                self._a[reg] = (self._a[reg] - imm) & _LONG_MASK
                return f"SUBQ #{imm},{self._ea_str(mode,reg)}"
            a   = self._ea_read(mode, reg, sz)
            raw = a - imm
            result = raw & _SZ_MASK[sz]
            self._ea_write(mode, reg, sz, result)
            self._set_ccr_nzvc_sub(a, imm, raw, sz=sz)
            return f"SUBQ{self._SZ_SUFFIX[sz]} #{imm},{self._ea_str(mode,reg)}"

    # ──────────────────────────────────────────────────────────────────────────
    # Line 6 — BRA, BSR, Bcc
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_line6(self, op: int) -> str:
        """BRA / BSR / Bcc.

        Encoding: 0110 cccc dddd dddd
          cccc: condition (0=BRA, 1=BSR, 2-15=Bcc)
          dddd dddd: 8-bit signed displacement (0x00 = use 16-bit extension)

        Displacement is relative to the address AFTER the instruction opword
        (i.e., PC after the 2-byte fetch, before extension word if any).
        If displacement byte == 0: fetch additional 16-bit displacement.
        """
        cc   = (op >> 8) & 0xF
        disp8 = op & 0xFF
        pc_base = self._pc    # PC after opword fetch; before possible extension

        disp = self._fetch_word_signed() if disp8 == 0 else _sign_extend(disp8, 8)

        target = (pc_base + disp) & _ADDR_MASK

        if cc == 0:   # BRA — always branch
            self._pc = target
            return f"BRA #{disp}"

        if cc == 1:   # BSR — branch to subroutine
            self._push_long(self._pc)   # push return address (after instr)
            self._pc = target
            return f"BSR #{disp}"

        # Bcc — conditional
        n, z, v, c = self._ccr_bits()
        if _CC_FUNCS[cc](n, z, v, c):
            self._pc = target
        return f"B{_CC_NAMES[cc]} #{disp}"

    # ──────────────────────────────────────────────────────────────────────────
    # Line 7 — MOVEQ
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_moveq(self, op: int) -> str:
        """MOVEQ #d8, Dn — sign-extend 8-bit immediate to 32 bits.

        Encoding: 0111 rrr0 dddddddd
          rrr: destination data register
          d: must be 0 (distinguishes MOVEQ from other line-7 encodings)
          dddddddd: 8-bit signed immediate
        """
        if op & 0x0100:
            raise RuntimeError(f"Not MOVEQ (bit 8 set): 0x{op:04X}")
        dn  = (op >> 9) & 7
        imm = _sign_extend(op & 0xFF, 8)
        self._d[dn] = imm & _LONG_MASK
        self._set_ccr_logic(imm & _LONG_MASK, 4)
        return f"MOVEQ #{imm},D{dn}"

    # ──────────────────────────────────────────────────────────────────────────
    # Line 8 — OR, DIVU, DIVS
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_line8(self, op: int) -> str:
        dn      = (op >> 9) & 7
        dir_bit = (op >> 8) & 1   # 0 = EA op Dn → Dn; 1 = Dn op EA → EA
        sz_code = (op >> 6) & 3
        mode    = (op >> 3) & 7
        reg     = op & 7

        # DIVU.W <ea>, Dn: 1000 rrr0 11 mm rrr
        if sz_code == 3 and dir_bit == 0:
            divisor = self._ea_read(mode, reg, 2) & _WORD_MASK
            if divisor == 0:
                raise RuntimeError("DIVU: division by zero")
            dividend = self._d[dn] & _LONG_MASK
            quotient  = dividend // divisor
            remainder = dividend % divisor
            if quotient > _WORD_MASK:   # overflow
                self._set_ccr(False, False, True, False)
                return f"DIVU {self._ea_str(mode,reg)},D{dn}"
            self._d[dn] = ((remainder & _WORD_MASK) << 16) | (quotient & _WORD_MASK)
            self._set_ccr_logic(quotient & _WORD_MASK, 2)
            return f"DIVU {self._ea_str(mode,reg)},D{dn}"

        # DIVS.W <ea>, Dn: 1000 rrr1 11 mm rrr
        if sz_code == 3 and dir_bit == 1:
            divisor_u = self._ea_read(mode, reg, 2) & _WORD_MASK
            divisor   = _sign_extend(divisor_u, 16)
            if divisor == 0:
                raise RuntimeError("DIVS: division by zero")
            dividend_u = self._d[dn] & _LONG_MASK
            dividend   = _to_signed(dividend_u, 4)
            quotient   = int(dividend / divisor)   # truncate toward zero
            remainder  = dividend - quotient * divisor
            # Check signed 16-bit overflow
            if quotient < -32768 or quotient > 32767:
                self._set_ccr(False, False, True, False)
                return f"DIVS {self._ea_str(mode,reg)},D{dn}"
            self._d[dn] = ((remainder & _WORD_MASK) << 16) | (quotient & _WORD_MASK)
            self._set_ccr_logic(quotient & _WORD_MASK, 2)
            return f"DIVS {self._ea_str(mode,reg)},D{dn}"

        # OR <ea>, Dn  (direction=0)
        # OR Dn, <ea>  (direction=1)
        sz = _SZ_ARITH.get(sz_code)
        if sz is None: raise RuntimeError(f"OR bad size 0x{op:04X}")

        if dir_bit == 0:   # OR <ea>, Dn
            b      = self._ea_read(mode, reg, sz)
            a      = self._d[dn] & _SZ_MASK[sz]
            result = (a | b) & _SZ_MASK[sz]
            self._set_dn(dn, result, sz)
            self._set_ccr_logic(result, sz)
            return f"OR{self._SZ_SUFFIX[sz]} {self._ea_str(mode,reg)},D{dn}"
        else:               # OR Dn, <ea>
            a     = self._d[dn] & _SZ_MASK[sz]
            val, addr = self._ea_read_addr(mode, reg, sz)
            result = (val | a) & _SZ_MASK[sz]
            self._mem_write(addr, sz, result)
            self._set_ccr_logic(result, sz)
            return f"OR{self._SZ_SUFFIX[sz]} D{dn},{self._ea_str(mode,reg)}"

    # ──────────────────────────────────────────────────────────────────────────
    # Line 9 — SUB, SUBA, SUBX
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_line9(self, op: int) -> str:
        dn      = (op >> 9) & 7
        dir_bit = (op >> 8) & 1
        sz_code = (op >> 6) & 3
        mode    = (op >> 3) & 7
        reg     = op & 7

        # SUBA — SUB to address register (no flags set; always 32-bit result)
        # SUBA.W: sz_code=3, dir_bit=0  (sign-extends 16-bit source to 32)
        # SUBA.L: sz_code=3, dir_bit=1  (full 32-bit subtraction)
        if sz_code == 3 and dir_bit == 0:   # SUBA.W
            src = self._ea_read(mode, reg, 2)
            src = _sign_extend(src & _WORD_MASK, 16) & _LONG_MASK
            self._a[dn] = (self._a[dn] - src) & _LONG_MASK
            return f"SUBA.W {self._ea_str(mode,reg)},A{dn}"
        if sz_code == 3 and dir_bit == 1:   # SUBA.L
            src = self._ea_read(mode, reg, 4)
            self._a[dn] = (self._a[dn] - src) & _LONG_MASK
            return f"SUBA.L {self._ea_str(mode,reg)},A{dn}"

        # SUBX Ds, Dd: 1001 rrr1 ss 00 0 rrr (register-to-register)
        sz = _SZ_ARITH.get(sz_code)
        if sz is None: raise RuntimeError(f"SUB bad size 0x{op:04X}")

        if dir_bit == 1 and mode == 0:   # SUBX register form
            x    = int(bool(self._sr & 0x10))
            a    = self._d[dn] & _SZ_MASK[sz]
            b    = self._d[reg] & _SZ_MASK[sz]
            raw  = a - b - x
            result = raw & _SZ_MASK[sz]
            self._set_dn(dn, result, sz)
            kw   = _sz_kwargs(sz)
            n_f  = compute_n(result, **kw)
            z_f  = bool(self._sr & 4) and (result == 0)
            v_f  = compute_v_sub(a, b, result, **kw)
            c_f  = compute_c_sub(a, b + x)
            self._set_ccr(n_f, z_f, v_f, c_f, x=c_f)
            return f"SUBX{self._SZ_SUFFIX[sz]} D{reg},D{dn}"

        if dir_bit == 0:   # SUB <ea>, Dn → Dn
            b   = self._ea_read(mode, reg, sz)
            a   = self._d[dn] & _SZ_MASK[sz]
            raw = a - b
            result = raw & _SZ_MASK[sz]
            self._set_dn(dn, result, sz)
            self._set_ccr_nzvc_sub(a, b, raw, sz=sz)
            return f"SUB{self._SZ_SUFFIX[sz]} {self._ea_str(mode,reg)},D{dn}"
        else:               # SUB Dn, <ea> → <ea>
            a     = self._d[dn] & _SZ_MASK[sz]
            val, addr = self._ea_read_addr(mode, reg, sz)
            raw   = val - a
            result = raw & _SZ_MASK[sz]
            self._mem_write(addr, sz, result)
            self._set_ccr_nzvc_sub(val, a, raw, sz=sz)
            return f"SUB{self._SZ_SUFFIX[sz]} D{dn},{self._ea_str(mode,reg)}"

    # ──────────────────────────────────────────────────────────────────────────
    # Line B — CMP, CMPA, EOR
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_lineB(self, op: int) -> str:
        dn      = (op >> 9) & 7
        dir_bit = (op >> 8) & 1
        sz_code = (op >> 6) & 3
        mode    = (op >> 3) & 7
        reg     = op & 7

        # CMPA — compare EA with address register
        if sz_code == 3 and dir_bit == 0:   # CMPA.W (sign-ext to long)
            src = self._ea_read(mode, reg, 2)
            src = _sign_extend(src & _WORD_MASK, 16)
            a   = self._a[dn] & _LONG_MASK
            raw = a - (src & _LONG_MASK)
            self._set_ccr_cmp(a, src & _LONG_MASK, raw, 4)
            return f"CMPA.W {self._ea_str(mode,reg)},A{dn}"

        if sz_code == 3 and dir_bit == 1:   # CMPA.L
            src = self._ea_read(mode, reg, 4)
            a   = self._a[dn] & _LONG_MASK
            raw = a - src
            self._set_ccr_cmp(a, src, raw, 4)
            return f"CMPA.L {self._ea_str(mode,reg)},A{dn}"

        sz = _SZ_ARITH.get(sz_code)
        if sz is None: raise RuntimeError(f"CMP/EOR bad size 0x{op:04X}")

        if dir_bit == 0:   # CMP <ea>, Dn
            b   = self._ea_read(mode, reg, sz)
            a   = self._d[dn] & _SZ_MASK[sz]
            raw = a - b
            self._set_ccr_cmp(a, b, raw, sz)
            return f"CMP{self._SZ_SUFFIX[sz]} {self._ea_str(mode,reg)},D{dn}"
        else:               # EOR Dn, <ea>
            a = self._d[dn] & _SZ_MASK[sz]
            if mode == 0:   # Dn destination — register-to-register XOR
                val    = self._d[reg] & _SZ_MASK[sz]
                result = (val ^ a) & _SZ_MASK[sz]
                self._set_dn(reg, result, sz)
            else:
                val, addr = self._ea_read_addr(mode, reg, sz)
                result    = (val ^ a) & _SZ_MASK[sz]
                self._mem_write(addr, sz, result)
            self._set_ccr_logic(result, sz)
            return f"EOR{self._SZ_SUFFIX[sz]} D{dn},{self._ea_str(mode,reg)}"

    # ──────────────────────────────────────────────────────────────────────────
    # Line C — AND, MULU, MULS, EXG
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_lineC(self, op: int) -> str:
        dn      = (op >> 9) & 7
        dir_bit = (op >> 8) & 1
        sz_code = (op >> 6) & 3
        mode    = (op >> 3) & 7
        reg     = op & 7

        # EXG — exchange registers (no flags affected)
        # EXG Dn, Dm:  1100 rrr1 0100 0 rrr  → (op & 0xF1F8) == 0xC140
        # EXG An, Am:  1100 rrr1 0100 1 rrr  → (op & 0xF1F8) == 0xC148
        # EXG Dn, An:  1100 rrr1 1000 1 rrr  → (op & 0xF1F8) == 0xC188
        if (op & 0xF1F8) == 0xC140:
            t = self._d[dn]; self._d[dn] = self._d[reg]; self._d[reg] = t
            return f"EXG D{dn},D{reg}"
        if (op & 0xF1F8) == 0xC148:
            t = self._a[dn]; self._a[dn] = self._a[reg]; self._a[reg] = t
            return f"EXG A{dn},A{reg}"
        if (op & 0xF1F8) == 0xC188:
            t = self._d[dn]; self._d[dn] = self._a[reg]; self._a[reg] = t
            return f"EXG D{dn},A{reg}"

        # MULU.W <ea>, Dn: 1100 rrr0 11 mm rrr
        if sz_code == 3 and dir_bit == 0:
            b = self._ea_read(mode, reg, 2) & _WORD_MASK
            a = self._d[dn] & _WORD_MASK
            result = (a * b) & _LONG_MASK
            self._d[dn] = result
            self._set_ccr_logic(result, 4)
            return f"MULU {self._ea_str(mode,reg)},D{dn}"

        # MULS.W <ea>, Dn: 1100 rrr1 11 mm rrr
        if sz_code == 3 and dir_bit == 1:
            b = _sign_extend(self._ea_read(mode, reg, 2) & _WORD_MASK, 16)
            a = _sign_extend(self._d[dn] & _WORD_MASK, 16)
            result = (a * b) & _LONG_MASK
            self._d[dn] = result
            self._set_ccr_logic(result, 4)
            return f"MULS {self._ea_str(mode,reg)},D{dn}"

        # AND <ea>, Dn  (direction=0)
        # AND Dn, <ea>  (direction=1)
        sz = _SZ_ARITH.get(sz_code)
        if sz is None: raise RuntimeError(f"AND bad size 0x{op:04X}")

        if dir_bit == 0:
            b      = self._ea_read(mode, reg, sz)
            a      = self._d[dn] & _SZ_MASK[sz]
            result = (a & b) & _SZ_MASK[sz]
            self._set_dn(dn, result, sz)
            self._set_ccr_logic(result, sz)
            return f"AND{self._SZ_SUFFIX[sz]} {self._ea_str(mode,reg)},D{dn}"
        else:
            a     = self._d[dn] & _SZ_MASK[sz]
            val, addr = self._ea_read_addr(mode, reg, sz)
            result = (val & a) & _SZ_MASK[sz]
            self._mem_write(addr, sz, result)
            self._set_ccr_logic(result, sz)
            return f"AND{self._SZ_SUFFIX[sz]} D{dn},{self._ea_str(mode,reg)}"

    # ──────────────────────────────────────────────────────────────────────────
    # Line D — ADD, ADDA, ADDX
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_lineD(self, op: int) -> str:
        dn      = (op >> 9) & 7
        dir_bit = (op >> 8) & 1
        sz_code = (op >> 6) & 3
        mode    = (op >> 3) & 7
        reg     = op & 7

        # ADDA — ADD to address register (no flags set; always 32-bit result)
        # ADDA.W: sz_code=3, dir_bit=0  (sign-extends 16-bit source to 32)
        # ADDA.L: sz_code=3, dir_bit=1  (full 32-bit addition)
        if sz_code == 3 and dir_bit == 0:   # ADDA.W
            src = self._ea_read(mode, reg, 2)
            src = _sign_extend(src & _WORD_MASK, 16) & _LONG_MASK
            self._a[dn] = (self._a[dn] + src) & _LONG_MASK
            return f"ADDA.W {self._ea_str(mode,reg)},A{dn}"
        if sz_code == 3 and dir_bit == 1:   # ADDA.L
            src = self._ea_read(mode, reg, 4)
            self._a[dn] = (self._a[dn] + src) & _LONG_MASK
            return f"ADDA.L {self._ea_str(mode,reg)},A{dn}"

        sz = _SZ_ARITH.get(sz_code)
        if sz is None: raise RuntimeError(f"ADD bad size 0x{op:04X}")

        # ADDX Ds, Dd: 1101 rrr1 ss 00 0 rrr
        if dir_bit == 1 and mode == 0:   # ADDX register form
            x    = int(bool(self._sr & 0x10))
            a    = self._d[dn] & _SZ_MASK[sz]
            b    = self._d[reg] & _SZ_MASK[sz]
            raw  = a + b + x
            result = raw & _SZ_MASK[sz]
            self._set_dn(dn, result, sz)
            kw   = _sz_kwargs(sz)
            n_f  = compute_n(result, **kw)
            z_f  = bool(self._sr & 4) and (result == 0)
            v_f  = compute_v_add(a, b, result, **kw)
            c_f  = compute_c_sub(a + b + x, 0) or raw > _SZ_MASK[sz]
            c_f  = raw > _SZ_MASK[sz]
            self._set_ccr(n_f, z_f, v_f, c_f, x=c_f)
            return f"ADDX{self._SZ_SUFFIX[sz]} D{reg},D{dn}"

        if dir_bit == 0:   # ADD <ea>, Dn → Dn
            b   = self._ea_read(mode, reg, sz)
            a   = self._d[dn] & _SZ_MASK[sz]
            raw = a + b
            result = raw & _SZ_MASK[sz]
            self._set_dn(dn, result, sz)
            self._set_ccr_nzvc_add(a, b, raw, sz=sz)
            return f"ADD{self._SZ_SUFFIX[sz]} {self._ea_str(mode,reg)},D{dn}"
        else:               # ADD Dn, <ea> → <ea>
            a     = self._d[dn] & _SZ_MASK[sz]
            val, addr = self._ea_read_addr(mode, reg, sz)
            raw   = val + a
            result = raw & _SZ_MASK[sz]
            self._mem_write(addr, sz, result)
            self._set_ccr_nzvc_add(val, a, raw, sz=sz)
            return f"ADD{self._SZ_SUFFIX[sz]} D{dn},{self._ea_str(mode,reg)}"

    # ──────────────────────────────────────────────────────────────────────────
    # Line E — shifts and rotates
    # ──────────────────────────────────────────────────────────────────────────

    def _exec_lineE(self, op: int) -> str:
        """Shift and rotate instructions.

        Two main sub-families:
          Memory shifts (bit 6-7 = 11, bits 5-3 = mode):
            1110 ddd1 11 tt mm rrr  (d=dir, tt=type, ea=mm/rrr)
          Register shifts/rotates:
            1110 ccc d ss r tt rrr
              ccc = count/register (0=8, 1–7=1–7)
              d   = direction (0=right, 1=left)
              ss  = size (00=byte, 01=word, 10=long)
              r   = 0=imm count, 1=register count
              tt  = type (00=AS, 01=LS, 10=ROX, 11=RO)
              rrr = destination Dn
        """
        # Memory shift/rotate: top 5 bits = 11100 and sz_code = 3
        sz_code = (op >> 6) & 3
        if sz_code == 3:
            # 1110 ddd1 11 tt mm rrr — memory form (always word)
            direction = (op >> 9) & 1   # 1=left, 0=right
            shift_type = (op >> 3) & 3  # already the "tt" field... wait
            # Actually memory shifts: 1110 xxx1 11 mm rrr
            # where xxx = shift count for these... no.
            # Memory shift: 1110 0001 11 mm rrr — ASL.W mem
            # Encoding: 1110 d tt1 11 mm rrr  where d=dir, tt=type
            # op bits:  15-12=1110, 11=d, 10-9=tt, 8=1, 7-6=11(=3), 5-3=mode, 2-0=reg
            direction  = (op >> 11) & 1
            shift_type = (op >> 9) & 3
            mode       = (op >> 3) & 7
            reg        = op & 7
            # Memory shifts work on word only
            addr = self._ea_address(mode, reg, 2)
            val  = self._mem_read_word(addr)
            new_val, n_f, z_f, v_f, c_f, x_f = self._shift_word(
                val, 1, direction, shift_type
            )
            self._mem_write_word(addr, new_val)
            self._set_ccr(n_f, z_f, v_f, c_f, x=x_f)
            names = ["AS", "LS", "ROX", "RO"]
            d_char = "L" if direction else "R"
            return f"{names[shift_type]}{d_char}.W {self._ea_str(mode,reg)}"

        # Register shift/rotate
        sz        = _SZ_ARITH[sz_code]
        direction = (op >> 8) & 1    # 1=left, 0=right
        reg_count = (op >> 5) & 1    # 0=immediate count, 1=register count
        shift_type = (op >> 3) & 3   # 00=AS, 01=LS, 10=ROX, 11=RO
        dn         = op & 7
        cnt_field  = (op >> 9) & 7   # count or Dn index

        if reg_count:
            count = self._d[cnt_field] % 64
        else:
            count = 8 if cnt_field == 0 else cnt_field

        val = self._d[dn] & _SZ_MASK[sz]
        msb = _SZ_MSB[sz]
        mask = _SZ_MASK[sz]
        bits = sz * 8
        x_in = int(bool(self._sr & 0x10))

        new_val, n_f, z_f, v_f, c_f, x_f = self._shift_value(
            val, count, direction, shift_type, bits, mask, msb, x_in
        )
        self._set_dn(dn, new_val, sz)
        self._set_ccr(n_f, z_f, v_f, c_f, x=x_f)

        names = ["AS", "LS", "ROX", "RO"]
        d_char = "L" if direction else "R"
        cnt_s  = f"D{cnt_field}" if reg_count else str(count)
        return f"{names[shift_type]}{d_char}{self._SZ_SUFFIX[sz]} {cnt_s},D{dn}"

    def _shift_word(
        self, val: int, count: int, direction: int, shift_type: int
    ) -> tuple[int, bool, bool, bool, bool, bool]:
        """Shift/rotate a word (16-bit) value.  Returns (result, N, Z, V, C, X)."""
        return self._shift_value(val, count, direction, shift_type,
                                 16, _WORD_MASK, _WORD_MSB,
                                 int(bool(self._sr & 0x10)))

    def _shift_value(
        self, val: int, count: int, direction: int, shift_type: int,
        bits: int, mask: int, msb: int, x_in: int,
    ) -> tuple[int, bool, bool, bool, bool, bool]:
        """Core shift/rotate logic for a value of `bits` bits.

        Parameters
        ----------
        val        : unsigned value to shift (already masked to `bits`)
        count      : number of positions to shift
        direction  : 1=left, 0=right
        shift_type : 0=AS, 1=LS, 2=ROX (through X), 3=RO (circular)
        bits       : bit width (8, 16, or 32)
        mask, msb  : precomputed masks
        x_in       : current X flag (0 or 1)

        Returns (new_val, N, Z, V, C, X).
        """
        result = val
        last_out = 0
        v_flag = False

        if shift_type == 0:   # ──── Arithmetic shift ────────────────────────
            if direction == 1:   # ASL
                orig_msb = bool(val & msb)
                v_flag   = False
                for _ in range(count):
                    last_out = int(bool(result & msb))
                    result   = (result << 1) & mask
                    if bool(result & msb) != orig_msb:
                        v_flag = True
            else:                # ASR — replicate current MSB (arithmetic sign extend)
                sign_bit = result & msb
                for _ in range(count):
                    last_out = result & 1
                    result   = ((result >> 1) | sign_bit) & mask
                    # sign bit is stable for ASR — no need to update
                result &= mask

        elif shift_type == 1:  # ──── Logical shift ───────────────────────────
            if direction == 1:   # LSL
                for _ in range(count):
                    last_out = int(bool(result & msb))
                    result   = (result << 1) & mask
            else:                # LSR
                for _ in range(count):
                    last_out = result & 1
                    result   = (result >> 1) & mask

        elif shift_type == 2:  # ──── Rotate through X ─────────────────────────
            x = x_in
            if direction == 1:   # ROXL
                for _ in range(count):
                    last_out = int(bool(result & msb))
                    result   = ((result << 1) | x) & mask
                    x        = last_out
            else:                # ROXR
                for _ in range(count):
                    last_out = result & 1
                    result   = ((result >> 1) | (x << (bits - 1))) & mask
                    x        = last_out

        else:                  # ──── Circular rotate ─────────────────────────
            if count == 0:
                last_out = 0 if direction == 1 else int(bool(result & 1))
            elif direction == 1:   # ROL
                count    = count % bits
                last_out = int(bool(result & (msb >> (count - 1)))) if count else 0
                result   = ((result << count) | (result >> (bits - count))) & mask
                last_out = result & 1   # C = last bit rotated in = new LSB
            else:                  # ROR
                count    = count % bits
                result   = ((result >> count) | (result << (bits - count))) & mask
                last_out = int(bool(result & msb))   # C = last bit rotated = new MSB

        result &= mask
        n_f = bool(result & msb)
        z_f = (result == 0)
        c_f = bool(last_out)
        # X is unchanged for ROL/ROR (rotate-without-extend); X = C for all shift/ROXL/ROXR.
        x_f = bool(self._sr & 16) if shift_type == 3 else c_f

        # For ROL/ROR: C = last bit rotated out; if count=0 no rotation occurred → C cleared.
        if shift_type == 3 and count == 0:
            c_f = False   # no rotation → C cleared

        return result, n_f, z_f, v_flag, c_f, x_f
