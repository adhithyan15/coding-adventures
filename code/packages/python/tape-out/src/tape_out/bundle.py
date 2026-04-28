"""Tape-out bundle assembly for the Efabless chipIgnite shuttle.

A tape-out bundle is a directory of files the foundry needs to manufacture
the chip:
- GDSII layout
- LEF / DEF
- behavioral Verilog (for testbench)
- DRC / LVS / antenna / density signoff reports
- timing report
- manifest.yaml with project metadata + pad locations
- IP statement
- README
"""

from __future__ import annotations

import shutil
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path


class Shuttle(Enum):
    CHIPIGNITE_OPEN_MPW = "chipignite_open_mpw"
    CHIPIGNITE_PAID_MPW = "chipignite_paid_mpw"
    TINY_TAPEOUT = "tiny_tapeout"


@dataclass(frozen=True, slots=True)
class PadLocation:
    name: str
    direction: str  # "input" | "output" | "inout" | "power" | "ground"
    x: float  # micrometers
    y: float


@dataclass
class TapeoutMetadata:
    project_name: str
    designer: str
    email: str
    shuttle: Shuttle = Shuttle.CHIPIGNITE_OPEN_MPW
    pdk: str = "sky130A"
    pdk_version: str | None = None
    license: str = "Apache-2.0"
    top_module: str = ""
    git_url: str | None = None
    clock_frequency_mhz: float = 0.0
    clock_signal: str = "clk"
    vdd_voltage: float = 1.8


@dataclass
class TapeoutBundle:
    metadata: TapeoutMetadata
    files: dict[str, Path] = field(default_factory=dict)  # logical-name -> source path
    pad_locations: list[PadLocation] = field(default_factory=list)
    signoff: dict[str, str] = field(default_factory=dict)
    # signoff: e.g. {"drc": "clean", "lvs": "clean", "density.met1": "32%"}


@dataclass
class ValidationReport:
    passed: bool
    errors: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)


REQUIRED_FILES = ("gds", "lef", "def", "verilog", "drc_report", "lvs_report")


def write_bundle(bundle: TapeoutBundle, out_dir: Path) -> Path:
    """Copy bundle files into out_dir and emit manifest.yaml + README.md.

    Returns the path to the created bundle directory."""
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    # Copy each file
    for _logical_name, source in bundle.files.items():
        dest = out_dir / source.name
        if source.exists():
            shutil.copy2(source, dest)

    # Manifest
    manifest_path = out_dir / "manifest.yaml"
    manifest_path.write_text(_render_manifest(bundle))

    # README
    readme_path = out_dir / "README.md"
    readme_path.write_text(_render_readme(bundle))

    return out_dir


def _render_manifest(bundle: TapeoutBundle) -> str:
    """Render manifest.yaml — minimal hand-rolled YAML."""
    m = bundle.metadata
    lines: list[str] = [
        f"project_name: {m.project_name}",
        f"designer: {m.designer}",
        f"email: {m.email}",
        f"shuttle: {m.shuttle.value}",
        f"pdk: {m.pdk}",
    ]
    if m.pdk_version is not None:
        lines.append(f"pdk_version: {m.pdk_version}")
    lines.append(f"license: {m.license}")
    lines.append(f"top_module: {m.top_module}")
    if m.git_url is not None:
        lines.append(f"git_url: {m.git_url}")

    lines.append("")
    lines.append("clock:")
    lines.append(f"  primary: {m.clock_signal}")
    lines.append(f"  frequency_mhz: {m.clock_frequency_mhz}")
    lines.append("")
    lines.append("power:")
    lines.append(f"  vdd_voltage: {m.vdd_voltage}")

    if bundle.pad_locations:
        lines.append("")
        lines.append("pads:")
        for pad in bundle.pad_locations:
            lines.append(f"  - {{name: '{pad.name}', dir: {pad.direction}, x: {pad.x}, y: {pad.y}}}")

    if bundle.signoff:
        lines.append("")
        lines.append("signoff:")
        for k, v in bundle.signoff.items():
            lines.append(f"  {k}: {v}")

    return "\n".join(lines) + "\n"


def _render_readme(bundle: TapeoutBundle) -> str:
    m = bundle.metadata
    return (
        f"# {m.project_name}\n\n"
        f"Tape-out bundle for {m.shuttle.value}.\n\n"
        f"- Designer: {m.designer} <{m.email}>\n"
        f"- PDK: {m.pdk}"
        + (f" ({m.pdk_version})\n" if m.pdk_version else "\n")
        + f"- Top module: {m.top_module}\n"
        + f"- License: {m.license}\n\n"
        + "## Files\n\n"
        + "\n".join(f"- {logical_name}: `{path.name}`"
                   for logical_name, path in bundle.files.items())
        + "\n"
    )


def validate_for_chipignite(bundle: TapeoutBundle) -> ValidationReport:
    """Check that the bundle meets chipIgnite acceptance criteria."""
    report = ValidationReport(passed=True)

    if not bundle.metadata.project_name:
        report.errors.append("project_name is required")
    if not bundle.metadata.designer:
        report.errors.append("designer is required")
    if not bundle.metadata.email:
        report.errors.append("email is required")
    if not bundle.metadata.top_module:
        report.errors.append("top_module is required")

    for required in REQUIRED_FILES:
        if required not in bundle.files:
            report.errors.append(f"missing required file: {required}")

    # Check signoff state
    drc = bundle.signoff.get("drc", "")
    lvs = bundle.signoff.get("lvs", "")
    if drc != "clean":
        report.errors.append(f"DRC not clean: {drc!r}")
    if lvs != "clean":
        report.errors.append(f"LVS not clean: {lvs!r}")

    # Pads required for an open MPW
    if bundle.metadata.shuttle == Shuttle.CHIPIGNITE_OPEN_MPW and not bundle.pad_locations:
        report.warnings.append("no pad_locations specified; chipIgnite may reject")

    if report.errors:
        report.passed = False
    return report
