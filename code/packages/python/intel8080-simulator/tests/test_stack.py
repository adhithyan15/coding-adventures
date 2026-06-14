"""Tests for Intel 8080 stack instructions.

Covers: PUSH rp, POP rp, PUSH PSW, POP PSW, XTHL, SPHL
"""

from __future__ import annotations

from intel8080_simulator import Intel8080Simulator


def make_sim() -> Intel8080Simulator:
    sim = Intel8080Simulator()
    sim._sp = 0xFF00  # noqa: SLF001 — set SP to safe area
    return sim


def run(program: list[int]) -> Intel8080Simulator:
    sim = make_sim()
    sim.load(bytes(program + [0x76]))
    while not sim._halted:  # noqa: SLF001
        sim.step()
    return sim


def run_with_sp(program: list[int], sp: int = 0xFF00) -> Intel8080Simulator:
    sim = Intel8080Simulator()
    sim._sp = sp  # noqa: SLF001
    sim.load(bytes(program + [0x76]))
    while not sim._halted:  # noqa: SLF001
        sim.step()
    return sim


class TestPUSHPOP:
    def test_push_pop_b(self) -> None:
        sim = run_with_sp([
            0x01, 0x34, 0x12,  # LXI B,0x1234
            0xC5,              # PUSH B
            0x01, 0x00, 0x00,  # LXI B,0 (clear)
            0xC1,              # POP B
        ])
        assert sim._b == 0x12  # noqa: SLF001
        assert sim._c == 0x34  # noqa: SLF001

    def test_push_pop_d(self) -> None:
        sim = run_with_sp([
            0x11, 0x78, 0x56,  # LXI D,0x5678
            0xD5,              # PUSH D
            0x11, 0x00, 0x00,  # LXI D,0
            0xD1,              # POP D
        ])
        assert sim._d == 0x56  # noqa: SLF001
        assert sim._e == 0x78  # noqa: SLF001

    def test_push_pop_h(self) -> None:
        sim = run_with_sp([
            0x21, 0xCD, 0xAB,  # LXI H,0xABCD
            0xE5,              # PUSH H
            0x21, 0x00, 0x00,  # LXI H,0
            0xE1,              # POP H
        ])
        assert sim._h == 0xAB  # noqa: SLF001
        assert sim._l == 0xCD  # noqa: SLF001

    def test_push_pop_lifo(self) -> None:
        # Push B, D, H; pop in reverse order
        sim = run_with_sp([
            0x01, 0x11, 0x11,  # LXI B,0x1111
            0x11, 0x22, 0x22,  # LXI D,0x2222
            0x21, 0x33, 0x33,  # LXI H,0x3333
            0xC5,              # PUSH B
            0xD5,              # PUSH D
            0xE5,              # PUSH H
            0xE1,              # POP H (gets 0x3333)
            0xD1,              # POP D (gets 0x2222)
            0xC1,              # POP B (gets 0x1111)
        ])
        assert sim._b == 0x11 and sim._c == 0x11  # noqa: SLF001
        assert sim._d == 0x22 and sim._e == 0x22  # noqa: SLF001
        assert sim._h == 0x33 and sim._l == 0x33  # noqa: SLF001

    def test_sp_decrements_on_push(self) -> None:
        sim = run_with_sp([0x01, 0x00, 0x00, 0xC5], sp=0xFF00)  # PUSH B
        assert sim._sp == 0xFEFE  # SP decremented by 2  # noqa: SLF001

    def test_sp_increments_on_pop(self) -> None:
        sim = run_with_sp([0x01, 0x00, 0x00, 0xC5, 0xC1], sp=0xFF00)  # PUSH B; POP B
        assert sim._sp == 0xFF00  # SP restored  # noqa: SLF001


class TestPUSHPOPPSW:
    def test_push_psw_stores_a_and_flags(self) -> None:
        sim = run_with_sp([
            0x3E, 0x42,  # MVI A,0x42
            0x37,        # STC (CY=1)
            0xF5,        # PUSH PSW
        ])
        # After PUSH: memory[SP+1]=A=0x42; memory[SP]=flags
        sp = sim._sp  # noqa: SLF001
        assert sim._memory[sp + 1] == 0x42  # noqa: SLF001
        assert sim._memory[sp] & 0x01 == 1  # CY=1 in flags byte  # noqa: SLF001

    def test_pop_psw_restores_a_and_flags(self) -> None:
        sim = run_with_sp([
            0x3E, 0x55,  # MVI A,0x55
            0x37,        # STC
            0xF5,        # PUSH PSW
            0x3E, 0x00,  # MVI A,0 (clear A)
            0x3F,        # CMC (toggle CY to 0)
            0xF1,        # POP PSW
        ])
        assert sim._a == 0x55  # noqa: SLF001
        assert sim._flag_cy is True  # restored from stack  # noqa: SLF001

    def test_push_pop_psw_round_trip(self) -> None:
        # After SUB A, A=0 and Z=1.  PUSH PSW captures A=0 and Z=1.
        # After MVI A,0xFF + CMC (toggles CY), POP PSW restores A=0 and original flags.
        sim = run_with_sp([
            0x3E, 0x42,  # MVI A, 0x42
            0x97,        # SUB A → A=0, Z=1, S=0, P=1
            0xF5,        # PUSH PSW  (A=0, Z=1)
            0x3E, 0xFF,  # MVI A, 0xFF  (A changed)
            0x3F,        # CMC  (flags changed)
            0xF1,        # POP PSW  (restores A=0, Z=1)
        ])
        assert sim._a == 0  # restored from stack (was 0 when pushed)  # noqa: SLF001
        assert sim._flag_z is True  # noqa: SLF001
        assert sim._flag_s is False  # noqa: SLF001


class TestXTHL:
    def test_xthl(self) -> None:
        sim = run_with_sp([
            0x21, 0x34, 0x12,  # LXI H,0x1234
            0x31, 0xFE, 0xFF,  # LXI SP,0xFFFE
            # manually write 0x5678 at SP
            # Use MVI to put values there:
            0x3E, 0x78, 0x32, 0xFE, 0xFF,  # MVI A,0x78; STA 0xFFFE
            0x3E, 0x56, 0x32, 0xFF, 0xFF,  # MVI A,0x56; STA 0xFFFF
            0xE3,              # XTHL
        ])
        # After XTHL: H=0x56, L=0x78; memory[SP]=0x34, memory[SP+1]=0x12
        assert sim._h == 0x56  # noqa: SLF001
        assert sim._l == 0x78  # noqa: SLF001
        assert sim._memory[0xFFFE] == 0x34  # noqa: SLF001
        assert sim._memory[0xFFFF] == 0x12  # noqa: SLF001


class TestSPHL:
    def test_sphl(self) -> None:
        sim = run([0x21, 0x00, 0x02, 0xF9])  # LXI H,0x0200; SPHL
        assert sim._sp == 0x0200  # noqa: SLF001
