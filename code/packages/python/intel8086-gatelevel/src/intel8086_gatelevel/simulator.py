"""Intel 8086 gate-level simulator.

=== Design philosophy ===

Every arithmetic and logical operation on data routes through:
  - AND, OR, XOR, NOT (logic_gates package)
  - ripple_carry_adder → via full_adder chains (arithmetic package)
  - add_8bit, add_16bit, add_20bit (bits module, wrapping ripple_carry_adder)

No Python integer arithmetic (+, -, &, |, ^) appears on the core data path.
The only exceptions are:
  1. MUL/DIV — host arithmetic used (gate-level multiplier out of scope)
  2. IP/SP address wrapping — modulo 0x10000 address bus operations
  3. Segment left-shift (seg × 16 = seg << 4) — wiring, not computation

=== Intel 8086 overview ===

Announced June 1978.  16-bit CPU.  Segmented 20-bit address space (1 MB).
~29,000 transistors.  Parent architecture of every x86 CPU today.

=== Memory model ===

Flat 1 MB bytearray.  Physical address = (seg × 16 + offset) & 0xFFFFF.
All memory accesses use _read_byte/_write_byte / _read_word/_write_word.

=== Interrupts ===

INT n: push FLAGS, CS, IP onto stack
jump through IVT at n × 4.
IRET: pop IP, CS, FLAGS from stack.

=== Halt ===

HLT opcode (0xF4) sets halted=True.  Calling step() when halted raises
RuntimeError.  execute() stops at HLT or max_steps.
"""

from __future__ import annotations

from intel_8086_simulator.state import X86State
from simulator_protocol import ExecutionResult, Simulator, StepTrace

from intel8086_gatelevel.alu import (
    ALUResult8086,
    aaa,
    aad,
    aam,
    aas,
    add8,
    add16,
    and8,
    and16,
    daa,
    das,
    dec8,
    dec16,
    div8,
    div16,
    idiv8,
    idiv16,
    imul8,
    imul16,
    inc8,
    inc16,
    mul8,
    mul16,
    neg8,
    neg16,
    not8,
    not16,
    or8,
    or16,
    rcl,
    rcr,
    rol,
    ror,
    sar,
    shl,
    shr,
    sub8,
    sub16,
    xor8,
    xor16,
)
from intel8086_gatelevel.bits import add_16bit, int_to_bits, invert_16bit
from intel8086_gatelevel.register_file import RegisterFile8086

_MEM_SIZE = 1_048_576
_PORT_SIZE = 256
_BYTE_MASK = 0xFF
_WORD_MASK = 0xFFFF
_PHYS_MASK = 0xFFFFF


class Intel8086GateLevelSimulator(Simulator[X86State]):
    """Gate-level simulator for the Intel 8086 (1978).

    All data-path operations route through logic gate primitives.
    Implements ``Simulator[X86State]`` (SIM00 protocol).

    Usage::

        sim = Intel8086GateLevelSimulator()
        result = sim.execute(bytes([
            0xB8, 0x0A, 0x00,   # MOV AX, 10
            0xF4,               # HLT
        ]))
        assert result.final_state.ax == 10
    """

    def __init__(self) -> None:
        self._mem: bytearray = bytearray(_MEM_SIZE)
        self._rf = RegisterFile8086()
        self._halted: bool = False
        self._input_ports: bytearray = bytearray(_PORT_SIZE)
        self._output_ports: bytearray = bytearray(_PORT_SIZE)

    # ── Protocol interface ────────────────────────────────────────────────────

    def reset(self) -> None:
        """Reset to power-on state: all registers/flags/memory zeroed."""
        self._mem = bytearray(_MEM_SIZE)
        self._rf = RegisterFile8086()
        self._halted = False
        self._input_ports = bytearray(_PORT_SIZE)
        self._output_ports = bytearray(_PORT_SIZE)

    def load(self, program: bytes, origin: int = 0) -> None:
        """Write program bytes into memory at physical address origin.

        Args:
            program: Raw machine-code bytes.
            origin:  Physical address (0–0xFFFFF) where writing begins.
        """
        end = min(origin + len(program), _MEM_SIZE)
        self._mem[origin:end] = program[:end - origin]

    def step(self) -> StepTrace:
        """Execute one fetch-decode-execute cycle.

        Returns:
            StepTrace with pc_before, pc_after, mnemonic, description.

        Raises:
            RuntimeError: If the simulator is halted.
        """
        if self._halted:
            raise RuntimeError(
                "Intel8086GateLevelSimulator is halted; call reset() to restart"
            )
        ip_before = self._rf.read16("ip")
        mnemonic = self._fetch_decode_execute()
        ip_after = self._rf.read16("ip")
        cs = self._rf.read16("cs")
        return StepTrace(
            pc_before=ip_before,
            pc_after=ip_after,
            mnemonic=mnemonic,
            description=f"{mnemonic} @ CS:IP={cs:04X}:{ip_before:04X}",
        )

    def execute(
        self, program: bytes, max_steps: int = 10_000
    ) -> ExecutionResult[X86State]:
        """Reset, load, run to HLT or max_steps; return full result.

        Args:
            program:   Raw machine-code bytes loaded at physical address 0.
            max_steps: Safety ceiling to prevent infinite loops.
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
        """Return an immutable X86State snapshot."""
        rf = self._rf
        return X86State(
            ax=rf.read16("ax"), bx=rf.read16("bx"),
            cx=rf.read16("cx"), dx=rf.read16("dx"),
            si=rf.read16("si"), di=rf.read16("di"),
            sp=rf.read16("sp"), bp=rf.read16("bp"),
            cs=rf.read16("cs"), ds=rf.read16("ds"),
            ss=rf.read16("ss"), es=rf.read16("es"),
            ip=rf.read16("ip"),
            cf=bool(rf._flag_cf), pf=bool(rf._flag_pf),
            af=bool(rf._flag_af), zf=bool(rf._flag_zf),
            sf=bool(rf._flag_sf), tf=bool(rf._flag_tf),
            if_=bool(rf._flag_if), df=bool(rf._flag_df),
            of=bool(rf._flag_of),
            halted=self._halted,
            input_ports=tuple(self._input_ports),
            output_ports=tuple(self._output_ports),
            memory=tuple(self._mem),
        )

    def set_input_port(self, port: int, value: int) -> None:
        """Set an I/O input port value."""
        self._input_ports[port & _BYTE_MASK] = value & _BYTE_MASK

    def get_output_port(self, port: int) -> int:
        """Read an I/O output port value."""
        return self._output_ports[port & _BYTE_MASK]

    def interrupt(self, vector: int) -> None:
        """Trigger a hardware interrupt (INT vector).

        Pushes FLAGS, CS, IP onto stack then loads new CS:IP from IVT.
        """
        self._trigger_interrupt(vector & _BYTE_MASK)

    def nmi(self) -> None:
        """Trigger a non-maskable interrupt (INT 2)."""
        self._trigger_interrupt(2)

    # ── Memory helpers ────────────────────────────────────────────────────────

    def _phys(self, seg: int, offset: int) -> int:
        """Compute 20-bit physical address from segment and offset."""
        return ((seg << 4) + (offset & _WORD_MASK)) & _PHYS_MASK

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
        self._mem[self._phys(seg, (offset + 1) & _WORD_MASK)] = (value >> 8) & _BYTE_MASK

    def _fetch8(self) -> int:
        """Read one byte from CS:IP and advance IP."""
        ip = self._rf.read16("ip")
        cs = self._rf.read16("cs")
        v = self._mem[self._phys(cs, ip)]
        new_ip, _, _ = add_16bit(ip, 1, 0)
        self._rf.write16("ip", new_ip)
        return v

    def _fetch16(self) -> int:
        lo = self._fetch8()
        hi = self._fetch8()
        return lo | (hi << 8)

    def _fetch_s8(self) -> int:
        v = self._fetch8()
        return v if v < 0x80 else v - 0x100

    def _fetch_s16(self) -> int:
        v = self._fetch16()
        return v if v < 0x8000 else v - 0x10000

    # ── Register access (by ModRM index) ─────────────────────────────────────

    _REG16_NAMES = ["ax", "cx", "dx", "bx", "sp", "bp", "si", "di"]
    _REG8_NAMES_LOW = ["ax", "cx", "dx", "bx"]   # AL/CL/DL/BL
    _SREG_NAMES = ["es", "cs", "ss", "ds"]

    def _get_reg16(self, reg: int) -> int:
        return self._rf.read16(self._REG16_NAMES[reg])

    def _set_reg16(self, reg: int, val: int) -> None:
        self._rf.write16(self._REG16_NAMES[reg], val & _WORD_MASK)

    def _get_reg8(self, reg: int) -> int:
        """AL=0, CL=1, DL=2, BL=3, AH=4, CH=5, DH=6, BH=7."""
        name = ["ax", "cx", "dx", "bx", "ax", "cx", "dx", "bx"][reg]
        if reg < 4:
            return self._rf.read8_low(name)
        return self._rf.read8_high(name)

    def _set_reg8(self, reg: int, val: int) -> None:
        val &= _BYTE_MASK
        name = ["ax", "cx", "dx", "bx", "ax", "cx", "dx", "bx"][reg]
        if reg < 4:
            self._rf.write8_low(name, val)
        else:
            self._rf.write8_high(name, val)

    def _get_sreg(self, reg: int) -> int:
        return self._rf.read16(self._SREG_NAMES[reg & 3])

    def _set_sreg(self, reg: int, val: int) -> None:
        self._rf.write16(self._SREG_NAMES[reg & 3], val & _WORD_MASK)

    # ── ModRM decode ──────────────────────────────────────────────────────────

    def _decode_modrm(
        self, modrm: int, word: bool, seg_override: int | None
    ) -> tuple[int, int, int, int]:
        """Decode ModRM byte.  Returns (mod, reg, rm, ea) where ea is the
        effective address offset.  For mod=11, ea = rm (register index).
        """
        mod = (modrm >> 6) & 3
        reg = (modrm >> 3) & 7
        rm = modrm & 7

        if mod == 3:
            return mod, reg, rm, rm   # Register mode: ea = register index

        rf = self._rf
        bx = rf.read16("bx")
        si = rf.read16("si")
        di = rf.read16("di")
        bp = rf.read16("bp")

        if rm == 0:
            base, _, _ = add_16bit(bx, si, 0)
            ea = base & _WORD_MASK
        elif rm == 1:
            base, _, _ = add_16bit(bx, di, 0)
            ea = base & _WORD_MASK
        elif rm == 2:
            base, _, _ = add_16bit(bp, si, 0)
            ea = base & _WORD_MASK
        elif rm == 3:
            base, _, _ = add_16bit(bp, di, 0)
            ea = base & _WORD_MASK
        elif rm == 4:
            ea = si
        elif rm == 5:
            ea = di
        elif rm == 6:
            ea = self._fetch16() if mod == 0 else bp
        else:  # rm == 7
            ea = bx

        if mod == 1:
            disp = self._fetch_s8()
            ea, _, _ = add_16bit(ea, disp & _WORD_MASK if disp >= 0 else (disp + 0x10000) & _WORD_MASK, 0)
            ea &= _WORD_MASK
        elif mod == 2:
            disp = self._fetch_s16()
            d_word = disp & _WORD_MASK if disp >= 0 else (disp + 0x10000) & _WORD_MASK
            ea, _, _ = add_16bit(ea, d_word, 0)
            ea &= _WORD_MASK

        return mod, reg, rm, ea

    def _effective_seg(
        self, rm: int, mod: int, seg_override: int | None
    ) -> int:
        """Return the effective segment register value for a memory access.

        Default: BP-based → SS
        otherwise → DS.
        """
        if seg_override is not None:
            return self._get_sreg(seg_override)
        uses_bp = rm in (2, 3) or (rm == 6 and mod != 0)
        if uses_bp:
            return self._rf.read16("ss")
        return self._rf.read16("ds")

    def _read_rm(
        self, mod: int, rm: int, seg: int, ea: int, word: bool
    ) -> int:
        """Read value from r/m operand (register or memory)."""
        if mod == 3:
            return self._get_reg16(rm) if word else self._get_reg8(rm)
        return self._read_word(seg, ea) if word else self._read_byte(seg, ea)

    def _write_rm(
        self, mod: int, rm: int, seg: int, ea: int, val: int, word: bool
    ) -> None:
        """Write value to r/m operand."""
        if mod == 3:
            if word:
                self._set_reg16(rm, val)
            else:
                self._set_reg8(rm, val)
        elif word:
            self._write_word(seg, ea, val)
        else:
            self._write_byte(seg, ea, val)

    # ── Stack helpers ─────────────────────────────────────────────────────────

    def _push16(self, val: int) -> None:
        sp = self._rf.read16("sp")
        new_sp, _, _ = add_16bit(sp, invert_16bit(1), 1)  # sp - 2
        new_sp2, _, _ = add_16bit(new_sp, invert_16bit(1), 1)
        new_sp2 &= _WORD_MASK
        self._rf.write16("sp", new_sp2)
        self._write_word(self._rf.read16("ss"), new_sp2, val)

    def _pop16(self) -> int:
        sp = self._rf.read16("sp")
        val = self._read_word(self._rf.read16("ss"), sp)
        new_sp, _, _ = add_16bit(sp, 2, 0)
        self._rf.write16("sp", new_sp & _WORD_MASK)
        return val

    # ── FLAGS helpers ─────────────────────────────────────────────────────────

    def _flags_val(self) -> int:
        return self._rf.pack_flags()

    def _load_flags(self, f: int) -> None:
        self._rf.unpack_flags(f)

    def _load_flags_low8(self, f: int) -> None:
        """Load CF/PF/AF/ZF/SF from low byte (SAHF)."""
        self._rf._flag_cf = (f >> 0) & 1
        self._rf._flag_pf = (f >> 2) & 1
        self._rf._flag_af = (f >> 4) & 1
        self._rf._flag_zf = (f >> 6) & 1
        self._rf._flag_sf = (f >> 7) & 1

    def _flags_low8(self) -> int:
        """Return low 8 bits of FLAGS (for LAHF)."""
        rf = self._rf
        return (
            (rf._flag_cf << 0) | (1 << 1) | (rf._flag_pf << 2)
            | (rf._flag_af << 4) | (rf._flag_zf << 6) | (rf._flag_sf << 7)
        )

    def _apply_alu_result(self, r: ALUResult8086, word: bool) -> None:
        """Apply all ALU flags from an ALUResult8086."""
        self._rf._flag_cf = r.flag_cf
        self._rf._flag_of = r.flag_of
        self._rf._flag_sf = r.flag_sf
        self._rf._flag_zf = r.flag_zf
        self._rf._flag_af = r.flag_af
        self._rf._flag_pf = r.flag_pf

    def _apply_alu_no_cf(self, r: ALUResult8086) -> None:
        """Apply ALU flags except CF (for INC/DEC)."""
        self._rf._flag_of = r.flag_of
        self._rf._flag_sf = r.flag_sf
        self._rf._flag_zf = r.flag_zf
        self._rf._flag_af = r.flag_af
        self._rf._flag_pf = r.flag_pf

    def _apply_logic_flags(self, r: ALUResult8086) -> None:
        """Apply ALU flags for logic ops: CF=0, OF=0, AF=0."""
        self._rf._flag_cf = 0
        self._rf._flag_of = 0
        self._rf._flag_af = 0
        self._rf._flag_sf = r.flag_sf
        self._rf._flag_zf = r.flag_zf
        self._rf._flag_pf = r.flag_pf

    def _set_szp_byte(self, val: int) -> None:
        """Set SF/ZF/PF from an 8-bit value."""
        bits = int_to_bits(val & _BYTE_MASK, 8)
        from intel8086_gatelevel.bits import compute_parity, compute_zero
        self._rf._flag_sf = bits[7]
        self._rf._flag_zf = compute_zero(bits)
        self._rf._flag_pf = compute_parity(bits)

    # ── String operation helpers ──────────────────────────────────────────────

    def _str_step(self, word: bool) -> int:
        """Return +1 or +2 (DF=0) or -1 or -2 (DF=1)."""
        inc = 2 if word else 1
        return -inc if self._rf._flag_df else inc

    # ── ALU dispatcher ────────────────────────────────────────────────────────

    def _alu_op(self, op: int, a: int, b: int, word: bool) -> tuple[int, str]:
        """Perform one of 8 ALU operations via gate-level functions.

        op: 0=ADD 1=OR 2=ADC 3=SBB 4=AND 5=SUB 6=XOR 7=CMP
        Returns (result, mnemonic).
        """
        cf = self._rf._flag_cf
        _names = ["ADD", "OR", "ADC", "SBB", "AND", "SUB", "XOR", "CMP"]
        if op == 0:
            r = add16(a, b, 0) if word else add8(a, b, 0)
            self._apply_alu_result(r, word)
            return r.result, "ADD"
        if op == 1:
            r = or16(a, b) if word else or8(a, b)
            self._apply_logic_flags(r)
            return r.result, "OR"
        if op == 2:  # ADC
            r = add16(a, b, cf) if word else add8(a, b, cf)
            self._apply_alu_result(r, word)
            return r.result, "ADC"
        if op == 3:  # SBB
            r = sub16(a, b, cf) if word else sub8(a, b, cf)
            self._apply_alu_result(r, word)
            return r.result, "SBB"
        if op == 4:  # AND
            r = and16(a, b) if word else and8(a, b)
            self._apply_logic_flags(r)
            return r.result, "AND"
        if op == 5:  # SUB
            r = sub16(a, b, 0) if word else sub8(a, b, 0)
            self._apply_alu_result(r, word)
            return r.result, "SUB"
        if op == 6:  # XOR
            r = xor16(a, b) if word else xor8(a, b)
            self._apply_logic_flags(r)
            return r.result, "XOR"
        # op == 7: CMP (flags only, result discarded)
        r = sub16(a, b, 0) if word else sub8(a, b, 0)
        self._apply_alu_result(r, word)
        return a, "CMP"

    # ── Conditional jump evaluation ───────────────────────────────────────────

    def _eval_cond(self, cond: int) -> bool:
        """Evaluate Jcc condition code (0–15)."""
        rf = self._rf
        cf = bool(rf._flag_cf)
        of = bool(rf._flag_of)
        sf = bool(rf._flag_sf)
        zf = bool(rf._flag_zf)
        pf = bool(rf._flag_pf)
        match cond:
            case 0:
                return of
            case 1:
                return not of
            case 2:
                return cf
            case 3:
                return not cf
            case 4:
                return zf
            case 5:
                return not zf
            case 6:
                return cf or zf
            case 7:
                return not cf and not zf
            case 8:
                return sf
            case 9:
                return not sf
            case 10:
                return pf
            case 11:
                return not pf
            case 12:
                return sf != of
            case 13:
                return sf == of
            case 14:
                return zf or (sf != of)
            case _:
                return not zf and (sf == of)

    # ── Interrupt helper ──────────────────────────────────────────────────────

    def _trigger_interrupt(self, vector: int) -> None:
        """Trigger interrupt vector: push FLAGS, CS, IP; load IVT entry."""
        self._push16(self._flags_val())
        self._push16(self._rf.read16("cs"))
        self._push16(self._rf.read16("ip"))
        # Load new CS:IP from IVT at vector × 4
        ivt_addr = vector * 4
        new_ip = self._mem[ivt_addr] | (self._mem[ivt_addr + 1] << 8)
        new_cs = self._mem[ivt_addr + 2] | (self._mem[ivt_addr + 3] << 8)
        self._rf.write16("ip", new_ip)
        self._rf.write16("cs", new_cs)
        self._rf._flag_if = 0
        self._rf._flag_tf = 0

    # ── Main execute dispatcher ───────────────────────────────────────────────

    def _fetch_decode_execute(self) -> str:  # noqa: C901
        """Decode and execute the instruction at CS:IP.  Returns mnemonic."""
        seg_override: int | None = None
        rep_prefix: int | None = None

        # Prefix loop
        while True:
            op = self._fetch8()
            if op == 0x26:
                seg_override = 0   # ES:
            elif op == 0x2E:
                seg_override = 1   # CS:
            elif op == 0x36:
                seg_override = 2   # SS:
            elif op == 0x3E:
                seg_override = 3   # DS:
            elif op in (0xF2, 0xF3):
                rep_prefix = op
            elif op == 0xF0:
                pass  # LOCK — ignored
            else:
                break

        return self._exec_op(op, seg_override, rep_prefix)

    def _exec_op(  # noqa: C901
        self, op: int, seg_override: int | None, rep_prefix: int | None
    ) -> str:
        """Execute decoded opcode.  Returns mnemonic string."""
        rf = self._rf

        # ── MOV instructions ──────────────────────────────────────────────────

        # MOV r/m, reg  or  MOV reg, r/m  (88/89/8A/8B)
        if op in (0x88, 0x89, 0x8A, 0x8B):
            word = bool(op & 1)
            d = bool(op & 2)
            modrm = self._fetch8()
            mod, reg, rm, ea = self._decode_modrm(modrm, word, seg_override)
            seg = self._effective_seg(rm, mod, seg_override)
            if d:   # reg ← r/m
                src = self._read_rm(mod, rm, seg, ea, word)
                if word:
                    self._set_reg16(reg, src)
                else:
                    self._set_reg8(reg, src)
            else:   # r/m ← reg
                src = self._get_reg16(reg) if word else self._get_reg8(reg)
                self._write_rm(mod, rm, seg, ea, src, word)
            return "MOV"

        # MOV r/m8, imm8  (C6)
        if op == 0xC6:
            modrm = self._fetch8()
            mod, reg, rm, ea = self._decode_modrm(modrm, False, seg_override)
            seg = self._effective_seg(rm, mod, seg_override)
            imm = self._fetch8()
            self._write_rm(mod, rm, seg, ea, imm, False)
            return f"MOV m8,{imm:#x}"

        # MOV r/m16, imm16  (C7)
        if op == 0xC7:
            modrm = self._fetch8()
            mod, reg, rm, ea = self._decode_modrm(modrm, True, seg_override)
            seg = self._effective_seg(rm, mod, seg_override)
            imm = self._fetch16()
            self._write_rm(mod, rm, seg, ea, imm, True)
            return f"MOV m16,{imm:#x}"

        # MOV reg8, imm8  (B0–B7)
        if 0xB0 <= op <= 0xB7:
            reg = op - 0xB0
            imm = self._fetch8()
            self._set_reg8(reg, imm)
            return f"MOV r8,{imm:#x}"

        # MOV reg16, imm16  (B8–BF)
        if 0xB8 <= op <= 0xBF:
            reg = op - 0xB8
            imm = self._fetch16()
            self._set_reg16(reg, imm)
            return f"MOV r16,{imm:#x}"

        # MOV AL/AX, [addr]  (A0/A1)
        if op in (0xA0, 0xA1):
            word = bool(op & 1)
            addr = self._fetch16()
            seg = self._get_sreg(seg_override) if seg_override is not None else rf.read16("ds")
            val = self._read_word(seg, addr) if word else self._read_byte(seg, addr)
            if word:
                rf.write16("ax", val)
            else:
                rf.write8_low("ax", val)
            return "MOV AX,m" if word else "MOV AL,m"

        # MOV [addr], AL/AX  (A2/A3)
        if op in (0xA2, 0xA3):
            word = bool(op & 1)
            addr = self._fetch16()
            seg = self._get_sreg(seg_override) if seg_override is not None else rf.read16("ds")
            val = rf.read16("ax") if word else rf.read8_low("ax")
            if word:
                self._write_word(seg, addr, val)
            else:
                self._write_byte(seg, addr, val)
            return "MOV m,AX" if word else "MOV m,AL"

        # MOV r/m, sreg  (8C)
        if op == 0x8C:
            modrm = self._fetch8()
            mod, reg, rm, ea = self._decode_modrm(modrm, True, seg_override)
            seg_r = self._effective_seg(rm, mod, seg_override)
            val = self._get_sreg(reg & 3)
            self._write_rm(mod, rm, seg_r, ea, val, True)
            return f"MOV m,{self._SREG_NAMES[reg & 3].upper()}"

        # MOV sreg, r/m  (8E)
        if op == 0x8E:
            modrm = self._fetch8()
            mod, reg, rm, ea = self._decode_modrm(modrm, True, seg_override)
            seg_r = self._effective_seg(rm, mod, seg_override)
            val = self._read_rm(mod, rm, seg_r, ea, True)
            self._set_sreg(reg & 3, val)
            return f"MOV {self._SREG_NAMES[reg & 3].upper()},m"

        # ── XCHG ─────────────────────────────────────────────────────────────

        # XCHG AX, reg (90–97; 90 = NOP)
        if 0x90 <= op <= 0x97:
            reg = op - 0x90
            if reg == 0:
                return "NOP"
            tmp = rf.read16("ax")
            rf.write16("ax", self._get_reg16(reg))
            self._set_reg16(reg, tmp)
            return "XCHG AX,r16"

        # XCHG r/m, reg (86/87)
        if op in (0x86, 0x87):
            word = bool(op & 1)
            modrm = self._fetch8()
            mod, reg, rm, ea = self._decode_modrm(modrm, word, seg_override)
            seg = self._effective_seg(rm, mod, seg_override)
            a = self._read_rm(mod, rm, seg, ea, word)
            b = self._get_reg16(reg) if word else self._get_reg8(reg)
            self._write_rm(mod, rm, seg, ea, b, word)
            if word:
                self._set_reg16(reg, a)
            else:
                self._set_reg8(reg, a)
            return "XCHG"

        # ── PUSH / POP ────────────────────────────────────────────────────────

        # PUSH reg (50–57)
        if 0x50 <= op <= 0x57:
            self._push16(self._get_reg16(op - 0x50))
            return f"PUSH {self._REG16_NAMES[op - 0x50].upper()}"

        # POP reg (58–5F)
        if 0x58 <= op <= 0x5F:
            self._set_reg16(op - 0x58, self._pop16())
            return f"POP {self._REG16_NAMES[op - 0x58].upper()}"

        # PUSH sreg
        if op in (0x06, 0x0E, 0x16, 0x1E):
            smap = {0x06: 0, 0x0E: 1, 0x16: 2, 0x1E: 3}
            self._push16(self._get_sreg(smap[op]))
            return f"PUSH {self._SREG_NAMES[smap[op]].upper()}"

        # POP sreg
        if op in (0x07, 0x17, 0x1F):
            smap = {0x07: 0, 0x17: 2, 0x1F: 3}
            self._set_sreg(smap[op], self._pop16())
            return f"POP {self._SREG_NAMES[smap[op]].upper()}"

        # POP r/m (8F)
        if op == 0x8F:
            modrm = self._fetch8()
            mod, reg, rm, ea = self._decode_modrm(modrm, True, seg_override)
            seg = self._effective_seg(rm, mod, seg_override)
            self._write_rm(mod, rm, seg, ea, self._pop16(), True)
            return "POP m"

        # PUSHF / POPF
        if op == 0x9C:
            self._push16(self._flags_val())
            return "PUSHF"
        if op == 0x9D:
            self._load_flags(self._pop16())
            return "POPF"

        # ── Load effective address ────────────────────────────────────────────

        # LEA reg, r/m (8D)
        if op == 0x8D:
            modrm = self._fetch8()
            mod, reg, rm, ea = self._decode_modrm(modrm, True, seg_override)
            self._set_reg16(reg, ea & _WORD_MASK)
            return f"LEA {self._REG16_NAMES[reg].upper()},m"

        # LDS reg, m32  (C5)
        if op == 0xC5:
            modrm = self._fetch8()
            mod, reg, rm, ea = self._decode_modrm(modrm, True, seg_override)
            seg_r = self._effective_seg(rm, mod, seg_override)
            off = self._read_word(seg_r, ea)
            new_ds = self._read_word(seg_r, (ea + 2) & _WORD_MASK)
            self._set_reg16(reg, off)
            rf.write16("ds", new_ds)
            return "LDS"

        # LES reg, m32  (C4)
        if op == 0xC4:
            modrm = self._fetch8()
            mod, reg, rm, ea = self._decode_modrm(modrm, True, seg_override)
            seg_r = self._effective_seg(rm, mod, seg_override)
            off = self._read_word(seg_r, ea)
            new_es = self._read_word(seg_r, (ea + 2) & _WORD_MASK)
            self._set_reg16(reg, off)
            rf.write16("es", new_es)
            return "LES"

        # ── LAHF / SAHF ───────────────────────────────────────────────────────

        if op == 0x9F:  # LAHF
            rf.write8_high("ax", self._flags_low8())
            return "LAHF"
        if op == 0x9E:  # SAHF
            self._load_flags_low8(rf.read8_high("ax"))
            return "SAHF"

        # ── CBW / CWD ─────────────────────────────────────────────────────────

        if op == 0x98:  # CBW
            al = rf.read8_low("ax")
            rf.write16("ax", al if al < 0x80 else al | 0xFF00)
            return "CBW"

        if op == 0x99:  # CWD
            rf.write16("dx", 0xFFFF if (rf.read16("ax") & 0x8000) else 0)
            return "CWD"

        # ── XLAT ─────────────────────────────────────────────────────────────

        if op == 0xD7:
            al = rf.read8_low("ax")
            seg = self._get_sreg(seg_override) if seg_override is not None else rf.read16("ds")
            xlat_addr, _, _ = add_16bit(rf.read16("bx"), al, 0)
            rf.write8_low("ax", self._read_byte(seg, xlat_addr & _WORD_MASK))
            return "XLAT"

        # ── 80-group ALU ──────────────────────────────────────────────────────

        if op in (0x80, 0x81, 0x82, 0x83):
            word = op == 0x81 or op == 0x83
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3
            ext = (modrm >> 3) & 7
            rm = modrm & 7
            _, _, _, ea = self._decode_modrm(modrm, word, seg_override)
            seg = self._effective_seg(rm, mod, seg_override)
            if op in (0x80, 0x82):
                imm = self._fetch8()
            elif op == 0x81:
                imm = self._fetch16()
            else:
                v = self._fetch8()
                imm = v if v < 0x80 else v - 0x100
                imm &= _WORD_MASK
            a = self._read_rm(mod, rm, seg, ea, word)
            result, mnem = self._alu_op(ext, a, imm, word)
            if ext != 7:
                self._write_rm(mod, rm, seg, ea, result, word)
            return f"{mnem} m,imm"

        # TEST r/m8, reg (84/85)
        if op in (0x84, 0x85):
            word = bool(op & 1)
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3
            reg = (modrm >> 3) & 7
            rm = modrm & 7
            _, _, _, ea = self._decode_modrm(modrm, word, seg_override)
            seg = self._effective_seg(rm, mod, seg_override)
            a = self._read_rm(mod, rm, seg, ea, word)
            b = self._get_reg16(reg) if word else self._get_reg8(reg)
            r = and16(a, b) if word else and8(a, b)
            self._apply_logic_flags(r)
            return "TEST"

        # Accumulator-imm ALU
        _acc_imm = {
            0x04: (0, False), 0x05: (0, True),
            0x0C: (1, False), 0x0D: (1, True),
            0x14: (2, False), 0x15: (2, True),
            0x1C: (3, False), 0x1D: (3, True),
            0x24: (4, False), 0x25: (4, True),
            0x2C: (5, False), 0x2D: (5, True),
            0x34: (6, False), 0x35: (6, True),
            0x3C: (7, False), 0x3D: (7, True),
            0xA8: (4, False), 0xA9: (4, True),
        }
        if op in _acc_imm:
            alu_op, word = _acc_imm[op]
            imm = self._fetch16() if word else self._fetch8()
            a = rf.read16("ax") if word else rf.read8_low("ax")
            result, mnem = self._alu_op(alu_op, a, imm, word)
            if alu_op != 7 and op not in (0xA8, 0xA9):
                if word:
                    rf.write16("ax", result)
                else:
                    rf.write8_low("ax", result)
            return f"{mnem} AX,imm" if word else f"{mnem} AL,imm"

        # Standard ALU r/m ↔ reg (00–3F pairs)
        _standard_alu = {
            0x00: (0, False, False), 0x01: (0, True, False),
            0x02: (0, False, True),  0x03: (0, True, True),
            0x08: (1, False, False), 0x09: (1, True, False),
            0x0A: (1, False, True),  0x0B: (1, True, True),
            0x10: (2, False, False), 0x11: (2, True, False),
            0x12: (2, False, True),  0x13: (2, True, True),
            0x18: (3, False, False), 0x19: (3, True, False),
            0x1A: (3, False, True),  0x1B: (3, True, True),
            0x20: (4, False, False), 0x21: (4, True, False),
            0x22: (4, False, True),  0x23: (4, True, True),
            0x28: (5, False, False), 0x29: (5, True, False),
            0x2A: (5, False, True),  0x2B: (5, True, True),
            0x30: (6, False, False), 0x31: (6, True, False),
            0x32: (6, False, True),  0x33: (6, True, True),
            0x38: (7, False, False), 0x39: (7, True, False),
            0x3A: (7, False, True),  0x3B: (7, True, True),
        }
        if op in _standard_alu:
            alu_op, word, d_bit = _standard_alu[op]
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3
            reg = (modrm >> 3) & 7
            rm = modrm & 7
            _, _, _, ea = self._decode_modrm(modrm, word, seg_override)
            seg = self._effective_seg(rm, mod, seg_override)
            if d_bit:
                a = self._get_reg16(reg) if word else self._get_reg8(reg)
                b = self._read_rm(mod, rm, seg, ea, word)
                result, mnem = self._alu_op(alu_op, a, b, word)
                if alu_op != 7:
                    if word:
                        self._set_reg16(reg, result)
                    else:
                        self._set_reg8(reg, result)
            else:
                a = self._read_rm(mod, rm, seg, ea, word)
                b = self._get_reg16(reg) if word else self._get_reg8(reg)
                result, mnem = self._alu_op(alu_op, a, b, word)
                if alu_op != 7:
                    self._write_rm(mod, rm, seg, ea, result, word)
            return mnem

        # ── INC / DEC ─────────────────────────────────────────────────────────

        # INC reg16 (40–47)
        if 0x40 <= op <= 0x47:
            reg = op - 0x40
            old_cf = rf._flag_cf
            r = inc16(self._get_reg16(reg))
            self._set_reg16(reg, r.result)
            self._apply_alu_no_cf(r)
            rf._flag_cf = old_cf
            return f"INC {self._REG16_NAMES[reg].upper()}"

        # DEC reg16 (48–4F)
        if 0x48 <= op <= 0x4F:
            reg = op - 0x48
            old_cf = rf._flag_cf
            r = dec16(self._get_reg16(reg))
            self._set_reg16(reg, r.result)
            self._apply_alu_no_cf(r)
            rf._flag_cf = old_cf
            return f"DEC {self._REG16_NAMES[reg].upper()}"

        # FE group: INC/DEC r/m8
        if op == 0xFE:
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3
            ext = (modrm >> 3) & 7
            rm = modrm & 7
            _, _, _, ea = self._decode_modrm(modrm, False, seg_override)
            seg = self._effective_seg(rm, mod, seg_override)
            a = self._read_rm(mod, rm, seg, ea, False)
            old_cf = rf._flag_cf
            if ext == 0:
                r = inc8(a)
                mnem = "INC"
            else:
                r = dec8(a)
                mnem = "DEC"
            self._write_rm(mod, rm, seg, ea, r.result, False)
            self._apply_alu_no_cf(r)
            rf._flag_cf = old_cf
            return f"{mnem} m8"

        # FF group
        if op == 0xFF:
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3
            ext = (modrm >> 3) & 7
            rm = modrm & 7
            _, _, _, ea = self._decode_modrm(modrm, True, seg_override)
            seg = self._effective_seg(rm, mod, seg_override)
            val = self._read_rm(mod, rm, seg, ea, True)
            if ext == 0:
                old_cf = rf._flag_cf
                r = inc16(val)
                self._write_rm(mod, rm, seg, ea, r.result, True)
                self._apply_alu_no_cf(r)
                rf._flag_cf = old_cf
                return "INC m16"
            if ext == 1:
                old_cf = rf._flag_cf
                r = dec16(val)
                self._write_rm(mod, rm, seg, ea, r.result, True)
                self._apply_alu_no_cf(r)
                rf._flag_cf = old_cf
                return "DEC m16"
            if ext == 2:
                self._push16(rf.read16("ip"))
                rf.write16("ip", val)
                return "CALL rm16"
            if ext == 3:
                new_off = self._read_word(seg, ea)
                new_cs = self._read_word(seg, (ea + 2) & _WORD_MASK)
                self._push16(rf.read16("cs"))
                self._push16(rf.read16("ip"))
                rf.write16("cs", new_cs)
                rf.write16("ip", new_off)
                return "CALL FAR m32"
            if ext == 4:
                rf.write16("ip", val)
                return "JMP rm16"
            if ext == 5:
                new_off = self._read_word(seg, ea)
                new_cs = self._read_word(seg, (ea + 2) & _WORD_MASK)
                rf.write16("cs", new_cs)
                rf.write16("ip", new_off)
                return "JMP FAR m32"
            if ext == 6:
                self._push16(val)
                return "PUSH m16"

        # ── F6/F7 group ───────────────────────────────────────────────────────

        if op in (0xF6, 0xF7):
            word = bool(op & 1)
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3
            ext = (modrm >> 3) & 7
            rm = modrm & 7
            _, _, _, ea = self._decode_modrm(modrm, word, seg_override)
            seg = self._effective_seg(rm, mod, seg_override)
            a = self._read_rm(mod, rm, seg, ea, word)

            if ext == 0:   # TEST
                imm = self._fetch16() if word else self._fetch8()
                r = and16(a, imm) if word else and8(a, imm)
                self._apply_logic_flags(r)
                return f"TEST m,{imm:#x}"

            if ext == 2:   # NOT
                result = not16(a) if word else not8(a)
                self._write_rm(mod, rm, seg, ea, result, word)
                return "NOT m"

            if ext == 3:   # NEG
                r = neg16(a) if word else neg8(a)
                self._write_rm(mod, rm, seg, ea, r.result, word)
                self._apply_alu_result(r, word)
                rf._flag_cf = 1 if a != 0 else 0
                return "NEG m"

            if ext == 4:   # MUL
                if word:
                    dx, ax, cf_of = mul16(rf.read16("ax"), a)
                    rf.write16("ax", ax)
                    rf.write16("dx", dx)
                    rf._flag_cf = rf._flag_of = cf_of
                else:
                    ax, cf_of = mul8(rf.read8_low("ax"), a)
                    rf.write16("ax", ax)
                    rf._flag_cf = rf._flag_of = cf_of
                return "MUL m"

            if ext == 5:   # IMUL
                if word:
                    dx, ax, cf_of = imul16(rf.read16("ax"), a)
                    rf.write16("ax", ax)
                    rf.write16("dx", dx)
                    rf._flag_cf = rf._flag_of = cf_of
                else:
                    ax, cf_of = imul8(rf.read8_low("ax"), a)
                    rf.write16("ax", ax)
                    rf._flag_cf = rf._flag_of = cf_of
                return "IMUL m"

            if ext == 6:   # DIV
                try:
                    if word:
                        dividend = (rf.read16("dx") << 16) | rf.read16("ax")
                        ax, dx = div16(dividend, a)
                        rf.write16("ax", ax)
                        rf.write16("dx", dx)
                    else:
                        q, r_val = div8(rf.read16("ax"), a)
                        rf.write16("ax", (r_val << 8) | q)
                except ZeroDivisionError:
                    self._halted = True
                    return "DIV /0"
                return "DIV m"

            if ext == 7:   # IDIV
                try:
                    if word:
                        d32 = (rf.read16("dx") << 16) | rf.read16("ax")
                        ax, dx = idiv16(d32, a)
                        rf.write16("ax", ax)
                        rf.write16("dx", dx)
                    else:
                        q, r_val = idiv8(rf.read16("ax"), a)
                        rf.write16("ax", ((r_val & _BYTE_MASK) << 8) | (q & _BYTE_MASK))
                except ZeroDivisionError:
                    self._halted = True
                    return "IDIV /0"
                return "IDIV m"

        # ── BCD ───────────────────────────────────────────────────────────────

        if op == 0x27:   # DAA
            al = rf.read8_low("ax")
            new_al, new_af, new_cf = daa(al, rf._flag_af, rf._flag_cf)
            rf.write8_low("ax", new_al)
            rf._flag_af = new_af
            rf._flag_cf = new_cf
            self._set_szp_byte(new_al)
            return "DAA"

        if op == 0x2F:   # DAS
            al = rf.read8_low("ax")
            new_al, new_af, new_cf = das(al, rf._flag_af, rf._flag_cf)
            rf.write8_low("ax", new_al)
            rf._flag_af = new_af
            rf._flag_cf = new_cf
            self._set_szp_byte(new_al)
            return "DAS"

        if op == 0x37:   # AAA
            al = rf.read8_low("ax")
            ah = rf.read8_high("ax")
            new_al, new_ah, af_cf = aaa(al, ah, rf._flag_af)
            rf.write8_low("ax", new_al)
            rf.write8_high("ax", new_ah)
            rf._flag_af = af_cf
            rf._flag_cf = af_cf
            return "AAA"

        if op == 0x3F:   # AAS
            al = rf.read8_low("ax")
            ah = rf.read8_high("ax")
            new_al, new_ah, af_cf = aas(al, ah, rf._flag_af)
            rf.write8_low("ax", new_al)
            rf.write8_high("ax", new_ah)
            rf._flag_af = af_cf
            rf._flag_cf = af_cf
            return "AAS"

        if op == 0xD4:   # AAM
            base = self._fetch8()
            al = rf.read8_low("ax")
            new_ah, new_al = aam(al, base)
            rf.write8_high("ax", new_ah)
            rf.write8_low("ax", new_al)
            self._set_szp_byte(new_al)
            return "AAM"

        if op == 0xD5:   # AAD
            base = self._fetch8()
            ah = rf.read8_high("ax")
            al = rf.read8_low("ax")
            new_al = aad(ah, al, base)
            rf.write16("ax", new_al)
            self._set_szp_byte(new_al)
            return "AAD"

        # ── Shifts / rotates ──────────────────────────────────────────────────

        if op in (0xD0, 0xD1, 0xD2, 0xD3):
            word = bool(op & 1)
            count = 1 if op < 0xD2 else (rf.read8_low("cx"))
            modrm = self._fetch8()
            mod = (modrm >> 6) & 3
            ext = (modrm >> 3) & 7
            rm = modrm & 7
            _, _, _, ea = self._decode_modrm(modrm, word, seg_override)
            seg = self._effective_seg(rm, mod, seg_override)
            a = self._read_rm(mod, rm, seg, ea, word)
            width = 16 if word else 8
            mask = _WORD_MASK if word else _BYTE_MASK

            _shift_names = {0: "ROL", 1: "ROR", 2: "RCL", 3: "RCR",
                            4: "SHL", 5: "SHR", 6: "SHL", 7: "SAR"}

            cf_old = rf._flag_cf
            if ext == 0:
                result, new_cf = rol(a, count, width, cf_old)
                msb = (result >> (width - 1)) & 1
                rf._flag_cf = new_cf
                rf._flag_of = (msb ^ new_cf) if count == 1 else rf._flag_of
            elif ext == 1:
                result, new_cf = ror(a, count, width, cf_old)
                bits_r = int_to_bits(result, width)
                msb = bits_r[width - 1]
                rf._flag_cf = new_cf
                rf._flag_of = (msb ^ bits_r[width - 2]) if count == 1 else rf._flag_of
            elif ext == 2:
                result, new_cf = rcl(a, count, width, cf_old)
                msb = (result >> (width - 1)) & 1
                rf._flag_cf = new_cf
                rf._flag_of = (msb ^ new_cf) if count == 1 else rf._flag_of
            elif ext == 3:
                result, new_cf = rcr(a, count, width, cf_old)
                bits_r = int_to_bits(result, width)
                msb = bits_r[width - 1]
                rf._flag_cf = new_cf
                rf._flag_of = (msb ^ bits_r[width - 2]) if count == 1 else rf._flag_of
            elif ext in (4, 6):  # SHL/SAL
                result, new_cf = shl(a, count, width)
                msb = (result >> (width - 1)) & 1
                rf._flag_cf = new_cf
                rf._flag_of = (msb ^ new_cf) if count == 1 else rf._flag_of
                self._set_szp_byte(result & _BYTE_MASK)
            elif ext == 5:  # SHR
                result, new_cf = shr(a, count, width)
                msb_orig = (a >> (width - 1)) & 1
                rf._flag_cf = new_cf
                rf._flag_of = msb_orig if count == 1 else rf._flag_of
                self._set_szp_byte(result & _BYTE_MASK)
            else:  # SAR
                result, new_cf = sar(a, count, width)
                rf._flag_cf = new_cf
                rf._flag_of = 0
                self._set_szp_byte(result & _BYTE_MASK)

            # Set SF/ZF/PF for all shifts
            if ext in (4, 5, 6, 7):
                bits_r = int_to_bits(result & mask, width)
                from intel8086_gatelevel.bits import compute_parity, compute_zero
                rf._flag_sf = bits_r[width - 1]
                rf._flag_zf = compute_zero(bits_r)
                rf._flag_pf = compute_parity(bits_r)

            self._write_rm(mod, rm, seg, ea, result & mask, word)
            return f"{_shift_names[ext]} m,{'1' if op < 0xD2 else 'CL'}"

        # ── Control flow ──────────────────────────────────────────────────────

        # JMP short (EB)
        if op == 0xEB:
            disp = self._fetch_s8()
            ip = rf.read16("ip")
            d_word = disp & _WORD_MASK if disp >= 0 else (disp + 0x10000) & _WORD_MASK
            new_ip, _, _ = add_16bit(ip, d_word, 0)
            rf.write16("ip", new_ip & _WORD_MASK)
            return f"JMP SHORT {disp:+d}"

        # JMP near (E9)
        if op == 0xE9:
            disp = self._fetch_s16()
            ip = rf.read16("ip")
            d_word = disp & _WORD_MASK if disp >= 0 else (disp + 0x10000) & _WORD_MASK
            new_ip, _, _ = add_16bit(ip, d_word, 0)
            rf.write16("ip", new_ip & _WORD_MASK)
            return f"JMP NEAR {disp:+d}"

        # JMP far (EA)
        if op == 0xEA:
            new_ip = self._fetch16()
            new_cs = self._fetch16()
            rf.write16("ip", new_ip)
            rf.write16("cs", new_cs)
            return f"JMP FAR {new_cs:04X}:{new_ip:04X}"

        # CALL near (E8)
        if op == 0xE8:
            disp = self._fetch_s16()
            self._push16(rf.read16("ip"))
            ip = rf.read16("ip")
            d_word = disp & _WORD_MASK if disp >= 0 else (disp + 0x10000) & _WORD_MASK
            new_ip, _, _ = add_16bit(ip, d_word, 0)
            rf.write16("ip", new_ip & _WORD_MASK)
            return f"CALL NEAR {disp:+d}"

        # CALL far (9A)
        if op == 0x9A:
            new_ip = self._fetch16()
            new_cs = self._fetch16()
            self._push16(rf.read16("cs"))
            self._push16(rf.read16("ip"))
            rf.write16("ip", new_ip)
            rf.write16("cs", new_cs)
            return f"CALL FAR {new_cs:04X}:{new_ip:04X}"

        # RET near (C3 / C2)
        if op == 0xC3:
            rf.write16("ip", self._pop16())
            return "RET"
        if op == 0xC2:
            n = self._fetch16()
            rf.write16("ip", self._pop16())
            sp = rf.read16("sp")
            new_sp, _, _ = add_16bit(sp, n, 0)
            rf.write16("sp", new_sp & _WORD_MASK)
            return f"RET {n}"

        # RETF (CB / CA)
        if op == 0xCB:
            rf.write16("ip", self._pop16())
            rf.write16("cs", self._pop16())
            return "RETF"
        if op == 0xCA:
            n = self._fetch16()
            rf.write16("ip", self._pop16())
            rf.write16("cs", self._pop16())
            sp = rf.read16("sp")
            new_sp, _, _ = add_16bit(sp, n, 0)
            rf.write16("sp", new_sp & _WORD_MASK)
            return f"RETF {n}"

        # Conditional jumps (70–7F)
        if 0x70 <= op <= 0x7F:
            disp = self._fetch_s8()
            if self._eval_cond(op - 0x70):
                ip = rf.read16("ip")
                d_word = disp & _WORD_MASK if disp >= 0 else (disp + 0x10000) & _WORD_MASK
                new_ip, _, _ = add_16bit(ip, d_word, 0)
                rf.write16("ip", new_ip & _WORD_MASK)
            _jcc = ["JO", "JNO", "JB", "JNB", "JZ", "JNZ", "JBE", "JA",
                    "JS", "JNS", "JP", "JNP", "JL", "JGE", "JLE", "JG"]
            return f"{_jcc[op - 0x70]} {disp:+d}"

        # LOOP / LOOPZ / LOOPNZ / JCXZ (E0–E3)
        if op in (0xE0, 0xE1, 0xE2, 0xE3):
            disp = self._fetch_s8()
            if op != 0xE3:
                cx = rf.read16("cx")
                new_cx, _, _ = add_16bit(cx, invert_16bit(1), 1)  # cx - 1
                rf.write16("cx", new_cx & _WORD_MASK)
            zf = bool(rf._flag_zf)
            cx_val = rf.read16("cx")
            if op == 0xE2:
                taken = cx_val != 0
            elif op == 0xE1:
                taken = cx_val != 0 and zf
            elif op == 0xE0:
                taken = cx_val != 0 and not zf
            else:  # JCXZ (no decrement)
                taken = cx_val == 0
            if taken:
                ip = rf.read16("ip")
                d_word = disp & _WORD_MASK if disp >= 0 else (disp + 0x10000) & _WORD_MASK
                new_ip, _, _ = add_16bit(ip, d_word, 0)
                rf.write16("ip", new_ip & _WORD_MASK)
            _ln = {0xE0: "LOOPNE", 0xE1: "LOOPE", 0xE2: "LOOP", 0xE3: "JCXZ"}
            return f"{_ln[op]} {disp:+d}"

        # ── Interrupts ────────────────────────────────────────────────────────

        if op in (0xCC, 0xCE):
            vector = 3 if op == 0xCC else 4
            self._trigger_interrupt(vector)
            self._halted = True
            return "INT"
        if op == 0xCD:
            n = self._fetch8()
            self._trigger_interrupt(n)
            self._halted = True
            return f"INT {n:#x}"
        if op == 0xCF:  # IRET
            rf.write16("ip", self._pop16())
            rf.write16("cs", self._pop16())
            self._load_flags(self._pop16())
            return "IRET"

        # ── String operations ─────────────────────────────────────────────────

        if op in (0xA4, 0xA5, 0xA6, 0xA7, 0xAE, 0xAF, 0xAC, 0xAD, 0xAA, 0xAB):
            word = bool(op & 1)
            step = self._str_step(word)
            seg_src = self._get_sreg(seg_override) if seg_override is not None else rf.read16("ds")

            if op in (0xAC, 0xAD):
                return self._exec_lods(word, step, seg_src, rep_prefix)
            if op in (0xAA, 0xAB):
                return self._exec_stos(word, step, rep_prefix)
            if op in (0xA4, 0xA5):
                return self._exec_movs(word, step, seg_src, rep_prefix)
            if op in (0xA6, 0xA7):
                return self._exec_cmps(word, step, seg_src, rep_prefix)
            return self._exec_scas(word, step, rep_prefix)

        # ── Miscellaneous ─────────────────────────────────────────────────────

        if op == 0xF4:
            self._halted = True
            return "HLT"

        if op == 0xF8:
            rf._flag_cf = 0
            return "CLC"
        if op == 0xF9:
            rf._flag_cf = 1
            return "STC"
        if op == 0xF5:
            rf._flag_cf = 1 - rf._flag_cf
            return "CMC"
        if op == 0xFC:
            rf._flag_df = 0
            return "CLD"
        if op == 0xFD:
            rf._flag_df = 1
            return "STD"
        if op == 0xFA:
            rf._flag_if = 0
            return "CLI"
        if op == 0xFB:
            rf._flag_if = 1
            return "STI"

        # IN AL/AX, imm8 (E4/E5)
        if op == 0xE4:
            port = self._fetch8()
            rf.write8_low("ax", self._input_ports[port])
            return f"IN AL,{port:#x}"
        if op == 0xE5:
            port = self._fetch8()
            lo = self._input_ports[port]
            hi = self._input_ports[(port + 1) & _BYTE_MASK]
            rf.write16("ax", lo | (hi << 8))
            return f"IN AX,{port:#x}"

        # IN AL/AX, DX (EC/ED)
        if op == 0xEC:
            port = rf.read8_low("dx")
            rf.write8_low("ax", self._input_ports[port])
            return "IN AL,DX"
        if op == 0xED:
            port = rf.read8_low("dx")
            lo = self._input_ports[port]
            hi = self._input_ports[(port + 1) & _BYTE_MASK]
            rf.write16("ax", lo | (hi << 8))
            return "IN AX,DX"

        # OUT imm8, AL/AX (E6/E7)
        if op == 0xE6:
            port = self._fetch8()
            self._output_ports[port] = rf.read8_low("ax")
            return f"OUT {port:#x},AL"
        if op == 0xE7:
            port = self._fetch8()
            ax = rf.read16("ax")
            self._output_ports[port] = ax & _BYTE_MASK
            self._output_ports[(port + 1) & _BYTE_MASK] = (ax >> 8) & _BYTE_MASK
            return f"OUT {port:#x},AX"

        # OUT DX, AL/AX (EE/EF)
        if op == 0xEE:
            port = rf.read8_low("dx")
            self._output_ports[port] = rf.read8_low("ax")
            return "OUT DX,AL"
        if op == 0xEF:
            port = rf.read8_low("dx")
            ax = rf.read16("ax")
            self._output_ports[port] = ax & _BYTE_MASK
            self._output_ports[(port + 1) & _BYTE_MASK] = (ax >> 8) & _BYTE_MASK
            return "OUT DX,AX"

        # WAIT (9B)
        if op == 0x9B:
            return "WAIT"

        # Unknown — halt
        self._halted = True
        return f"DB {op:#04x}"

    # ── String operation helpers ──────────────────────────────────────────────

    def _exec_lods(self, word: bool, step: int, seg_src: int, rep: int | None) -> str:
        rf = self._rf
        count = rf.read16("cx") if rep else 1
        for _ in range(count):
            si = rf.read16("si")
            if word:
                rf.write16("ax", self._read_word(seg_src, si))
            else:
                rf.write8_low("ax", self._read_byte(seg_src, si))
            step_word = step & _WORD_MASK if step >= 0 else (step + 0x10000) & _WORD_MASK
            new_si, _, _ = add_16bit(si, step_word, 0)
            rf.write16("si", new_si & _WORD_MASK)
            if rep:
                cx = rf.read16("cx")
                new_cx, _, _ = add_16bit(cx, invert_16bit(1), 1)
                rf.write16("cx", new_cx & _WORD_MASK)
                if new_cx == 0:
                    break
        return "LODS"

    def _exec_stos(self, word: bool, step: int, rep: int | None) -> str:
        rf = self._rf
        es = rf.read16("es")
        count = rf.read16("cx") if rep else 1
        for _ in range(count):
            di = rf.read16("di")
            if word:
                self._write_word(es, di, rf.read16("ax"))
            else:
                self._write_byte(es, di, rf.read8_low("ax"))
            step_word = step & _WORD_MASK if step >= 0 else (step + 0x10000) & _WORD_MASK
            new_di, _, _ = add_16bit(di, step_word, 0)
            rf.write16("di", new_di & _WORD_MASK)
            if rep:
                cx = rf.read16("cx")
                new_cx, _, _ = add_16bit(cx, invert_16bit(1), 1)
                rf.write16("cx", new_cx & _WORD_MASK)
                if new_cx == 0:
                    break
        return "STOS"

    def _exec_movs(self, word: bool, step: int, seg_src: int, rep: int | None) -> str:
        rf = self._rf
        es = rf.read16("es")
        count = rf.read16("cx") if rep else 1
        step_word = step & _WORD_MASK if step >= 0 else (step + 0x10000) & _WORD_MASK
        for _ in range(count):
            si = rf.read16("si")
            di = rf.read16("di")
            if word:
                val = self._read_word(seg_src, si)
                self._write_word(es, di, val)
            else:
                val = self._read_byte(seg_src, si)
                self._write_byte(es, di, val)
            new_si, _, _ = add_16bit(si, step_word, 0)
            new_di, _, _ = add_16bit(di, step_word, 0)
            rf.write16("si", new_si & _WORD_MASK)
            rf.write16("di", new_di & _WORD_MASK)
            if rep:
                cx = rf.read16("cx")
                new_cx, _, _ = add_16bit(cx, invert_16bit(1), 1)
                rf.write16("cx", new_cx & _WORD_MASK)
                if new_cx == 0:
                    break
        return "MOVS"

    def _exec_cmps(self, word: bool, step: int, seg_src: int, rep: int | None) -> str:
        rf = self._rf
        es = rf.read16("es")
        count = rf.read16("cx") if rep else 1
        step_word = step & _WORD_MASK if step >= 0 else (step + 0x10000) & _WORD_MASK
        for _ in range(count):
            si = rf.read16("si")
            di = rf.read16("di")
            a = self._read_word(seg_src, si) if word else self._read_byte(seg_src, si)
            b = self._read_word(es, di) if word else self._read_byte(es, di)
            r = sub16(a, b, 0) if word else sub8(a, b, 0)
            self._apply_alu_result(r, word)
            new_si, _, _ = add_16bit(si, step_word, 0)
            new_di, _, _ = add_16bit(di, step_word, 0)
            rf.write16("si", new_si & _WORD_MASK)
            rf.write16("di", new_di & _WORD_MASK)
            if rep:
                cx = rf.read16("cx")
                new_cx, _, _ = add_16bit(cx, invert_16bit(1), 1)
                rf.write16("cx", new_cx & _WORD_MASK)
                if new_cx == 0:
                    break
                zf = bool(rf._flag_zf)
                if rep == 0xF3 and not zf:
                    break
                if rep == 0xF2 and zf:
                    break
        return "CMPS"

    def _exec_scas(self, word: bool, step: int, rep: int | None) -> str:
        rf = self._rf
        es = rf.read16("es")
        count = rf.read16("cx") if rep else 1
        step_word = step & _WORD_MASK if step >= 0 else (step + 0x10000) & _WORD_MASK
        for _ in range(count):
            di = rf.read16("di")
            b = self._read_word(es, di) if word else self._read_byte(es, di)
            a = rf.read16("ax") if word else rf.read8_low("ax")
            r = sub16(a, b, 0) if word else sub8(a, b, 0)
            self._apply_alu_result(r, word)
            new_di, _, _ = add_16bit(di, step_word, 0)
            rf.write16("di", new_di & _WORD_MASK)
            if rep:
                cx = rf.read16("cx")
                new_cx, _, _ = add_16bit(cx, invert_16bit(1), 1)
                rf.write16("cx", new_cx & _WORD_MASK)
                if new_cx == 0:
                    break
                zf = bool(rf._flag_zf)
                if rep == 0xF3 and not zf:
                    break
                if rep == 0xF2 and zf:
                    break
        return "SCAS"
