"""Tests for Intel 8080 data transfer instructions.

Covers: MOV r1,r2; MVI r,d8; LXI rp,d16; LDA; STA; LHLD; SHLD;
        LDAX B/D; STAX B/D; XCHG
"""

from __future__ import annotations

import pytest

from intel8080_simulator import Intel8080Simulator


def sim_with(program: list[int]) -> Intel8080Simulator:
    """Create a simulator, load program, return after execution."""
    sim = Intel8080Simulator()
    result = sim.execute(bytes(program + [0x76]))  # append HLT
    assert result.ok, f"Execution failed: {result.error}"
    return sim


def run(program: list[int]) -> Intel8080Simulator:
    sim = Intel8080Simulator()
    sim.reset()
    sim.load(bytes(program + [0x76]))
    # Run manually to access final state
    while not sim._halted:  # noqa: SLF001
        sim.step()
    return sim


class TestMOV:
    def test_mov_b_c(self) -> None:
        # MVI C,0x42; MOV B,C; HLT → B=0x42
        sim = run([0x0E, 0x42, 0x41])  # MVI C,0x42; MOV B,C
        assert sim._b == 0x42  # noqa: SLF001

    def test_mov_a_b(self) -> None:
        sim = run([0x06, 0x55, 0x78])  # MVI B,0x55; MOV A,B
        assert sim._a == 0x55  # noqa: SLF001

    def test_mov_to_m(self) -> None:
        # LXI H,0x0200; MVI A,0xAB; MOV M,A
        sim = run([0x21, 0x00, 0x02, 0x3E, 0xAB, 0x77])
        assert sim._memory[0x0200] == 0xAB  # noqa: SLF001

    def test_mov_from_m(self) -> None:
        # LXI H,0x0200; MVI A,0xCD; MOV M,A; MVI A,0x00; MOV A,M
        sim = run([0x21, 0x00, 0x02, 0x3E, 0xCD, 0x77, 0x3E, 0x00, 0x7E])
        assert sim._a == 0xCD  # noqa: SLF001

    def test_all_register_mov(self) -> None:
        # Load 0x11 into A, MOV it to B, C, D, E, H, L in sequence
        sim = run([
            0x3E, 0x11,  # MVI A, 0x11
            0x47,        # MOV B,A
            0x4F,        # MOV C,A
            0x57,        # MOV D,A
            0x5F,        # MOV E,A
            0x67,        # MOV H,A
            # L set via MOV L,A but we need H still valid for M
        ])
        assert sim._b == 0x11  # noqa: SLF001
        assert sim._c == 0x11  # noqa: SLF001
        assert sim._d == 0x11  # noqa: SLF001
        assert sim._e == 0x11  # noqa: SLF001
        assert sim._h == 0x11  # noqa: SLF001


class TestMVI:
    @pytest.mark.parametrize("opcode,attr,value", [
        (0x06, "_b", 0x11),
        (0x0E, "_c", 0x22),
        (0x16, "_d", 0x33),
        (0x1E, "_e", 0x44),
        (0x26, "_h", 0x55),
        (0x2E, "_l", 0x66),
        (0x3E, "_a", 0x77),
    ])
    def test_mvi_register(self, opcode: int, attr: str, value: int) -> None:
        sim = run([opcode, value])
        assert getattr(sim, attr) == value  # noqa: SLF001

    def test_mvi_m(self) -> None:
        # LXI H,0x0300; MVI M,0x99
        sim = run([0x21, 0x00, 0x03, 0x36, 0x99])
        assert sim._memory[0x0300] == 0x99  # noqa: SLF001


class TestLXI:
    def test_lxi_b(self) -> None:
        sim = run([0x01, 0x34, 0x12])  # LXI B, 0x1234
        assert sim._b == 0x12  # noqa: SLF001
        assert sim._c == 0x34  # noqa: SLF001

    def test_lxi_d(self) -> None:
        sim = run([0x11, 0x78, 0x56])  # LXI D, 0x5678
        assert sim._d == 0x56  # noqa: SLF001
        assert sim._e == 0x78  # noqa: SLF001

    def test_lxi_h(self) -> None:
        sim = run([0x21, 0xCD, 0xAB])  # LXI H, 0xABCD
        assert sim._h == 0xAB  # noqa: SLF001
        assert sim._l == 0xCD  # noqa: SLF001

    def test_lxi_sp(self) -> None:
        sim = run([0x31, 0x00, 0xFF])  # LXI SP, 0xFF00
        assert sim._sp == 0xFF00  # noqa: SLF001


class TestLDASTA:
    def test_sta_then_lda(self) -> None:
        # MVI A,0x42; STA 0x0400; LDA 0x0400
        sim = run([0x3E, 0x42, 0x32, 0x00, 0x04, 0x3A, 0x00, 0x04])
        assert sim._a == 0x42  # noqa: SLF001

    def test_sta_writes_correct_address(self) -> None:
        sim = run([0x3E, 0x77, 0x32, 0x50, 0x01])  # MVI A,0x77; STA 0x0150
        assert sim._memory[0x0150] == 0x77  # noqa: SLF001


class TestLHLDSHLD:
    def test_shld_then_lhld(self) -> None:
        # LXI H,0x1234; SHLD 0x0500; LXI H,0x0000; LHLD 0x0500
        sim = run([
            0x21, 0x34, 0x12,  # LXI H, 0x1234
            0x22, 0x00, 0x05,  # SHLD 0x0500
            0x21, 0x00, 0x00,  # LXI H, 0x0000 (clear)
            0x2A, 0x00, 0x05,  # LHLD 0x0500
        ])
        assert sim._h == 0x12  # noqa: SLF001
        assert sim._l == 0x34  # noqa: SLF001

    def test_shld_stores_l_first(self) -> None:
        sim = run([0x21, 0xAB, 0xCD, 0x22, 0x00, 0x06])  # LXI H,0xCDAB; SHLD 0x0600
        assert sim._memory[0x0600] == 0xAB  # L at addr
        assert sim._memory[0x0601] == 0xCD  # H at addr+1


class TestLDAXSTAX:
    def test_stax_b_then_ldax_b(self) -> None:
        # LXI B,0x0500; MVI A,0x88; STAX B; MVI A,0; LDAX B
        sim = run([
            0x01, 0x00, 0x05,  # LXI B,0x0500
            0x3E, 0x88,        # MVI A,0x88
            0x02,              # STAX B
            0x3E, 0x00,        # MVI A,0
            0x0A,              # LDAX B
        ])
        assert sim._a == 0x88  # noqa: SLF001

    def test_stax_d_then_ldax_d(self) -> None:
        sim = run([
            0x11, 0x00, 0x06,  # LXI D,0x0600
            0x3E, 0x99,        # MVI A,0x99
            0x12,              # STAX D
            0x3E, 0x00,        # MVI A,0
            0x1A,              # LDAX D
        ])
        assert sim._a == 0x99  # noqa: SLF001


class TestXCHG:
    def test_xchg_swaps_hl_de(self) -> None:
        sim = run([
            0x21, 0x34, 0x12,  # LXI H,0x1234
            0x11, 0x78, 0x56,  # LXI D,0x5678
            0xEB,              # XCHG
        ])
        assert sim._h == 0x56  # noqa: SLF001
        assert sim._l == 0x78  # noqa: SLF001
        assert sim._d == 0x12  # noqa: SLF001
        assert sim._e == 0x34  # noqa: SLF001
