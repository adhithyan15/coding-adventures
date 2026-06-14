"""Tests for Intel 8080 branch instructions.

Covers: JMP, conditional jumps (JNZ/JZ/JNC/JC/JPO/JPE/JP/JM),
        CALL, conditional calls, RET, conditional returns, RST, PCHL
"""

from __future__ import annotations

from intel8080_simulator import Intel8080Simulator


def run(program: list[int]) -> Intel8080Simulator:
    sim = Intel8080Simulator()
    sim.reset()
    sim.load(bytes(program + [0x76]))
    while not sim._halted:  # noqa: SLF001
        sim.step()
    return sim


class TestJMP:
    def test_jmp_unconditional(self) -> None:
        # JMP to address 0x0006; at 0x0006: MVI A,0x42; HLT
        # Layout: [0x00] JMP 0x0006 = C3 06 00 [0x03] MVI A,0 [0x05] HLT ... skip
        # Actually: [0x00] C3 06 00; [0x03] 3E 00 76 (dead); [0x06] 3E 42 76
        program = [0xC3, 0x06, 0x00, 0x3E, 0x00, 0x76, 0x3E, 0x42]
        sim = run(program)
        assert sim._a == 0x42  # noqa: SLF001


class TestConditionalJumps:
    def test_jnz_taken(self) -> None:
        # MVI A,1; DCR A (Z=0 after 1-1... wait, DCR A on value 1 → 0, Z=1)
        # Let's set Z=0 with MVI A,2; DCR A (2-1=1, Z=0)
        # then JNZ target should jump
        # [0x00] MVI A,2 = 3E 02
        # [0x02] DCR A = 3D  (A=1, Z=0)
        # [0x03] JNZ 0x0008 = C2 08 00
        # [0x06] MVI A,0x00 = 3E 00 (skipped)
        # [0x08] MVI A,0x77 = 3E 77
        program = [0x3E, 0x02, 0x3D, 0xC2, 0x08, 0x00, 0x3E, 0x00, 0x3E, 0x77]
        sim = run(program)
        assert sim._a == 0x77  # noqa: SLF001

    def test_jnz_not_taken(self) -> None:
        # A=1; SUB A (Z=1); JNZ → not taken → MVI A,0xBB
        # [0x00] 3E 01  [0x02] 97  [0x03] C2 08 00  [0x06] 3E BB  [0x08] HLT
        program = [0x3E, 0x01, 0x97, 0xC2, 0x08, 0x00, 0x3E, 0xBB]
        sim = run(program)
        assert sim._a == 0xBB  # fell through  # noqa: SLF001

    def test_jz_taken(self) -> None:
        # A=1; SUB A (Z=1); JZ 0x0008 → taken
        # [0x00] 3E 01  [0x02] 97  [0x03] CA 08 00  [0x06] 3E 00  [0x08] 3E 42
        program = [0x3E, 0x01, 0x97, 0xCA, 0x08, 0x00, 0x3E, 0x00, 0x3E, 0x42]
        sim = run(program)
        assert sim._a == 0x42  # noqa: SLF001

    def test_jc_taken(self) -> None:
        # STC; JC 0x0006 → jump
        # [0x00] 37  [0x01] DA 06 00  [0x04] 3E 00  [0x06] 3E 55
        program = [0x37, 0xDA, 0x06, 0x00, 0x3E, 0x00, 0x3E, 0x55]
        sim = run(program)
        assert sim._a == 0x55  # noqa: SLF001

    def test_jnc_taken(self) -> None:
        # No STC; JNC target → taken (CY=0 at start)
        program = [0xD2, 0x06, 0x00, 0x3E, 0x00, 0x76, 0x3E, 0x33]
        sim = run(program)
        assert sim._a == 0x33  # noqa: SLF001

    def test_jp_taken(self) -> None:
        # MVI A,1 (S=0); JP target → positive
        program = [0x3E, 0x01, 0xF2, 0x08, 0x00, 0x3E, 0x00, 0x76, 0x3E, 0x11]
        sim = run(program)
        assert sim._a == 0x11  # noqa: SLF001

    def test_jm_taken(self) -> None:
        # ADD A on A=0x40 → A=0x80, S=1 (bit 7 set); JM target
        # [0x00] 3E 40 (MVI A,0x40)
        # [0x02] 87 (ADD A → 0x80, S=1)
        # [0x03] FA 09 00 (JM 0x0009)
        # [0x06] 3E 00 (dead)
        # [0x08] 76 (dead HLT)
        # [0x09] 3E 22 (target: MVI A,0x22)
        program = [0x3E, 0x40, 0x87, 0xFA, 0x09, 0x00, 0x3E, 0x00, 0x76, 0x3E, 0x22]
        sim = run(program)
        assert sim._a == 0x22  # noqa: SLF001

    def test_jpe_taken(self) -> None:
        # 0xFF has even parity → P=1; JPE target
        program = [0x3E, 0xFF, 0x87, 0xEA, 0x09, 0x00, 0x3E, 0x00, 0x76, 0x3E, 0x77]
        # ADD A (A=0xFE=0b11111110, 7 ones, odd parity P=0)
        # Hmm let me use XRA A to get 0 (even parity)
        # [0x00] 3E 01 (MVI A,1)
        # [0x02] AF (XRA A → A=0, P=1)
        # [0x03] EA 08 00 (JPE 0x0008)
        # [0x06] 3E 00
        # [0x07] 76 (HLT)
        # [0x08] 3E 55
        program = [0x3E, 0x01, 0xAF, 0xEA, 0x08, 0x00, 0x3E, 0x00, 0x3E, 0x55]
        sim = run(program)
        assert sim._a == 0x55  # noqa: SLF001


class TestPCHL:
    def test_pchl_jumps_to_hl(self) -> None:
        # LXI H,0x0007; PCHL; dead code; [0x07] MVI A,0xAA
        program = [0x21, 0x07, 0x00, 0xE9, 0x3E, 0x00, 0x76, 0x3E, 0xAA]
        sim = run(program)
        assert sim._a == 0xAA  # noqa: SLF001


class TestCALLRET:
    def test_call_and_ret(self) -> None:
        # Simple call/ret: program calls subroutine at 0x000A that sets A=0x42 and rets
        # [0x00] CALL 0x000A = CD 0A 00
        # [0x03] HLT = 76
        # (padding)
        # [0x0A] MVI A,0x42 = 3E 42
        # [0x0C] RET = C9
        program = [
            0xCD, 0x0A, 0x00,  # CALL 0x000A
            0x76,              # HLT  (at 0x03)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  # padding 0x04–0x09
            0x3E, 0x42,        # MVI A,0x42  (at 0x0A)
            0xC9,              # RET  (at 0x0C)
        ]
        sim = Intel8080Simulator()
        sim.reset()
        sim._sp = 0xFF00  # noqa: SLF001  — set SP high so stack doesn't collide
        sim.load(bytes(program))
        sim._memory[0] = 0xCD  # noqa: SLF001
        sim.load(bytes(program))
        sim._sp = 0xFF00  # noqa: SLF001
        while not sim._halted:  # noqa: SLF001
            sim.step()
        assert sim._a == 0x42  # noqa: SLF001

    def test_conditional_call_taken(self) -> None:
        # STC; CC (call if carry) 0x000A
        program = [
            0x31, 0x00, 0xFF,  # LXI SP,0xFF00
            0x37,              # STC
            0xDC, 0x0B, 0x00,  # CC 0x000B
            0x76,              # HLT (at 0x07)
            0x00, 0x00, 0x00,  # padding
            0x3E, 0x99,        # MVI A,0x99  (at 0x0B)
            0xC9,              # RET
        ]
        sim = run(program[:-1])  # run will append HLT at end, but we handle via RET
        # Actually use execute directly
        sim = Intel8080Simulator()
        result = sim.execute(bytes(program))
        assert result.ok
        assert result.final_state.a == 0x99

    def test_conditional_call_not_taken(self) -> None:
        # CY=0; CC (call if carry) — not taken; fall through to MVI A,0x42; HLT
        program = [
            0x31, 0x00, 0xFF,  # LXI SP,0xFF00
            0xDC, 0x0C, 0x00,  # CC 0x000C (not taken, CY=0)
            0x3E, 0x42,        # MVI A,0x42
            0x76,              # HLT
        ]
        result = Intel8080Simulator().execute(bytes(program))
        assert result.ok
        assert result.final_state.a == 0x42

    def test_conditional_ret_taken(self) -> None:
        program = [
            0x31, 0x00, 0xFF,  # LXI SP,0xFF00
            0x37,              # STC (CY=1)
            0xCD, 0x0B, 0x00,  # CALL 0x000B
            0x76,              # HLT at 0x07
            0x00, 0x00, 0x00,  # padding
            0x3E, 0x55,        # MVI A,0x55  at 0x0B
            0xD8,              # RC (ret if carry — CY is still 1 from STC)
        ]
        result = Intel8080Simulator().execute(bytes(program))
        assert result.ok
        assert result.final_state.a == 0x55

    def test_conditional_ret_not_taken(self) -> None:
        program = [
            0x31, 0x00, 0xFF,  # LXI SP,0xFF00
            0xCD, 0x0A, 0x00,  # CALL 0x000A
            0x76,              # HLT at 0x06
            0x00, 0x00, 0x00,  # padding
            0x3E, 0x77,        # MVI A,0x77  at 0x0A
            0xD8,              # RC (CY=0, not taken)
            0x3E, 0x88,        # MVI A,0x88
            0xC9,              # RET
        ]
        result = Intel8080Simulator().execute(bytes(program))
        assert result.ok
        assert result.final_state.a == 0x88


class TestRST:
    def test_rst_0(self) -> None:
        # RST 0 pushes PC and jumps to 0x0000
        # Place HLT at 0x0000; RST 0 at some later address
        # Actually RST 0 jumps to 0x0000 which has the code we started at — tricky.
        # Let's use RST 1 (jumps to 0x0008) and place code there.
        program = [
            0x31, 0x00, 0xFF,  # LXI SP,0xFF00  at 0x00
            0xCF,              # RST 1  at 0x03 — jump to 0x0008
            0x76,              # HLT at 0x04 (return address)
            0x00, 0x00, 0x00,  # padding
            0x3E, 0xBC,        # MVI A,0xBC  at 0x08
            0xC9,              # RET  at 0x0A — return to 0x04 → HLT
        ]
        result = Intel8080Simulator().execute(bytes(program))
        assert result.ok
        assert result.final_state.a == 0xBC
