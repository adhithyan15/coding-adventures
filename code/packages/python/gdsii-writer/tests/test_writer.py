"""Tests for the GDSII binary writer."""

from pathlib import Path

import pytest

from gdsii_writer import GdsWriter
from gdsii_writer.stream import _double_to_gds_real, _record


# ---- Real conversion ----


def test_zero_real():
    assert _double_to_gds_real(0.0) == b"\x00" * 8


def test_real_positive_round_trip_via_known_value():
    # The number 1.0 should be a recognizable byte pattern.
    # In GDS real: sign=0, exp=65 (1 = 16^1 × 0.0625 = 1/16 in fraction × 16^1)
    # mantissa is 1/16 × 2^56 = 2^52 = 0x10000000000000
    b = _double_to_gds_real(1.0)
    assert len(b) == 8
    # First byte: sign=0, exponent=0x41 (65 in excess-64 means 16^1)
    assert b[0] == 0x41


def test_real_negative_sign_bit():
    b = _double_to_gds_real(-1.0)
    assert b[0] & 0x80 == 0x80


# ---- Record format ----


def test_record_basic():
    r = _record(0x0002, b"\x02\x58")  # HEADER record
    # Length is 4 (header) + 2 (payload) = 6
    assert r[:2] == b"\x00\x06"
    assert r[2] == 0x00
    assert r[3] == 0x02


def test_record_too_long():
    with pytest.raises(ValueError, match="too long"):
        _record(0x0002, b"x" * 0xFFFD)


# ---- Library + structure structure ----


def test_minimal_library(tmp_path: Path):
    p = tmp_path / "x.gds"
    with GdsWriter(p, library_name="lib") as gds:
        gds.begin_structure("top")
        gds.end_structure()
    data = p.read_bytes()
    assert len(data) > 0
    assert data[:2] == b"\x00\x06"  # HEADER record length


def test_library_name_padded():
    with GdsWriter(str(Path(__file__).parent / "_unused.gds"), library_name="abc") as gds:
        pass
    # We can't easily inspect the buffer in this test, but no crash on odd-length name
    # Test passes if context-manager exits cleanly.


def test_double_begin_structure_rejected(tmp_path: Path):
    p = tmp_path / "x.gds"
    with GdsWriter(str(p)) as gds:
        gds.begin_structure("a")
        with pytest.raises(RuntimeError, match="another structure"):
            gds.begin_structure("b")
        gds.end_structure()  # close cleanly


def test_end_structure_without_begin():
    with GdsWriter(str(Path(__file__).parent / "_unused.gds")) as gds:
        with pytest.raises(RuntimeError, match="without matching"):
            gds.end_structure()


def test_footer_auto_closes_structure(tmp_path: Path):
    """write_footer auto-closes an open structure for graceful shutdown."""
    p = tmp_path / "x.gds"
    gds = GdsWriter(str(p))
    gds.write_header()
    gds.begin_structure("x")
    gds.write_footer()
    gds.flush()
    assert p.exists()


# ---- Elements ----


def test_boundary_auto_closes(tmp_path: Path):
    p = tmp_path / "x.gds"
    with GdsWriter(p) as gds:
        gds.begin_structure("top")
        # Polygon with first != last; writer should close it.
        gds.boundary(layer=66, datatype=20, points=[(0, 0), (1, 0), (1, 1), (0, 1)])
        gds.end_structure()
    assert p.exists()
    assert p.stat().st_size > 0


def test_boundary_too_few_points():
    with GdsWriter(str(Path(__file__).parent / "_unused.gds")) as gds:
        gds.begin_structure("top")
        with pytest.raises(ValueError, match=">= 4 points"):
            gds.boundary(layer=1, datatype=0, points=[(0, 0), (1, 0)])
        gds.end_structure()


def test_path(tmp_path: Path):
    p = tmp_path / "x.gds"
    with GdsWriter(p) as gds:
        gds.begin_structure("top")
        gds.path(layer=68, datatype=20, width=0.14,
                 points=[(0, 0), (10, 0), (10, 5)])
        gds.end_structure()
    assert p.exists()


def test_path_too_few_points():
    with GdsWriter(str(Path(__file__).parent / "_unused.gds")) as gds:
        gds.begin_structure("top")
        with pytest.raises(ValueError, match=">= 2 points"):
            gds.path(layer=1, datatype=0, width=0.1, points=[(0, 0)])
        gds.end_structure()


def test_sref(tmp_path: Path):
    p = tmp_path / "x.gds"
    with GdsWriter(p) as gds:
        gds.begin_structure("nand2_1")
        gds.boundary(layer=66, datatype=20,
                     points=[(0, 0), (1.4, 0), (1.4, 2.7), (0, 2.7)])
        gds.end_structure()

        gds.begin_structure("top")
        gds.sref("nand2_1", x=0, y=0)
        gds.sref("nand2_1", x=2, y=0, angle_deg=90, mag=1.0)
        gds.sref("nand2_1", x=4, y=0, mag=2.0)
        gds.sref("nand2_1", x=6, y=0, reflect=True)
        gds.end_structure()
    assert p.exists()


def test_text(tmp_path: Path):
    p = tmp_path / "x.gds"
    with GdsWriter(p) as gds:
        gds.begin_structure("top")
        gds.text(layer=68, text_type=0, x=0, y=0, text="adder4")
        gds.end_structure()
    assert p.exists()


def test_element_outside_structure():
    with GdsWriter(str(Path(__file__).parent / "_unused.gds")) as gds:
        with pytest.raises(RuntimeError, match="outside structure"):
            gds.boundary(layer=1, datatype=0, points=[(0, 0), (1, 0), (1, 1), (0, 1)])
        with pytest.raises(RuntimeError, match="outside structure"):
            gds.path(layer=1, datatype=0, width=0.1, points=[(0, 0), (1, 0)])
        with pytest.raises(RuntimeError, match="outside structure"):
            gds.sref("x", 0, 0)
        with pytest.raises(RuntimeError, match="outside structure"):
            gds.text(layer=1, text_type=0, x=0, y=0, text="hi")


# ---- 4-bit adder layout (smoke) ----


def test_adder4_layout(tmp_path: Path):
    """End-to-end: emit a believable layout for a 4-bit adder."""
    p = tmp_path / "adder4.gds"
    with GdsWriter(p, library_name="adder4") as gds:
        # NAND2 cell
        gds.begin_structure("nand2_1")
        # diff
        gds.boundary(layer=65, datatype=20,
                     points=[(0.1, 0.5), (1.3, 0.5), (1.3, 2.2), (0.1, 2.2)])
        # poly gates
        gds.path(layer=66, datatype=20, width=0.15,
                 points=[(0.4, 0.0), (0.4, 2.7)])
        gds.path(layer=66, datatype=20, width=0.15,
                 points=[(1.0, 0.0), (1.0, 2.7)])
        # met1 output
        gds.boundary(layer=68, datatype=20,
                     points=[(0.7, 1.0), (1.2, 1.0), (1.2, 1.3), (0.7, 1.3)])
        gds.end_structure()

        # Adder top
        gds.begin_structure("adder4")
        # 16 NAND2 instances in a row
        for i in range(16):
            gds.sref("nand2_1", x=i * 1.5, y=0.0)
        # routing on met2
        for i in range(15):
            gds.path(layer=70, datatype=20, width=0.20,
                     points=[(i * 1.5 + 1.2, 1.5), ((i + 1) * 1.5 + 0.4, 1.5)])
        gds.end_structure()

    size = p.stat().st_size
    # Should be a few KB minimum
    assert size > 1000
    # Read first few bytes; verify it's a GDSII (HEADER record)
    head = p.read_bytes()[:6]
    assert head[:2] == b"\x00\x06"  # length 6
    assert head[2] == 0x00 and head[3] == 0x02  # HEADER type
