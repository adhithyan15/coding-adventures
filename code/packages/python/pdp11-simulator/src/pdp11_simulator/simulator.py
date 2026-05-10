"""DEC PDP-11 (1970) behavioral simulator — Layer 07o.

──────────────────────────────────────────────────────────────────────────────
OVERVIEW
──────────────────────────────────────────────────────────────────────────────

This module implements ``PDP11Simulator``, a Python behavioral simulator for
the DEC PDP-11 minicomputer.  It implements the SIM00 ``Simulator[PDP11State]``
protocol used by all architecture simulators in this repository.

The PDP-11 is one of the most influential computer architectures ever designed.
Key features:

  1. ORTHOGONAL ISA: Any addressing mode applies to any register in any
     instruction.  There are 8 addressing modes and 8 registers — the full
     combinatorial product works everywhere.

  2. PC IN THE REGISTER FILE (R7 = PC): The program counter is register R7.
     This means "immediate" addressing (``#n``) is just the autoincrement mode
     applied to R7 — the CPU reads the next instruction word as a literal
     operand and increments PC past it.  Clever!

  3. SP IN THE REGISTER FILE (R6 = SP): Same idea for the stack.  Push is
     autodecrement on R6 (``-(R6)``); pop is autoincrement on R6 (``(R6)+``).

  4. JSR/RTS ELEGANCE: JSR and RTS use the same autoincrement/autodecrement
     modes.  ``JSR PC, addr`` is literally "push PC, jump to addr."

──────────────────────────────────────────────────────────────────────────────
INSTRUCTION DECODING
──────────────────────────────────────────────────────────────────────────────

PDP-11 instructions are 16-bit words (little-endian in memory).

Double-operand (word):   opcode[15:12] src[11:6] dst[5:0]
Double-operand (byte):   bit15=1, opcode[14:12] src[11:6] dst[5:0]
Single-operand:          opcode[15:6] dst[5:0]
Branch:                  opcode[15:8] offset[7:0]  (offset in words, signed)
JSR:                     0000 100 reg[8:6] dst[5:0]
RTS:                     0000 0000 1000 0 reg[2:0]
SOB:                     0111 11 reg[8:6] offset[5:0]
HALT:                    0x0000

──────────────────────────────────────────────────────────────────────────────
ADDRESSING MODE EVALUATION
──────────────────────────────────────────────────────────────────────────────

Each 6-bit operand field encodes:  mode[5:3]  reg[2:0]

Mode 0: Register direct     — operand IS the register (no memory)
Mode 1: Register deferred   — EA = R; operand = M[EA]
Mode 2: Autoincrement       — EA = R; R += size;  operand = M[EA]
Mode 3: Autoincrement def.  — EA = M[R]; R += 2;  operand = M[EA]
Mode 4: Autodecrement       — R -= size; EA = R;  operand = M[EA]
Mode 5: Autodecrement def.  — R -= 2; EA = M[R];  operand = M[EA]
Mode 6: Index               — EA = R + fetch_word(); operand = M[EA]
Mode 7: Index deferred      — EA = M[R + fetch_word()]; operand = M[EA]

Special cases when reg = R7 (PC):
  Mode 2 + R7 → Immediate    #n       (fetch next word as literal)
  Mode 3 + R7 → Absolute     @#addr   (fetch next word as address)
  Mode 6 + R7 → Relative     addr     (PC-relative; EA = PC + disp)
  Mode 7 + R7 → Rel. Deferred @addr   (EA = M[PC + disp])

For byte instructions, autoincrement/autodecrement step by 1 — EXCEPT when
the register is SP (R6) or PC (R7), which always step by 2.

──────────────────────────────────────────────────────────────────────────────
"""

from __future__ import annotations

from simulator_protocol import ExecutionResult, StepTrace

from pdp11_simulator.flags import nzvc_add, nzvc_logic, nzvc_sub, pack_psw
from pdp11_simulator.state import (
    ADDR_MASK,
    BYTE_MASK,
    BYTE_MSB,
    INIT_SP,
    LOAD_ADDR,
    MEM_SIZE,
    PC,
    SP,
    WORD_MASK,
    WORD_MSB,
    PDP11State,
)

# ── Internal helpers ──────────────────────────────────────────────────────────

def _sign_extend(value: int, bits: int) -> int:
    """Sign-extend *value* from *bits* wide to Python int.

    >>> _sign_extend(0xFF, 8)   # -1
    -1
    >>> _sign_extend(0x7F, 8)   # +127
    127
    >>> _sign_extend(0x8000, 16)  # -32768
    -32768
    """
    sign_bit = 1 << (bits - 1)
    return (value & (sign_bit - 1)) - (value & sign_bit)


def _w(hi: int, lo: int) -> bytes:
    """Pack two bytes as a little-endian word (helper for test programs)."""
    return bytes([lo & 0xFF, hi & 0xFF])


# ── Simulator ─────────────────────────────────────────────────────────────────

class PDP11Simulator:
    """Behavioral simulator for the DEC PDP-11 (1970).

    Implements ``Simulator[PDP11State]`` from the SIM00 protocol.

    Internal state
    --------------
    _r : list[int]
        Eight 16-bit registers R0–R7 (R6=SP, R7=PC), stored unsigned.
    _psw : int
        Processor Status Word (16-bit); only bits 3–0 (N,Z,V,C) are live.
    _mem : bytearray
        64 KB flat address space.
    _halted : bool
        Set when HALT executes; step() becomes a no-op thereafter.

    Examples
    --------
    >>> sim = PDP11Simulator()
    >>> # HALT = 0x0000 (two zero bytes, little-endian)
    >>> result = sim.execute(bytes([0x00, 0x00]))
    >>> result.ok
    True
    >>> result.final_state.halted
    True
    """

    def __init__(self) -> None:
        self._r: list[int] = [0] * 8
        self._psw: int = 0
        self._mem: bytearray = bytearray(MEM_SIZE)
        self._halted: bool = False
        self.reset()   # initialise SP=0xF000, PC=0x1000

    # ── SIM00 protocol ────────────────────────────────────────────────────────

    def reset(self) -> None:
        """Reset to power-on state: zero registers, zero memory, SP=0xF000, PC=0x1000."""
        self._r = [0] * 8
        self._r[SP] = INIT_SP
        self._r[PC] = LOAD_ADDR
        self._psw = 0
        self._mem = bytearray(MEM_SIZE)
        self._halted = False

    def load(self, program: bytes) -> None:
        """Reset then copy *program* into memory starting at 0x1000."""
        self.reset()
        end = LOAD_ADDR + len(program)
        if end > MEM_SIZE:
            raise ValueError(f"Program too large: {len(program)} bytes > {MEM_SIZE - LOAD_ADDR}")
        self._mem[LOAD_ADDR:end] = program

    def get_state(self) -> PDP11State:
        """Return an immutable snapshot of the current CPU state."""
        return PDP11State(
            r=tuple(self._r),
            psw=self._psw,
            halted=self._halted,
            memory=tuple(self._mem),
        )

    def step(self) -> StepTrace:
        """Fetch, decode, and execute one instruction; return a StepTrace."""
        if self._halted:
            pc = self._r[PC]
            return StepTrace(pc_before=pc, pc_after=pc,
                             mnemonic="HALT", description="HALT (already halted)")
        pc_before = self._r[PC]
        mnemonic, description = self._execute_one()
        return StepTrace(
            pc_before=pc_before,
            pc_after=self._r[PC],
            mnemonic=mnemonic,
            description=f"{mnemonic} @ 0x{pc_before:04X}",
        )

    def execute(self, program: bytes, max_steps: int = 100_000) -> ExecutionResult[PDP11State]:
        """Load *program*, run to HALT or *max_steps*, return full result."""
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
            halted=self._halted,
            steps=steps,
            final_state=self.get_state(),
            error=error,
            traces=traces,
        )

    # ── Memory primitives ─────────────────────────────────────────────────────

    def _read_word(self, addr: int) -> int:
        """Read a 16-bit little-endian word from *addr* (must be even)."""
        addr &= ADDR_MASK
        if addr & 1:
            raise ValueError(f"Odd address word read: 0x{addr:04X}")
        return self._mem[addr] | (self._mem[addr + 1] << 8)

    def _write_word(self, addr: int, value: int) -> None:
        """Write a 16-bit little-endian word to *addr* (must be even)."""
        addr &= ADDR_MASK
        if addr & 1:
            raise ValueError(f"Odd address word write: 0x{addr:04X}")
        self._mem[addr]     = value & 0xFF
        self._mem[addr + 1] = (value >> 8) & 0xFF

    def _read_byte(self, addr: int) -> int:
        """Read a single byte from *addr*."""
        return self._mem[addr & ADDR_MASK]

    def _write_byte(self, addr: int, value: int) -> None:
        """Write a single byte to *addr*."""
        self._mem[addr & ADDR_MASK] = value & BYTE_MASK

    def _fetch_word(self) -> int:
        """Fetch next instruction/immediate word, advancing PC by 2."""
        word = self._read_word(self._r[PC])
        self._r[PC] = (self._r[PC] + 2) & WORD_MASK
        return word

    # ── Addressing mode evaluation ────────────────────────────────────────────
    #
    # Returns (ea, is_register) where:
    #   is_register=True  → operand is register _r[reg]; ea=reg index
    #   is_register=False → operand is M[ea]
    #
    # For write, the caller uses _write_ea().
    #

    def _ea(self, mode: int, reg: int, *, word: bool) -> tuple[int, bool]:
        """Compute effective address for (mode, reg) operand.

        Returns ``(address_or_regnum, is_register_direct)``.

        Mode 0 → (reg, True)  — register direct, no memory involved.
        Modes 1-7 → (ea, False) — memory effective address.
        """
        # Mode 0: Register direct — operand is the register itself
        if mode == 0:
            return (reg, True)

        # Byte step = 1 for general registers, but ALWAYS 2 for SP (R6) and PC (R7).
        # This ensures #immediate (mode 2 on R7) always advances PC by 2 words,
        # and push/pop on R6 always moves the stack by one word slot.
        step = 2 if (word or reg >= SP) else 1

        if mode == 1:
            # Register deferred: EA = R
            return (self._r[reg] & ADDR_MASK, False)

        if mode == 2:
            # Autoincrement: EA = R; R += step
            ea = self._r[reg] & ADDR_MASK
            self._r[reg] = (self._r[reg] + step) & WORD_MASK
            return (ea, False)

        if mode == 3:
            # Autoincrement deferred: EA = M[R]; R += 2 (always pointer-sized)
            ptr = self._r[reg] & ADDR_MASK
            self._r[reg] = (self._r[reg] + 2) & WORD_MASK
            ea = self._read_word(ptr)
            return (ea & ADDR_MASK, False)

        if mode == 4:
            # Autodecrement: R -= step; EA = R
            self._r[reg] = (self._r[reg] - step) & WORD_MASK
            return (self._r[reg] & ADDR_MASK, False)

        if mode == 5:
            # Autodecrement deferred: R -= 2; EA = M[R]
            self._r[reg] = (self._r[reg] - 2) & WORD_MASK
            ptr = self._r[reg] & ADDR_MASK
            ea = self._read_word(ptr)
            return (ea & ADDR_MASK, False)

        if mode == 6:
            # Index: EA = R + next-word (displacement)
            disp = self._fetch_word()
            ea = (self._r[reg] + disp) & ADDR_MASK
            return (ea, False)

        # mode == 7
        # Index deferred: EA = M[R + next-word]
        disp = self._fetch_word()
        ptr  = (self._r[reg] + disp) & ADDR_MASK
        ea   = self._read_word(ptr) & ADDR_MASK
        return (ea, False)

    def _read_ea(self, ea: int, is_reg: bool, *, word: bool) -> int:
        """Read operand from EA (or register)."""
        if is_reg:
            return self._r[ea] & (WORD_MASK if word else BYTE_MASK)
        return self._read_word(ea) if word else self._read_byte(ea)

    def _write_ea(self, ea: int, is_reg: bool, value: int, *, word: bool) -> None:
        """Write operand to EA (or register).

        For byte writes to a register, PDP-11 sign-extends the byte to 16 bits
        (MOVB convention) — but that applies only to MOV; other byte ops to
        registers just write the low byte into the low byte of the register.
        The caller handles sign extension for MOVB.
        """
        if is_reg:
            if word:
                self._r[ea] = value & WORD_MASK
            else:
                # Byte to register: write only low byte
                self._r[ea] = (self._r[ea] & 0xFF00) | (value & BYTE_MASK)
        elif word:
            self._write_word(ea, value & WORD_MASK)
        else:
            self._write_byte(ea, value & BYTE_MASK)

    # ── PSW helpers ───────────────────────────────────────────────────────────

    def _set_nzvc(self, n: bool, z: bool, v: bool, c: bool) -> None:
        """Update N, Z, V, C bits in PSW (preserve upper bits)."""
        self._psw = (self._psw & 0xFFF0) | pack_psw(n, z, v, c)

    def _c(self) -> int:     return self._psw & 1
    def _get_n(self) -> bool: return bool(self._psw & 0b1000)
    def _get_z(self) -> bool: return bool(self._psw & 0b0100)
    def _get_v(self) -> bool: return bool(self._psw & 0b0010)
    def _get_c(self) -> bool: return bool(self._psw & 0b0001)

    # ── Main decode/execute dispatcher ────────────────────────────────────────

    def _execute_one(self) -> tuple[str, str]:
        """Fetch and execute one instruction, return (mnemonic, description)."""
        iw = self._fetch_word()   # instruction word

        # ── HALT (0x0000) ─────────────────────────────────────────────────────
        if iw == 0x0000:
            self._halted = True
            return ("HALT", "Halt processor")

        # ── NOP (0x00A0) ──────────────────────────────────────────────────────
        if iw == 0x00A0:
            return ("NOP", "No operation")

        # ── RTI (0x0002) ──────────────────────────────────────────────────────
        if iw == 0x0002:
            # Return from interrupt: pop PC then PSW from stack
            new_pc  = self._read_word(self._r[SP]);  self._r[SP] = (self._r[SP] + 2) & WORD_MASK
            new_psw = self._read_word(self._r[SP]);  self._r[SP] = (self._r[SP] + 2) & WORD_MASK
            self._r[PC]  = new_pc & WORD_MASK
            self._psw    = new_psw & WORD_MASK
            return ("RTI", "Return from interrupt")

        # ── RTS reg — octal 000 200 to 000 207 = 0x0080 to 0x0087 ──────────────
        #  Binary: 0000 0000 1000 0 rrr
        #  (NOT 0x0200: that range is BNE.)
        if (iw & 0xFFF8) == 0x0080:
            reg = iw & 0x7
            old_pc = self._r[reg]
            self._r[PC]  = old_pc & WORD_MASK
            self._r[reg] = self._read_word(self._r[SP]) & WORD_MASK
            self._r[SP]  = (self._r[SP] + 2) & WORD_MASK
            return ("RTS", f"RTS R{reg}")

        # ── SOB reg, offset (0x7E00 … 0x7FFF) ────────────────────────────────
        #  Encoding: 0111 11 rrr oooooo  (offset 6-bit unsigned, step backward)
        if (iw & 0xFE00) == 0x7E00:
            reg    = (iw >> 6) & 0x7
            offset = iw & 0x3F
            self._r[reg] = (self._r[reg] - 1) & WORD_MASK
            if self._r[reg] != 0:
                self._r[PC] = (self._r[PC] - 2 * offset) & WORD_MASK
            return ("SOB", f"SOB R{reg}, -{offset}")

        # ── Branches (bits 15-8 = opcode byte) ───────────────────────────────
        hi = (iw >> 8) & 0xFF
        if hi in _BRANCH_OPCODES:
            offset = _sign_extend(iw & 0xFF, 8)
            taken  = _BRANCH_OPCODES[hi](self._get_n(), self._get_z(),
                                          self._get_v(), self._get_c())
            if taken:
                self._r[PC] = (self._r[PC] + 2 * offset) & WORD_MASK
            return (_BRANCH_NAMES[hi], f"branch offset {offset:+d}")

        # ── JMP (0000 0000 01 mm rrr) ─────────────────────────────────────────
        #  Encoding: 0x0040 | (mode<<3) | reg
        if (iw & 0xFFC0) == 0x0040:
            mode = (iw >> 3) & 0x7
            reg  = iw & 0x7
            if mode == 0:
                raise ValueError("JMP with register direct mode is illegal")
            ea, _ = self._ea(mode, reg, word=True)
            self._r[PC] = ea & WORD_MASK
            return ("JMP", f"JMP mode={mode} R{reg}")

        # ── JSR reg, dst (0x0800 | link<<6 | mode<<3 | dst_reg) ──────────────
        #  Bits: 0000 1000 rrr mmm ddd
        if (iw & 0xFE00) == 0x0800:
            link = (iw >> 6) & 0x7
            mode = (iw >> 3) & 0x7
            dst_reg = iw & 0x7
            if mode == 0:
                raise ValueError("JSR with register direct mode is illegal")
            ea, _ = self._ea(mode, dst_reg, word=True)
            old_link   = self._r[link]
            ret_addr   = self._r[PC]          # PC already past JSR word
            self._r[SP] = (self._r[SP] - 2) & WORD_MASK
            self._write_word(self._r[SP], old_link)
            self._r[link] = ret_addr
            self._r[PC]   = ea & WORD_MASK
            return ("JSR", f"JSR R{link}")

        # ── Single-operand instructions ───────────────────────────────────────
        #  Bits 15-6 = opcode (10 bits), bits 5-0 = dst (mode + reg)
        single_op = (iw >> 6) & 0x3FF
        dst_mode  = (iw >> 3) & 0x7
        dst_reg   = iw & 0x7

        if single_op in _SINGLE_OPS:
            return _SINGLE_OPS[single_op](self, dst_mode, dst_reg)

        # ── Double-operand instructions ───────────────────────────────────────
        #  Bits 15-12 = opcode (4 bits)
        #  bit 15 = byte flag (1 = byte, 0 = word)
        #  src = bits 11-6, dst = bits 5-0
        src_field = (iw >> 6) & 0x3F
        src_mode  = (src_field >> 3) & 0x7
        src_reg   = src_field & 0x7

        byte_op   = bool(iw & 0x8000)
        op4       = (iw >> 12) & 0x7   # 3-bit opcode within byte/word space
        word      = not byte_op

        op4 = (iw >> 12) & 0xF if (iw & 0x8000) == 0 else ((iw >> 12) & 0x7) | 0x8

        if op4 in _DOUBLE_OPS:
            return _DOUBLE_OPS[op4](self, src_mode, src_reg, dst_mode, dst_reg, word)

        raise ValueError(f"Unknown opcode: 0x{iw:04X}")

    # ── Single-operand implementations ────────────────────────────────────────

    def _op_clr(self, dst_mode: int, dst_reg: int, *, word: bool) -> tuple[str, str]:
        """CLR/CLRB: dst ← 0; N=0, Z=1, V=0, C=0."""
        ea, is_reg = self._ea(dst_mode, dst_reg, word=word)
        self._write_ea(ea, is_reg, 0, word=word)
        self._set_nzvc(False, True, False, False)
        name = "CLR" if word else "CLRB"
        return (name, f"{name} mode={dst_mode} R{dst_reg}")

    def _op_com(self, dst_mode: int, dst_reg: int, *, word: bool) -> tuple[str, str]:
        """COM/COMB: dst ← ~dst; N,Z from result; V=0, C=1."""
        ea, is_reg = self._ea(dst_mode, dst_reg, word=word)
        src = self._read_ea(ea, is_reg, word=word)
        mask = WORD_MASK if word else BYTE_MASK
        result = (~src) & mask
        self._write_ea(ea, is_reg, result, word=word)
        n, z, _, _ = nzvc_logic(result, word=word)
        self._set_nzvc(n, z, False, True)
        name = "COM" if word else "COMB"
        return (name, f"{name} mode={dst_mode} R{dst_reg}")

    def _op_inc(self, dst_mode: int, dst_reg: int, *, word: bool) -> tuple[str, str]:
        """INC/INCB: dst ← dst + 1; N,Z,V updated; C not changed."""
        ea, is_reg = self._ea(dst_mode, dst_reg, word=word)
        src = self._read_ea(ea, is_reg, word=word)
        n, z, v, _ = nzvc_add(src, 1, word=word)
        mask = WORD_MASK if word else BYTE_MASK
        self._write_ea(ea, is_reg, (src + 1) & mask, word=word)
        # INC does NOT change C
        self._set_nzvc(n, z, v, self._get_c())
        name = "INC" if word else "INCB"
        return (name, f"{name} mode={dst_mode} R{dst_reg}")

    def _op_dec(self, dst_mode: int, dst_reg: int, *, word: bool) -> tuple[str, str]:
        """DEC/DECB: dst ← dst - 1; N,Z,V updated; C not changed."""
        ea, is_reg = self._ea(dst_mode, dst_reg, word=word)
        src = self._read_ea(ea, is_reg, word=word)
        n, z, v, _ = nzvc_sub(src, 1, word=word)
        mask = WORD_MASK if word else BYTE_MASK
        self._write_ea(ea, is_reg, (src - 1) & mask, word=word)
        # DEC does NOT change C
        self._set_nzvc(n, z, v, self._get_c())
        name = "DEC" if word else "DECB"
        return (name, f"{name} mode={dst_mode} R{dst_reg}")

    def _op_neg(self, dst_mode: int, dst_reg: int, *, word: bool) -> tuple[str, str]:
        """NEG/NEGB: dst ← 0 - dst; N,Z,V,C updated (C=1 unless result=0)."""
        ea, is_reg = self._ea(dst_mode, dst_reg, word=word)
        src = self._read_ea(ea, is_reg, word=word)
        mask = WORD_MASK if word else BYTE_MASK
        msb  = WORD_MSB  if word else BYTE_MSB
        result = (-src) & mask
        n = bool(result & msb)
        z = result == 0
        v = src == msb   # NEG(most-negative) = most-negative → overflow
        c = result != 0
        self._write_ea(ea, is_reg, result, word=word)
        self._set_nzvc(n, z, v, c)
        name = "NEG" if word else "NEGB"
        return (name, f"{name} mode={dst_mode} R{dst_reg}")

    def _op_tst(self, dst_mode: int, dst_reg: int, *, word: bool) -> tuple[str, str]:
        """TST/TSTB: set N,Z from src; V=0, C=0."""
        ea, is_reg = self._ea(dst_mode, dst_reg, word=word)
        src = self._read_ea(ea, is_reg, word=word)
        n, z, _, _ = nzvc_logic(src, word=word)
        self._set_nzvc(n, z, False, False)
        name = "TST" if word else "TSTB"
        return (name, f"{name} mode={dst_mode} R{dst_reg}")

    def _op_asr(self, dst_mode: int, dst_reg: int, *, word: bool) -> tuple[str, str]:
        """ASR/ASRB: arithmetic shift right by 1; sign bit preserved."""
        ea, is_reg = self._ea(dst_mode, dst_reg, word=word)
        src = self._read_ea(ea, is_reg, word=word)
        msb  = WORD_MSB if word else BYTE_MSB
        mask = WORD_MASK if word else BYTE_MASK
        c_out = bool(src & 1)
        result = ((src >> 1) | (src & msb)) & mask  # sign-fill
        n = bool(result & msb)
        z = result == 0
        v = n ^ c_out   # V = N XOR C (detects sign change)
        self._write_ea(ea, is_reg, result, word=word)
        self._set_nzvc(n, z, v, c_out)
        name = "ASR" if word else "ASRB"
        return (name, f"{name} mode={dst_mode} R{dst_reg}")

    def _op_asl(self, dst_mode: int, dst_reg: int, *, word: bool) -> tuple[str, str]:
        """ASL/ASLB: arithmetic shift left by 1; bit 15/7 goes to C."""
        ea, is_reg = self._ea(dst_mode, dst_reg, word=word)
        src = self._read_ea(ea, is_reg, word=word)
        msb  = WORD_MSB if word else BYTE_MSB
        mask = WORD_MASK if word else BYTE_MASK
        c_out = bool(src & msb)
        result = (src << 1) & mask
        n = bool(result & msb)
        z = result == 0
        v = n ^ c_out   # V = N XOR C
        self._write_ea(ea, is_reg, result, word=word)
        self._set_nzvc(n, z, v, c_out)
        name = "ASL" if word else "ASLB"
        return (name, f"{name} mode={dst_mode} R{dst_reg}")

    def _op_ror(self, dst_mode: int, dst_reg: int, *, word: bool) -> tuple[str, str]:
        """ROR/RORB: rotate right through C."""
        ea, is_reg = self._ea(dst_mode, dst_reg, word=word)
        src = self._read_ea(ea, is_reg, word=word)
        msb  = WORD_MSB if word else BYTE_MSB
        mask = WORD_MASK if word else BYTE_MASK
        old_c  = self._get_c()
        new_c  = bool(src & 1)
        result = ((src >> 1) | (int(old_c) << (15 if word else 7))) & mask
        n = bool(result & msb)
        z = result == 0
        v = n ^ new_c
        self._write_ea(ea, is_reg, result, word=word)
        self._set_nzvc(n, z, v, new_c)
        name = "ROR" if word else "RORB"
        return (name, f"{name} mode={dst_mode} R{dst_reg}")

    def _op_rol(self, dst_mode: int, dst_reg: int, *, word: bool) -> tuple[str, str]:
        """ROL/ROLB: rotate left through C."""
        ea, is_reg = self._ea(dst_mode, dst_reg, word=word)
        src = self._read_ea(ea, is_reg, word=word)
        msb  = WORD_MSB if word else BYTE_MSB
        mask = WORD_MASK if word else BYTE_MASK
        old_c  = self._get_c()
        new_c  = bool(src & msb)
        result = (((src << 1) & mask) | int(old_c)) & mask
        n = bool(result & msb)
        z = result == 0
        v = n ^ new_c
        self._write_ea(ea, is_reg, result, word=word)
        self._set_nzvc(n, z, v, new_c)
        name = "ROL" if word else "ROLB"
        return (name, f"{name} mode={dst_mode} R{dst_reg}")

    def _op_swab(self, dst_mode: int, dst_reg: int) -> tuple[str, str]:
        """SWAB: swap high and low bytes of dst word; N,Z from low byte; V=0, C=0."""
        ea, is_reg = self._ea(dst_mode, dst_reg, word=True)
        src    = self._read_ea(ea, is_reg, word=True)
        result = ((src & 0xFF) << 8) | ((src >> 8) & 0xFF)
        self._write_ea(ea, is_reg, result, word=True)
        lo = result & 0xFF
        n = bool(lo & BYTE_MSB)
        z = lo == 0
        self._set_nzvc(n, z, False, False)
        return ("SWAB", f"SWAB mode={dst_mode} R{dst_reg}")

    def _op_adc(self, dst_mode: int, dst_reg: int, *, word: bool) -> tuple[str, str]:
        """ADC/ADCB: dst ← dst + C."""
        ea, is_reg = self._ea(dst_mode, dst_reg, word=word)
        src = self._read_ea(ea, is_reg, word=word)
        carry_in = self._c()
        n, z, v, c = nzvc_add(src, carry_in, word=word)
        mask = WORD_MASK if word else BYTE_MASK
        self._write_ea(ea, is_reg, (src + carry_in) & mask, word=word)
        self._set_nzvc(n, z, v, c)
        name = "ADC" if word else "ADCB"
        return (name, f"{name} mode={dst_mode} R{dst_reg}")

    def _op_sbc(self, dst_mode: int, dst_reg: int, *, word: bool) -> tuple[str, str]:
        """SBC/SBCB: dst ← dst - C."""
        ea, is_reg = self._ea(dst_mode, dst_reg, word=word)
        src = self._read_ea(ea, is_reg, word=word)
        carry_in = self._c()
        n, z, v, c = nzvc_sub(src, carry_in, word=word)
        mask = WORD_MASK if word else BYTE_MASK
        self._write_ea(ea, is_reg, (src - carry_in) & mask, word=word)
        self._set_nzvc(n, z, v, c)
        name = "SBC" if word else "SBCB"
        return (name, f"{name} mode={dst_mode} R{dst_reg}")

    # ── Double-operand implementations ────────────────────────────────────────

    def _op_mov(self, sm: int, sr: int, dm: int, dr: int, word: bool) -> tuple[str, str]:
        """MOV/MOVB: dst ← src; N,Z from result; V=0, C unchanged."""
        ea_s, is_reg_s = self._ea(sm, sr, word=word)
        src = self._read_ea(ea_s, is_reg_s, word=word)
        ea_d, is_reg_d = self._ea(dm, dr, word=word)
        # MOVB to register: sign-extend byte to 16 bits
        store_val = src
        if not word and is_reg_d:
            store_val = _sign_extend(src, 8) & WORD_MASK
            self._r[ea_d] = store_val
        else:
            self._write_ea(ea_d, is_reg_d, src, word=word)
        n, z, _, _ = nzvc_logic(src, word=word)
        self._set_nzvc(n, z, False, self._get_c())
        name = "MOV" if word else "MOVB"
        return (name, f"{name} mode={sm},{dm}")

    def _op_cmp(self, sm: int, sr: int, dm: int, dr: int, word: bool) -> tuple[str, str]:
        """CMP/CMPB: src − dst (result discarded); set N,Z,V,C."""
        ea_s, is_reg_s = self._ea(sm, sr, word=word)
        src = self._read_ea(ea_s, is_reg_s, word=word)
        ea_d, is_reg_d = self._ea(dm, dr, word=word)
        dst = self._read_ea(ea_d, is_reg_d, word=word)
        n, z, v, c = nzvc_sub(src, dst, word=word)
        self._set_nzvc(n, z, v, c)
        name = "CMP" if word else "CMPB"
        return (name, f"{name} mode={sm},{dm}")

    def _op_bit(self, sm: int, sr: int, dm: int, dr: int, word: bool) -> tuple[str, str]:
        """BIT/BITB: src AND dst (result discarded); N,Z,V=0; C unchanged."""
        ea_s, is_reg_s = self._ea(sm, sr, word=word)
        src = self._read_ea(ea_s, is_reg_s, word=word)
        ea_d, is_reg_d = self._ea(dm, dr, word=word)
        dst = self._read_ea(ea_d, is_reg_d, word=word)
        result = src & dst
        n, z, _, _ = nzvc_logic(result, word=word)
        self._set_nzvc(n, z, False, self._get_c())
        name = "BIT" if word else "BITB"
        return (name, f"{name} mode={sm},{dm}")

    def _op_bic(self, sm: int, sr: int, dm: int, dr: int, word: bool) -> tuple[str, str]:
        """BIC/BICB: dst ← dst AND NOT src; N,Z,V=0; C unchanged."""
        ea_s, is_reg_s = self._ea(sm, sr, word=word)
        src = self._read_ea(ea_s, is_reg_s, word=word)
        ea_d, is_reg_d = self._ea(dm, dr, word=word)
        dst = self._read_ea(ea_d, is_reg_d, word=word)
        mask = WORD_MASK if word else BYTE_MASK
        result = dst & (~src & mask)
        self._write_ea(ea_d, is_reg_d, result, word=word)
        n, z, _, _ = nzvc_logic(result, word=word)
        self._set_nzvc(n, z, False, self._get_c())
        name = "BIC" if word else "BICB"
        return (name, f"{name} mode={sm},{dm}")

    def _op_bis(self, sm: int, sr: int, dm: int, dr: int, word: bool) -> tuple[str, str]:
        """BIS/BISB: dst ← dst OR src; N,Z,V=0; C unchanged."""
        ea_s, is_reg_s = self._ea(sm, sr, word=word)
        src = self._read_ea(ea_s, is_reg_s, word=word)
        ea_d, is_reg_d = self._ea(dm, dr, word=word)
        dst = self._read_ea(ea_d, is_reg_d, word=word)
        result = dst | src
        self._write_ea(ea_d, is_reg_d, result & (WORD_MASK if word else BYTE_MASK), word=word)
        n, z, _, _ = nzvc_logic(result, word=word)
        self._set_nzvc(n, z, False, self._get_c())
        name = "BIS" if word else "BISB"
        return (name, f"{name} mode={sm},{dm}")

    def _op_add(self, sm: int, sr: int, dm: int, dr: int, word: bool) -> tuple[str, str]:
        """ADD (word only): dst ← dst + src; N,Z,V,C set."""
        ea_s, is_reg_s = self._ea(sm, sr, word=True)
        src = self._read_ea(ea_s, is_reg_s, word=True)
        ea_d, is_reg_d = self._ea(dm, dr, word=True)
        dst = self._read_ea(ea_d, is_reg_d, word=True)
        n, z, v, c = nzvc_add(dst, src, word=True)
        self._write_ea(ea_d, is_reg_d, (dst + src) & WORD_MASK, word=True)
        self._set_nzvc(n, z, v, c)
        return ("ADD", f"ADD mode={sm},{dm}")

    def _op_sub(self, sm: int, sr: int, dm: int, dr: int, word: bool) -> tuple[str, str]:
        """SUB (word only): dst ← dst − src; N,Z,V,C set."""
        ea_s, is_reg_s = self._ea(sm, sr, word=True)
        src = self._read_ea(ea_s, is_reg_s, word=True)
        ea_d, is_reg_d = self._ea(dm, dr, word=True)
        dst = self._read_ea(ea_d, is_reg_d, word=True)
        n, z, v, c = nzvc_sub(dst, src, word=True)
        self._write_ea(ea_d, is_reg_d, (dst - src) & WORD_MASK, word=True)
        self._set_nzvc(n, z, v, c)
        return ("SUB", f"SUB mode={sm},{dm}")


# ── Dispatch tables ───────────────────────────────────────────────────────────
#
# These tables are built after the class definition to reference the methods.
# They map opcode numbers → bound methods.

def _make_single_ops(sim_class):
    """Build the single-operand dispatch table for PDP11Simulator."""
    def _wrap_word(fn):
        def wrapper(self, dm, dr): return fn(self, dm, dr, word=True)
        return wrapper
    def _wrap_byte(fn):
        def wrapper(self, dm, dr): return fn(self, dm, dr, word=False)
        return wrapper

    return {
        # SWAB: 0000 0000 11 mmm rrr → single_op bits 15-6 = 0x0003 (binary 0000000011)
        0x003: lambda self, dm, dr: self._op_swab(dm, dr),
        # CLR  0000 1000 0x  → 0x0200 >> 0 ...  single_op = (0x0200 >> 6) wait
        # Let me compute: single_op = (iw >> 6) & 0x3FF
        # CLR: iw = 0x0A00 | (mode<<3) | reg  → single_op = 0x0A00>>6 = 0x028
        0x028: lambda self, dm, dr: self._op_clr(dm, dr, word=True),
        # CLRB: iw = 0x8A00 → single_op = 0x8A00>>6 = 0x228
        0x228: lambda self, dm, dr: self._op_clr(dm, dr, word=False),
        # COM: iw = 0x0A40 → single_op = 0x0A40>>6 = 0x029
        0x029: lambda self, dm, dr: self._op_com(dm, dr, word=True),
        # COMB: iw = 0x8A40 → single_op = 0x229
        0x229: lambda self, dm, dr: self._op_com(dm, dr, word=False),
        # INC: iw = 0x0A80 → single_op = 0x02A
        0x02A: lambda self, dm, dr: self._op_inc(dm, dr, word=True),
        # INCB: iw = 0x8A80 → single_op = 0x22A
        0x22A: lambda self, dm, dr: self._op_inc(dm, dr, word=False),
        # DEC: iw = 0x0AC0 → single_op = 0x02B
        0x02B: lambda self, dm, dr: self._op_dec(dm, dr, word=True),
        # DECB: iw = 0x8AC0 → single_op = 0x22B
        0x22B: lambda self, dm, dr: self._op_dec(dm, dr, word=False),
        # NEG: iw = 0x0B00 → single_op = 0x02C
        0x02C: lambda self, dm, dr: self._op_neg(dm, dr, word=True),
        # NEGB: iw = 0x8B00 → single_op = 0x22C
        0x22C: lambda self, dm, dr: self._op_neg(dm, dr, word=False),
        # ADC: iw = 0x0B40 → single_op = 0x02D
        0x02D: lambda self, dm, dr: self._op_adc(dm, dr, word=True),
        # ADCB: iw = 0x8B40 → single_op = 0x22D
        0x22D: lambda self, dm, dr: self._op_adc(dm, dr, word=False),
        # SBC: iw = 0x0B80 → single_op = 0x02E
        0x02E: lambda self, dm, dr: self._op_sbc(dm, dr, word=True),
        # SBCB: iw = 0x8B80 → single_op = 0x22E
        0x22E: lambda self, dm, dr: self._op_sbc(dm, dr, word=False),
        # TST: iw = 0x0BC0 → single_op = 0x02F
        0x02F: lambda self, dm, dr: self._op_tst(dm, dr, word=True),
        # TSTB: iw = 0x8BC0 → single_op = 0x22F
        0x22F: lambda self, dm, dr: self._op_tst(dm, dr, word=False),
        # ROR:  octal 006000 = 0x0C00 → single_op = (0x0C00 >> 6) & 0x3FF = 0x030
        0x030: lambda self, dm, dr: self._op_ror(dm, dr, word=True),
        # RORB: bit15=1 → 0x8C00 → single_op = (0x8C00 >> 6) & 0x3FF = 0x230
        0x230: lambda self, dm, dr: self._op_ror(dm, dr, word=False),
        # ROL:  octal 006100 = 0x0C40 → single_op = 0x031
        0x031: lambda self, dm, dr: self._op_rol(dm, dr, word=True),
        # ROLB: 0x8C40 → single_op = 0x231
        0x231: lambda self, dm, dr: self._op_rol(dm, dr, word=False),
        # ASR:  octal 006200 = 0x0C80 → single_op = 0x032
        0x032: lambda self, dm, dr: self._op_asr(dm, dr, word=True),
        # ASRB: 0x8C80 → single_op = 0x232
        0x232: lambda self, dm, dr: self._op_asr(dm, dr, word=False),
        # ASL:  octal 006300 = 0x0CC0 → single_op = 0x033
        0x033: lambda self, dm, dr: self._op_asl(dm, dr, word=True),
        # ASLB: 0x8CC0 → single_op = 0x233
        0x233: lambda self, dm, dr: self._op_asl(dm, dr, word=False),
    }


def _make_double_ops(sim_class):
    """Build the double-operand dispatch table."""
    return {
        # op4 = 0x1 → MOV (word): iw bits 15-12 = 0001
        0x1: lambda self, sm, sr, dm, dr, word: self._op_mov(sm, sr, dm, dr, True),
        # op4 = 0x9 → MOVB (byte): bit15=1, bits 14-12 = 001
        0x9: lambda self, sm, sr, dm, dr, word: self._op_mov(sm, sr, dm, dr, False),
        # op4 = 0x2 → CMP
        0x2: lambda self, sm, sr, dm, dr, word: self._op_cmp(sm, sr, dm, dr, True),
        # op4 = 0xA → CMPB
        0xA: lambda self, sm, sr, dm, dr, word: self._op_cmp(sm, sr, dm, dr, False),
        # op4 = 0x3 → BIT
        0x3: lambda self, sm, sr, dm, dr, word: self._op_bit(sm, sr, dm, dr, True),
        # op4 = 0xB → BITB
        0xB: lambda self, sm, sr, dm, dr, word: self._op_bit(sm, sr, dm, dr, False),
        # op4 = 0x4 → BIC
        0x4: lambda self, sm, sr, dm, dr, word: self._op_bic(sm, sr, dm, dr, True),
        # op4 = 0xC → BICB
        0xC: lambda self, sm, sr, dm, dr, word: self._op_bic(sm, sr, dm, dr, False),
        # op4 = 0x5 → BIS
        0x5: lambda self, sm, sr, dm, dr, word: self._op_bis(sm, sr, dm, dr, True),
        # op4 = 0xD → BISB
        0xD: lambda self, sm, sr, dm, dr, word: self._op_bis(sm, sr, dm, dr, False),
        # op4 = 0x6 → ADD (word only)
        0x6: lambda self, sm, sr, dm, dr, word: self._op_add(sm, sr, dm, dr, True),
        # op4 = 0xE → SUB (word only): iw bits 15-12 = 1110
        0xE: lambda self, sm, sr, dm, dr, word: self._op_sub(sm, sr, dm, dr, True),
    }


# ── Branch condition functions ────────────────────────────────────────────────
#
# Each function takes (n, z, v, c) and returns True if the branch is taken.

def _br_always(n, z, v, c):  return True
def _br_ne(n, z, v, c):      return not z
def _br_eq(n, z, v, c):      return z
def _br_ge(n, z, v, c):      return not (n ^ v)
def _br_lt(n, z, v, c):      return n ^ v
def _br_gt(n, z, v, c):      return not z and not (n ^ v)
def _br_le(n, z, v, c):      return z or (n ^ v)
def _br_pl(n, z, v, c):      return not n
def _br_mi(n, z, v, c):      return n
def _br_hi(n, z, v, c):      return not c and not z
def _br_los(n, z, v, c):     return c or z
def _br_vc(n, z, v, c):      return not v
def _br_vs(n, z, v, c):      return v
def _br_cc(n, z, v, c):      return not c
def _br_cs(n, z, v, c):      return c

# Maps opcode byte → (condition_fn, mnemonic)
_BRANCH_TABLE: dict[int, tuple] = {
    0x01: (_br_always, "BR"),
    0x02: (_br_ne,     "BNE"),
    0x03: (_br_eq,     "BEQ"),
    0x04: (_br_ge,     "BGE"),
    0x05: (_br_lt,     "BLT"),
    0x06: (_br_gt,     "BGT"),
    0x07: (_br_le,     "BLE"),
    0x80: (_br_pl,     "BPL"),
    0x81: (_br_mi,     "BMI"),
    0x82: (_br_hi,     "BHI"),
    0x83: (_br_los,    "BLOS"),
    0x84: (_br_vc,     "BVC"),
    0x85: (_br_vs,     "BVS"),
    0x86: (_br_cc,     "BCC"),
    0x87: (_br_cs,     "BCS"),
}

_BRANCH_OPCODES = {k: v[0] for k, v in _BRANCH_TABLE.items()}
_BRANCH_NAMES   = {k: v[1] for k, v in _BRANCH_TABLE.items()}

# ── Wire up dispatch tables ───────────────────────────────────────────────────

_SINGLE_OPS = _make_single_ops(PDP11Simulator)
_DOUBLE_OPS = _make_double_ops(PDP11Simulator)
