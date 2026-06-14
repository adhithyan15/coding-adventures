"""Intel 8086 (1978) behavioral simulator.

──────────────────────────────────────────────────────────────────────────────
BACKGROUND
──────────────────────────────────────────────────────────────────────────────

On 8 June 1978, Intel announced the 8086 — a 16-bit extension of the 8080
architecture.  It introduced the segmented memory model that ruled the PC world
for over a decade, the ModRM addressing byte that encodes two register operands
or a register and memory address in a single byte, and the full 16-bit register
file (AX/BX/CX/DX with byte halves, SI/DI/SP/BP, CS/DS/SS/ES).

The IBM PC (1981) used the cheaper 8088 (same ISA, 8-bit external bus), making
the 8086 architecture the dominant computing platform for 40+ years.

──────────────────────────────────────────────────────────────────────────────
FETCH-DECODE-EXECUTE CYCLE
──────────────────────────────────────────────────────────────────────────────

1.  Physical address = CS × 16 + IP (modulo 2²⁰)
2.  Read opcode byte; IP += 1
3.  If opcode is a prefix (segment override, REP/REPNE, LOCK), record it;
    repeat step 2 for the actual opcode
4.  For opcodes that use a ModRM byte: read it; decode mod, reg, r/m
5.  Read displacement (disp8 or disp16) if mod requires it
6.  Read immediate bytes (imm8 or imm16) if the opcode requires it
7.  Compute effective address if memory operand is involved
8.  Execute: read operands, perform operation, write result, update flags
9.  Return StepTrace

──────────────────────────────────────────────────────────────────────────────
INSTRUCTION ENCODING
──────────────────────────────────────────────────────────────────────────────

Most instructions have the pattern:

    [prefix]  OPCODE  [ModRM]  [disp8|disp16]  [imm8|imm16]

OPCODE often encodes:
    bit 1 (d): direction — 0: r/m is dest; 1: reg is dest
    bit 0 (w): width     — 0: byte (8-bit); 1: word (16-bit)

ModRM byte:  mod[7:6]  reg[5:3]  r/m[2:0]

    mod=00: indirect via EA table (r/m=110 → [disp16])
    mod=01: indirect + disp8 (sign-extended)
    mod=10: indirect + disp16
    mod=11: register-to-register

Effective address base (r/m, mod ≠ 11):
    000 → BX+SI    100 → SI
    001 → BX+DI    101 → DI
    010 → BP+SI    110 → BP  (or [disp16] if mod=00)
    011 → BP+DI    111 → BX

──────────────────────────────────────────────────────────────────────────────
"""

from __future__ import annotations

from simulator_protocol import ExecutionResult, StepTrace

from intel_8086_simulator.flags import (
    compute_af_add,
    compute_af_sub,
    compute_cf_add,
    compute_cf_sub,
    compute_of_add,
    compute_of_sub,
    compute_szp,
    unpack_flags,
)
from intel_8086_simulator.state import X86State

# ── Constants ────────────────────────────────────────────────────────────────
_MEM_SIZE = 1_048_576
_PORT_SIZE = 256
_BYTE_MASK = 0xFF
_WORD_MASK = 0xFFFF
_PHYS_MASK = 0xFFFFF   # 20-bit physical address mask

# ModRM reg encoding → register index (0=AX/AL, 1=CX/CL, 2=DX/DL, 3=BX/BL,
#                                       4=SP/AH, 5=BP/CH, 6=SI/DH, 7=DI/BH)
_REG_AX = 0; _REG_CX = 1; _REG_DX = 2; _REG_BX = 3
_REG_SP = 4; _REG_BP = 5; _REG_SI = 6; _REG_DI = 7

# Segment register encoding for MOV sreg (reg field):
# 0=ES, 1=CS, 2=SS, 3=DS
_SREG_ES = 0; _SREG_CS = 1; _SREG_SS = 2; _SREG_DS = 3


class X86Simulator:
    """Behavioral simulator for the Intel 8086 (1978).

    Implements ``Simulator[X86State]`` (SIM00).

    Usage
    -----
    The simplest entry point is ``execute()``:

        sim = X86Simulator()
        result = sim.execute(bytes([
            0xB8, 0x0A, 0x00,   # MOV AX, 10
            0xF4,               # HLT
        ]))
        assert result.final_state.ax == 10

    For step-by-step debugging:

        sim.reset()
        sim.load(program)
        while not sim._halted:
            trace = sim.step()
            print(trace.mnemonic)

    Memory layout
    -------------
    ``load(program, origin=0)`` writes bytes into the 1 MB flat memory starting
    at physical address ``origin``.  CS=0 and IP=0 at reset, so instructions at
    address 0 execute first.

    Reset state
    -----------
    All registers = 0.  All flags = False.  All memory = 0.  IP = 0.
    """

    def __init__(self) -> None:
        self._mem: bytearray = bytearray(_MEM_SIZE)
        self._ax: int = 0; self._bx: int = 0
        self._cx: int = 0; self._dx: int = 0
        self._si: int = 0; self._di: int = 0
        self._sp: int = 0; self._bp: int = 0
        self._cs: int = 0; self._ds: int = 0
        self._ss: int = 0; self._es: int = 0
        self._ip: int = 0
        self._cf: bool = False; self._pf: bool = False
        self._af: bool = False; self._zf: bool = False
        self._sf: bool = False; self._tf: bool = False
        self._if: bool = False; self._df: bool = False
        self._of: bool = False
        self._halted: bool = False
        self._input_ports: bytearray = bytearray(_PORT_SIZE)
        self._output_ports: bytearray = bytearray(_PORT_SIZE)

    # ------------------------------------------------------------------
    # SIM00 protocol — public interface
    # ------------------------------------------------------------------

    def reset(self) -> None:
        """Reset to power-on state: all registers/flags/memory zeroed."""
        self._mem = bytearray(_MEM_SIZE)
        self._ax = self._bx = self._cx = self._dx = 0
        self._si = self._di = self._sp = self._bp = 0
        self._cs = self._ds = self._ss = self._es = 0
        self._ip = 0
        self._cf = self._pf = self._af = self._zf = False
        self._sf = self._tf = self._if = self._df = self._of = False
        self._halted = False
        self._input_ports = bytearray(_PORT_SIZE)
        self._output_ports = bytearray(_PORT_SIZE)

    def load(self, program: bytes, origin: int = 0) -> None:
        """Write ``program`` bytes into memory at physical address ``origin``.

        Bytes that would fall beyond 0xFFFFF are silently dropped.
        Does not reset any registers.

        Parameters
        ----------
        program :
            Raw machine-code bytes.
        origin :
            Physical address (0–0xFFFFF) where writing begins.
        """
        end = min(origin + len(program), _MEM_SIZE)
        self._mem[origin:end] = program[: end - origin]

    def step(self) -> StepTrace:
        """Execute one fetch-decode-execute cycle.

        Returns
        -------
        StepTrace :
            pc_before = IP before fetch, pc_after = IP after execution,
            mnemonic = disassembly string, description = detail string.

        Raises
        ------
        RuntimeError :
            If the simulator is halted.
        """
        if self._halted:
            raise RuntimeError("X86Simulator is halted; call reset() to restart")

        ip_before = self._ip
        mnemonic = self._fetch_decode_execute()
        description = f"{mnemonic} @ CS:IP={self._cs:04X}:{ip_before:04X}"

        return StepTrace(
            pc_before=ip_before,
            pc_after=self._ip,
            mnemonic=mnemonic,
            description=description,
        )

    def execute(
        self, program: bytes, max_steps: int = 10_000
    ) -> ExecutionResult[X86State]:
        """Reset, load, run to HLT or max_steps; return full result.

        Parameters
        ----------
        program :
            Raw machine-code bytes loaded at physical address 0.
        max_steps :
            Safety ceiling to prevent infinite loops.
        """
        self.reset()
        self.load(program)

        traces: list[StepTrace] = []
        steps = 0
        while not self._halted and steps < max_steps:
            trace = self.step()
            traces.append(trace)
            steps += 1

        error = None if self._halted else f"max_steps ({max_steps}) exceeded"

        return ExecutionResult(
            halted=self._halted,
            steps=steps,
            final_state=self.get_state(),
            error=error,
            traces=traces,
        )

    def get_state(self) -> X86State:
        """Return an immutable ``X86State`` snapshot of the current CPU."""
        return X86State(
            ax=self._ax, bx=self._bx, cx=self._cx, dx=self._dx,
            si=self._si, di=self._di, sp=self._sp, bp=self._bp,
            cs=self._cs, ds=self._ds, ss=self._ss, es=self._es,
            ip=self._ip,
            cf=self._cf, pf=self._pf, af=self._af, zf=self._zf,
            sf=self._sf, tf=self._tf, if_=self._if, df=self._df,
            of=self._of,
            halted=self._halted,
            input_ports=tuple(self._input_ports),
            output_ports=tuple(self._output_ports),
            memory=tuple(self._mem),
        )

    # ------------------------------------------------------------------
    # Internal: memory helpers
    # ------------------------------------------------------------------

    def _phys(self, seg: int, offset: int) -> int:
        """Compute 20-bit physical address from segment and offset."""
        return ((seg << 4) + offset) & _PHYS_MASK

    def _read_byte(self, seg: int, offset: int) -> int:
        return self._mem[self._phys(seg, offset)]

    def _write_byte(self, seg: int, offset: int, value: int) -> None:
        self._mem[self._phys(seg, offset)] = value & _BYTE_MASK

    def _read_word(self, seg: int, offset: int) -> int:
        lo = self._mem[self._phys(seg, offset)]
        hi = self._mem[self._phys(seg, (offset + 1) & _WORD_MASK)]
        return lo | (hi << 8)

    def _write_word(self, seg: int, offset: int, value: int) -> None:
        value &= _WORD_MASK
        self._mem[self._phys(seg, offset)] = value & _BYTE_MASK
        hi_addr = self._phys(seg, (offset + 1) & _WORD_MASK)
        self._mem[hi_addr] = (value >> 8) & _BYTE_MASK

    def _fetch8(self) -> int:
        """Read one byte from CS:IP and advance IP."""
        v = self._mem[self._phys(self._cs, self._ip)]
        self._ip = (self._ip + 1) & _WORD_MASK
        return v

    def _fetch16(self) -> int:
        lo = self._fetch8()
        hi = self._fetch8()
        return lo | (hi << 8)

    def _fetch_s8(self) -> int:
        """Fetch signed 8-bit byte (sign-extended to Python int)."""
        v = self._fetch8()
        return v if v < 0x80 else v - 0x100

    def _fetch_s16(self) -> int:
        v = self._fetch16()
        return v if v < 0x8000 else v - 0x10000

    # ------------------------------------------------------------------
    # Internal: register read/write (by index)
    # ------------------------------------------------------------------

    def _get_reg16(self, reg: int) -> int:
        """Read 16-bit general-purpose register by ModRM reg index."""
        match reg:
            case 0: return self._ax
            case 1: return self._cx
            case 2: return self._dx
            case 3: return self._bx
            case 4: return self._sp
            case 5: return self._bp
            case 6: return self._si
            case _: return self._di  # 7

    def _set_reg16(self, reg: int, val: int) -> None:
        val &= _WORD_MASK
        match reg:
            case 0: self._ax = val
            case 1: self._cx = val
            case 2: self._dx = val
            case 3: self._bx = val
            case 4: self._sp = val
            case 5: self._bp = val
            case 6: self._si = val
            case _: self._di = val  # 7

    def _get_reg8(self, reg: int) -> int:
        """Read 8-bit register by ModRM reg index (AL/CL/DL/BL/AH/CH/DH/BH)."""
        match reg:
            case 0: return self._ax & _BYTE_MASK            # AL
            case 1: return self._cx & _BYTE_MASK            # CL
            case 2: return self._dx & _BYTE_MASK            # DL
            case 3: return self._bx & _BYTE_MASK            # BL
            case 4: return (self._ax >> 8) & _BYTE_MASK     # AH
            case 5: return (self._cx >> 8) & _BYTE_MASK     # CH
            case 6: return (self._dx >> 8) & _BYTE_MASK     # DH
            case _: return (self._bx >> 8) & _BYTE_MASK     # BH (7)

    def _set_reg8(self, reg: int, val: int) -> None:
        val &= _BYTE_MASK
        match reg:
            case 0: self._ax = (self._ax & 0xFF00) | val          # AL
            case 1: self._cx = (self._cx & 0xFF00) | val          # CL
            case 2: self._dx = (self._dx & 0xFF00) | val          # DL
            case 3: self._bx = (self._bx & 0xFF00) | val          # BL
            case 4: self._ax = (self._ax & 0x00FF) | (val << 8)   # AH
            case 5: self._cx = (self._cx & 0x00FF) | (val << 8)   # CH
            case 6: self._dx = (self._dx & 0x00FF) | (val << 8)   # DH
            case _: self._bx = (self._bx & 0x00FF) | (val << 8)   # BH (7)

    _REG8_NAMES = ["AL", "CL", "DL", "BL", "AH", "CH", "DH", "BH"]
    _REG16_NAMES = ["AX", "CX", "DX", "BX", "SP", "BP", "SI", "DI"]
    _SREG_NAMES = ["ES", "CS", "SS", "DS"]

    def _get_sreg(self, reg: int) -> int:
        """Read segment register by MOV sreg encoding (0=ES,1=CS,2=SS,3=DS)."""
        match reg:
            case 0: return self._es
            case 1: return self._cs
            case 2: return self._ss
            case _: return self._ds  # 3

    def _set_sreg(self, reg: int, val: int) -> None:
        val &= _WORD_MASK
        match reg:
            case 0: self._es = val
            case 1: self._cs = val
            case 2: self._ss = val
            case _: self._ds = val  # 3

    # ------------------------------------------------------------------
    # Internal: ModRM decode
    # ------------------------------------------------------------------

    def _decode_modrm(
        self, modrm: int, word: bool, seg_override: int | None
    ) -> tuple[int, int, str]:
        """Decode a ModRM byte, returning (effective_seg, effective_offset, ea_str).

        For mod=11 (register), returns (0, reg_index, "reg_name") — the caller
        must recognise mod=11 separately and use reg_index to read/write a reg.
        """
        mod = (modrm >> 6) & 0x3
        rm = modrm & 0x7

        # Register-to-register: no memory access
        if mod == 0b11:
            return 0, rm, ""

        # Default segment: BP-based addressing → SS; all else → DS.
        # BP is used when rm ∈ {2, 3} (BP+SI, BP+DI) or rm=6 with mod≠0 (just BP).
        uses_bp = rm in (2, 3) or (rm == 6 and mod != 0)
        default_seg = self._ss if uses_bp else self._ds

        # Compute effective address offset
        if rm == 0:
            ea = (self._bx + self._si) & _WORD_MASK
        elif rm == 1:
            ea = (self._bx + self._di) & _WORD_MASK
        elif rm == 2:
            ea = (self._bp + self._si) & _WORD_MASK
        elif rm == 3:
            ea = (self._bp + self._di) & _WORD_MASK
        elif rm == 4:
            ea = self._si
        elif rm == 5:
            ea = self._di
        elif rm == 6:
            ea = self._fetch16() if mod == 0 else self._bp
        else:  # rm == 7
            ea = self._bx

        # Displacement
        if mod == 0b01:
            disp = self._fetch_s8()
            ea = (ea + disp) & _WORD_MASK
        elif mod == 0b10:
            disp = self._fetch_s16()
            ea = (ea + disp) & _WORD_MASK

        seg = seg_override if seg_override is not None else default_seg
        ea_names = ["BX+SI", "BX+DI", "BP+SI", "BP+DI", "SI", "DI", "BP", "BX"]
        ea_str = f"[{ea_names[rm] if not (rm == 6 and mod == 0) else hex(ea)}]"
        return seg, ea, ea_str

    def _read_rm(
        self, mod: int, rm: int, seg: int, ea: int, word: bool
    ) -> int:
        """Read value from r/m operand (register or memory)."""
        if mod == 0b11:
            return self._get_reg16(rm) if word else self._get_reg8(rm)
        return self._read_word(seg, ea) if word else self._read_byte(seg, ea)

    def _write_rm(
        self, mod: int, rm: int, seg: int, ea: int, val: int, word: bool
    ) -> None:
        """Write value to r/m operand (register or memory)."""
        if mod == 0b11:
            if word:
                self._set_reg16(rm, val)
            else:
                self._set_reg8(rm, val)
        elif word:
            self._write_word(seg, ea, val)
        else:
            self._write_byte(seg, ea, val)

    # ------------------------------------------------------------------
    # Internal: stack
    # ------------------------------------------------------------------

    def _push16(self, val: int) -> None:
        self._sp = (self._sp - 2) & _WORD_MASK
        self._write_word(self._ss, self._sp, val)

    def _pop16(self) -> int:
        val = self._read_word(self._ss, self._sp)
        self._sp = (self._sp + 2) & _WORD_MASK
        return val

    # ------------------------------------------------------------------
    # Internal: flag helpers
    # ------------------------------------------------------------------

    def _set_szp(self, result: int, *, word: bool) -> None:
        self._sf, self._zf, self._pf = compute_szp(result, word=word)

    def _set_flags_add(
        self, a: int, b: int, result: int, *, word: bool, carry_in: int = 0
    ) -> None:
        mask = _WORD_MASK if word else _BYTE_MASK
        r = result & mask
        self._cf = compute_cf_add(result, word=word)
        self._af = compute_af_add(a, b, carry_in)
        self._of = compute_of_add(a & mask, b & mask, r, word=word)
        self._set_szp(r, word=word)

    def _set_flags_sub(
        self, a: int, b: int, result: int, *, word: bool, borrow: int = 0
    ) -> None:
        mask = _WORD_MASK if word else _BYTE_MASK
        r = result & mask
        self._cf = compute_cf_sub(a, b, borrow)
        self._af = compute_af_sub(a, b, borrow)
        self._of = compute_of_sub(a & mask, b & mask, r, word=word)
        self._set_szp(r, word=word)

    def _set_flags_logic(self, result: int, *, word: bool) -> None:
        """AND/OR/XOR: CF=0, OF=0, PF/ZF/SF from result, AF undefined (cleared)."""
        self._cf = False
        self._of = False
        self._af = False
        self._set_szp(result, word=word)

    # ------------------------------------------------------------------
    # Internal: arithmetic operations (return masked result + set flags)
    # ------------------------------------------------------------------

    def _add(self, a: int, b: int, *, word: bool, carry: int = 0) -> int:
        raw = a + b + carry
        self._set_flags_add(a, b, raw, word=word, carry_in=carry)
        return raw & (_WORD_MASK if word else _BYTE_MASK)

    def _sub(self, a: int, b: int, *, word: bool, borrow: int = 0) -> int:
        raw = a - b - borrow
        self._set_flags_sub(a, b, raw, word=word, borrow=borrow)
        return raw & (_WORD_MASK if word else _BYTE_MASK)

    def _and(self, a: int, b: int, *, word: bool) -> int:
        r = (a & b) & (_WORD_MASK if word else _BYTE_MASK)
        self._set_flags_logic(r, word=word)
        return r

    def _or(self, a: int, b: int, *, word: bool) -> int:
        r = (a | b) & (_WORD_MASK if word else _BYTE_MASK)
        self._set_flags_logic(r, word=word)
        return r

    def _xor(self, a: int, b: int, *, word: bool) -> int:
        r = (a ^ b) & (_WORD_MASK if word else _BYTE_MASK)
        self._set_flags_logic(r, word=word)
        return r

    # ------------------------------------------------------------------
    # Internal: shift / rotate helpers
    # ------------------------------------------------------------------

    def _shl(self, val: int, count: int, *, word: bool) -> int:
        """SHL/SAL: logical left shift."""
        mask = _WORD_MASK if word else _BYTE_MASK
        msb = 0x8000 if word else 0x80
        count &= 0x1F  # real 8086 limits shift count (we mirror this)
        if count == 0:
            return val & mask
        # CF = last bit shifted out
        self._cf = bool((val << (count - 1)) & msb)
        result = (val << count) & mask
        self._of = bool(result & msb) != self._cf  # OF meaningful for count==1
        self._set_szp(result, word=word)
        return result

    def _shr(self, val: int, count: int, *, word: bool) -> int:
        """SHR: logical right shift."""
        mask = _WORD_MASK if word else _BYTE_MASK
        count &= 0x1F
        if count == 0:
            return val & mask
        self._cf = bool((val >> (count - 1)) & 1)
        msb = 0x8000 if word else 0x80
        self._of = bool(val & msb)  # OF = original MSB (meaningful for count==1)
        result = (val >> count) & mask
        self._set_szp(result, word=word)
        return result

    def _sar(self, val: int, count: int, *, word: bool) -> int:
        """SAR: arithmetic right shift (sign-filling)."""
        mask = _WORD_MASK if word else _BYTE_MASK
        msb = 0x8000 if word else 0x80
        count &= 0x1F
        if count == 0:
            return val & mask
        # Sign-extend
        signed = val if not (val & msb) else val - (mask + 1)
        self._cf = bool((val >> (count - 1)) & 1)
        self._of = False  # SAR never sets OF (for count==1; for others undefined)
        result = (signed >> count) & mask
        self._set_szp(result, word=word)
        return result

    def _rol(self, val: int, count: int, *, word: bool) -> int:
        """ROL: rotate left."""
        bits = 16 if word else 8
        mask = _WORD_MASK if word else _BYTE_MASK
        count = count % bits
        if count == 0:
            return val & mask
        result = ((val << count) | (val >> (bits - count))) & mask
        self._cf = bool(result & 1)
        msb = 0x8000 if word else 0x80
        self._of = bool(result & msb) != self._cf
        return result

    def _ror(self, val: int, count: int, *, word: bool) -> int:
        """ROR: rotate right."""
        bits = 16 if word else 8
        mask = _WORD_MASK if word else _BYTE_MASK
        count = count % bits
        if count == 0:
            return val & mask
        result = ((val >> count) | (val << (bits - count))) & mask
        msb = 0x8000 if word else 0x80
        self._cf = bool(result & msb)
        self._of = bool(result & msb) != bool(result & (msb >> 1))
        return result

    def _rcl(self, val: int, count: int, *, word: bool) -> int:
        """RCL: rotate left through carry."""
        bits = 16 if word else 8
        mask = _WORD_MASK if word else _BYTE_MASK
        count = count % (bits + 1)
        if count == 0:
            return val & mask
        combined = val | (int(self._cf) << bits)
        rotated = (combined << count) | (combined >> (bits + 1 - count))
        result = rotated & ((mask << 1) | 1)
        self._cf = bool(result & (1 << bits))
        result &= mask
        msb = 0x8000 if word else 0x80
        self._of = bool(result & msb) != self._cf
        return result

    def _rcr(self, val: int, count: int, *, word: bool) -> int:
        """RCR: rotate right through carry."""
        bits = 16 if word else 8
        mask = _WORD_MASK if word else _BYTE_MASK
        count = count % (bits + 1)
        if count == 0:
            return val & mask
        combined = val | (int(self._cf) << bits)
        result_wide = (combined >> count) | (combined << (bits + 1 - count))
        new_cf = bool(result_wide & (1 << bits))
        result = result_wide & mask
        self._cf = new_cf
        msb = 0x8000 if word else 0x80
        self._of = bool(result & msb) != bool((result << 1) & msb)
        return result

    # ------------------------------------------------------------------
    # Internal: string increment/decrement
    # ------------------------------------------------------------------

    def _str_inc(self, word: bool) -> int:
        return 2 if word else 1

    def _str_step(self, word: bool) -> int:
        return -self._str_inc(word) if self._df else self._str_inc(word)

    # ------------------------------------------------------------------
    # Internal: fetch-decode-execute (returns mnemonic string)
    # ------------------------------------------------------------------

    def _fetch_decode_execute(self) -> str:  # noqa: C901 (complex but intentional)
        """Decode and execute the instruction at CS:IP. Advance IP. Return mnemonic."""
        seg_override: int | None = None
        rep_prefix: int | None = None   # 0xF3 = REP/REPE, 0xF2 = REPNE

        # ── Prefix loop ────────────────────────────────────────────────
        while True:
            op = self._fetch8()
            if op in (0x26, 0x2E, 0x36, 0x3E):
                # Segment override prefixes
                seg_override = {0x26: self._es, 0x2E: self._cs,
                                0x36: self._ss, 0x3E: self._ds}[op]
            elif op in (0xF2, 0xF3):
                rep_prefix = op
            elif op == 0xF0:
                pass  # LOCK — ignored
            else:
                break  # real opcode

        # ── Decode opcode ──────────────────────────────────────────────
        return self._exec_op(op, seg_override, rep_prefix)

    def _exec_op(  # noqa: C901
        self, op: int, seg_override: int | None, rep_prefix: int | None
    ) -> str:
        """Execute decoded opcode. Returns mnemonic string."""

        # ── Data transfer ──────────────────────────────────────────────

        # MOV r/m, reg  or  MOV reg, r/m  (88/89/8A/8B)
        if op in (0x88, 0x89, 0x8A, 0x8B):
            word = bool(op & 1)
            d = bool(op & 2)         # d=1: reg is destination
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3
            reg = (modrm >> 3) & 7
            rm = modrm & 7
            seg, ea, _ = self._decode_modrm(modrm, word, seg_override)
            reg_name = self._REG16_NAMES[reg] if word else self._REG8_NAMES[reg]
            rm_name = self._REG16_NAMES[rm] if (word and mod == 3) else (
                self._REG8_NAMES[rm] if (not word and mod == 3) else "m")
            if d:  # reg ← r/m
                src = self._read_rm(mod, rm, seg, ea, word)
                if word:
                    self._set_reg16(reg, src)
                else:
                    self._set_reg8(reg, src)
                return f"MOV {reg_name},{rm_name}"
            else:   # r/m ← reg
                src = self._get_reg16(reg) if word else self._get_reg8(reg)
                self._write_rm(mod, rm, seg, ea, src, word)
                return f"MOV {rm_name},{reg_name}"

        # MOV r/m8, imm8  (C6 /0)
        if op == 0xC6:
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3; rm = modrm & 7
            seg, ea, _ = self._decode_modrm(modrm, False, seg_override)
            imm = self._fetch8()
            self._write_rm(mod, rm, seg, ea, imm, False)
            return f"MOV m,{imm:#x}"

        # MOV r/m16, imm16  (C7 /0)
        if op == 0xC7:
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3; rm = modrm & 7
            seg, ea, _ = self._decode_modrm(modrm, True, seg_override)
            imm = self._fetch16()
            self._write_rm(mod, rm, seg, ea, imm, True)
            return f"MOV m,{imm:#x}"

        # MOV reg8, imm8  (B0–B7)
        if 0xB0 <= op <= 0xB7:
            reg = op - 0xB0
            imm = self._fetch8()
            self._set_reg8(reg, imm)
            return f"MOV {self._REG8_NAMES[reg]},{imm:#x}"

        # MOV reg16, imm16  (B8–BF)
        if 0xB8 <= op <= 0xBF:
            reg = op - 0xB8
            imm = self._fetch16()
            self._set_reg16(reg, imm)
            return f"MOV {self._REG16_NAMES[reg]},{imm:#x}"

        # MOV AL/AX, [imm16]  (A0/A1)
        if op in (0xA0, 0xA1):
            word = bool(op & 1)
            addr = self._fetch16()
            seg = seg_override if seg_override is not None else self._ds
            val = self._read_word(seg, addr) if word else self._read_byte(seg, addr)
            if word:
                self._ax = val
            else:
                self._ax = (self._ax & 0xFF00) | val
            return f"MOV {'AX' if word else 'AL'},[{addr:#x}]"

        # MOV [imm16], AL/AX  (A2/A3)
        if op in (0xA2, 0xA3):
            word = bool(op & 1)
            addr = self._fetch16()
            seg = seg_override if seg_override is not None else self._ds
            val = self._ax if word else (self._ax & _BYTE_MASK)
            if word:
                self._write_word(seg, addr, val)
            else:
                self._write_byte(seg, addr, val)
            return f"MOV [{addr:#x}],{'AX' if word else 'AL'}"

        # MOV r/m, sreg  (8C)
        if op == 0x8C:
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3; reg = (modrm >> 3) & 7; rm = modrm & 7
            seg_r, ea, _ = self._decode_modrm(modrm, True, seg_override)
            val = self._get_sreg(reg & 3)
            self._write_rm(mod, rm, seg_r, ea, val, True)
            return f"MOV m,{self._SREG_NAMES[reg & 3]}"

        # MOV sreg, r/m  (8E)
        if op == 0x8E:
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3; reg = (modrm >> 3) & 7; rm = modrm & 7
            seg_r, ea, _ = self._decode_modrm(modrm, True, seg_override)
            val = self._read_rm(mod, rm, seg_r, ea, True)
            self._set_sreg(reg & 3, val)
            return f"MOV {self._SREG_NAMES[reg & 3]},m"

        # XCHG AX, reg (90-97; 90=NOP=XCHG AX,AX)
        if 0x90 <= op <= 0x97:
            reg = op - 0x90
            if reg == 0:
                return "NOP"
            tmp = self._ax
            self._ax = self._get_reg16(reg)
            self._set_reg16(reg, tmp)
            return f"XCHG AX,{self._REG16_NAMES[reg]}"

        # XCHG r/m, reg  (86/87)
        if op in (0x86, 0x87):
            word = bool(op & 1)
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3; reg = (modrm >> 3) & 7; rm = modrm & 7
            seg, ea, _ = self._decode_modrm(modrm, word, seg_override)
            a = self._read_rm(mod, rm, seg, ea, word)
            b = self._get_reg16(reg) if word else self._get_reg8(reg)
            self._write_rm(mod, rm, seg, ea, b, word)
            if word:
                self._set_reg16(reg, a)
            else:
                self._set_reg8(reg, a)
            if mod != 3:
                rm_s = "m"
            elif word:
                rm_s = self._REG16_NAMES[rm]
            else:
                rm_s = self._REG8_NAMES[rm]
            rg_s = self._REG16_NAMES[reg] if word else self._REG8_NAMES[reg]
            return f"XCHG {rm_s},{rg_s}"

        # PUSH reg (50-57)
        if 0x50 <= op <= 0x57:
            reg = op - 0x50
            self._push16(self._get_reg16(reg))
            return f"PUSH {self._REG16_NAMES[reg]}"

        # POP reg (58-5F)
        if 0x58 <= op <= 0x5F:
            reg = op - 0x58
            self._set_reg16(reg, self._pop16())
            return f"POP {self._REG16_NAMES[reg]}"

        # PUSH sreg: ES=06, CS=0E, SS=16, DS=1E
        if op in (0x06, 0x0E, 0x16, 0x1E):
            smap = {0x06: 0, 0x0E: 1, 0x16: 2, 0x1E: 3}
            sreg_idx = smap[op]
            self._push16(self._get_sreg(sreg_idx))
            return f"PUSH {self._SREG_NAMES[sreg_idx]}"

        # POP sreg: ES=07, SS=17, DS=1F  (CS cannot be popped — use 0F for future)
        if op in (0x07, 0x17, 0x1F):
            smap = {0x07: 0, 0x17: 2, 0x1F: 3}
            sreg_idx = smap[op]
            self._set_sreg(sreg_idx, self._pop16())
            return f"POP {self._SREG_NAMES[sreg_idx]}"

        # PUSH r/m (FF /6)  — handled in FF group below
        # POP r/m  (8F /0)
        if op == 0x8F:
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3; rm = modrm & 7
            seg, ea, _ = self._decode_modrm(modrm, True, seg_override)
            val = self._pop16()
            self._write_rm(mod, rm, seg, ea, val, True)
            return "POP m"

        # PUSHF / POPF
        if op == 0x9C:
            self._push16(self._flags_val())
            return "PUSHF"
        if op == 0x9D:
            f = self._pop16()
            self._load_flags(f)
            return "POPF"

        # LEA reg, r/m  (8D)
        if op == 0x8D:
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3; reg = (modrm >> 3) & 7; rm = modrm & 7
            _, ea, _ = self._decode_modrm(modrm, True, seg_override)
            # For LEA we want the offset only, not the memory value
            self._set_reg16(reg, ea & _WORD_MASK)
            return f"LEA {self._REG16_NAMES[reg]},m"

        # LDS reg, m32  (C5)
        if op == 0xC5:
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3; reg = (modrm >> 3) & 7; rm = modrm & 7
            seg_r, ea, _ = self._decode_modrm(modrm, True, seg_override)
            off = self._read_word(seg_r, ea)
            new_ds = self._read_word(seg_r, (ea + 2) & _WORD_MASK)
            self._set_reg16(reg, off)
            self._ds = new_ds
            return f"LDS {self._REG16_NAMES[reg]},m"

        # LES reg, m32  (C4)
        if op == 0xC4:
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3; reg = (modrm >> 3) & 7; rm = modrm & 7
            seg_r, ea, _ = self._decode_modrm(modrm, True, seg_override)
            off = self._read_word(seg_r, ea)
            new_es = self._read_word(seg_r, (ea + 2) & _WORD_MASK)
            self._set_reg16(reg, off)
            self._es = new_es
            return f"LES {self._REG16_NAMES[reg]},m"

        # LAHF / SAHF
        if op == 0x9F:   # LAHF
            self._ax = (self._ax & 0x00FF) | (self._flags_low8() << 8)
            return "LAHF"
        if op == 0x9E:   # SAHF
            self._load_flags_low8((self._ax >> 8) & _BYTE_MASK)
            return "SAHF"

        # CBW  (98)
        if op == 0x98:
            al = self._ax & _BYTE_MASK
            self._ax = al if al < 0x80 else al | 0xFF00
            return "CBW"

        # CWD  (99)
        if op == 0x99:
            self._dx = 0xFFFF if (self._ax & 0x8000) else 0
            return "CWD"

        # XLAT  (D7)
        if op == 0xD7:
            al = self._ax & _BYTE_MASK
            seg = seg_override if seg_override is not None else self._ds
            xlat_addr = (self._bx + al) & _WORD_MASK
            self._ax = (self._ax & 0xFF00) | self._read_byte(seg, xlat_addr)
            return "XLAT"

        # ── Arithmetic / logical (80-group)  ───────────────────────────
        # 80: op r/m8, imm8
        # 81: op r/m16, imm16
        # 82: op r/m8, imm8 (alias for 80)
        # 83: op r/m16, sign-extended imm8
        if op in (0x80, 0x81, 0x82, 0x83):
            word = op == 0x81 or op == 0x83
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3; ext = (modrm >> 3) & 7; rm = modrm & 7
            seg, ea, _ = self._decode_modrm(modrm, word, seg_override)
            if op in (0x80, 0x82):
                imm = self._fetch8()
            elif op == 0x81:
                imm = self._fetch16()
            else:  # 0x83 sign-extend
                imm = self._fetch_s8() & _WORD_MASK
            a = self._read_rm(mod, rm, seg, ea, word)
            result, mnem = self._alu_op(ext, a, imm, word)
            if ext != 7:  # CMP does not write
                self._write_rm(mod, rm, seg, ea, result, word)
            return f"{mnem} m,{imm:#x}"

        # ADD/OR/ADC/SBB/AND/SUB/XOR/CMP reg/mem pairs (00-3F)
        _standard_alu_ops = {
            0x00: (0, False, False), 0x01: (0, True, False),
            0x02: (0, False, True),  0x03: (0, True, True),
            0x04: None,              0x05: None,   # handled below as AL/AX,imm
            0x08: (1, False, False), 0x09: (1, True, False),
            0x0A: (1, False, True),  0x0B: (1, True, True),
            0x0C: None,              0x0D: None,
            0x10: (2, False, False), 0x11: (2, True, False),
            0x12: (2, False, True),  0x13: (2, True, True),
            0x14: None,              0x15: None,
            0x18: (3, False, False), 0x19: (3, True, False),
            0x1A: (3, False, True),  0x1B: (3, True, True),
            0x1C: None,              0x1D: None,
            0x20: (4, False, False), 0x21: (4, True, False),
            0x22: (4, False, True),  0x23: (4, True, True),
            0x24: None,              0x25: None,
            0x28: (5, False, False), 0x29: (5, True, False),
            0x2A: (5, False, True),  0x2B: (5, True, True),
            0x2C: None,              0x2D: None,
            0x30: (6, False, False), 0x31: (6, True, False),
            0x32: (6, False, True),  0x33: (6, True, True),
            0x34: None,              0x35: None,
            0x38: (7, False, False), 0x39: (7, True, False),
            0x3A: (7, False, True),  0x3B: (7, True, True),
            0x3C: None,              0x3D: None,
        }
        _alu_names = ["ADD", "OR", "ADC", "SBB", "AND", "SUB", "XOR", "CMP"]
        _acc_imm_ops = {
            0x04: (0, False), 0x05: (0, True),
            0x0C: (1, False), 0x0D: (1, True),
            0x14: (2, False), 0x15: (2, True),
            0x1C: (3, False), 0x1D: (3, True),
            0x24: (4, False), 0x25: (4, True),
            0x2C: (5, False), 0x2D: (5, True),
            0x34: (6, False), 0x35: (6, True),
            0x3C: (7, False), 0x3D: (7, True),
            0xA8: (4, False), 0xA9: (4, True),  # TEST AL/AX, imm
        }

        # TEST r/m8, reg8 (84) / TEST r/m16, reg16 (85)
        # Like AND but result is discarded — only flags are updated.
        if op in (0x84, 0x85):
            word = bool(op & 1)
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3; reg = (modrm >> 3) & 7; rm = modrm & 7
            seg, ea, _ = self._decode_modrm(modrm, word, seg_override)
            a = self._read_rm(mod, rm, seg, ea, word)
            b = self._get_reg16(reg) if word else self._get_reg8(reg)
            self._and(a, b, word=word)   # flags updated; result discarded
            rm_name = (self._REG16_NAMES[rm] if (word and mod == 3) else
                       (self._REG8_NAMES[rm] if (not word and mod == 3) else "m"))
            reg_name = self._REG16_NAMES[reg] if word else self._REG8_NAMES[reg]
            return f"TEST {rm_name},{reg_name}"

        if op in _acc_imm_ops:
            alu_op, word = _acc_imm_ops[op]
            imm = self._fetch16() if word else self._fetch8()
            a = self._ax if word else (self._ax & _BYTE_MASK)
            result, mnem = self._alu_op(alu_op, a, imm, word)
            # CMP (alu_op=7) and TEST (0xA8/0xA9) only affect flags; others write back
            if alu_op != 7 and op not in (0xA8, 0xA9):
                if word:
                    self._ax = result
                else:
                    self._ax = (self._ax & 0xFF00) | result
            return f"{mnem} {'AX' if word else 'AL'},{imm:#x}"

        if op in _standard_alu_ops and _standard_alu_ops[op] is not None:
            alu_op, word, d = _standard_alu_ops[op]
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3; reg = (modrm >> 3) & 7; rm = modrm & 7
            seg, ea, _ = self._decode_modrm(modrm, word, seg_override)
            mnem = _alu_names[alu_op]
            if d:  # reg ← reg op r/m
                a = self._get_reg16(reg) if word else self._get_reg8(reg)
                b = self._read_rm(mod, rm, seg, ea, word)
                result, _ = self._alu_op(alu_op, a, b, word)
                if alu_op != 7:  # CMP
                    if word:
                        self._set_reg16(reg, result)
                    else:
                        self._set_reg8(reg, result)
                rm_name = self._REG16_NAMES[rm] if (word and mod == 3) else (
                    self._REG8_NAMES[rm] if (not word and mod == 3) else "m")
                rn = self._REG16_NAMES[reg] if word else self._REG8_NAMES[reg]
                return f"{mnem} {rn},{rm_name}"
            else:  # r/m ← r/m op reg
                a = self._read_rm(mod, rm, seg, ea, word)
                b = self._get_reg16(reg) if word else self._get_reg8(reg)
                result, _ = self._alu_op(alu_op, a, b, word)
                if alu_op != 7:
                    self._write_rm(mod, rm, seg, ea, result, word)
                rm_name = self._REG16_NAMES[rm] if (word and mod == 3) else (
                    self._REG8_NAMES[rm] if (not word and mod == 3) else "m")
                rn = self._REG16_NAMES[reg] if word else self._REG8_NAMES[reg]
                return f"{mnem} {rm_name},{rn}"

        # INC reg16 (40-47) / DEC reg16 (48-4F)
        if 0x40 <= op <= 0x47:
            reg = op - 0x40
            old_cf = self._cf
            result = self._add(self._get_reg16(reg), 1, word=True)
            self._set_reg16(reg, result)
            self._cf = old_cf  # INC does not affect CF
            return f"INC {self._REG16_NAMES[reg]}"

        if 0x48 <= op <= 0x4F:
            reg = op - 0x48
            old_cf = self._cf
            result = self._sub(self._get_reg16(reg), 1, word=True)
            self._set_reg16(reg, result)
            self._cf = old_cf  # DEC does not affect CF
            return f"DEC {self._REG16_NAMES[reg]}"

        # FE group: INC/DEC r/m8
        if op == 0xFE:
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3; ext = (modrm >> 3) & 7; rm = modrm & 7
            seg, ea, _ = self._decode_modrm(modrm, False, seg_override)
            a = self._read_rm(mod, rm, seg, ea, False)
            old_cf = self._cf
            if ext == 0:
                result = self._add(a, 1, word=False)
                mnem = "INC"
            else:
                result = self._sub(a, 1, word=False)
                mnem = "DEC"
            self._write_rm(mod, rm, seg, ea, result, False)
            self._cf = old_cf
            return f"{mnem} m8"

        # FF group: INC/DEC/CALL/JMP/PUSH r/m16
        if op == 0xFF:
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3; ext = (modrm >> 3) & 7; rm = modrm & 7
            seg, ea, _ = self._decode_modrm(modrm, True, seg_override)
            val = self._read_rm(mod, rm, seg, ea, True)
            if ext == 0:   # INC r/m16
                old_cf = self._cf
                result = self._add(val, 1, word=True)
                self._write_rm(mod, rm, seg, ea, result, True)
                self._cf = old_cf
                return "INC m16"
            if ext == 1:   # DEC r/m16
                old_cf = self._cf
                result = self._sub(val, 1, word=True)
                self._write_rm(mod, rm, seg, ea, result, True)
                self._cf = old_cf
                return "DEC m16"
            if ext == 2:   # CALL near indirect
                self._push16(self._ip)
                self._ip = val
                return "CALL rm16"
            if ext == 3:   # CALL far indirect
                new_off = self._read_word(seg, ea)
                new_cs = self._read_word(seg, (ea + 2) & _WORD_MASK)
                self._push16(self._cs)
                self._push16(self._ip)
                self._cs = new_cs
                self._ip = new_off
                return "CALL FAR m32"
            if ext == 4:   # JMP near indirect
                self._ip = val
                return "JMP rm16"
            if ext == 5:   # JMP far indirect
                new_off = self._read_word(seg, ea)
                new_cs = self._read_word(seg, (ea + 2) & _WORD_MASK)
                self._cs = new_cs
                self._ip = new_off
                return "JMP FAR m32"
            if ext == 6:   # PUSH r/m16
                self._push16(val)
                return "PUSH m16"

        # F6/F7 group: TEST/NOT/NEG/MUL/IMUL/DIV/IDIV r/m
        if op in (0xF6, 0xF7):
            word = bool(op & 1)
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3; ext = (modrm >> 3) & 7; rm = modrm & 7
            seg, ea, _ = self._decode_modrm(modrm, word, seg_override)
            a = self._read_rm(mod, rm, seg, ea, word)
            if ext == 0:   # TEST r/m, imm
                imm = self._fetch16() if word else self._fetch8()
                self._and(a, imm, word=word)
                return f"TEST m,{imm:#x}"
            if ext == 2:   # NOT r/m
                mask = _WORD_MASK if word else _BYTE_MASK
                self._write_rm(mod, rm, seg, ea, (~a) & mask, word)
                return "NOT m"
            if ext == 3:   # NEG r/m
                mask = _WORD_MASK if word else _BYTE_MASK
                result = self._sub(0, a, word=word)
                self._write_rm(mod, rm, seg, ea, result, word)
                self._cf = a != 0
                return "NEG m"
            if ext == 4:   # MUL (unsigned)
                if word:
                    result32 = (self._ax & _WORD_MASK) * (a & _WORD_MASK)
                    self._ax = result32 & _WORD_MASK
                    self._dx = (result32 >> 16) & _WORD_MASK
                    self._cf = self._of = (self._dx != 0)
                else:
                    result16 = (self._ax & _BYTE_MASK) * (a & _BYTE_MASK)
                    self._ax = result16 & _WORD_MASK
                    self._cf = self._of = ((self._ax >> 8) != 0)
                return "MUL m"
            if ext == 5:   # IMUL (signed)
                if word:
                    a_s = self._ax if self._ax < 0x8000 else self._ax - 0x10000
                    b_s = a if a < 0x8000 else a - 0x10000
                    result32 = a_s * b_s
                    self._ax = result32 & _WORD_MASK
                    self._dx = (result32 >> 16) & _WORD_MASK
                    expected_hi = 0xFFFF if (self._ax & 0x8000) else 0
                    self._cf = self._of = (self._dx != expected_hi)
                else:
                    a_s = (self._ax & _BYTE_MASK)
                    if a_s >= 0x80:
                        a_s -= 0x100
                    b_s = a if a < 0x80 else a - 0x100
                    result16 = a_s * b_s
                    self._ax = result16 & _WORD_MASK
                    expected_hi = 0xFF if (self._ax & 0x80) else 0
                    self._cf = self._of = ((self._ax >> 8) != expected_hi)
                return "IMUL m"
            if ext == 6:   # DIV (unsigned)
                if word:
                    dividend = ((self._dx & _WORD_MASK) << 16) | (self._ax & _WORD_MASK)
                    if a == 0:
                        self._halted = True
                        return "DIV /0"
                    self._ax = (dividend // a) & _WORD_MASK
                    self._dx = (dividend % a) & _WORD_MASK
                else:
                    dividend = self._ax & _WORD_MASK
                    if a == 0:
                        self._halted = True
                        return "DIV /0"
                    quotient = (dividend // a) & _BYTE_MASK
                    remainder = (dividend % a) & _BYTE_MASK
                    self._ax = (remainder << 8) | quotient
                return "DIV m"
            if ext == 7:   # IDIV (signed)
                if word:
                    dividend32 = (
                        ((self._dx & _WORD_MASK) << 16) | (self._ax & _WORD_MASK)
                    )
                    if dividend32 >= 0x80000000:
                        dividend32 -= 0x100000000
                    b_s = a if a < 0x8000 else a - 0x10000
                    if b_s == 0:
                        self._halted = True
                        return "IDIV /0"
                    q = int(dividend32 / b_s)
                    r = dividend32 - q * b_s
                    self._ax = q & _WORD_MASK
                    self._dx = r & _WORD_MASK
                else:
                    dividend16 = self._ax & _WORD_MASK
                    if dividend16 >= 0x8000:
                        dividend16 -= 0x10000
                    b_s = a if a < 0x80 else a - 0x100
                    if b_s == 0:
                        self._halted = True
                        return "IDIV /0"
                    q = int(dividend16 / b_s)
                    r = dividend16 - q * b_s
                    self._ax = ((r & _BYTE_MASK) << 8) | (q & _BYTE_MASK)
                return "IDIV m"

        # DAA / DAS / AAA / AAS / AAM / AAD
        if op == 0x27:   # DAA
            al = self._ax & _BYTE_MASK
            old_al = al; old_cf = self._cf; old_af = self._af
            if (al & 0xF) > 9 or old_af:
                al = (al + 6) & _BYTE_MASK
                self._af = True
            else:
                self._af = False
            if old_al > 0x99 or old_cf:
                al = (al + 0x60) & _BYTE_MASK
                self._cf = True
            else:
                self._cf = False
            self._ax = (self._ax & 0xFF00) | al
            self._sf, self._zf, self._pf = compute_szp(al, word=False)
            return "DAA"

        if op == 0x2F:   # DAS
            al = self._ax & _BYTE_MASK
            old_al = al; old_cf = self._cf; old_af = self._af
            if (al & 0xF) > 9 or old_af:
                al = (al - 6) & _BYTE_MASK
                self._af = True
            else:
                self._af = False
            if old_al > 0x99 or old_cf:
                al = (al - 0x60) & _BYTE_MASK
                self._cf = True
            else:
                self._cf = False
            self._ax = (self._ax & 0xFF00) | al
            self._sf, self._zf, self._pf = compute_szp(al, word=False)
            return "DAS"

        if op == 0x37:   # AAA
            al = self._ax & _BYTE_MASK
            if (al & 0xF) > 9 or self._af:
                al = (al + 6) & _BYTE_MASK
                ah = ((self._ax >> 8) + 1) & _BYTE_MASK
                self._af = True; self._cf = True
            else:
                ah = (self._ax >> 8) & _BYTE_MASK
                self._af = False; self._cf = False
            self._ax = (ah << 8) | (al & 0xF)
            return "AAA"

        if op == 0x3F:   # AAS
            al = self._ax & _BYTE_MASK
            if (al & 0xF) > 9 or self._af:
                al = (al - 6) & _BYTE_MASK
                ah = ((self._ax >> 8) - 1) & _BYTE_MASK
                self._af = True; self._cf = True
            else:
                ah = (self._ax >> 8) & _BYTE_MASK
                self._af = False; self._cf = False
            self._ax = (ah << 8) | (al & 0xF)
            return "AAS"

        if op == 0xD4:   # AAM
            base = self._fetch8()
            al = self._ax & _BYTE_MASK
            ah = al // base
            al = al % base
            self._ax = (ah << 8) | al
            self._sf, self._zf, self._pf = compute_szp(al, word=False)
            return "AAM"

        if op == 0xD5:   # AAD
            base = self._fetch8()
            al = self._ax & _BYTE_MASK
            ah = (self._ax >> 8) & _BYTE_MASK
            result = (ah * base + al) & _BYTE_MASK
            self._ax = result  # AH=0, AL=result
            self._sf, self._zf, self._pf = compute_szp(result, word=False)
            return "AAD"

        # ── Shifts / rotates  ──────────────────────────────────────────
        # D0: shift/rot r/m8, 1
        # D1: shift/rot r/m16, 1
        # D2: shift/rot r/m8, CL
        # D3: shift/rot r/m16, CL
        if op in (0xD0, 0xD1, 0xD2, 0xD3):
            word = bool(op & 1)
            count = 1 if op < 0xD2 else (self._cx & _BYTE_MASK)
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3; ext = (modrm >> 3) & 7; rm = modrm & 7
            seg, ea, _ = self._decode_modrm(modrm, word, seg_override)
            a = self._read_rm(mod, rm, seg, ea, word)
            _shift_ops = {0: "ROL", 1: "ROR", 2: "RCL", 3: "RCR",
                          4: "SHL", 5: "SHR", 6: "SHL", 7: "SAR"}
            match ext:
                case 0: result = self._rol(a, count, word=word)
                case 1: result = self._ror(a, count, word=word)
                case 2: result = self._rcl(a, count, word=word)
                case 3: result = self._rcr(a, count, word=word)
                case 4 | 6: result = self._shl(a, count, word=word)
                case 5: result = self._shr(a, count, word=word)
                case _: result = self._sar(a, count, word=word)  # 7
            self._write_rm(mod, rm, seg, ea, result, word)
            return f"{_shift_ops[ext]} m,{'1' if op < 0xD2 else 'CL'}"

        # ── Control flow  ──────────────────────────────────────────────

        # JMP short (EB)
        if op == 0xEB:
            disp = self._fetch_s8()
            self._ip = (self._ip + disp) & _WORD_MASK
            return f"JMP SHORT {disp:+d}"

        # JMP near (E9)
        if op == 0xE9:
            disp = self._fetch_s16()
            self._ip = (self._ip + disp) & _WORD_MASK
            return f"JMP NEAR {disp:+d}"

        # JMP far (EA)
        if op == 0xEA:
            new_ip = self._fetch16()
            new_cs = self._fetch16()
            self._ip = new_ip; self._cs = new_cs
            return f"JMP FAR {new_cs:04X}:{new_ip:04X}"

        # CALL near (E8)
        if op == 0xE8:
            disp = self._fetch_s16()
            self._push16(self._ip)
            self._ip = (self._ip + disp) & _WORD_MASK
            return f"CALL NEAR {disp:+d}"

        # CALL far (9A)
        if op == 0x9A:
            new_ip = self._fetch16()
            new_cs = self._fetch16()
            self._push16(self._cs)
            self._push16(self._ip)
            self._ip = new_ip; self._cs = new_cs
            return f"CALL FAR {new_cs:04X}:{new_ip:04X}"

        # RET near (C3 / C2)
        if op == 0xC3:
            self._ip = self._pop16()
            return "RET"
        if op == 0xC2:
            n = self._fetch16()
            self._ip = self._pop16()
            self._sp = (self._sp + n) & _WORD_MASK
            return f"RET {n}"

        # RETF (CB / CA)
        if op == 0xCB:
            self._ip = self._pop16()
            self._cs = self._pop16()
            return "RETF"
        if op == 0xCA:
            n = self._fetch16()
            self._ip = self._pop16()
            self._cs = self._pop16()
            self._sp = (self._sp + n) & _WORD_MASK
            return f"RETF {n}"

        # Conditional jumps (70-7F)
        if 0x70 <= op <= 0x7F:
            disp = self._fetch_s8()
            cond = self._eval_cond(op - 0x70)
            if cond:
                self._ip = (self._ip + disp) & _WORD_MASK
            _jcc_names = ["JO", "JNO", "JB", "JNB", "JZ", "JNZ", "JBE", "JA",
                          "JS", "JNS", "JP", "JNP", "JL", "JGE", "JLE", "JG"]
            return f"{_jcc_names[op - 0x70]} {disp:+d}"

        # LOOP / LOOPE / LOOPNE / JCXZ (E0-E3)
        if op in (0xE0, 0xE1, 0xE2, 0xE3):
            disp = self._fetch_s8()
            self._cx = (self._cx - 1) & _WORD_MASK if op != 0xE3 else self._cx
            if op == 0xE2:
                taken = self._cx != 0
            elif op == 0xE1:
                taken = self._cx != 0 and self._zf
            elif op == 0xE0:
                taken = self._cx != 0 and not self._zf
            else:   # JCXZ — no CX decrement; restore
                self._cx = (self._cx + 1) & _WORD_MASK if op != 0xE3 else self._cx
                taken = self._cx == 0
            if taken:
                self._ip = (self._ip + disp) & _WORD_MASK
            _loop_names = {0xE0: "LOOPNE", 0xE1: "LOOPE", 0xE2: "LOOP", 0xE3: "JCXZ"}
            return f"{_loop_names[op]} {disp:+d}"

        # INT n (CD n) / INT 3 (CC) / INTO (CE) — treat as HLT
        if op in (0xCC, 0xCE):
            self._halted = True
            return "INT"
        if op == 0xCD:
            _ = self._fetch8()  # consume interrupt number
            self._halted = True
            return "INT n"

        # IRET (CF)
        if op == 0xCF:
            self._ip = self._pop16()
            self._cs = self._pop16()
            self._load_flags(self._pop16())
            return "IRET"

        # ── String operations ──────────────────────────────────────────

        # MOVS (A4/A5) / CMPS (A6/A7) / SCAS (AE/AF) / LODS (AC/AD) / STOS (AA/AB)
        if op in (0xA4, 0xA5, 0xA6, 0xA7, 0xAE, 0xAF, 0xAC, 0xAD, 0xAA, 0xAB):
            word = bool(op & 1)
            step = self._str_step(word)
            seg_src = seg_override if seg_override is not None else self._ds

            if op in (0xAC, 0xAD):   # LODS
                return self._exec_string_lods(word, step, seg_src, rep_prefix)
            if op in (0xAA, 0xAB):   # STOS
                return self._exec_string_stos(word, step, rep_prefix)
            if op in (0xA4, 0xA5):   # MOVS
                return self._exec_string_movs(word, step, seg_src, rep_prefix)
            if op in (0xA6, 0xA7):   # CMPS
                return self._exec_string_cmps(word, step, seg_src, rep_prefix)
            # SCAS (AE/AF)
            return self._exec_string_scas(word, step, rep_prefix)

        # ── Miscellaneous ──────────────────────────────────────────────

        # NOP (handled above as XCHG AX,AX = 0x90)

        # HLT (F4)
        if op == 0xF4:
            self._halted = True
            return "HLT"

        # CLC/STC/CMC
        if op == 0xF8: self._cf = False; return "CLC"
        if op == 0xF9: self._cf = True;  return "STC"
        if op == 0xF5: self._cf = not self._cf; return "CMC"

        # CLD/STD
        if op == 0xFC: self._df = False; return "CLD"
        if op == 0xFD: self._df = True;  return "STD"

        # CLI/STI
        if op == 0xFA: self._if = False; return "CLI"
        if op == 0xFB: self._if = True;  return "STI"

        # IN AL/AX, imm8  (E4/E5)
        if op == 0xE4:
            port = self._fetch8()
            self._ax = (self._ax & 0xFF00) | self._input_ports[port]
            return f"IN AL,{port:#x}"
        if op == 0xE5:
            port = self._fetch8()
            lo = self._input_ports[port]
            hi = self._input_ports[(port + 1) & _BYTE_MASK]
            self._ax = lo | (hi << 8)
            return f"IN AX,{port:#x}"

        # IN AL/AX, DX  (EC/ED)
        if op == 0xEC:
            port = self._dx & _BYTE_MASK
            self._ax = (self._ax & 0xFF00) | self._input_ports[port]
            return "IN AL,DX"
        if op == 0xED:
            port = self._dx & _BYTE_MASK
            lo = self._input_ports[port]
            hi = self._input_ports[(port + 1) & _BYTE_MASK]
            self._ax = lo | (hi << 8)
            return "IN AX,DX"

        # OUT imm8, AL/AX  (E6/E7)
        if op == 0xE6:
            port = self._fetch8()
            self._output_ports[port] = self._ax & _BYTE_MASK
            return f"OUT {port:#x},AL"
        if op == 0xE7:
            port = self._fetch8()
            self._output_ports[port] = self._ax & _BYTE_MASK
            self._output_ports[(port + 1) & _BYTE_MASK] = (self._ax >> 8) & _BYTE_MASK
            return f"OUT {port:#x},AX"

        # OUT DX, AL/AX  (EE/EF)
        if op == 0xEE:
            port = self._dx & _BYTE_MASK
            self._output_ports[port] = self._ax & _BYTE_MASK
            return "OUT DX,AL"
        if op == 0xEF:
            port = self._dx & _BYTE_MASK
            self._output_ports[port] = self._ax & _BYTE_MASK
            self._output_ports[(port + 1) & _BYTE_MASK] = (self._ax >> 8) & _BYTE_MASK
            return "OUT DX,AX"

        # WAIT (9B) / LOCK (F0) — already consumed as prefix, but handle standalone
        if op == 0x9B:
            return "WAIT"

        # Unknown opcode — treat as HLT
        self._halted = True
        return f"DB {op:#04x}"

    # ------------------------------------------------------------------
    # Internal: ALU operation dispatcher
    # ------------------------------------------------------------------

    def _alu_op(self, op: int, a: int, b: int, word: bool) -> tuple[int, str]:
        """Perform one of 8 ALU operations; return (result, mnemonic)."""
        match op:
            case 0: return self._add(a, b, word=word), "ADD"
            case 1: return self._or(a, b, word=word), "OR"
            case 2:
                r = self._add(a, b, word=word, carry=int(self._cf))
                return r, "ADC"
            case 3:
                r = self._sub(a, b, word=word, borrow=int(self._cf))
                return r, "SBB"
            case 4: return self._and(a, b, word=word), "AND"
            case 5: return self._sub(a, b, word=word), "SUB"
            case 6: return self._xor(a, b, word=word), "XOR"
            case _:  # 7 = CMP
                self._sub(a, b, word=word)   # flags only
                return a, "CMP"

    # ------------------------------------------------------------------
    # Internal: conditional jump evaluation
    # ------------------------------------------------------------------

    def _eval_cond(self, cond: int) -> bool:
        """Evaluate Jcc condition code (0–15)."""
        match cond:
            case 0:  return self._of                          # JO
            case 1:  return not self._of                      # JNO
            case 2:  return self._cf                          # JB/JC
            case 3:  return not self._cf                      # JNB/JNC
            case 4:  return self._zf                          # JZ/JE
            case 5:  return not self._zf                      # JNZ/JNE
            case 6:  return self._cf or self._zf              # JBE
            case 7:  return not self._cf and not self._zf     # JA
            case 8:  return self._sf                          # JS
            case 9:  return not self._sf                      # JNS
            case 10: return self._pf                          # JP
            case 11: return not self._pf                      # JNP
            case 12: return self._sf != self._of              # JL
            case 13: return self._sf == self._of              # JGE
            case 14: return self._zf or (self._sf != self._of)  # JLE
            case _:  return not self._zf and (self._sf == self._of)  # JG (15)

    # ------------------------------------------------------------------
    # Internal: FLAGS helpers
    # ------------------------------------------------------------------

    def _flags_val(self) -> int:
        """Return packed 16-bit FLAGS value."""
        return (
            (int(self._cf) << 0) | (1 << 1) | (int(self._pf) << 2)
            | (int(self._af) << 4) | (int(self._zf) << 6) | (int(self._sf) << 7)
            | (int(self._tf) << 8) | (int(self._if) << 9) | (int(self._df) << 10)
            | (int(self._of) << 11)
        )

    def _flags_low8(self) -> int:
        """Return low 8 bits of FLAGS (for LAHF)."""
        return (
            (int(self._cf) << 0) | (1 << 1) | (int(self._pf) << 2)
            | (int(self._af) << 4) | (int(self._zf) << 6) | (int(self._sf) << 7)
        )

    def _load_flags(self, f: int) -> None:
        d = unpack_flags(f)
        self._cf = d["cf"]; self._pf = d["pf"]; self._af = d["af"]
        self._zf = d["zf"]; self._sf = d["sf"]; self._tf = d["tf"]
        self._if = d["if_"]; self._df = d["df"]; self._of = d["of"]

    def _load_flags_low8(self, f: int) -> None:
        """Load CF/PF/AF/ZF/SF from low byte (SAHF)."""
        self._cf = bool(f & 1)
        self._pf = bool(f & 4)
        self._af = bool(f & 16)
        self._zf = bool(f & 64)
        self._sf = bool(f & 128)

    # ------------------------------------------------------------------
    # Internal: string operation helpers
    # ------------------------------------------------------------------

    def _exec_string_lods(
        self, word: bool, step: int, seg_src: int, rep: int | None
    ) -> str:
        count = self._cx if rep else 1
        for _ in range(count):
            if word:
                self._ax = self._read_word(seg_src, self._si)
            else:
                self._ax = (self._ax & 0xFF00) | self._read_byte(seg_src, self._si)
            self._si = (self._si + step) & _WORD_MASK
            if rep:
                self._cx = (self._cx - 1) & _WORD_MASK
                if self._cx == 0:
                    break
        return "LODS"

    def _exec_string_stos(self, word: bool, step: int, rep: int | None) -> str:
        count = self._cx if rep else 1
        for _ in range(count):
            if word:
                self._write_word(self._es, self._di, self._ax)
            else:
                self._write_byte(self._es, self._di, self._ax & _BYTE_MASK)
            self._di = (self._di + step) & _WORD_MASK
            if rep:
                self._cx = (self._cx - 1) & _WORD_MASK
                if self._cx == 0:
                    break
        return "STOS"

    def _exec_string_movs(
        self, word: bool, step: int, seg_src: int, rep: int | None
    ) -> str:
        count = self._cx if rep else 1
        for _ in range(count):
            if word:
                val = self._read_word(seg_src, self._si)
                self._write_word(self._es, self._di, val)
            else:
                val = self._read_byte(seg_src, self._si)
                self._write_byte(self._es, self._di, val)
            self._si = (self._si + step) & _WORD_MASK
            self._di = (self._di + step) & _WORD_MASK
            if rep:
                self._cx = (self._cx - 1) & _WORD_MASK
                if self._cx == 0:
                    break
        return "MOVS"

    def _exec_string_cmps(
        self, word: bool, step: int, seg_src: int, rep: int | None
    ) -> str:
        count = self._cx if rep else 1
        for _ in range(count):
            a = (self._read_word(seg_src, self._si) if word
                 else self._read_byte(seg_src, self._si))
            b = (self._read_word(self._es, self._di) if word
                 else self._read_byte(self._es, self._di))
            self._sub(a, b, word=word)
            self._si = (self._si + step) & _WORD_MASK
            self._di = (self._di + step) & _WORD_MASK
            if rep:
                self._cx = (self._cx - 1) & _WORD_MASK
                if self._cx == 0:
                    break
                if rep == 0xF3 and not self._zf:  # REPE: stop when ZF=0
                    break
                if rep == 0xF2 and self._zf:       # REPNE: stop when ZF=1
                    break
        return "CMPS"

    def _exec_string_scas(self, word: bool, step: int, rep: int | None) -> str:
        count = self._cx if rep else 1
        for _ in range(count):
            b = (self._read_word(self._es, self._di) if word
                 else self._read_byte(self._es, self._di))
            a = self._ax if word else (self._ax & _BYTE_MASK)
            self._sub(a, b, word=word)
            self._di = (self._di + step) & _WORD_MASK
            if rep:
                self._cx = (self._cx - 1) & _WORD_MASK
                if self._cx == 0:
                    break
                if rep == 0xF3 and not self._zf:
                    break
                if rep == 0xF2 and self._zf:
                    break
        return "SCAS"
