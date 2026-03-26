"""Cross-validation: behavioral vs gate-level simulators.

Run the same programs on both simulators and verify identical results.
This is the ultimate correctness test — the gate-level simulator must
produce exactly the same output as the behavioral one for any program.
"""

import pytest

from intel4004_gatelevel import Intel4004GateLevel

# The behavioral simulator may or may not be installed
try:
    from intel4004_simulator import Intel4004Simulator

    HAS_BEHAVIORAL = True
except ImportError:
    HAS_BEHAVIORAL = False


PROGRAMS = {
    "x = 1 + 2": bytes([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01]),
    "multiply 3x4": bytes([
        0xD3, 0xB0, 0xDC, 0xB1,
        0xD0, 0x80, 0x71, 0x05,
        0xB2, 0x01,
    ]),
    "BCD 7+8": bytes([
        0xD8, 0xB0, 0xD7, 0x80, 0xFB, 0x01,
    ]),
    "complement and add": bytes([
        0xD5, 0xF4, 0xB0,  # LDM 5, CMA, XCH R0 (R0=10)
        0xD3, 0x80,        # LDM 3, ADD R0 (A=13)
        0xB1, 0x01,        # XCH R1, HLT
    ]),
    "rotate left": bytes([
        0xD5, 0xF5, 0xF5, 0xB0, 0x01,  # LDM 5, RAL, RAL, XCH R0, HLT
    ]),
    "subroutine call": bytes([
        0x50, 0x04,  # JMS 0x004
        0x01,        # HLT
        0x00,        # padding
        0xC7,        # BBL 7
    ]),
    "countdown": bytes([
        0xD5, 0xF8, 0x1C, 0x01, 0x01,
    ]),
    "all accum ops": bytes([
        0xD5,        # LDM 5
        0xFA,        # STC (carry=1)
        0xF7,        # TCC (A=1, carry=0)
        0xF2,        # IAC (A=2)
        0xF3,        # CMC (carry=1)
        0xF1,        # CLC (carry=0)
        0xF4,        # CMA (A=~2=13)
        0xF8,        # DAC (A=12)
        0xB0, 0x01,  # XCH R0, HLT
    ]),
}


@pytest.mark.skipif(not HAS_BEHAVIORAL, reason="behavioral simulator not installed")
class TestCrossValidation:
    """Run programs on both simulators, compare register and flag state."""

    @pytest.mark.parametrize("name,program", list(PROGRAMS.items()))
    def test_program(self, name: str, program: bytes) -> None:
        """Same program must produce identical results on both simulators."""
        behavioral = Intel4004Simulator()
        gate_level = Intel4004GateLevel()

        b_traces = behavioral.run(program)
        g_traces = gate_level.run(program)

        # Same number of instructions executed
        assert len(b_traces) == len(g_traces), (
            f"[{name}] trace length: behavioral={len(b_traces)}, "
            f"gate-level={len(g_traces)}"
        )

        # Same final register state
        for i in range(16):
            b_val = behavioral.registers[i]
            g_val = gate_level.registers[i]
            assert b_val == g_val, (
                f"[{name}] R{i}: behavioral={b_val}, gate-level={g_val}"
            )

        # Same accumulator and carry
        assert behavioral.accumulator == gate_level.accumulator, (
            f"[{name}] A: behavioral={behavioral.accumulator}, "
            f"gate-level={gate_level.accumulator}"
        )
        assert behavioral.carry == gate_level.carry, (
            f"[{name}] carry: behavioral={behavioral.carry}, "
            f"gate-level={gate_level.carry}"
        )

        # Same mnemonic trace
        for i, (bt, gt) in enumerate(zip(b_traces, g_traces, strict=True)):
            assert bt.mnemonic == gt.mnemonic, (
                f"[{name}] step {i}: behavioral={bt.mnemonic}, "
                f"gate-level={gt.mnemonic}"
            )
