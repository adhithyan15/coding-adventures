"""End-to-end program tests for the gate-level simulator."""

from __future__ import annotations

from intel8080_gatelevel import Intel8080GateLevelSimulator


def run(program: list[int]) -> Intel8080GateLevelSimulator:
    """Run a program and return the simulator after HLT."""
    sim = Intel8080GateLevelSimulator()
    sim.execute(bytes(program + [0x76]))
    return sim


class TestBasicPrograms:
    def test_nop_loop(self) -> None:
        result = Intel8080GateLevelSimulator().execute(bytes([0x00, 0x00, 0x00, 0x76]))
        assert result.final_state.a == 0
        assert result.steps == 4

    def test_sum_1_to_5(self) -> None:
        # Compute 1+2+3+4+5 = 15 in A
        prog = [
            0x3E, 0x00,   # MVI A, 0
            0x06, 0x05,   # MVI B, 5
            # loop:
            0x80,          # ADD B       (A = A + B)
            0x05,          # DCR B
            0xC2, 0x04, 0x00,   # JNZ loop (addr 0x04)
        ]
        result = Intel8080GateLevelSimulator().execute(bytes(prog + [0x76]))
        assert result.final_state.a == 15

    def test_memory_store_load(self) -> None:
        prog = [
            0x3E, 0xAB,          # MVI A, 0xAB
            0x32, 0x00, 0x02,    # STA 0x0200
            0x3E, 0x00,          # MVI A, 0
            0x3A, 0x00, 0x02,    # LDA 0x0200
        ]
        result = Intel8080GateLevelSimulator().execute(bytes(prog + [0x76]))
        assert result.final_state.a == 0xAB

    def test_call_return(self) -> None:
        # CALL subroutine that sets A=0x42; return; HLT
        prog = bytes([
            0x31, 0x00, 0x04,    # 0x00: LXI SP, 0x0400
            0xCD, 0x09, 0x00,    # 0x03: CALL 0x0009
            0x76,                # 0x06: HLT (never reached)
            0x00, 0x00,          # 0x07-08: padding
            0x3E, 0x42,          # 0x09: MVI A, 0x42
            0xC9,                # 0x0B: RET
            0x76,                # 0x0C: HLT
        ])
        result = Intel8080GateLevelSimulator().execute(prog)
        assert result.final_state.a == 0x42

    def test_xchg(self) -> None:
        prog = [
            0x21, 0x34, 0x12,   # LXI H, 0x1234
            0x11, 0x78, 0x56,   # LXI D, 0x5678
            0xEB,                # XCHG
        ]
        result = Intel8080GateLevelSimulator().execute(bytes(prog + [0x76]))
        s = result.final_state
        assert s.h == 0x56
        assert s.l == 0x78
        assert s.d == 0x12
        assert s.e == 0x34

    def test_pchl(self) -> None:
        # LXI H,0x0006; PCHL; MVI A,0xFF (skip); HLT
        prog = bytes([
            0x21, 0x06, 0x00,   # LXI H, 0x0006
            0xE9,                # PCHL → jump to 0x0006
            0x3E, 0xFF,          # MVI A,0xFF (skipped)
            0x76,                # HLT (at 0x0006)
        ])
        result = Intel8080GateLevelSimulator().execute(prog)
        assert result.final_state.a == 0   # MVI A,0xFF was skipped


class TestIOPrograms:
    def test_in_out(self) -> None:
        sim = Intel8080GateLevelSimulator()
        sim.set_input_port(3, 0xCD)
        result = sim.execute(bytes([
            0xDB, 0x03,    # IN 3
            0xD3, 0x07,    # OUT 7
            0x76,
        ]))
        assert result.final_state.a == 0xCD
        assert sim.get_output_port(7) == 0xCD

    def test_port_boundary(self) -> None:
        sim = Intel8080GateLevelSimulator()
        sim.set_input_port(255, 0xFF)
        result = sim.execute(bytes([0xDB, 0xFF, 0x76]))
        assert result.final_state.a == 0xFF


class TestStackPrograms:
    def test_xthl(self) -> None:
        prog = bytes([
            0x31, 0x00, 0x04,    # LXI SP, 0x0400
            0x21, 0x34, 0x12,    # LXI H, 0x1234
            0x01, 0xCD, 0xAB,    # LXI B, 0xABCD
            0xC5,                # PUSH B → push 0xABCD
            0xE3,                # XTHL → exchange HL with stack top
            0x76,
        ])
        result = Intel8080GateLevelSimulator().execute(prog)
        s = result.final_state
        assert s.h == 0xAB
        assert s.l == 0xCD

    def test_sphl(self) -> None:
        prog = [0x21, 0x00, 0x04, 0xF9]   # LXI H, 0x0400; SPHL
        result = Intel8080GateLevelSimulator().execute(bytes(prog + [0x76]))
        assert result.final_state.sp == 0x0400
