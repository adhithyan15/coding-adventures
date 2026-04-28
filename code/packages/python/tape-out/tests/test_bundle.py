"""Tests for tape-out bundle assembly."""

from pathlib import Path

import pytest

from tape_out import (
    PadLocation,
    Shuttle,
    TapeoutBundle,
    TapeoutMetadata,
    validate_for_chipignite,
    write_bundle,
)


def make_basic_bundle(tmp_path: Path) -> TapeoutBundle:
    # Create some dummy source files
    (tmp_path / "src").mkdir()
    files = {}
    for name in ("adder4.gds", "adder4.lef", "adder4.def",
                 "adder4.v", "drc.rpt", "lvs.rpt"):
        p = tmp_path / "src" / name
        p.write_text(f"placeholder for {name}\n")
        files[name.split(".")[-1] if name != "adder4.v" else "verilog"] = p
    # Map: adder4.gds -> "gds", etc.
    keymap = {"gds": files["gds"], "lef": files["lef"], "def": files["def"],
              "verilog": files["verilog"], "drc_report": files["rpt"], "lvs_report": files["rpt"]}
    # Fix the rpt mapping
    drc = tmp_path / "src" / "drc.rpt"
    lvs = tmp_path / "src" / "lvs.rpt"
    keymap = {
        "gds": tmp_path / "src" / "adder4.gds",
        "lef": tmp_path / "src" / "adder4.lef",
        "def": tmp_path / "src" / "adder4.def",
        "verilog": tmp_path / "src" / "adder4.v",
        "drc_report": drc,
        "lvs_report": lvs,
    }

    metadata = TapeoutMetadata(
        project_name="adder4",
        designer="Test Designer",
        email="test@example.com",
        top_module="adder4",
        clock_frequency_mhz=50.0,
    )
    bundle = TapeoutBundle(metadata=metadata)
    bundle.files = keymap
    bundle.signoff = {"drc": "clean", "lvs": "clean"}
    return bundle


# ---- write_bundle ----


def test_write_bundle_creates_dir(tmp_path: Path):
    bundle = make_basic_bundle(tmp_path)
    out = write_bundle(bundle, tmp_path / "tapeout")
    assert out.exists()
    assert (out / "manifest.yaml").exists()
    assert (out / "README.md").exists()


def test_write_bundle_copies_files(tmp_path: Path):
    bundle = make_basic_bundle(tmp_path)
    out = write_bundle(bundle, tmp_path / "tapeout")
    assert (out / "adder4.gds").exists()
    assert (out / "adder4.lef").exists()
    assert (out / "adder4.def").exists()


def test_manifest_contains_metadata(tmp_path: Path):
    bundle = make_basic_bundle(tmp_path)
    out = write_bundle(bundle, tmp_path / "tapeout")
    text = (out / "manifest.yaml").read_text()
    assert "project_name: adder4" in text
    assert "designer: Test Designer" in text
    assert "shuttle: chipignite_open_mpw" in text
    assert "frequency_mhz: 50.0" in text


def test_manifest_includes_pad_locations(tmp_path: Path):
    bundle = make_basic_bundle(tmp_path)
    bundle.pad_locations = [
        PadLocation("a[0]", "input", x=0.0, y=100.0),
        PadLocation("y", "output", x=1000.0, y=100.0),
    ]
    out = write_bundle(bundle, tmp_path / "tapeout")
    text = (out / "manifest.yaml").read_text()
    assert "pads:" in text
    assert "a[0]" in text


def test_manifest_includes_signoff(tmp_path: Path):
    bundle = make_basic_bundle(tmp_path)
    out = write_bundle(bundle, tmp_path / "tapeout")
    text = (out / "manifest.yaml").read_text()
    assert "signoff:" in text
    assert "drc: clean" in text


def test_readme_emitted(tmp_path: Path):
    bundle = make_basic_bundle(tmp_path)
    out = write_bundle(bundle, tmp_path / "tapeout")
    text = (out / "README.md").read_text()
    assert "# adder4" in text
    assert "Test Designer" in text


def test_missing_source_file_skipped_silently(tmp_path: Path):
    bundle = make_basic_bundle(tmp_path)
    bundle.files["extra"] = tmp_path / "nonexistent.txt"
    # Should not crash
    write_bundle(bundle, tmp_path / "tapeout")


# ---- validate_for_chipignite ----


def test_validate_passes_basic_bundle(tmp_path: Path):
    bundle = make_basic_bundle(tmp_path)
    bundle.pad_locations = [PadLocation("a", "input", 0, 0)]
    r = validate_for_chipignite(bundle)
    assert r.passed
    assert r.errors == []


def test_validate_missing_metadata():
    bundle = TapeoutBundle(metadata=TapeoutMetadata(
        project_name="", designer="", email="", top_module=""
    ))
    r = validate_for_chipignite(bundle)
    assert not r.passed
    assert any("project_name" in e for e in r.errors)
    assert any("designer" in e for e in r.errors)
    assert any("email" in e for e in r.errors)
    assert any("top_module" in e for e in r.errors)


def test_validate_missing_required_files():
    metadata = TapeoutMetadata(
        project_name="x", designer="d", email="e@x", top_module="x"
    )
    bundle = TapeoutBundle(metadata=metadata)
    r = validate_for_chipignite(bundle)
    assert not r.passed
    # All 6 required files missing
    assert sum(1 for e in r.errors if "missing required file" in e) == 6


def test_validate_dirty_drc_fails(tmp_path: Path):
    bundle = make_basic_bundle(tmp_path)
    bundle.signoff = {"drc": "5 violations", "lvs": "clean"}
    r = validate_for_chipignite(bundle)
    assert not r.passed
    assert any("DRC not clean" in e for e in r.errors)


def test_validate_dirty_lvs_fails(tmp_path: Path):
    bundle = make_basic_bundle(tmp_path)
    bundle.signoff = {"drc": "clean", "lvs": "mismatch"}
    r = validate_for_chipignite(bundle)
    assert not r.passed
    assert any("LVS not clean" in e for e in r.errors)


def test_validate_no_pads_warns(tmp_path: Path):
    bundle = make_basic_bundle(tmp_path)
    # No pads
    r = validate_for_chipignite(bundle)
    assert any("pad_locations" in w for w in r.warnings)


def test_validate_other_shuttle_no_pad_warning(tmp_path: Path):
    bundle = make_basic_bundle(tmp_path)
    bundle.metadata.shuttle = Shuttle.TINY_TAPEOUT
    r = validate_for_chipignite(bundle)
    # TinyTapeout doesn't require pads
    assert all("pad_locations" not in w for w in r.warnings)
