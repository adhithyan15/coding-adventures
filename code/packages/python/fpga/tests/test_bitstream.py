"""Tests for Bitstream — FPGA configuration data.

Coverage targets:
- from_dict parsing (CLBs, routing, I/O)
- from_json file loading
- Default values
- Edge cases (empty config, missing fields)
"""

from __future__ import annotations

import json
import tempfile
from pathlib import Path

from fpga.bitstream import Bitstream, CLBConfig, IOConfig, RouteConfig, SliceConfig

# ─── SliceConfig / CLBConfig ──────────────────────────────────────────

class TestDataclasses:
    def test_slice_config_defaults(self) -> None:
        sc = SliceConfig(lut_a=[0] * 16, lut_b=[0] * 16)
        assert sc.ff_a_enabled is False
        assert sc.ff_b_enabled is False
        assert sc.carry_enabled is False

    def test_slice_config_custom(self) -> None:
        sc = SliceConfig(
            lut_a=[1] * 16, lut_b=[0] * 16,
            ff_a_enabled=True, carry_enabled=True,
        )
        assert sc.ff_a_enabled is True
        assert sc.carry_enabled is True

    def test_clb_config(self) -> None:
        s0 = SliceConfig(lut_a=[0] * 16, lut_b=[0] * 16)
        s1 = SliceConfig(lut_a=[1] * 16, lut_b=[1] * 16)
        cfg = CLBConfig(slice0=s0, slice1=s1)
        assert cfg.slice0.lut_a == [0] * 16
        assert cfg.slice1.lut_a == [1] * 16

    def test_route_config(self) -> None:
        rc = RouteConfig(source="clb_out", destination="east")
        assert rc.source == "clb_out"
        assert rc.destination == "east"

    def test_io_config(self) -> None:
        ic = IOConfig(mode="output")
        assert ic.mode == "output"


# ─── from_dict ────────────────────────────────────────────────────────

class TestFromDict:
    def test_empty_config(self) -> None:
        bs = Bitstream.from_dict({})
        assert bs.clbs == {}
        assert bs.routing == {}
        assert bs.io == {}
        assert bs.lut_k == 4

    def test_custom_lut_k(self) -> None:
        bs = Bitstream.from_dict({"lut_k": 3})
        assert bs.lut_k == 3

    def test_parses_clb(self) -> None:
        and_tt = [0] * 16
        and_tt[3] = 1
        data = {
            "clbs": {
                "clb_0": {
                    "slice0": {
                        "lut_a": and_tt,
                        "lut_b": [0] * 16,
                        "ff_a": True,
                        "carry": True,
                    },
                    "slice1": {
                        "lut_a": [0] * 16,
                        "lut_b": [0] * 16,
                    },
                }
            }
        }
        bs = Bitstream.from_dict(data)
        assert "clb_0" in bs.clbs
        assert bs.clbs["clb_0"].slice0.lut_a == and_tt
        assert bs.clbs["clb_0"].slice0.ff_a_enabled is True
        assert bs.clbs["clb_0"].slice0.carry_enabled is True
        assert bs.clbs["clb_0"].slice1.ff_a_enabled is False

    def test_parses_routing(self) -> None:
        data = {
            "routing": {
                "sw_0": [
                    {"src": "clb_out", "dst": "east"},
                    {"src": "north", "dst": "south"},
                ]
            }
        }
        bs = Bitstream.from_dict(data)
        assert "sw_0" in bs.routing
        assert len(bs.routing["sw_0"]) == 2
        assert bs.routing["sw_0"][0].source == "clb_out"
        assert bs.routing["sw_0"][0].destination == "east"

    def test_parses_io(self) -> None:
        data = {
            "io": {
                "pin_A": {"mode": "input"},
                "pin_B": {"mode": "output"},
                "pin_C": {"mode": "tristate"},
            }
        }
        bs = Bitstream.from_dict(data)
        assert bs.io["pin_A"].mode == "input"
        assert bs.io["pin_B"].mode == "output"
        assert bs.io["pin_C"].mode == "tristate"

    def test_missing_slice_defaults(self) -> None:
        """If slice0/slice1 dicts are empty, LUTs default to all zeros."""
        data = {
            "clbs": {
                "clb_0": {
                    "slice0": {},
                    "slice1": {},
                }
            }
        }
        bs = Bitstream.from_dict(data)
        assert bs.clbs["clb_0"].slice0.lut_a == [0] * 16

    def test_missing_io_mode_defaults_to_input(self) -> None:
        data = {"io": {"pin_0": {}}}
        bs = Bitstream.from_dict(data)
        assert bs.io["pin_0"].mode == "input"


# ─── from_json ────────────────────────────────────────────────────────

class TestFromJSON:
    def test_load_from_file(self) -> None:
        data = {
            "clbs": {
                "clb_0": {
                    "slice0": {"lut_a": [0] * 16, "lut_b": [0] * 16},
                    "slice1": {"lut_a": [0] * 16, "lut_b": [0] * 16},
                }
            },
            "io": {"led": {"mode": "output"}},
        }

        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".json", delete=False
        ) as f:
            json.dump(data, f)
            f.flush()
            path = f.name

        bs = Bitstream.from_json(path)
        assert "clb_0" in bs.clbs
        assert bs.io["led"].mode == "output"

        # Cleanup
        Path(path).unlink()

    def test_load_with_path_object(self) -> None:
        data = {"io": {"pin": {"mode": "input"}}}

        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".json", delete=False
        ) as f:
            json.dump(data, f)
            f.flush()
            path = Path(f.name)

        bs = Bitstream.from_json(path)
        assert bs.io["pin"].mode == "input"

        path.unlink()
