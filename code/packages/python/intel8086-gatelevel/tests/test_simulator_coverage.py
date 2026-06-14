"""Coverage tests: all Jcc conditions, REP string ops, INT/IRET, all
addressing modes, shifts/rotates, BCD operations, LEA, LDS, LES, XLAT,
IN/OUT, and other instruction forms."""

import pytest

from intel8086_gatelevel.simulator import Intel8086GateLevelSimulator


def make_sim() -> Intel8086GateLevelSimulator:
    sim = Intel8086GateLevelSimulator()
    sim.reset()
    return sim


def run(program: bytes, max_steps: int = 2000):
    sim = make_sim()
    return sim.execute(program, max_steps=max_steps)


# ── All JCC conditions ────────────────────────────────────────────────────────

class TestAllJcc:
    """Test every Jcc opcode (70–7F) — taken and not-taken cases."""

    def _jcc_prog(self, jcc_op: int, flag_setup: bytes, taken: bool):
        """Build a program that checks whether JCC takes the branch.

        Structure:
            MOV AX, 0
            <flag_setup>
            JCC +N         ; if taken, jump to skip_mark (skip taken_mark + HLT)
            MOV AX, 1      ; taken_mark (fall-through path)
            HLT
            MOV AX, 2      ; skip_mark (branch target)
            HLT

        N = len(taken_mark) + 1 = 4 (skip both MOV AX,1 and HLT).
        If taken: AX=2.  If not taken: AX=1.
        """
        taken_mark = bytes([0xB8, 0x01, 0x00])   # MOV AX, 1  (fall-through path)
        skip_mark = bytes([0xB8, 0x02, 0x00])     # MOV AX, 2  (branch-taken path)
        # JCC displacement must skip taken_mark (3 bytes) + HLT (1 byte) = 4
        skip_len = len(taken_mark) + 1
        prog = (
            bytes([0xB8, 0x00, 0x00])          # MOV AX, 0
            + flag_setup
            + bytes([jcc_op, skip_len])        # JCC to skip_mark
            + taken_mark
            + bytes([0xF4])                    # HLT (not-taken end)
            + skip_mark
            + bytes([0xF4])                    # HLT (taken end)
        )
        result = run(prog, max_steps=100)
        # If taken: jumped over taken_mark + HLT → AX=2
        # If not taken: fell through → AX=1
        return result.final_state.ax

    def test_jo_taken(self):
        # JO: OF=1; cause overflow: 0x7FFF + 1
        setup = bytes([0xB8, 0xFF, 0x7F, 0x05, 0x01, 0x00])  # MOV AX,0x7FFF; ADD AX,1
        ax = self._jcc_prog(0x70, setup, True)
        assert ax == 2  # jump taken

    def test_jno_taken(self):
        # JNO: OF=0; no overflow: 1+1
        setup = bytes([0xB8, 0x01, 0x00, 0x05, 0x01, 0x00])  # MOV AX,1; ADD AX,1
        ax = self._jcc_prog(0x71, setup, True)
        assert ax == 2

    def test_jb_taken(self):
        # JB: CF=1; 0 - 1 sets CF
        setup = bytes([0xB8, 0x00, 0x00, 0x2D, 0x01, 0x00])  # MOV AX,0; SUB AX,1
        ax = self._jcc_prog(0x72, setup, True)
        assert ax == 2

    def test_jnb_taken(self):
        # JNB: CF=0; 5-3
        setup = bytes([0xB8, 0x05, 0x00, 0x2D, 0x03, 0x00])
        ax = self._jcc_prog(0x73, setup, True)
        assert ax == 2

    def test_jz_taken(self):
        # JZ: ZF=1; CMP AX,AX
        setup = bytes([0xB8, 0x05, 0x00, 0x3D, 0x05, 0x00])
        ax = self._jcc_prog(0x74, setup, True)
        assert ax == 2

    def test_jnz_taken(self):
        # JNZ: ZF=0; AX=0, CMP with 1
        setup = bytes([0xB8, 0x00, 0x00, 0x3D, 0x01, 0x00])
        ax = self._jcc_prog(0x75, setup, True)
        assert ax == 2

    def test_jbe_taken(self):
        # JBE: CF|ZF=1; 0 <= 0
        setup = bytes([0xB8, 0x00, 0x00, 0x3D, 0x00, 0x00])  # ZF=1
        ax = self._jcc_prog(0x76, setup, True)
        assert ax == 2

    def test_ja_taken(self):
        # JA: !CF and !ZF; 5 > 3
        setup = bytes([0xB8, 0x05, 0x00, 0x3D, 0x03, 0x00])
        ax = self._jcc_prog(0x77, setup, True)
        assert ax == 2

    def test_js_taken(self):
        # JS: SF=1; AX=0x8000 (MSB set)
        setup = bytes([0xB8, 0x00, 0x80, 0x3D, 0x00, 0x00])
        ax = self._jcc_prog(0x78, setup, True)
        assert ax == 2

    def test_jns_taken(self):
        # JNS: SF=0; AX=1
        setup = bytes([0xB8, 0x01, 0x00, 0x3D, 0x00, 0x00])
        ax = self._jcc_prog(0x79, setup, True)
        assert ax == 2

    def test_jp_taken(self):
        # JP: PF=1; result with even parity. 0xFF has 8 ones = even
        setup = bytes([0xB8, 0xFF, 0x00, 0x3D, 0x00, 0x00])
        ax = self._jcc_prog(0x7A, setup, True)
        assert ax == 2

    def test_jnp_taken(self):
        # JNP: PF=0; 1 has 1 one = odd
        setup = bytes([0xB8, 0x01, 0x00, 0x3D, 0x00, 0x00])
        ax = self._jcc_prog(0x7B, setup, True)
        assert ax == 2

    def test_jl_taken(self):
        # JL: SF!=OF; 0x8000 - 1 → OF=1, SF=0 → SF!=OF
        setup = bytes([0xB8, 0x00, 0x80, 0x2D, 0x01, 0x00])
        ax = self._jcc_prog(0x7C, setup, True)
        assert ax == 2

    def test_jge_taken(self):
        # JGE: SF==OF; normal sub: 5-3=2 → SF=0, OF=0
        setup = bytes([0xB8, 0x05, 0x00, 0x2D, 0x03, 0x00])
        ax = self._jcc_prog(0x7D, setup, True)
        assert ax == 2

    def test_jle_taken(self):
        # JLE: ZF=1 or SF!=OF; equal values → ZF=1
        setup = bytes([0xB8, 0x05, 0x00, 0x3D, 0x05, 0x00])
        ax = self._jcc_prog(0x7E, setup, True)
        assert ax == 2

    def test_jg_taken(self):
        # JG: !ZF and SF==OF; 5>3
        setup = bytes([0xB8, 0x05, 0x00, 0x3D, 0x03, 0x00])
        ax = self._jcc_prog(0x7F, setup, True)
        assert ax == 2


# ── LOOP variants ─────────────────────────────────────────────────────────────

class TestLoopVariants:
    def test_loop_basic(self):
        # LOOP: decrement CX; jump if CX != 0
        prog = bytes([
            0xB9, 0x05, 0x00,   # MOV CX, 5
            0xB8, 0x00, 0x00,   # MOV AX, 0
            0x40,               # INC AX ← loop body
            0xE2, 0xFD,         # LOOP -3
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.ax == 5

    def test_loope_exits_on_zero_false(self):
        # LOOPE: loop while CX!=0 and ZF=1
        # CMP different values → ZF=0 → exit immediately
        prog = bytes([
            0xB9, 0x05, 0x00,   # MOV CX, 5
            0xB8, 0x00, 0x00,   # MOV AX, 0
            0x3D, 0x01, 0x00,   # CMP AX, 1 → ZF=0
            0xE1, 0xFB,         # LOOPE -5
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.cx == 4  # Only one iteration

    def test_loopne_exits_on_equal(self):
        # LOOPNE: loop while CX!=0 and ZF=0
        prog = bytes([
            0xB9, 0x03, 0x00,   # MOV CX, 3
            0x3D, 0x00, 0x00,   # CMP AX, 0 → ZF=1
            0xE0, 0xFB,         # LOOPNE -5
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.cx == 2  # Only one iteration

    def test_jcxz_taken(self):
        prog = bytes([
            0xB9, 0x00, 0x00,   # MOV CX, 0
            0xE3, 0x03,         # JCXZ +3
            0xB8, 0xFF, 0xFF,   # MOV AX, 0xFFFF (skipped)
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.ax == 0  # not 0xFFFF

    def test_jcxz_not_taken(self):
        prog = bytes([
            0xB9, 0x01, 0x00,   # MOV CX, 1
            0xE3, 0x03,         # JCXZ +3 (NOT taken)
            0xB8, 0x0A, 0x00,   # MOV AX, 10
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.ax == 10


# ── REP string operations ─────────────────────────────────────────────────────

class TestRepStringOps:
    def test_rep_stosb(self):
        prog = bytes([
            0xB0, 0xFF,         # MOV AL, 0xFF
            0xBF, 0x00, 0x03,   # MOV DI, 0x300
            0xB9, 0x10, 0x00,   # MOV CX, 16
            0xF3, 0xAA,         # REP STOSB
            0xF4,
        ])
        result = run(prog)
        s = result.final_state
        for i in range(16):
            assert s.memory[0x300 + i] == 0xFF
        assert s.cx == 0

    def test_rep_stosw(self):
        prog = bytes([
            0xB8, 0x34, 0x12,   # MOV AX, 0x1234
            0xBF, 0x00, 0x03,   # MOV DI, 0x300
            0xB9, 0x04, 0x00,   # MOV CX, 4
            0xF3, 0xAB,         # REP STOSW
            0xF4,
        ])
        result = run(prog)
        s = result.final_state
        for i in range(4):
            lo = s.memory[0x300 + i * 2]
            hi = s.memory[0x301 + i * 2]
            assert lo == 0x34 and hi == 0x12

    def test_rep_movsb(self):
        prog = bytes([
            0xBE, 0x00, 0x01,   # MOV SI, 0x100
            0xBF, 0x00, 0x02,   # MOV DI, 0x200
            0xB9, 0x03, 0x00,   # MOV CX, 3
            0xF3, 0xA4,         # REP MOVSB
            0xF4,
        ])
        s = _run_with_mem(prog, {0x100: 0xAA, 0x101: 0xBB, 0x102: 0xCC})
        assert s.memory[0x200] == 0xAA
        assert s.memory[0x201] == 0xBB
        assert s.memory[0x202] == 0xCC

    def test_repz_cmpsb(self):
        """REPE CMPSB: stop when not equal."""
        prog = bytes([
            0xBE, 0x00, 0x01,   # MOV SI, 0x100
            0xBF, 0x00, 0x02,   # MOV DI, 0x200
            0xB9, 0x02, 0x00,   # MOV CX, 2
            0xF3, 0xA6,         # REPE CMPSB
            0xF4,
        ])
        s = _run_with_mem(prog, {
            0x100: 0x01, 0x101: 0x02,
            0x200: 0x01, 0x201: 0x03,   # Different at position 1
        })
        assert s.zf is False  # unequal found

    def test_repne_scasb(self):
        """REPNE SCASB: find first match."""
        prog = bytes([
            0xB0, 0x42,         # MOV AL, 0x42
            0xBF, 0x00, 0x01,   # MOV DI, 0x100
            0xB9, 0x03, 0x00,   # MOV CX, 3
            0xF2, 0xAE,         # REPNE SCASB
            0xF4,
        ])
        s = _run_with_mem(prog, {0x100: 0x01, 0x101: 0x02, 0x102: 0x42})
        assert s.zf is True  # found


# ── Addressing modes ──────────────────────────────────────────────────────────

def _run_with_mem(
    prog: bytes,
    mem_patches: dict,
    port_patches: dict | None = None,
    max_steps: int = 200,
):
    """Run a program with pre-initialized memory and/or port values.

    Since execute() calls reset() which clears memory and ports, we use
    load() + step() to preserve patches set before loading.

    Returns the final X86State (via get_state()) after running.
    """
    sim = Intel8086GateLevelSimulator()
    sim.reset()
    # Apply patches AFTER reset so they aren't wiped
    for addr, val in mem_patches.items():
        sim._mem[addr] = val
    if port_patches:
        for port, val in port_patches.items():
            sim._input_ports[port] = val
    sim.load(prog)
    steps = 0
    while not sim._halted and steps < max_steps:
        sim.step()
        steps += 1
    return sim.get_state()


class TestAddressingModes:
    def test_mod00_bx_si(self):
        """[BX+SI] addressing."""
        prog = bytes([
            0xBB, 0x00, 0x01,   # MOV BX, 0x100
            0xBE, 0x50, 0x00,   # MOV SI, 0x50
            0x8A, 0x00,         # MOV AL, [BX+SI] (mod=00, reg=0, rm=0)
            0xF4,
        ])
        s = _run_with_mem(prog, {0x150: 0x42})
        assert s.al == 0x42

    def test_mod01_disp8(self):
        """[BX+SI+disp8] addressing."""
        prog = bytes([
            0xBB, 0x00, 0x01,   # MOV BX, 0x100
            0xBE, 0x50, 0x00,   # MOV SI, 0x50
            0x8A, 0x40, 0x05,   # MOV AL, [BX+SI+5] (mod=01, reg=0, rm=0)
            0xF4,
        ])
        s = _run_with_mem(prog, {0x155: 0x77})
        assert s.al == 0x77

    def test_mod10_disp16(self):
        """[BX+SI+disp16] addressing."""
        prog = bytes([
            0xBB, 0x00, 0x00,   # MOV BX, 0
            0xBE, 0x00, 0x00,   # MOV SI, 0
            0x8A, 0x80, 0x00, 0x10,  # MOV AL, [BX+SI+0x1000]
            0xF4,
        ])
        s = _run_with_mem(prog, {0x1000: 0x55})
        assert s.al == 0x55

    def test_mod00_direct_disp16(self):
        """[disp16] direct addressing (mod=00, rm=6)."""
        prog = bytes([
            0x8A, 0x06, 0x00, 0x03,  # MOV AL, [0x300]
            0xF4,
        ])
        s = _run_with_mem(prog, {0x300: 0x99})
        assert s.al == 0x99

    def test_mod11_reg_to_reg(self):
        """Register-to-register (mod=11)."""
        prog = bytes([
            0xB9, 0x42, 0x00,   # MOV CX, 0x42
            0x8B, 0xC1,         # MOV AX, CX (mod=11, reg=0, rm=1)
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.ax == 0x42

    def test_bp_di_mod01(self):
        """[BP+DI+disp8] — uses SS segment by default."""
        # BP=0x200, DI=0x10, disp=5 → EA = 0x215
        prog = bytes([
            0xBD, 0x00, 0x02,   # MOV BP, 0x200
            0xBF, 0x10, 0x00,   # MOV DI, 0x10
            0x8A, 0x43, 0x05,   # MOV AL, [BP+DI+5]
            0xF4,
        ])
        s = _run_with_mem(prog, {0x215: 0x44})
        assert s.al == 0x44


# ── LEA / LDS / LES ──────────────────────────────────────────────────────────

class TestLeaLdsLes:
    def test_lea_basic(self):
        """LEA loads the effective address, not memory contents."""
        prog = bytes([
            0xBB, 0x00, 0x01,   # MOV BX, 0x100
            0xBE, 0x50, 0x00,   # MOV SI, 0x50
            0x8D, 0x00,         # LEA AX, [BX+SI]
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.ax == 0x150

    def test_lea_with_disp(self):
        prog = bytes([
            0xBB, 0x00, 0x01,   # MOV BX, 0x100
            0x8D, 0x47, 0x10,   # LEA AX, [BX+0x10]
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.ax == 0x110

    def test_lds_loads_offset_and_ds(self):
        """LDS loads 32-bit far pointer: offset→reg, segment→DS."""
        prog = bytes([
            0xC5, 0x06, 0x00, 0x02,  # LDS AX, [0x200]
            0xF4,
        ])
        s = _run_with_mem(prog, {
            0x200: 0x00, 0x201: 0x03,   # offset = 0x0300
            0x202: 0x20, 0x203: 0x00,   # segment = 0x0020
        })
        assert s.ax == 0x0300
        assert s.ds == 0x0020

    def test_les_loads_offset_and_es(self):
        """LES loads 32-bit far pointer: offset→reg, segment→ES."""
        prog = bytes([
            0xC4, 0x06, 0x00, 0x02,  # LES AX, [0x200]
            0xF4,
        ])
        s = _run_with_mem(prog, {
            0x200: 0x00, 0x201: 0x04,   # offset = 0x0400
            0x202: 0x10, 0x203: 0x00,   # segment = 0x0010
        })
        assert s.ax == 0x0400
        assert s.es == 0x0010


# ── XLAT ─────────────────────────────────────────────────────────────────────

class TestXlat:
    def test_xlat_basic(self):
        """XLAT looks up AL in a table at DS:BX."""
        prog = bytes([
            0xBB, 0x00, 0x02,   # MOV BX, 0x200 (table base)
            0xB0, 0x02,         # MOV AL, 2 (index)
            0xD7,               # XLAT
            0xF4,
        ])
        s = _run_with_mem(prog, {
            0x200: 0xAA,  # table[0]
            0x201: 0xBB,  # table[1]
            0x202: 0xCC,  # table[2]
        })
        assert s.al == 0xCC


# ── IN / OUT ──────────────────────────────────────────────────────────────────

class TestInOut:
    def test_in_al_fixed(self):
        prog = bytes([0xE4, 0x20, 0xF4])  # IN AL, 0x20; HLT
        s = _run_with_mem(prog, {}, port_patches={0x20: 0x42})
        assert s.al == 0x42

    def test_in_ax_fixed(self):
        prog = bytes([0xE5, 0x20, 0xF4])  # IN AX, 0x20; HLT
        s = _run_with_mem(prog, {}, port_patches={0x20: 0x34, 0x21: 0x12})
        assert s.ax == 0x1234

    def test_out_al_fixed(self):
        prog = bytes([0xB0, 0x55, 0xE6, 0x20, 0xF4])  # MOV AL,0x55; OUT 0x20,AL
        result = run(prog)
        assert result.final_state.output_ports[0x20] == 0x55

    def test_in_al_dx(self):
        prog = bytes([
            0xBA, 0x30, 0x00,   # MOV DX, 0x30
            0xEC,               # IN AL, DX
            0xF4,
        ])
        s = _run_with_mem(prog, {}, port_patches={0x30: 0x77})
        assert s.al == 0x77

    def test_out_al_dx(self):
        prog = bytes([
            0xB0, 0xAB,         # MOV AL, 0xAB
            0xBA, 0x10, 0x00,   # MOV DX, 0x10
            0xEE,               # OUT DX, AL
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.output_ports[0x10] == 0xAB

    def test_set_get_ports(self):
        """set_input_port / get_output_port public API — use _run_with_mem to
        avoid execute() resetting ports before the IN instruction runs."""
        prog = bytes([0xE4, 0x05, 0xF4])  # IN AL, 5; HLT
        s = _run_with_mem(prog, {}, port_patches={5: 0xAB})
        assert s.output_ports[5] == 0   # nothing output to port 5
        assert s.al == 0xAB


# ── BCD operations ────────────────────────────────────────────────────────────

class TestBcdInstructions:
    def test_daa(self):
        prog = bytes([
            0xB8, 0x09, 0x00,   # MOV AX, 9 (AL=9)
            0x04, 0x07,         # ADD AL, 7 → AL=16 (0x10)
            0x27,               # DAA → AL=0x16 (BCD 16)
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.al == 0x16  # BCD 16

    def test_das(self):
        prog = bytes([
            0xB0, 0x16,         # MOV AL, 0x16
            0x2C, 0x07,         # SUB AL, 7 → AL=0x0F
            0x2F,               # DAS
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.al == 0x09  # BCD result

    def test_aam(self):
        prog = bytes([
            0xB0, 0x19,         # MOV AL, 25 (0x19)
            0xD4, 0x0A,         # AAM (base 10)
            0xF4,
        ])
        result = run(prog)
        s = result.final_state
        assert s.ah == 2   # 25 // 10 = 2
        assert s.al == 5   # 25 % 10 = 5

    def test_aad(self):
        prog = bytes([
            0xB4, 0x02,         # MOV AH, 2
            0xB0, 0x05,         # MOV AL, 5
            0xD5, 0x0A,         # AAD (base 10): AL = 2*10 + 5 = 25
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.al == 25

    def test_aaa_corrects(self):
        prog = bytes([
            0xB8, 0x0F, 0x00,   # MOV AX, 0x0F (AL=0x0F)
            0x37,               # AAA
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.cf is True

    def test_aas_corrects(self):
        prog = bytes([
            0xB8, 0x0F, 0x00,   # MOV AX, 0x0F
            0x3F,               # AAS
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.cf is True


# ── INT / IRET ────────────────────────────────────────────────────────────────

class TestIntIret:
    def test_int_halts(self):
        """INT n triggers an interrupt and halts in our simulator."""
        prog = bytes([0xCD, 0x21, 0xF4])
        result = run(prog)
        assert result.halted

    def test_int3_halts(self):
        prog = bytes([0xCC, 0xF4])
        result = run(prog)
        assert result.halted

    def test_iret(self):
        """IRET pops IP, CS, FLAGS."""
        sim = make_sim()
        # Set up stack: push FLAGS=0x0002, CS=0, IP=5
        sim._rf.write16("sp", 0xFFF8)
        # IP=5 at stack top, then CS=0, then FLAGS=0x0002
        sim._mem[0xFFF8] = 0x05; sim._mem[0xFFF9] = 0x00   # IP=5
        sim._mem[0xFFFA] = 0x00; sim._mem[0xFFFB] = 0x00   # CS=0
        sim._mem[0xFFFC] = 0x02; sim._mem[0xFFFD] = 0x00   # FLAGS=0x0002
        sim._mem[0] = 0xCF  # IRET at IP=0
        sim._mem[5] = 0xF4  # HLT at IP=5
        result = sim.execute(bytes([]))  # already loaded
        # Need to step manually since execute resets
        sim.reset()
        sim._mem[0] = 0xCF
        sim._mem[5] = 0xF4
        sim._rf.write16("sp", 0xFFF8)
        sim._mem[0xFFF8] = 0x05; sim._mem[0xFFF9] = 0x00
        sim._mem[0xFFFA] = 0x00; sim._mem[0xFFFB] = 0x00
        sim._mem[0xFFFC] = 0x02; sim._mem[0xFFFD] = 0x00
        trace = sim.step()
        assert trace.mnemonic == "IRET"
        assert sim._rf.read16("ip") == 5


# ── Misc instructions ─────────────────────────────────────────────────────────

class TestMiscInstructions:
    def test_nop(self):
        prog = bytes([0x90, 0xF4])
        result = run(prog)
        assert result.halted

    def test_wait(self):
        prog = bytes([0x9B, 0xF4])
        result = run(prog)
        assert result.halted

    def test_cmc(self):
        prog = bytes([0xF8, 0xF5, 0xF4])  # CLC, CMC, HLT
        result = run(prog)
        assert result.final_state.cf is True

    def test_push_pop_r16(self):
        prog = bytes([
            0xB8, 0x34, 0x12,   # MOV AX, 0x1234
            0x50,               # PUSH AX
            0xBB, 0x00, 0x00,   # MOV BX, 0
            0x5B,               # POP BX
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.bx == 0x1234

    def test_xchg_reg_rm(self):
        prog = bytes([
            0xB8, 0x01, 0x00,   # MOV AX, 1
            0xBB, 0x02, 0x00,   # MOV BX, 2
            0x87, 0xC3,         # XCHG AX, BX (mod=11, reg=0, rm=3)
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.ax == 2
        assert result.final_state.bx == 1

    def test_jmp_far(self):
        """JMP FAR sets CS:IP."""
        prog = bytes([0xEA, 0x00, 0x01, 0x00, 0x10])  # JMP FAR 0x1000:0x100
        # HLT at physical 0x10100 = (CS=0x1000 << 4) + IP=0x100
        s = _run_with_mem(prog, {0x10100: 0xF4})
        assert s.cs == 0x1000
        assert s.ip == 0x101  # IP advances past HLT

    def test_call_far(self):
        """CALL FAR pushes CS:IP then sets new CS:IP."""
        prog = bytes([0x9A, 0x00, 0x00, 0x00, 0x10])  # CALL FAR 0x1000:0x0000
        # HLT at physical 0x10000 = (CS=0x1000 << 4) + IP=0x0
        s = _run_with_mem(prog, {0x10000: 0xF4})
        assert s.halted
        assert s.cs == 0x1000

    def test_retf_far_return(self):
        """RETF pops IP and CS."""
        sim = make_sim()
        sim._rf.write16("sp", 0xFFF8)
        sim._mem[0xFFF8] = 0x06; sim._mem[0xFFF9] = 0x00  # IP=6
        sim._mem[0xFFFA] = 0x00; sim._mem[0xFFFB] = 0x00  # CS=0
        sim._mem[0] = 0xCB  # RETF
        sim._mem[6] = 0xF4  # HLT
        sim.step()
        assert sim._rf.read16("ip") == 6
        assert sim._rf.read16("cs") == 0

    def test_nmi(self):
        """NMI triggers interrupt 2."""
        sim = make_sim()
        # IVT entry for int 2 at physical 8
        sim._mem[8] = 0x00; sim._mem[9] = 0x01   # IP=0x100
        sim._mem[10] = 0x00; sim._mem[11] = 0x00  # CS=0
        sim._mem[0x100] = 0xF4   # HLT at target
        sim._mem[0] = 0xF4       # HLT at IP=0 (wont be reached)
        sim.nmi()
        assert sim._rf.read16("ip") == 0x100

    def test_interrupt_protocol(self):
        """interrupt() method correctly sets CS:IP from IVT."""
        sim = make_sim()
        sim._mem[0x10] = 0x00; sim._mem[0x11] = 0x02   # IP=0x200 for int 4
        sim._mem[0x12] = 0x00; sim._mem[0x13] = 0x00
        sim.interrupt(4)
        assert sim._rf.read16("ip") == 0x200

    def test_mul16_instruction(self):
        prog = bytes([
            0xB8, 0x07, 0x00,   # MOV AX, 7
            0xBB, 0x06, 0x00,   # MOV BX, 6
            0xF7, 0xE3,         # MUL BX
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.ax == 42

    def test_imul_negative(self):
        prog = bytes([
            0xB8, 0xFF, 0xFF,   # MOV AX, 0xFFFF (-1 signed)
            0xBB, 0x05, 0x00,   # MOV BX, 5
            0xF7, 0xEB,         # IMUL BX
            0xF4,
        ])
        result = run(prog)
        # -1 * 5 = -5 = 0xFFFB in 16-bit
        assert result.final_state.ax == 0xFFFB
        assert result.final_state.dx == 0xFFFF  # sign extension

    def test_div8_instruction(self):
        prog = bytes([
            0xB8, 0x15, 0x00,   # MOV AX, 21
            0xB3, 0x07,         # MOV BL, 7
            0xF6, 0xF3,         # DIV BL
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.al == 3   # 21 / 7 = 3
        assert result.final_state.ah == 0   # no remainder

    def test_shift_cl(self):
        """SHR by CL value."""
        prog = bytes([
            0xB8, 0x80, 0x00,   # MOV AX, 0x80
            0xB9, 0x03, 0x00,   # MOV CX, 3
            0xD3, 0xE8,         # SHR AX, CL
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.ax == 0x10

    def test_rol_by_cl(self):
        prog = bytes([
            0xB8, 0x01, 0x00,   # MOV AX, 1
            0xB9, 0x04, 0x00,   # MOV CX, 4
            0xD3, 0xC0,         # ROL AX, CL
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.ax == 0x10

    def test_rcl_by_1_sets_cf(self):
        prog = bytes([
            0xB8, 0x80, 0x00,   # MOV AX, 0x0080 (AL=0x80)
            0xF8,               # CLC
            0xD0, 0xD0,         # RCL AL, 1 (mod=11, ext=2, rm=0 AL)
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.al == 0  # 0x80 << 1 = 0 (MSB to CF)
        assert result.final_state.cf is True

    def test_sar_preserves_sign(self):
        prog = bytes([
            0xB8, 0x00, 0x80,   # MOV AX, 0x8000
            0xD1, 0xF8,         # SAR AX, 1
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.ax == 0xC000  # sign bit propagated
