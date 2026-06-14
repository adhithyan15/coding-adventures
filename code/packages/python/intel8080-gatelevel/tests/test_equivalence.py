"""Equivalence tests: gate-level simulator == behavioral simulator.

For every instruction group and register combination, both simulators
must produce identical output state. This is the ultimate correctness
test for the gate-level simulator.
"""

from __future__ import annotations

from intel8080_simulator import Intel8080Simulator

from intel8080_gatelevel import Intel8080GateLevelSimulator


def _run_gl(program: bytes) -> tuple[int, bool, bool, bool, bool, bool]:
    """Run gate-level simulator; return (A, cy, z, s, p, ac)."""
    sim = Intel8080GateLevelSimulator()
    r = sim.execute(program)
    s = r.final_state
    return s.a, s.flag_cy, s.flag_z, s.flag_s, s.flag_p, s.flag_ac


def _run_bh(program: bytes) -> tuple[int, bool, bool, bool, bool, bool]:
    """Run behavioral simulator; return (A, cy, z, s, p, ac)."""
    sim = Intel8080Simulator()
    sim.reset()
    sim.load(program)
    while not sim._halted:  # noqa: SLF001
        sim.step()
    return (
        sim._a,  # noqa: SLF001
        sim._flag_cy,  # noqa: SLF001
        sim._flag_z,  # noqa: SLF001
        sim._flag_s,  # noqa: SLF001
        sim._flag_p,  # noqa: SLF001
        sim._flag_ac,  # noqa: SLF001
    )


def _assert_equiv(program: bytes) -> None:
    """Assert gate-level and behavioral produce identical results."""
    gl = _run_gl(program)
    bh = _run_bh(program)
    assert gl == bh, f"Mismatch: gate-level={gl}, behavioral={bh}"


class TestEquivalenceALU:
    """Each 8080 ALU opcode should produce identical results."""

    def test_add_b(self) -> None:
        _assert_equiv(bytes([0x3E, 0x0A, 0x06, 0x05, 0x80, 0x76]))

    def test_add_overflow(self) -> None:
        _assert_equiv(bytes([0x3E, 0xFF, 0x06, 0x01, 0x80, 0x76]))

    def test_adc_carry(self) -> None:
        _assert_equiv(bytes([0x3E, 0x05, 0x06, 0x03, 0x37, 0x88, 0x76]))

    def test_sub_b(self) -> None:
        _assert_equiv(bytes([0x3E, 0x0A, 0x06, 0x03, 0x90, 0x76]))

    def test_sub_self(self) -> None:
        _assert_equiv(bytes([0x3E, 0x42, 0x97, 0x76]))

    def test_sub_borrow(self) -> None:
        _assert_equiv(bytes([0x3E, 0x03, 0x06, 0x0A, 0x90, 0x76]))

    def test_sbb_with_borrow(self) -> None:
        _assert_equiv(bytes([0x3E, 0x08, 0x06, 0x03, 0x37, 0x98, 0x76]))

    def test_ana(self) -> None:
        _assert_equiv(bytes([0x3E, 0xFF, 0x06, 0x0F, 0xA0, 0x76]))

    def test_ani(self) -> None:
        _assert_equiv(bytes([0x3E, 0xFF, 0xE6, 0xAA, 0x76]))

    def test_xra_a(self) -> None:
        _assert_equiv(bytes([0x3E, 0xFF, 0xAF, 0x76]))

    def test_xri(self) -> None:
        _assert_equiv(bytes([0x3E, 0xFF, 0xEE, 0x0F, 0x76]))

    def test_ora_b(self) -> None:
        _assert_equiv(bytes([0x3E, 0x0F, 0x06, 0xF0, 0xB0, 0x76]))

    def test_ori(self) -> None:
        _assert_equiv(bytes([0x3E, 0x0F, 0xF6, 0xF0, 0x76]))

    def test_cmp_equal(self) -> None:
        _assert_equiv(bytes([0x3E, 0x05, 0x06, 0x05, 0xB8, 0x76]))

    def test_cpi(self) -> None:
        _assert_equiv(bytes([0x3E, 0x10, 0xFE, 0x10, 0x76]))

    def test_adi(self) -> None:
        _assert_equiv(bytes([0x3E, 0x20, 0xC6, 0x10, 0x76]))

    def test_sui(self) -> None:
        _assert_equiv(bytes([0x3E, 0x20, 0xD6, 0x10, 0x76]))


class TestEquivalenceRotates:
    def test_rlc(self) -> None:
        _assert_equiv(bytes([0x3E, 0x85, 0x07, 0x76]))

    def test_rlc_no_carry(self) -> None:
        _assert_equiv(bytes([0x3E, 0x05, 0x07, 0x76]))

    def test_rrc(self) -> None:
        _assert_equiv(bytes([0x3E, 0x85, 0x0F, 0x76]))

    def test_ral(self) -> None:
        _assert_equiv(bytes([0x37, 0x3E, 0x85, 0x17, 0x76]))

    def test_ral_no_carry(self) -> None:
        _assert_equiv(bytes([0x3E, 0x85, 0x17, 0x76]))

    def test_rar(self) -> None:
        _assert_equiv(bytes([0x37, 0x3E, 0x85, 0x1F, 0x76]))

    def test_rar_no_carry(self) -> None:
        _assert_equiv(bytes([0x3E, 0x84, 0x1F, 0x76]))


class TestEquivalenceDataTransfer:
    def test_mvi_a(self) -> None:
        _assert_equiv(bytes([0x3E, 0x42, 0x76]))

    def test_mov_ba(self) -> None:
        _assert_equiv(bytes([0x3E, 0x10, 0x47, 0x76]))   # MVI A,16; MOV B,A

    def test_lxi_b(self) -> None:
        prog = bytes([0x01, 0x78, 0x56, 0x76])   # LXI B,0x5678
        gl_state = Intel8080GateLevelSimulator().execute(prog).final_state
        bh = Intel8080Simulator()
        bh.reset()
        bh.load(prog)
        while not bh._halted:  # noqa: SLF001
            bh.step()  # noqa: SLF001
        assert gl_state.b == bh._b  # noqa: SLF001
        assert gl_state.c == bh._c  # noqa: SLF001


class TestEquivalenceIncrDecr:
    def test_inr_a(self) -> None:
        _assert_equiv(bytes([0x3E, 0x05, 0x3C, 0x76]))

    def test_inr_does_not_affect_carry(self) -> None:
        # STC; MVI A,5; INR A — CY should still be True
        prog = bytes([0x37, 0x3E, 0x05, 0x3C, 0x76])
        gl = Intel8080GateLevelSimulator().execute(prog).final_state
        bh = Intel8080Simulator()
        bh.reset()
        bh.load(prog)
        while not bh._halted:  # noqa: SLF001
            bh.step()  # noqa: SLF001
        assert gl.flag_cy == bh._flag_cy  # noqa: SLF001

    def test_dcr_a(self) -> None:
        _assert_equiv(bytes([0x3E, 0x05, 0x3D, 0x76]))

    def test_dcr_wrap(self) -> None:
        _assert_equiv(bytes([0x3E, 0x00, 0x3D, 0x76]))

    def test_inx_b(self) -> None:
        prog = bytes([0x01, 0xFF, 0x00, 0x03, 0x76])   # LXI B,0xFF; INX B
        gl = Intel8080GateLevelSimulator().execute(prog).final_state
        bh = Intel8080Simulator()
        bh.reset()
        bh.load(prog)
        while not bh._halted:  # noqa: SLF001
            bh.step()  # noqa: SLF001
        assert gl.b == bh._b  # noqa: SLF001
        assert gl.c == bh._c  # noqa: SLF001

    def test_dcx_h(self) -> None:
        prog = bytes([0x21, 0x00, 0x01, 0x2B, 0x76])
        gl = Intel8080GateLevelSimulator().execute(prog).final_state
        bh = Intel8080Simulator()
        bh.reset()
        bh.load(prog)
        while not bh._halted:  # noqa: SLF001
            bh.step()  # noqa: SLF001
        assert gl.h == bh._h  # noqa: SLF001
        assert gl.l == bh._l  # noqa: SLF001


class TestEquivalenceSpecial:
    def test_cma(self) -> None:
        _assert_equiv(bytes([0x3E, 0xAA, 0x2F, 0x76]))

    def test_stc(self) -> None:
        prog = bytes([0x37, 0x76])
        gl = Intel8080GateLevelSimulator().execute(prog).final_state
        bh = Intel8080Simulator()
        bh.reset()
        bh.load(prog)
        while not bh._halted:  # noqa: SLF001
            bh.step()  # noqa: SLF001
        assert gl.flag_cy == bh._flag_cy  # noqa: SLF001

    def test_cmc(self) -> None:
        _assert_equiv(bytes([0x3F, 0x76]))

    def test_cmc_twice(self) -> None:
        prog = bytes([0x37, 0x3F, 0x3F, 0x76])
        gl = Intel8080GateLevelSimulator().execute(prog).final_state
        bh = Intel8080Simulator()
        bh.reset()
        bh.load(prog)
        while not bh._halted:  # noqa: SLF001
            bh.step()  # noqa: SLF001
        assert gl.flag_cy == bh._flag_cy  # noqa: SLF001

    def test_daa(self) -> None:
        # BCD: 25 + 38 = 63
        _assert_equiv(bytes([0x3E, 0x25, 0x06, 0x38, 0x80, 0x27, 0x76]))

    def test_dad(self) -> None:
        prog = bytes([0x21, 0x34, 0x12, 0x01, 0x78, 0x56, 0x09, 0x76])
        gl = Intel8080GateLevelSimulator().execute(prog).final_state
        bh = Intel8080Simulator()
        bh.reset()
        bh.load(prog)
        while not bh._halted:  # noqa: SLF001
            bh.step()  # noqa: SLF001
        assert gl.h == bh._h  # noqa: SLF001
        assert gl.l == bh._l  # noqa: SLF001


class TestEquivalenceBranches:
    def test_jmp(self) -> None:
        # JMP over NOP to HLT
        prog = bytes([0xC3, 0x04, 0x00, 0x00, 0x76])
        gl = Intel8080GateLevelSimulator().execute(prog)
        bh = Intel8080Simulator()
        bh.reset()
        bh.load(prog)
        while not bh._halted:  # noqa: SLF001
            bh.step()  # noqa: SLF001
        assert gl.final_state.pc == bh._pc  # noqa: SLF001

    def test_jnz_taken(self) -> None:
        # MVI B,1; DCR B; JNZ ...
        prog = bytes([0x06, 0x01, 0x05, 0xC2, 0x02, 0x00, 0x76])
        _assert_equiv(prog)

    def test_call_ret(self) -> None:
        # CALL 0x0006; MVI A,0xFF; HLT; [subroutine: MVI A,0x42; RET]
        prog = bytes([
            0xCD, 0x07, 0x00,   # 0x0000: CALL 0x0007
            0x3E, 0xFF,          # 0x0003: MVI A,0xFF (never reached)
            0x76,                # 0x0005: HLT
            0x00,                # 0x0006: padding
            0x3E, 0x42,          # 0x0007: MVI A,0x42
            0xC9,                # 0x0009: RET
        ])
        _assert_equiv(prog)


class TestEquivalenceStack:
    def test_push_pop_bc(self) -> None:
        # LXI SP,0x0300; LXI B,0x1234; PUSH B; POP D
        prog = bytes([
            0x31, 0x00, 0x03,    # LXI SP,0x300
            0x01, 0x34, 0x12,    # LXI B,0x1234
            0xC5,                # PUSH B
            0xD1,                # POP D
            0x76,
        ])
        gl = Intel8080GateLevelSimulator().execute(prog).final_state
        bh = Intel8080Simulator()
        bh.reset()
        bh.load(prog)
        while not bh._halted:  # noqa: SLF001
            bh.step()  # noqa: SLF001
        assert gl.d == bh._d  # noqa: SLF001
        assert gl.e == bh._e  # noqa: SLF001

    def test_push_pop_psw(self) -> None:
        # MVI A,0x42; STC; PUSH PSW; XRA A; POP PSW → should restore A=0x42, CY=1
        prog = bytes([
            0x31, 0x00, 0x03,    # LXI SP,0x300
            0x3E, 0x42,          # MVI A,0x42
            0x37,                # STC
            0xF5,                # PUSH PSW
            0xAF,                # XRA A (clears A and flags)
            0xF1,                # POP PSW
            0x76,
        ])
        gl = Intel8080GateLevelSimulator().execute(prog).final_state
        bh = Intel8080Simulator()
        bh.reset()
        bh.load(prog)
        while not bh._halted:  # noqa: SLF001
            bh.step()  # noqa: SLF001
        assert gl.a == bh._a  # noqa: SLF001
        assert gl.flag_cy == bh._flag_cy  # noqa: SLF001


class TestEquivalenceIO:
    def test_in_port(self) -> None:
        gl_sim = Intel8080GateLevelSimulator()
        gl_sim.set_input_port(5, 0xAB)
        r = gl_sim.execute(bytes([0xDB, 0x05, 0x76]))
        assert r.final_state.a == 0xAB

    def test_out_port(self) -> None:
        gl_sim = Intel8080GateLevelSimulator()
        gl_sim.execute(bytes([0x3E, 0x77, 0xD3, 0x03, 0x76]))
        assert gl_sim.get_output_port(3) == 0x77

    def test_ei_di(self) -> None:
        prog = bytes([0xFB, 0xF3, 0x76])   # EI; DI; HLT
        gl = Intel8080GateLevelSimulator().execute(prog).final_state
        bh = Intel8080Simulator()
        bh.reset()
        bh.load(prog)
        while not bh._halted:  # noqa: SLF001
            bh.step()  # noqa: SLF001
        assert gl.interrupts_enabled == bh._inte  # noqa: SLF001


class TestEquivalenceMemory:
    def test_lda_sta(self) -> None:
        prog = bytes([
            0x3E, 0x42,             # MVI A, 0x42
            0x32, 0x00, 0x02,       # STA 0x0200
            0x3E, 0x00,             # MVI A, 0
            0x3A, 0x00, 0x02,       # LDA 0x0200
            0x76,
        ])
        _assert_equiv(prog)

    def test_lhld_shld(self) -> None:
        prog = bytes([
            0x21, 0x34, 0x12,       # LXI H, 0x1234
            0x22, 0x00, 0x02,       # SHLD 0x0200
            0x21, 0x00, 0x00,       # LXI H, 0
            0x2A, 0x00, 0x02,       # LHLD 0x0200
            0x76,
        ])
        gl = Intel8080GateLevelSimulator().execute(prog).final_state
        bh = Intel8080Simulator()
        bh.reset()
        bh.load(prog)
        while not bh._halted:  # noqa: SLF001
            bh.step()  # noqa: SLF001
        assert gl.h == bh._h  # noqa: SLF001
        assert gl.l == bh._l  # noqa: SLF001
