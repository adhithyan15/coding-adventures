"""x86-64 (AMD64) behavioral simulator — Layer 07w.

This module implements the SIM00 ``Simulator[X86_64State]`` protocol for the
x86-64 ISA in 64-bit long mode.  It covers the full integer ISA; floating-
point, SSE/AVX, and privilege levels are out of scope (see *Simplifications*
in the spec).

Architecture summary
--------------------
* 16 × 64-bit GPRs: RAX RCX RDX RBX RSP RBP RSI RDI R8–R15
* RIP (instruction pointer) and RFLAGS (CF PF ZF SF OF tracked)
* 64 KiB flat byte-addressed little-endian memory (wraps modulo 65 536)
* Variable-length instructions decoded via opcode prefix + ModRM + SIB

Instruction encoding pipeline
------------------------------
Every x86-64 instruction is decoded by scanning bytes left-to-right:

    1. Skip legacy prefixes (F0/F2/F3/26/2E/36/3E/64/65/66/67)
    2. Check for REX prefix (0x40–0x4F):
         REX.W (bit 3) = 1 → 64-bit operand size (default without REX.W = 32-bit)
         REX.R (bit 2) = extends ModRM.reg by 8
         REX.X (bit 1) = extends SIB.index by 8
         REX.B (bit 0) = extends ModRM.rm / SIB.base / opcode-register by 8
    3. Fetch opcode byte.  If 0x0F, fetch second opcode byte.
    4. If the instruction uses a ModRM byte, fetch it:
         mod [7:6], reg [5:3], rm [2:0]
       * mod=11 → rm is register
       * mod=00 → rm is memory [reg] (rm=4 → SIB; rm=5 → [RIP+disp32])
       * mod=01 → [reg+disp8]  (rm=4 → SIB+disp8)
       * mod=10 → [reg+disp32] (rm=4 → SIB+disp32)
    5. If rm=4 and mod≠11, fetch SIB byte:
         scale [7:6], index [5:3], base [2:0]
         EA = base + index*(2^scale) [+ disp]
         index=4 means no index; base=5,mod=00 means disp32 only
    6. Fetch displacement (signed 8-bit or 32-bit).
    7. Fetch immediate (signed 8-, 32-, or 64-bit depending on instruction).

RFLAGS update rules
-------------------
ADD/ADC: CF=unsigned overflow, OF=signed overflow, SF=MSB, ZF=zero, PF=parity(low-byte)
SUB/SBB/CMP/NEG: same, using borrow-complement carry convention
AND/OR/XOR/TEST/NOT: CF=0, OF=0, SF=MSB, ZF=zero, PF=parity
INC/DEC: updates SF/ZF/PF/OF but NOT CF
SHL/SHR/SAR: CF=last bit shifted out, OF=(1-bit shift and MSB changed)

Condition codes (Jcc / CMOVcc / SETcc)
---------------------------------------
Code  Mnemonic  Condition
 0    O         OF=1
 1    NO        OF=0
 2    B/NAE/C   CF=1
 3    NB/AE/NC  CF=0
 4    Z/E       ZF=1
 5    NZ/NE     ZF=0
 6    BE/NA     CF=1 or ZF=1
 7    NBE/A     CF=0 and ZF=0
 8    S         SF=1
 9    NS        SF=0
10    P/PE      PF=1
11    NP/PO     PF=0
12    L/NGE     SF≠OF
13    NL/GE     SF=OF
14    LE/NG     ZF=1 or SF≠OF
15    NLE/G     ZF=0 and SF=OF
"""

from __future__ import annotations

from dataclasses import dataclass

from x86_64_simulator.state import (
    CF_BIT,
    MASK8,
    MASK16,
    MASK32,
    MASK64,
    MEM_SIZE,
    OF_BIT,
    PF_BIT,
    RAX,
    RCX,
    RDI,
    RDX,
    RSP,
    SF_BIT,
    ZF_BIT,
    X86_64State,
)

# ---------------------------------------------------------------------------
# Internal mutable CPU state (used only during execution; never exposed)
# ---------------------------------------------------------------------------

_LEGACY_PREFIXES = frozenset([
    0xF0, 0xF2, 0xF3,              # LOCK, REPNE, REP
    0x26, 0x2E, 0x36, 0x3E,       # segment overrides
    0x64, 0x65,                    # FS/GS segment overrides
    0x66, 0x67,                    # operand-size / address-size
])


class _CPU:
    """Mutable CPU state used during a single execution context.

    This is an implementation detail — callers only ever see ``X86_64State``
    snapshots.  All arithmetic is performed on Python arbitrary-precision
    integers; masking is applied explicitly at the write-back step.
    """

    __slots__ = (
        "gpr",    # list[int] — 16 × 64-bit unsigned values
        "pc",     # int — 64-bit unsigned RIP
        "rflags", # int — full RFLAGS (only CF/PF/ZF/SF/OF bits used)
        "memory", # bytearray — MEM_SIZE bytes, little-endian
        "halted", # bool
    )

    def __init__(self) -> None:
        self.gpr:    list[int] = [0] * 16
        self.pc:     int = 0
        self.rflags: int = 0
        self.memory: bytearray = bytearray(MEM_SIZE)
        self.halted: bool = False

    # ------------------------------------------------------------------
    # Snapshot
    # ------------------------------------------------------------------

    def snapshot(self) -> X86_64State:
        return X86_64State(
            pc=self.pc,
            gpr=tuple(self.gpr),
            rflags=self.rflags,
            memory=tuple(self.memory),
            halted=self.halted,
        )

    # ------------------------------------------------------------------
    # Memory helpers — little-endian, wrapping
    # ------------------------------------------------------------------

    def mem_read8(self, addr: int) -> int:
        return self.memory[addr & (MEM_SIZE - 1)]

    def mem_read16(self, addr: int) -> int:
        a = addr & (MEM_SIZE - 1)
        return self.memory[a] | (self.memory[(a + 1) & (MEM_SIZE - 1)] << 8)

    def mem_read32(self, addr: int) -> int:
        a = addr & (MEM_SIZE - 1)
        v = 0
        for i in range(4):
            v |= self.memory[(a + i) & (MEM_SIZE - 1)] << (8 * i)
        return v

    def mem_read64(self, addr: int) -> int:
        a = addr & (MEM_SIZE - 1)
        v = 0
        for i in range(8):
            v |= self.memory[(a + i) & (MEM_SIZE - 1)] << (8 * i)
        return v

    def mem_write8(self, addr: int, val: int) -> None:
        self.memory[addr & (MEM_SIZE - 1)] = val & MASK8

    def mem_write16(self, addr: int, val: int) -> None:
        a = addr & (MEM_SIZE - 1)
        self.memory[a]                     = val & 0xFF
        self.memory[(a + 1) & (MEM_SIZE - 1)] = (val >> 8) & 0xFF

    def mem_write32(self, addr: int, val: int) -> None:
        a = addr & (MEM_SIZE - 1)
        for i in range(4):
            self.memory[(a + i) & (MEM_SIZE - 1)] = (val >> (8 * i)) & 0xFF

    def mem_write64(self, addr: int, val: int) -> None:
        a = addr & (MEM_SIZE - 1)
        for i in range(8):
            self.memory[(a + i) & (MEM_SIZE - 1)] = (val >> (8 * i)) & 0xFF

    # ------------------------------------------------------------------
    # Register read/write helpers (all widths)
    # ------------------------------------------------------------------

    def read_reg(self, reg: int, bits: int) -> int:
        """Read *reg* (0–15) at the given *bits* width (8/16/32/64).

        For 8-bit reads with ``rex_present=False``, registers 4–7 are AH/CH/DH/BH.
        This simulator only calls read_reg8 from instruction handlers that
        already resolved the correct register index accounting for REX.
        """
        v = self.gpr[reg]
        if bits == 64: return v
        if bits == 32: return v & MASK32
        if bits == 16: return v & MASK16
        return v & MASK8

    def write_reg(self, reg: int, val: int, bits: int) -> None:
        """Write *val* (masked to *bits*) into *reg*.

        Writing a 32-bit value zero-extends to 64 bits (x86-64 rule).
        Writing 8- or 16-bit values does NOT zero the upper bits.
        """
        if bits == 64:
            self.gpr[reg] = val & MASK64
        elif bits == 32:
            # Writing 32-bit ALWAYS zeros the upper 32 bits in x86-64.
            self.gpr[reg] = val & MASK32
        elif bits == 16:
            self.gpr[reg] = (self.gpr[reg] & ~MASK16) | (val & MASK16)
        else:  # 8
            self.gpr[reg] = (self.gpr[reg] & ~MASK8) | (val & MASK8)

    # ------------------------------------------------------------------
    # Flag helpers
    # ------------------------------------------------------------------

    def _flag(self, bit: int) -> int:
        return (self.rflags >> bit) & 1

    def _set_flag(self, bit: int, val: int) -> None:
        if val:
            self.rflags |= 1 << bit
        else:
            self.rflags &= ~(1 << bit)

    def cf(self) -> int: return self._flag(CF_BIT)
    def pf(self) -> int: return self._flag(PF_BIT)
    def zf(self) -> int: return self._flag(ZF_BIT)
    def sf(self) -> int: return self._flag(SF_BIT)
    def of(self) -> int: return self._flag(OF_BIT)


# ---------------------------------------------------------------------------
# Arithmetic flag computation helpers
# ---------------------------------------------------------------------------

def _parity(val: int) -> int:
    """PF = 1 if the low byte of *val* has an even number of set bits."""
    b = val & 0xFF
    b ^= b >> 4
    b ^= b >> 2
    b ^= b >> 1
    return 1 - (b & 1)  # 1 = even parity (even number of 1 bits)


def _add_flags(cpu: _CPU, a: int, b: int, bits: int, result: int) -> None:
    """Set RFLAGS after result = a + b (all values unsigned, masked to bits)."""
    mask = (1 << bits) - 1
    r = result & mask
    msb = bits - 1
    N = (r >> msb) & 1
    Z = 1 if r == 0 else 0
    C = 1 if result > mask else 0
    a_s = (a >> msb) & 1
    b_s = (b >> msb) & 1
    r_s = N
    V = 1 if (a_s == b_s) and (r_s != a_s) else 0
    cpu._set_flag(SF_BIT, N)
    cpu._set_flag(ZF_BIT, Z)
    cpu._set_flag(CF_BIT, C)
    cpu._set_flag(OF_BIT, V)
    cpu._set_flag(PF_BIT, _parity(r))


def _sub_flags(cpu: _CPU, a: int, b: int, bits: int, result: int) -> None:
    """Set RFLAGS after result = a - b.

    Subtraction uses the borrow-complement carry convention:
    CF = 1 if an unsigned borrow occurred (a < b unsigned).
    """
    mask = (1 << bits) - 1
    r = result & mask
    msb = bits - 1
    N = (r >> msb) & 1
    Z = 1 if r == 0 else 0
    C = 1 if a < b else 0           # borrow = unsigned a < unsigned b (mod 2^bits)
    a_s = (a >> msb) & 1
    b_s = (b >> msb) & 1
    r_s = N
    # Overflow: subtraction overflows when operands have different signs and
    # result sign differs from minuend sign.
    V = 1 if (a_s != b_s) and (r_s != a_s) else 0
    cpu._set_flag(SF_BIT, N)
    cpu._set_flag(ZF_BIT, Z)
    cpu._set_flag(CF_BIT, C)
    cpu._set_flag(OF_BIT, V)
    cpu._set_flag(PF_BIT, _parity(r))


def _logic_flags(cpu: _CPU, result: int, bits: int) -> None:
    """Set RFLAGS after a logical operation (AND/OR/XOR).

    CF = 0, OF = 0.  SF, ZF, PF updated from result.
    """
    mask = (1 << bits) - 1
    r = result & mask
    cpu._set_flag(SF_BIT, (r >> (bits - 1)) & 1)
    cpu._set_flag(ZF_BIT, 1 if r == 0 else 0)
    cpu._set_flag(CF_BIT, 0)
    cpu._set_flag(OF_BIT, 0)
    cpu._set_flag(PF_BIT, _parity(r))


# ---------------------------------------------------------------------------
# Condition evaluation
# ---------------------------------------------------------------------------

def _condition_holds(cpu: _CPU, code: int) -> bool:
    """Return True if condition *code* (0–15) is satisfied by current RFLAGS.

    Code  Mnemonic  Condition
    ────────────────────────────────────────────────────
     0    O         OF
     1    NO        ¬OF
     2    B         CF
     3    NB/AE     ¬CF
     4    Z/E       ZF
     5    NZ/NE     ¬ZF
     6    BE        CF ∨ ZF
     7    NBE/A     ¬CF ∧ ¬ZF
     8    S         SF
     9    NS        ¬SF
    10    P/PE      PF
    11    NP/PO     ¬PF
    12    L/NGE     SF ≠ OF
    13    NL/GE     SF = OF
    14    LE        ZF ∨ (SF ≠ OF)
    15    NLE/G     ¬ZF ∧ (SF = OF)
    """
    cf = cpu.cf(); zf = cpu.zf(); sf = cpu.sf(); of = cpu.of(); pf = cpu.pf()
    match code:
        case 0:  return bool(of)
        case 1:  return not of
        case 2:  return bool(cf)
        case 3:  return not cf
        case 4:  return bool(zf)
        case 5:  return not zf
        case 6:  return bool(cf or zf)
        case 7:  return not cf and not zf
        case 8:  return bool(sf)
        case 9:  return not sf
        case 10: return bool(pf)
        case 11: return not pf
        case 12: return sf != of
        case 13: return sf == of
        case 14: return bool(zf) or (sf != of)
        case 15: return not zf and (sf == of)
        case _:  return False


# ---------------------------------------------------------------------------
# Instruction decoder helpers
# ---------------------------------------------------------------------------

class _Decoder:
    """Stateful byte stream decoder for one instruction.

    Reads bytes from *cpu.memory* starting at *cpu.pc* and advances
    an internal cursor.  After full decode, ``cursor`` holds the byte
    count of the entire instruction (used to advance RIP).
    """

    __slots__ = ("cpu", "cursor", "rex", "rex_w", "rex_r", "rex_x", "rex_b",
                 "opcode", "opcode2", "modrm", "mod", "reg", "rm",
                 "sib", "disp", "imm", "operand_bits", "addr_bits",
                 "_sib_scale", "_sib_index", "_sib_base")

    def __init__(self, cpu: _CPU) -> None:
        self.cpu = cpu
        self.cursor = 0
        self.rex = 0
        self.rex_w = 0
        self.rex_r = 0
        self.rex_x = 0
        self.rex_b = 0
        self.opcode = 0
        self.opcode2 = -1
        self.modrm = -1
        self.mod = 0
        self.reg = 0
        self.rm = 0
        self.sib = -1
        self.disp = 0
        self.imm = 0
        self.operand_bits = 32   # default; becomes 64 with REX.W
        self.addr_bits = 64
        self._sib_scale = 0
        self._sib_index = 0
        self._sib_base  = 0

    def _fetch(self) -> int:
        """Read one byte at pc+cursor and advance cursor."""
        b = self.cpu.memory[(self.cpu.pc + self.cursor) & (MEM_SIZE - 1)]
        self.cursor += 1
        return b

    def _fetch_s8(self) -> int:
        """Fetch one byte, sign-extended to Python int."""
        b = self._fetch()
        return b if b < 0x80 else b - 0x100

    def _fetch_u16(self) -> int:
        lo = self._fetch()
        hi = self._fetch()
        return lo | (hi << 8)

    def _fetch_s32(self) -> int:
        v = 0
        for i in range(4):
            v |= self._fetch() << (8 * i)
        # Sign-extend from 32 bits
        if v >= 0x8000_0000:
            v -= 0x1_0000_0000
        return v

    def _fetch_u32(self) -> int:
        v = 0
        for i in range(4):
            v |= self._fetch() << (8 * i)
        return v

    def _fetch_u64(self) -> int:
        v = 0
        for i in range(8):
            v |= self._fetch() << (8 * i)
        return v

    # ------------------------------------------------------------------
    # Prefix + opcode parsing
    # ------------------------------------------------------------------

    def decode_prefixes_and_opcode(self) -> None:
        """Skip legacy prefixes, consume optional REX, read opcode byte(s)."""
        while True:
            b = self._fetch()
            if b in _LEGACY_PREFIXES:
                continue  # skip all legacy prefixes
            if 0x40 <= b <= 0x4F:
                # REX prefix
                self.rex = b
                self.rex_w = (b >> 3) & 1
                self.rex_r = (b >> 2) & 1
                self.rex_x = (b >> 1) & 1
                self.rex_b = (b >> 0) & 1
                continue
            # Not a prefix — this is the opcode byte
            self.opcode = b
            break
        if self.rex_w:
            self.operand_bits = 64
        if self.opcode == 0x0F:
            self.opcode2 = self._fetch()

    # ------------------------------------------------------------------
    # ModRM + SIB + displacement
    # ------------------------------------------------------------------

    def decode_modrm(self) -> None:
        """Fetch and decode the ModRM byte."""
        b = self._fetch()
        self.modrm = b
        self.mod = (b >> 6) & 0x3
        self.reg  = ((b >> 3) & 0x7) | (self.rex_r << 3)
        self.rm   = (b & 0x7) | (self.rex_b << 3)

        if self.mod == 0b11:
            # Register addressing — no displacement or SIB
            return

        # Memory operand
        if (self.rm & 0x7) == 4:
            # SIB follows
            sib = self._fetch()
            self.sib = sib
            scale = (sib >> 6) & 0x3
            index = ((sib >> 3) & 0x7) | (self.rex_x << 3)
            base  = (sib & 0x7) | (self.rex_b << 3)
            # Store decoded SIB for use in _resolve_rm_addr
            self._sib_scale = scale
            self._sib_index = index
            self._sib_base  = base

        if self.mod == 0b00:
            if (self.rm & 0x7) == 5:
                # [RIP + disp32]
                self.disp = self._fetch_s32()
            elif self.sib != -1 and (self._sib_base & 0x7) == 5:
                # SIB with base=5, mod=00 → disp32 only (no base register)
                self.disp = self._fetch_s32()
        elif self.mod == 0b01:
            self.disp = self._fetch_s8()
        elif self.mod == 0b10:
            self.disp = self._fetch_s32()

    def _resolve_rm_addr(self) -> int:
        """Return the effective memory address for the rm field (mod != 11)."""
        cpu = self.cpu
        if self.sib != -1:
            scale = self._sib_scale
            index = self._sib_index
            base  = self._sib_base
            # index=4 (RSP in the low 3 bits) means no index register
            idx_val = 0 if (index & 0x7) == 4 else cpu.gpr[index]
            # base=5, mod=00 → no base (disp32 only)
            if (base & 0x7) == 5 and self.mod == 0:
                base_val = 0
            else:
                base_val = cpu.gpr[base]
            ea = base_val + idx_val * (1 << scale) + self.disp
        elif (self.rm & 0x7) == 5 and self.mod == 0:
            # [RIP + disp32]
            # RIP here is the PC *after* the current instruction.
            # We will fix this up after advance_pc() has been called from
            # the instruction handler; but since the decoder does not know
            # the instruction length yet, we approximate with pc + cursor
            # (which equals RIP after the fetch).
            ea = (cpu.pc + self.cursor + self.disp) & MASK64
        else:
            ea = (cpu.gpr[self.rm & 0xF] + self.disp) & MASK64
        return ea & (MEM_SIZE - 1)


# ---------------------------------------------------------------------------
# Operand read/write helpers (register or memory)
# ---------------------------------------------------------------------------

def _read_rm(cpu: _CPU, dec: _Decoder, bits: int) -> int:
    """Read the r/m operand (register or memory) at *bits* width."""
    if dec.mod == 0b11:
        return cpu.read_reg(dec.rm, bits)
    addr = dec._resolve_rm_addr()
    if bits == 64: return cpu.mem_read64(addr)
    if bits == 32: return cpu.mem_read32(addr)
    if bits == 16: return cpu.mem_read16(addr)
    return cpu.mem_read8(addr)


def _write_rm(cpu: _CPU, dec: _Decoder, val: int, bits: int) -> None:
    """Write *val* to the r/m operand (register or memory) at *bits* width."""
    if dec.mod == 0b11:
        cpu.write_reg(dec.rm, val, bits)
        return
    addr = dec._resolve_rm_addr()
    if bits == 64: cpu.mem_write64(addr, val)
    elif bits == 32: cpu.mem_write32(addr, val)
    elif bits == 16: cpu.mem_write16(addr, val)
    else: cpu.mem_write8(addr, val)


# ---------------------------------------------------------------------------
# Stack helpers
# ---------------------------------------------------------------------------

def _push64(cpu: _CPU, val: int) -> None:
    """Decrement RSP by 8 and write *val* as a 64-bit little-endian qword."""
    cpu.gpr[RSP] = (cpu.gpr[RSP] - 8) & MASK64
    cpu.mem_write64(cpu.gpr[RSP], val & MASK64)


def _pop64(cpu: _CPU) -> int:
    """Read a 64-bit qword from [RSP] and increment RSP by 8."""
    val = cpu.mem_read64(cpu.gpr[RSP])
    cpu.gpr[RSP] = (cpu.gpr[RSP] + 8) & MASK64
    return val


# ---------------------------------------------------------------------------
# Sign extension helpers
# ---------------------------------------------------------------------------

def _sx(val: int, from_bits: int) -> int:
    """Sign-extend *val* from *from_bits* to a Python int."""
    sign_bit = 1 << (from_bits - 1)
    return (val & (sign_bit - 1)) - (val & sign_bit)


# ---------------------------------------------------------------------------
# Instruction executor
# ---------------------------------------------------------------------------

def _step(cpu: _CPU) -> None:
    """Fetch, decode, and execute one instruction; advance RIP."""
    dec = _Decoder(cpu)
    dec.decode_prefixes_and_opcode()

    op = dec.opcode
    op2 = dec.opcode2
    bits = dec.operand_bits  # 64 with REX.W, else 32 (64-bit long mode default)

    # Advance RIP after full decode is complete; branches overwrite pc directly.
    # We keep the raw cursor length so we can advance pc below.

    # ------------------------------------------------------------------
    # 0x0F extended opcodes
    # ------------------------------------------------------------------
    if op == 0x0F:
        op = op2

        # --- Jcc rel32: 0F 80–8F — no ModRM ---
        if 0x80 <= op <= 0x8F:
            disp = dec._fetch_s32()
            target = (cpu.pc + dec.cursor + disp) & MASK64
            cc = op & 0xF
            cpu.pc += dec.cursor
            if _condition_holds(cpu, cc):
                cpu.pc = target
            return

        # --- BSWAP r64: 0F C8–CF — no ModRM, register in opcode byte ---
        if 0xC8 <= op <= 0xCF:
            reg = (op - 0xC8) | (dec.rex_b << 3)
            v = cpu.gpr[reg] & MASK64
            # Reverse the 8 bytes
            b = [(v >> (8 * i)) & 0xFF for i in range(8)]
            b.reverse()
            result = sum(b[i] << (8 * i) for i in range(8))
            cpu.write_reg(reg, result, 64)
            cpu.pc += dec.cursor
            return

        # All remaining 0F instructions use ModRM
        dec.decode_modrm()
        reg = dec.reg   # /r or opcode extension
        rm_val = _read_rm(cpu, dec, bits)

        # --- 0F AF: IMUL r64, r/m64 ---
        if op == 0xAF:
            a = _sx(cpu.gpr[reg], bits)
            b = _sx(rm_val, bits)
            result = (a * b) & ((1 << bits) - 1)
            cpu.write_reg(reg, result, bits)
            # CF=OF=1 if result was truncated (high bits differ from sign of low)
            hi = a * b >> bits
            trunc = 1 if hi not in (0, -1) else 0
            cpu._set_flag(CF_BIT, trunc)
            cpu._set_flag(OF_BIT, trunc)

        # --- 0F B6: MOVZX r64, r/m8 ---
        elif op == 0xB6:
            val8 = _read_rm(cpu, dec, 8)
            cpu.write_reg(reg, val8 & MASK8, bits)

        # --- 0F B7: MOVZX r64, r/m16 ---
        elif op == 0xB7:
            val16 = _read_rm(cpu, dec, 16)
            cpu.write_reg(reg, val16 & MASK16, bits)

        # --- 0F BE: MOVSX r64, r/m8 ---
        elif op == 0xBE:
            val8 = _read_rm(cpu, dec, 8)
            cpu.write_reg(reg, _sx(val8, 8) & MASK64, bits)

        # --- 0F BF: MOVSX r64, r/m16 ---
        elif op == 0xBF:
            val16 = _read_rm(cpu, dec, 16)
            cpu.write_reg(reg, _sx(val16, 16) & MASK64, bits)

        # --- 0F 63: MOVSXD r64, r/m32 (REX.W) ---
        # Note: 63h without REX.W is ARPL (not used in 64-bit mode); we treat it
        # as MOVSXD whenever REX.W=1.

        # --- CMOVcc: 0F 40–4F ---
        elif 0x40 <= op <= 0x4F:
            cc = op & 0xF
            if _condition_holds(cpu, cc):
                cpu.write_reg(reg, rm_val, bits)

        # --- SETcc: 0F 90–9F ---
        elif 0x90 <= op <= 0x9F:
            cc = op & 0xF
            _write_rm(cpu, dec, 1 if _condition_holds(cpu, cc) else 0, 8)

        # --- 0F BC: BSF r64, r/m64 ---
        elif op == 0xBC:
            if rm_val == 0:
                cpu._set_flag(ZF_BIT, 1)
            else:
                cpu._set_flag(ZF_BIT, 0)
                idx = 0
                while not (rm_val >> idx) & 1:
                    idx += 1
                cpu.write_reg(reg, idx, bits)

        # --- 0F BD: BSR r64, r/m64 ---
        elif op == 0xBD:
            if rm_val == 0:
                cpu._set_flag(ZF_BIT, 1)
            else:
                cpu._set_flag(ZF_BIT, 0)
                idx = bits - 1
                while idx > 0 and not (rm_val >> idx) & 1:
                    idx -= 1
                cpu.write_reg(reg, idx, bits)

        # --- 0F A3: BT r/m64, r64 ---
        elif op == 0xA3:
            bit_idx = cpu.gpr[reg] % bits
            cpu._set_flag(CF_BIT, (rm_val >> bit_idx) & 1)

        # --- 0F BA: BT r/m64, imm8 (/4) ---
        elif op == 0xBA:
            imm = dec._fetch() & (bits - 1)
            cpu._set_flag(CF_BIT, (rm_val >> imm) & 1)

        # --- 0F C8–CF: BSWAP r64 --- no ModRM; register in low 3 bits of opcode
        # NOTE: We already called decode_modrm() above for the general 0F handler.
        # BSWAP does NOT use ModRM.  This is handled by checking op BEFORE calling
        # decode_modrm.  See the early-exit check below.

        cpu.pc += dec.cursor
        return

    # ------------------------------------------------------------------
    # BSWAP (0F C8+rd) — single byte without ModRM, decoded before 0F handler
    # Actual opcode range 0xC8–0xCF (after 0x0F prefix).
    # Re-check here since op was overwritten with op2 above.
    # We handle this inline in the 0F block implicitly; register is
    # encoded in low 3 bits of the second byte.
    # ------------------------------------------------------------------

    # ------------------------------------------------------------------
    # Single-byte opcodes (no 0x0F prefix)
    # ------------------------------------------------------------------

    # --- NOP (90) ---
    if op == 0x90:
        cpu.pc += dec.cursor
        return

    # --- HLT (F4) ---
    if op == 0xF4:
        cpu.halted = True
        cpu.pc += dec.cursor
        return

    # --- PUSH r64 (50–57 + REX.B) ---
    if 0x50 <= op <= 0x57:
        reg = (op - 0x50) | (dec.rex_b << 3)
        _push64(cpu, cpu.gpr[reg])
        cpu.pc += dec.cursor
        return

    # --- POP r64 (58–5F + REX.B) ---
    if 0x58 <= op <= 0x5F:
        reg = (op - 0x58) | (dec.rex_b << 3)
        cpu.write_reg(reg, _pop64(cpu), 64)
        cpu.pc += dec.cursor
        return

    # --- MOV r64, imm64 (REX.W B8+rd io) ---
    if 0xB8 <= op <= 0xBF:
        reg = (op - 0xB8) | (dec.rex_b << 3)
        if dec.rex_w:
            imm = dec._fetch_u64()
        else:
            imm = dec._fetch_u32()
        cpu.write_reg(reg, imm, bits)
        cpu.pc += dec.cursor
        return

    # --- BSWAP (0F C8+rd) — handled here for completeness; reachable via
    # the extended-opcode branch above when op (originally op2) is 0xC8–0xCF
    # We already re-assigned op=op2 above so this branch is NOT reached.
    # Real bswap is handled in the 0F block above after op is re-assigned. ---

    # --- PUSH imm8 (6A ib) ---
    if op == 0x6A:
        imm = dec._fetch_s8()
        _push64(cpu, imm & MASK64)
        cpu.pc += dec.cursor
        return

    # --- PUSH imm32 (68 id) --- sign-extended to 64 bits
    if op == 0x68:
        imm = dec._fetch_s32()
        _push64(cpu, imm & MASK64)
        cpu.pc += dec.cursor
        return

    # --- JMP rel8 (EB cb) ---
    if op == 0xEB:
        disp = dec._fetch_s8()
        target = (cpu.pc + dec.cursor + disp) & MASK64
        cpu.pc = target
        return

    # --- JMP rel32 (E9 cd) ---
    if op == 0xE9:
        disp = dec._fetch_s32()
        target = (cpu.pc + dec.cursor + disp) & MASK64
        cpu.pc = target
        return

    # --- CALL rel32 (E8 cd) ---
    if op == 0xE8:
        disp = dec._fetch_s32()
        ret_addr = (cpu.pc + dec.cursor) & MASK64
        _push64(cpu, ret_addr)
        cpu.pc = (ret_addr + disp) & MASK64
        return

    # --- RET (C3) ---
    if op == 0xC3:
        cpu.pc = _pop64(cpu)
        return

    # --- RET imm16 (C2 iw) — pop RIP then add imm16 to RSP ---
    if op == 0xC2:
        imm = dec._fetch_u16()
        cpu.pc = _pop64(cpu)
        cpu.gpr[RSP] = (cpu.gpr[RSP] + imm) & MASK64
        return

    # --- Jcc rel8 (70–7F) ---
    if 0x70 <= op <= 0x7F:
        cc = op & 0xF
        disp = dec._fetch_s8()
        target = (cpu.pc + dec.cursor + disp) & MASK64
        cpu.pc += dec.cursor
        if _condition_holds(cpu, cc):
            cpu.pc = target
        return

    # --- LOOP rel8 (E2); LOOPE (E1); LOOPNE (E0) ---
    if op in (0xE0, 0xE1, 0xE2):
        disp = dec._fetch_s8()
        # Decrement RCX (64-bit)
        cpu.gpr[RCX] = (cpu.gpr[RCX] - 1) & MASK64
        rcx_nz = cpu.gpr[RCX] != 0
        target = (cpu.pc + dec.cursor + disp) & MASK64
        cpu.pc += dec.cursor
        if op == 0xE2:               # LOOP: branch if RCX ≠ 0
            if rcx_nz:
                cpu.pc = target
        elif op == 0xE1:             # LOOPE: branch if RCX ≠ 0 and ZF=1
            if rcx_nz and cpu.zf():
                cpu.pc = target
        else:                        # LOOPNE: branch if RCX ≠ 0 and ZF=0
            if rcx_nz and not cpu.zf():
                cpu.pc = target
        return

    # --- JRCXZ rel8 (E3 cb) ---
    if op == 0xE3:
        disp = dec._fetch_s8()
        target = (cpu.pc + dec.cursor + disp) & MASK64
        cpu.pc += dec.cursor
        if cpu.gpr[RCX] == 0:
            cpu.pc = target
        return

    # --- REP STOSQ (F3 AB) — store RAX into [RDI] × RCX qwords ---
    # Note: the F3 prefix is consumed as a legacy prefix.  By the time we
    # reach here op=0xAB which encodes STOSD/STOSQ.  We implement STOSQ
    # (REX.W=1) only; without REX.W it stores 32-bit DWORD.
    if op == 0xAB:
        # REP was already handled as prefix; we simulate the full REP loop.
        store_bits = 64 if dec.rex_w else 32
        store_bytes = store_bits // 8
        while cpu.gpr[RCX] > 0:
            addr = cpu.gpr[RDI] & (MEM_SIZE - 1)
            if store_bits == 64:
                cpu.mem_write64(addr, cpu.gpr[RAX])
            else:
                cpu.mem_write32(addr, cpu.gpr[RAX] & MASK32)
            # Direction flag (DF) not tracked; we always increment.
            cpu.gpr[RDI] = (cpu.gpr[RDI] + store_bytes) & MASK64
            cpu.gpr[RCX] = (cpu.gpr[RCX] - 1) & MASK64
        cpu.pc += dec.cursor
        return

    # ------------------------------------------------------------------
    # All remaining instructions require a ModRM byte
    # ------------------------------------------------------------------
    dec.decode_modrm()
    reg = dec.reg
    mask = (1 << bits) - 1

    # --- MOV r/m, r (88 = byte; 89 = dword/qword) ---
    if op == 0x89:
        _write_rm(cpu, dec, cpu.gpr[reg], bits)
    elif op == 0x88:
        _write_rm(cpu, dec, cpu.gpr[reg] & MASK8, 8)

    # --- MOV r, r/m (8A = byte; 8B = dword/qword) ---
    elif op == 0x8B:
        cpu.write_reg(reg, _read_rm(cpu, dec, bits), bits)
    elif op == 0x8A:
        cpu.write_reg(reg, _read_rm(cpu, dec, 8), 8)

    # --- MOVSXD r64, r/m32 (63) — with REX.W=1 ---
    elif op == 0x63:
        if dec.rex_w:
            val32 = _read_rm(cpu, dec, 32)
            cpu.write_reg(reg, _sx(val32, 32) & MASK64, 64)
        else:
            # Without REX.W behaves as MOV r32, r/m32
            cpu.write_reg(reg, _read_rm(cpu, dec, 32), 32)

    # --- MOV r/m64, imm32-sign-extended (C7 /0) ---
    elif op == 0xC7:
        imm = dec._fetch_s32()
        _write_rm(cpu, dec, imm & mask, bits)

    # --- IMUL r64, r/m64, imm8  (6B /r ib)  — three-operand signed multiply ---
    # Encodes as: [REX.W] 6B /r ib
    # dest (reg) = src1 (r/m) × sign-extended-imm8
    elif op == 0x6B:
        src = _read_rm(cpu, dec, bits)
        imm = dec._fetch_s8()
        result = (_sx(src, bits) * imm) & mask
        cpu.write_reg(reg, result, bits)
        # CF=OF=1 if result was truncated (signed overflow)
        full = _sx(src, bits) * imm
        trunc = 1 if (full != _sx(result, bits)) else 0
        cpu._set_flag(CF_BIT, trunc)
        cpu._set_flag(OF_BIT, trunc)

    # --- IMUL r64, r/m64, imm32 (69 /r id) — three-operand, 32-bit immediate ---
    elif op == 0x69:
        src = _read_rm(cpu, dec, bits)
        imm = dec._fetch_s32()
        result = (_sx(src, bits) * imm) & mask
        cpu.write_reg(reg, result, bits)
        full = _sx(src, bits) * imm
        trunc = 1 if (full != _sx(result, bits)) else 0
        cpu._set_flag(CF_BIT, trunc)
        cpu._set_flag(OF_BIT, trunc)

    # --- XCHG r/m64, r64 (87 /r) ---
    elif op == 0x87:
        rm_v = _read_rm(cpu, dec, bits)
        r_v  = cpu.gpr[reg]
        _write_rm(cpu, dec, r_v, bits)
        cpu.write_reg(reg, rm_v, bits)

    # --- LEA r64, m (8D /r) ---
    elif op == 0x8D:
        if dec.mod == 0b11:
            raise ValueError("LEA requires a memory operand")
        ea = dec._resolve_rm_addr()
        cpu.write_reg(reg, ea, bits)

    # --- PUSH r/m64 (FF /6) ---
    elif op == 0xFF and reg == 6:
        val = _read_rm(cpu, dec, 64)
        _push64(cpu, val)

    # --- POP r/m64 (8F /0) ---
    elif op == 0x8F and reg == 0:
        val = _pop64(cpu)
        _write_rm(cpu, dec, val, 64)

    # --- JMP r/m64 (FF /4) ---
    elif op == 0xFF and reg == 4:
        target = _read_rm(cpu, dec, 64)
        cpu.pc += dec.cursor
        cpu.pc = target
        return

    # --- CALL r/m64 (FF /2) ---
    elif op == 0xFF and reg == 2:
        target = _read_rm(cpu, dec, 64)
        ret_addr = (cpu.pc + dec.cursor) & MASK64
        _push64(cpu, ret_addr)
        cpu.pc = target
        return

    # --- INC r/m64 (FF /0) ---
    elif op == 0xFF and reg == 0:
        rm_v = _read_rm(cpu, dec, bits)
        result = rm_v + 1
        r = result & mask
        # INC does not update CF
        a_s = (rm_v >> (bits - 1)) & 1
        r_s = (r >> (bits - 1)) & 1
        cpu._set_flag(OF_BIT, 1 if a_s == 0 and r_s == 1 else 0)
        cpu._set_flag(SF_BIT, r_s)
        cpu._set_flag(ZF_BIT, 1 if r == 0 else 0)
        cpu._set_flag(PF_BIT, _parity(r))
        _write_rm(cpu, dec, r, bits)

    # --- DEC r/m64 (FF /1) ---
    elif op == 0xFF and reg == 1:
        rm_v = _read_rm(cpu, dec, bits)
        result = rm_v - 1
        r = result & mask
        a_s = (rm_v >> (bits - 1)) & 1
        r_s = (r >> (bits - 1)) & 1
        cpu._set_flag(OF_BIT, 1 if a_s == 1 and r_s == 0 else 0)
        cpu._set_flag(SF_BIT, r_s)
        cpu._set_flag(ZF_BIT, 1 if r == 0 else 0)
        cpu._set_flag(PF_BIT, _parity(r))
        _write_rm(cpu, dec, r, bits)

    # ------------------------------------------------------------------
    # Arithmetic: ADD / ADC / SUB / SBB / AND / XOR / OR / CMP (80–83)
    # Opcode group 0x80: r/m8, imm8
    #              0x81: r/m64, imm32 (sign-extended)
    #              0x83: r/m64, imm8  (sign-extended)
    # ------------------------------------------------------------------
    elif op in (0x80, 0x81, 0x83):
        if op == 0x80:
            op_bits = 8
            imm = dec._fetch_s8()
        elif op == 0x81:
            op_bits = bits
            imm = dec._fetch_s32()
        else:  # 0x83
            op_bits = bits
            imm = dec._fetch_s8()
        rm_v = _read_rm(cpu, dec, op_bits)
        op_mask = (1 << op_bits) - 1
        imm_m = imm & op_mask
        rm_m = rm_v & op_mask
        ext = reg  # /digit selects operation
        if ext == 0:   # ADD
            r = (rm_m + imm_m) & op_mask
            _add_flags(cpu, rm_m, imm_m, op_bits, rm_m + imm_m)
            _write_rm(cpu, dec, r, op_bits)
        elif ext == 1: # OR
            r = rm_m | imm_m
            _logic_flags(cpu, r, op_bits)
            _write_rm(cpu, dec, r, op_bits)
        elif ext == 2: # ADC
            c = cpu.cf()
            full = rm_m + imm_m + c
            r = full & op_mask
            _add_flags(cpu, rm_m, imm_m + c, op_bits, full)
            _write_rm(cpu, dec, r, op_bits)
        elif ext == 3: # SBB
            c = cpu.cf()
            b = (imm_m + c) & op_mask
            full = rm_m - b
            r = full & op_mask
            _sub_flags(cpu, rm_m, b, op_bits, full)
            _write_rm(cpu, dec, r, op_bits)
        elif ext == 4: # AND
            r = rm_m & imm_m
            _logic_flags(cpu, r, op_bits)
            _write_rm(cpu, dec, r, op_bits)
        elif ext == 5: # SUB
            full = rm_m - imm_m
            r = full & op_mask
            _sub_flags(cpu, rm_m, imm_m, op_bits, full)
            _write_rm(cpu, dec, r, op_bits)
        elif ext == 6: # XOR
            r = rm_m ^ imm_m
            _logic_flags(cpu, r, op_bits)
            _write_rm(cpu, dec, r, op_bits)
        elif ext == 7: # CMP (SUB, discard result)
            full = rm_m - imm_m
            _sub_flags(cpu, rm_m, imm_m, op_bits, full)

    # ------------------------------------------------------------------
    # ADD r/m64, r64 (01); ADD r64, r/m64 (03)
    # ------------------------------------------------------------------
    elif op == 0x01:
        rm_v = _read_rm(cpu, dec, bits)
        r_v  = cpu.gpr[reg] & mask
        result = rm_v + r_v
        _add_flags(cpu, rm_v, r_v, bits, result)
        _write_rm(cpu, dec, result & mask, bits)
    elif op == 0x03:
        rm_v = _read_rm(cpu, dec, bits)
        r_v  = cpu.gpr[reg] & mask
        result = r_v + rm_v
        _add_flags(cpu, r_v, rm_v, bits, result)
        cpu.write_reg(reg, result & mask, bits)

    # --- ADC r/m64, r64 (11); ADC r64, r/m64 (13) ---
    elif op == 0x11:
        rm_v = _read_rm(cpu, dec, bits)
        c = cpu.cf()
        full = rm_v + (cpu.gpr[reg] & mask) + c
        _add_flags(cpu, rm_v, (cpu.gpr[reg] & mask) + c, bits, full)
        _write_rm(cpu, dec, full & mask, bits)
    elif op == 0x13:
        rm_v = _read_rm(cpu, dec, bits)
        c = cpu.cf()
        full = (cpu.gpr[reg] & mask) + rm_v + c
        _add_flags(cpu, cpu.gpr[reg] & mask, rm_v + c, bits, full)
        cpu.write_reg(reg, full & mask, bits)

    # --- SUB r/m64, r64 (29); SUB r64, r/m64 (2B) ---
    elif op == 0x29:
        rm_v = _read_rm(cpu, dec, bits)
        r_v  = cpu.gpr[reg] & mask
        full = rm_v - r_v
        _sub_flags(cpu, rm_v, r_v, bits, full)
        _write_rm(cpu, dec, full & mask, bits)
    elif op == 0x2B:
        rm_v = _read_rm(cpu, dec, bits)
        r_v  = cpu.gpr[reg] & mask
        full = r_v - rm_v
        _sub_flags(cpu, r_v, rm_v, bits, full)
        cpu.write_reg(reg, full & mask, bits)

    # --- SBB r/m64, r64 (19); SBB r64, r/m64 (1B) ---
    # Intel SBB: DEST ← DEST − (SRC + CF_in)
    # Borrow (CF_out) = 1 if DEST < SRC + CF_in in infinite-precision arithmetic.
    # We must pass the *unmasked* sum (b = SRC + CF_in) to _sub_flags so that
    # the carry check `a < b` uses the true mathematical value — critical when
    # SRC = MASK64 and CF_in = 1, making b = 2^64 (which would wrap to 0 if masked).
    elif op == 0x19:
        rm_v = _read_rm(cpu, dec, bits)
        r_v = cpu.gpr[reg] & mask
        c = cpu.cf()
        b = r_v + c                         # unmasked: can be up to MASK64+1
        full = rm_v - b
        _sub_flags(cpu, rm_v, b, bits, full)   # b unmasked for correct CF
        _write_rm(cpu, dec, full & mask, bits)
    elif op == 0x1B:
        rm_v = _read_rm(cpu, dec, bits)
        r_v = cpu.gpr[reg] & mask
        c = cpu.cf()
        b = rm_v + c                         # unmasked: can be up to MASK64+1
        full = r_v - b
        _sub_flags(cpu, r_v, b, bits, full)    # b unmasked for correct CF
        cpu.write_reg(reg, full & mask, bits)

    # --- AND r/m64, r64 (21); AND r64, r/m64 (23) ---
    elif op == 0x21:
        rm_v = _read_rm(cpu, dec, bits)
        r = (rm_v & cpu.gpr[reg]) & mask
        _logic_flags(cpu, r, bits)
        _write_rm(cpu, dec, r, bits)
    elif op == 0x23:
        rm_v = _read_rm(cpu, dec, bits)
        r = (cpu.gpr[reg] & rm_v) & mask
        _logic_flags(cpu, r, bits)
        cpu.write_reg(reg, r, bits)

    # --- OR r/m64, r64 (09); OR r64, r/m64 (0B) ---
    elif op == 0x09:
        rm_v = _read_rm(cpu, dec, bits)
        r = (rm_v | cpu.gpr[reg]) & mask
        _logic_flags(cpu, r, bits)
        _write_rm(cpu, dec, r, bits)
    elif op == 0x0B:
        rm_v = _read_rm(cpu, dec, bits)
        r = (cpu.gpr[reg] | rm_v) & mask
        _logic_flags(cpu, r, bits)
        cpu.write_reg(reg, r, bits)

    # --- XOR r/m64, r64 (31); XOR r64, r/m64 (33) ---
    elif op == 0x31:
        rm_v = _read_rm(cpu, dec, bits)
        r = (rm_v ^ cpu.gpr[reg]) & mask
        _logic_flags(cpu, r, bits)
        _write_rm(cpu, dec, r, bits)
    elif op == 0x33:
        rm_v = _read_rm(cpu, dec, bits)
        r = (cpu.gpr[reg] ^ rm_v) & mask
        _logic_flags(cpu, r, bits)
        cpu.write_reg(reg, r, bits)

    # --- CMP r/m64, r64 (39); CMP r64, r/m64 (3B) ---
    elif op == 0x39:
        rm_v = _read_rm(cpu, dec, bits)
        r_v  = cpu.gpr[reg] & mask
        _sub_flags(cpu, rm_v, r_v, bits, rm_v - r_v)
    elif op == 0x3B:
        rm_v = _read_rm(cpu, dec, bits)
        r_v  = cpu.gpr[reg] & mask
        _sub_flags(cpu, r_v, rm_v, bits, r_v - rm_v)

    # --- TEST r/m64, r64 (85); TEST r/m8, r8 (84) ---
    elif op == 0x85:
        rm_v = _read_rm(cpu, dec, bits)
        r = (rm_v & cpu.gpr[reg]) & mask
        _logic_flags(cpu, r, bits)
    elif op == 0x84:
        rm_v = _read_rm(cpu, dec, 8)
        r = rm_v & (cpu.gpr[reg] & MASK8)
        _logic_flags(cpu, r, 8)

    # --- F7 group: TEST/NOT/NEG/MUL/IMUL/DIV/IDIV ---
    elif op == 0xF7:
        rm_v = _read_rm(cpu, dec, bits)
        if reg == 0:   # TEST r/m64, imm32 (sign-extended)
            imm = dec._fetch_s32() & mask
            r = (rm_v & imm) & mask
            _logic_flags(cpu, r, bits)
        elif reg == 2:  # NOT
            _write_rm(cpu, dec, (~rm_v) & mask, bits)
        elif reg == 3:  # NEG
            r = (-rm_v) & mask
            _sub_flags(cpu, 0, rm_v & mask, bits, -rm_v)
            _write_rm(cpu, dec, r, bits)
        elif reg == 4:  # MUL RAX, r/m64 → RDX:RAX
            a = cpu.gpr[RAX] & mask
            product = a * (rm_v & mask)
            lo = product & mask
            hi = (product >> bits) & mask
            cpu.write_reg(RAX, lo, bits)
            cpu.write_reg(RDX, hi, bits)
            cpu._set_flag(CF_BIT, 1 if hi != 0 else 0)
            cpu._set_flag(OF_BIT, 1 if hi != 0 else 0)
        elif reg == 5:  # IMUL RAX, r/m64 → RDX:RAX (signed)
            a = _sx(cpu.gpr[RAX] & mask, bits)
            b = _sx(rm_v & mask, bits)
            product = a * b
            lo = product & mask
            hi = (product >> bits) & mask
            cpu.write_reg(RAX, lo, bits)
            cpu.write_reg(RDX, hi, bits)
            trunc = 1 if hi not in (0, -1) else 0
            cpu._set_flag(CF_BIT, trunc)
            cpu._set_flag(OF_BIT, trunc)
        elif reg == 6:  # DIV RDX:RAX / r/m64 → quotient=RAX, remainder=RDX
            divisor = rm_v & mask
            if divisor == 0:
                # Divide by zero: leave registers unchanged (simulator choice)
                pass
            else:
                dividend = ((cpu.gpr[RDX] & mask) << bits) | (cpu.gpr[RAX] & mask)
                cpu.write_reg(RAX, (dividend // divisor) & mask, bits)
                cpu.write_reg(RDX, (dividend % divisor) & mask, bits)
        elif reg == 7:  # IDIV (signed)
            divisor = _sx(rm_v & mask, bits)
            if divisor == 0:
                pass
            else:
                # Rebuild signed dividend from RDX:RAX
                rdx_v = _sx(cpu.gpr[RDX] & mask, bits)
                rax_v = cpu.gpr[RAX] & mask
                dividend = (rdx_v << bits) | rax_v
                # Python // truncates toward -inf; C truncates toward 0
                q = int(dividend / divisor)  # truncate toward zero
                r = dividend - q * divisor
                cpu.write_reg(RAX, q & mask, bits)
                cpu.write_reg(RDX, r & mask, bits)

    # --- Shift group: D0/D1 (×1), D2/D3 (×CL), C0/C1 (×imm8) ---
    elif op in (0xC0, 0xC1, 0xD0, 0xD1, 0xD2, 0xD3):
        sh_bits = 8 if op in (0xD0, 0xD2, 0xC0) else bits
        rm_v = _read_rm(cpu, dec, sh_bits)
        if op in (0xD0, 0xD1):
            raw_count = 1
        elif op in (0xD2, 0xD3):
            raw_count = cpu.gpr[RCX] & 0x3F
        else:  # C0/C1
            raw_count = dec._fetch() & 0x3F
        sh_mask_full = (1 << sh_bits) - 1
        count = raw_count & (sh_bits - 1)  # effective shift count
        ext = reg  # /4=SHL /5=SHR /7=SAR /0=ROL /1=ROR

        def _sh_flags(r: int, new_cf: int) -> None:
            """Set SF/ZF/PF from result; set CF to new_cf; leave OF."""
            cpu._set_flag(SF_BIT, (r >> (sh_bits - 1)) & 1)
            cpu._set_flag(ZF_BIT, 1 if r == 0 else 0)
            cpu._set_flag(PF_BIT, _parity(r))
            cpu._set_flag(CF_BIT, new_cf)

        if count == 0:
            pass  # shifts by 0 leave all flags unchanged
        elif ext in (4, 6):  # SHL/SAL
            new_cf = (rm_v >> (sh_bits - count)) & 1
            r = (rm_v << count) & sh_mask_full
            _sh_flags(r, new_cf)
            if raw_count == 1:
                cpu._set_flag(OF_BIT, ((r >> (sh_bits - 1)) ^ new_cf) & 1)
            _write_rm(cpu, dec, r, sh_bits)
        elif ext == 5:  # SHR (logical)
            new_cf = (rm_v >> (count - 1)) & 1
            r = (rm_v >> count) & sh_mask_full
            _sh_flags(r, new_cf)
            if raw_count == 1:
                cpu._set_flag(OF_BIT, (rm_v >> (sh_bits - 1)) & 1)
            _write_rm(cpu, dec, r, sh_bits)
        elif ext == 7:  # SAR (arithmetic)
            new_cf = (rm_v >> (count - 1)) & 1
            signed_v = _sx(rm_v, sh_bits)
            r = (signed_v >> count) & sh_mask_full
            _sh_flags(r, new_cf)
            if raw_count == 1:
                cpu._set_flag(OF_BIT, 0)
            _write_rm(cpu, dec, r, sh_bits)
        elif ext == 0:  # ROL
            eff = count % sh_bits if sh_bits > 0 else 0
            r = ((rm_v << eff) | (rm_v >> (sh_bits - eff))) & sh_mask_full
            new_cf = r & 1
            cpu._set_flag(CF_BIT, new_cf)
            if raw_count == 1:
                cpu._set_flag(OF_BIT, ((r >> (sh_bits - 1)) ^ new_cf) & 1)
            _write_rm(cpu, dec, r, sh_bits)
        elif ext == 1:  # ROR
            eff = count % sh_bits if sh_bits > 0 else 0
            r = ((rm_v >> eff) | (rm_v << (sh_bits - eff))) & sh_mask_full
            new_cf = (r >> (sh_bits - 1)) & 1
            cpu._set_flag(CF_BIT, new_cf)
            if raw_count == 1:
                msb = (r >> (sh_bits - 1)) & 1
                next_msb = (r >> (sh_bits - 2)) & 1 if sh_bits > 1 else 0
                cpu._set_flag(OF_BIT, msb ^ next_msb)
            _write_rm(cpu, dec, r, sh_bits)

    # ------------------------------------------------------------------
    # BSWAP r64 — 0F C8+rd; at this point op has been replaced by op2
    # Already handled inside the 0x0F block above.  Placeholder to avoid
    # falling through to the unknown-opcode handler if reached indirectly.
    # ------------------------------------------------------------------

    # --- Unknown: skip (treat as NOP) ---
    # else: just advance PC

    cpu.pc += dec.cursor


# ---------------------------------------------------------------------------
# Public Simulator class — SIM00 protocol
# ---------------------------------------------------------------------------

@dataclass
class StepTrace:
    """Result of a single ``step()`` call."""
    state:   X86_64State
    pc_before: int
    halted:  bool


class X86_64Simulator:
    """x86-64 (AMD64) behavioral simulator — Layer 07w.

    Implements the SIM00 ``Simulator[X86_64State]`` protocol:

    * ``reset()`` — zero all registers, memory, RIP, RFLAGS, halted
    * ``load(program)`` — reset then copy bytes to memory[0x0000…]
    * ``step()`` — fetch/decode/execute one instruction; return ``StepTrace``
    * ``execute(program, max_steps)`` — load then loop until HLT or max_steps
    * ``get_state()`` — return a frozen ``X86_64State`` snapshot

    Memory layout
    -------------
    64 KiB flat byte-addressed little-endian array.
    RSP is initialised to 0xFFF8 (top of memory minus 8) so stack pushes
    work correctly out of the box.
    All addresses wrap modulo 65 536.

    HALT
    ----
    Opcode 0xF4 (HLT) sets ``cpu.halted = True`` and stops ``execute()``.
    ``step()`` never blocks; it returns after executing the HLT.

    Unrecognised opcodes
    --------------------
    Unknown opcodes advance RIP over the decoded bytes and return without
    side-effects (treated as NOP).  This matches real hardware behaviour for
    undefined opcodes in user space (a #UD exception would be raised in
    hardware; we silently skip).
    """

    def __init__(self) -> None:
        self._cpu = _CPU()

    # ------------------------------------------------------------------
    # SIM00 protocol
    # ------------------------------------------------------------------

    def reset(self) -> None:
        """Zero all state.  RSP is set to 0xFFF8 (stack grows downward)."""
        cpu = self._cpu
        cpu.gpr = [0] * 16
        cpu.gpr[RSP] = 0xFFF8
        cpu.pc = 0
        cpu.rflags = 0
        cpu.memory = bytearray(MEM_SIZE)
        cpu.halted = False

    def load(self, program: bytes | list[int]) -> None:
        """Reset then copy *program* bytes to memory starting at address 0."""
        self.reset()
        data = bytes(program)
        n = min(len(data), MEM_SIZE)
        self._cpu.memory[:n] = data[:n]

    def step(self) -> StepTrace:
        """Execute one instruction and return a ``StepTrace``."""
        cpu = self._cpu
        pc_before = cpu.pc
        if not cpu.halted:
            _step(cpu)
        return StepTrace(state=cpu.snapshot(), pc_before=pc_before, halted=cpu.halted)

    def execute(self, program: bytes | list[int], max_steps: int = 100_000) -> X86_64State:
        """Load *program* and execute until HLT or *max_steps* instructions."""
        self.load(program)
        for _ in range(max_steps):
            if self._cpu.halted:
                break
            _step(self._cpu)
        return self._cpu.snapshot()

    def get_state(self) -> X86_64State:
        """Return a frozen snapshot of the current CPU state."""
        return self._cpu.snapshot()

    # ------------------------------------------------------------------
    # Convenience: set_input_port / get_output_port (stubs — no I/O model)
    # ------------------------------------------------------------------

    def set_input_port(self, _port: int, _val: int) -> None:  # noqa: D401
        """No-op — I/O ports are not modelled."""

    def get_output_port(self, _port: int) -> int:  # noqa: D401
        """No-op — I/O ports are not modelled.  Returns 0."""
        return 0

    def interrupt(self, _vector: int) -> None:  # noqa: D401
        """No-op — interrupts are not modelled."""

    def nmi(self) -> None:  # noqa: D401
        """No-op — NMI is not modelled."""
