"""Driver: shell out to yosys / nextpnr / icepack / iceprog."""

from __future__ import annotations

import shutil
import subprocess
from dataclasses import dataclass, field
from pathlib import Path

from hdl_ir import HIR

from real_fpga_export.verilog_writer import write_verilog


@dataclass
class ToolchainOptions:
    yosys: str = "yosys"
    nextpnr_ice40: str = "nextpnr-ice40"
    icepack: str = "icepack"
    iceprog: str = "iceprog"
    timeout_s: int = 600


@dataclass
class ToolchainResult:
    """What the driver produced. Useful for downstream verification."""

    verilog_path: Path
    json_path: Path | None = None
    asc_path: Path | None = None
    bin_path: Path | None = None
    log_lines: list[str] = field(default_factory=list)


def to_ice40(
    hir: HIR,
    top: str,
    *,
    pcf: Path | None = None,
    out_dir: Path = Path("build"),
    part: str = "hx1k",
    package: str = "tq144",
    opts: ToolchainOptions | None = None,
    skip_missing: bool = False,
) -> ToolchainResult:
    if opts is None:
        opts = ToolchainOptions()
    """Run the full toolchain. ``skip_missing=True`` returns after Verilog
    emission if any of the external tools aren't on PATH (useful for testing)."""
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    v_path = out_dir / f"{top}.v"
    write_verilog(hir, v_path)

    result = ToolchainResult(verilog_path=v_path)

    if skip_missing and not shutil.which(opts.yosys):
        result.log_lines.append("yosys not found; skipping toolchain")
        return result

    json_path = out_dir / f"{top}.json"
    _run([
        opts.yosys, "-q",
        "-p", f"synth_ice40 -top {top} -json {json_path}",
        str(v_path),
    ], opts.timeout_s, result)
    result.json_path = json_path

    if not pcf:
        result.log_lines.append("no PCF provided; skipping place-route")
        return result

    asc_path = out_dir / f"{top}.asc"
    _run([
        opts.nextpnr_ice40,
        f"--{part}",
        "--package", package,
        "--json", str(json_path),
        "--pcf", str(pcf),
        "--asc", str(asc_path),
    ], opts.timeout_s, result)
    result.asc_path = asc_path

    bin_path = out_dir / f"{top}.bin"
    _run([opts.icepack, str(asc_path), str(bin_path)], opts.timeout_s, result)
    result.bin_path = bin_path

    return result


def program_ice40(
    bin_path: Path, opts: ToolchainOptions | None = None
) -> None:
    """Flash a bitstream to a real iCE40 board via iceprog."""
    if opts is None:
        opts = ToolchainOptions()
    if not shutil.which(opts.iceprog):
        raise RuntimeError(f"{opts.iceprog} not on PATH")
    subprocess.run([opts.iceprog, str(bin_path)], check=True, timeout=opts.timeout_s)


def _run(cmd: list[str], timeout_s: int, result: ToolchainResult) -> None:
    """Run a tool; record stdout/stderr in result.log_lines."""
    if not shutil.which(cmd[0]):
        raise RuntimeError(f"{cmd[0]!r} not on PATH")
    proc = subprocess.run(
        cmd, check=False, capture_output=True, text=True, timeout=timeout_s
    )
    if proc.stdout:
        result.log_lines.append(proc.stdout)
    if proc.stderr:
        result.log_lines.append(proc.stderr)
    if proc.returncode != 0:
        raise RuntimeError(
            f"{cmd[0]} failed with exit code {proc.returncode}\n"
            f"stdout:\n{proc.stdout}\nstderr:\n{proc.stderr}"
        )
