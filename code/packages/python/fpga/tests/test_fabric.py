"""Tests for FPGA fabric — top-level integration.

Coverage targets:
- FPGA creation from bitstream
- CLB evaluation through fabric
- Routing through fabric
- I/O pin operations
- Error handling (missing CLB, switch, pin)
- End-to-end: configure → route → evaluate
"""

from __future__ import annotations

import pytest

from fpga.bitstream import Bitstream
from fpga.fabric import FPGA, SimResult

# ─── Helpers ──────────────────────────────────────────────────────────

def and_tt() -> list[int]:
    tt = [0] * 16
    tt[3] = 1
    return tt


def xor_tt() -> list[int]:
    tt = [0] * 16
    tt[1] = 1
    tt[2] = 1
    return tt


def make_simple_config() -> dict:  # type: ignore[type-arg]
    """A simple FPGA with one CLB, routing, and I/O."""
    return {
        "clbs": {
            "clb_0": {
                "slice0": {
                    "lut_a": and_tt(),
                    "lut_b": xor_tt(),
                },
                "slice1": {
                    "lut_a": [0] * 16,
                    "lut_b": [0] * 16,
                },
            },
        },
        "routing": {
            "sw_0": [
                {"src": "clb_out", "dst": "east"},
                {"src": "north", "dst": "south"},
            ],
        },
        "io": {
            "in_a": {"mode": "input"},
            "in_b": {"mode": "input"},
            "out_0": {"mode": "output"},
            "hiz": {"mode": "tristate"},
        },
    }


# ─── SimResult ────────────────────────────────────────────────────────

class TestSimResult:
    def test_defaults(self) -> None:
        sr = SimResult()
        assert sr.outputs == {}
        assert sr.cycles == 0


# ─── FPGA Creation ───────────────────────────────────────────────────

class TestFPGACreation:
    def test_creates_clbs(self) -> None:
        bs = Bitstream.from_dict(make_simple_config())
        fpga = FPGA(bs)
        assert "clb_0" in fpga.clbs

    def test_creates_switches(self) -> None:
        bs = Bitstream.from_dict(make_simple_config())
        fpga = FPGA(bs)
        assert "sw_0" in fpga.switches
        assert fpga.switches["sw_0"].connection_count == 2

    def test_creates_ios(self) -> None:
        bs = Bitstream.from_dict(make_simple_config())
        fpga = FPGA(bs)
        assert "in_a" in fpga.ios
        assert "out_0" in fpga.ios

    def test_empty_config(self) -> None:
        bs = Bitstream.from_dict({})
        fpga = FPGA(bs)
        assert fpga.clbs == {}
        assert fpga.switches == {}
        assert fpga.ios == {}

    def test_bitstream_property(self) -> None:
        bs = Bitstream.from_dict(make_simple_config())
        fpga = FPGA(bs)
        assert fpga.bitstream is bs


# ─── CLB Evaluation ──────────────────────────────────────────────────

class TestFPGACLBEvaluation:
    def test_evaluate_and_gate(self) -> None:
        bs = Bitstream.from_dict(make_simple_config())
        fpga = FPGA(bs)
        out = fpga.evaluate_clb(
            "clb_0",
            slice0_inputs_a=[1, 1, 0, 0],
            slice0_inputs_b=[1, 0, 0, 0],
            slice1_inputs_a=[0, 0, 0, 0],
            slice1_inputs_b=[0, 0, 0, 0],
            clock=0,
        )
        assert out.slice0.output_a == 1  # AND(1,1)
        assert out.slice0.output_b == 1  # XOR(1,0)

    def test_missing_clb_raises_key_error(self) -> None:
        bs = Bitstream.from_dict(make_simple_config())
        fpga = FPGA(bs)
        with pytest.raises(KeyError, match="not found"):
            fpga.evaluate_clb(
                "nonexistent",
                [0, 0, 0, 0], [0, 0, 0, 0],
                [0, 0, 0, 0], [0, 0, 0, 0],
                clock=0,
            )


# ─── Routing ──────────────────────────────────────────────────────────

class TestFPGARouting:
    def test_route_signals(self) -> None:
        bs = Bitstream.from_dict(make_simple_config())
        fpga = FPGA(bs)
        result = fpga.route("sw_0", {"clb_out": 1, "north": 0})
        assert result == {"east": 1, "south": 0}

    def test_missing_switch_raises_key_error(self) -> None:
        bs = Bitstream.from_dict(make_simple_config())
        fpga = FPGA(bs)
        with pytest.raises(KeyError, match="not found"):
            fpga.route("nonexistent", {"a": 1})


# ─── I/O ──────────────────────────────────────────────────────────────

class TestFPGAIO:
    def test_set_and_read_input(self) -> None:
        bs = Bitstream.from_dict(make_simple_config())
        fpga = FPGA(bs)
        fpga.set_input("in_a", 1)
        # Input pins: read_pad returns the driven value
        assert fpga.read_output("in_a") == 1

    def test_drive_and_read_output(self) -> None:
        bs = Bitstream.from_dict(make_simple_config())
        fpga = FPGA(bs)
        fpga.drive_output("out_0", 1)
        assert fpga.read_output("out_0") == 1

    def test_tristate_reads_none(self) -> None:
        bs = Bitstream.from_dict(make_simple_config())
        fpga = FPGA(bs)
        assert fpga.read_output("hiz") is None

    def test_missing_pin_set_input_raises(self) -> None:
        bs = Bitstream.from_dict(make_simple_config())
        fpga = FPGA(bs)
        with pytest.raises(KeyError, match="not found"):
            fpga.set_input("nonexistent", 1)

    def test_missing_pin_read_output_raises(self) -> None:
        bs = Bitstream.from_dict(make_simple_config())
        fpga = FPGA(bs)
        with pytest.raises(KeyError, match="not found"):
            fpga.read_output("nonexistent")

    def test_missing_pin_drive_output_raises(self) -> None:
        bs = Bitstream.from_dict(make_simple_config())
        fpga = FPGA(bs)
        with pytest.raises(KeyError, match="not found"):
            fpga.drive_output("nonexistent", 1)


# ─── End-to-End ──────────────────────────────────────────────────────

class TestEndToEnd:
    def test_and_gate_through_fabric(self) -> None:
        """Configure an AND gate, feed inputs, check output."""
        config = {
            "clbs": {
                "clb_0": {
                    "slice0": {"lut_a": and_tt(), "lut_b": [0] * 16},
                    "slice1": {"lut_a": [0] * 16, "lut_b": [0] * 16},
                },
            },
            "io": {
                "in_0": {"mode": "input"},
                "in_1": {"mode": "input"},
                "out_0": {"mode": "output"},
            },
        }
        bs = Bitstream.from_dict(config)
        fpga = FPGA(bs)

        # Set inputs
        fpga.set_input("in_0", 1)
        fpga.set_input("in_1", 1)

        # Evaluate CLB with those inputs
        out = fpga.evaluate_clb(
            "clb_0",
            slice0_inputs_a=[1, 1, 0, 0],  # AND(1,1) = 1
            slice0_inputs_b=[0, 0, 0, 0],
            slice1_inputs_a=[0, 0, 0, 0],
            slice1_inputs_b=[0, 0, 0, 0],
            clock=0,
        )

        # Drive output with CLB result
        fpga.drive_output("out_0", out.slice0.output_a)
        assert fpga.read_output("out_0") == 1

    def test_xor_gate_through_fabric(self) -> None:
        config = {
            "clbs": {
                "clb_0": {
                    "slice0": {"lut_a": xor_tt(), "lut_b": [0] * 16},
                    "slice1": {"lut_a": [0] * 16, "lut_b": [0] * 16},
                },
            },
            "io": {
                "out_0": {"mode": "output"},
            },
        }
        bs = Bitstream.from_dict(config)
        fpga = FPGA(bs)

        # XOR(1, 0) = 1
        out = fpga.evaluate_clb(
            "clb_0",
            [1, 0, 0, 0], [0, 0, 0, 0],
            [0, 0, 0, 0], [0, 0, 0, 0],
            clock=0,
        )
        fpga.drive_output("out_0", out.slice0.output_a)
        assert fpga.read_output("out_0") == 1

        # XOR(1, 1) = 0
        out = fpga.evaluate_clb(
            "clb_0",
            [1, 1, 0, 0], [0, 0, 0, 0],
            [0, 0, 0, 0], [0, 0, 0, 0],
            clock=0,
        )
        fpga.drive_output("out_0", out.slice0.output_a)
        assert fpga.read_output("out_0") == 0
