#!/usr/bin/env python3
"""Regenerate the frozen ZPU4-small corpus from checksum-pinned VHDL."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import tempfile
import urllib.request
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ORACLE = ROOT / "oracle"
MANIFEST = ORACLE / "sources.json"
DEFAULT_OUTPUT = ROOT / "tests" / "fixtures" / "zpu4_small_oracle.json"
BASE = "https://raw.githubusercontent.com/zylin/zpu/{commit}/{path}"


@dataclass(frozen=True)
class Boundary:
    time_fs: int
    case_number: int
    raw: int
    pc: int
    sp: int
    top: int
    next_word: int
    idim_after: bool
    in_interrupt_after: bool
    break_after: bool
    executed_pc_after: int
    executed_sp_after: int

    def as_json(self) -> dict[str, int | bool | str]:
        return {
            "time_ns": self.time_fs // 1_000_000,
            "raw": f"{self.raw:02x}",
            "pc": self.pc,
            "sp": self.sp,
            "top": self.top,
            "next": self.next_word,
            "idim_after": self.idim_after,
            "in_interrupt_after": self.in_interrupt_after,
            "break_after": self.break_after,
        }


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def run(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        command, cwd=cwd, text=True, capture_output=True, check=False
    )
    if result.returncode:
        raise RuntimeError(
            f"command failed ({result.returncode}): {' '.join(command)}\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )
    return result


def fetch_sources(directory: Path) -> tuple[dict[str, object], dict[str, Path]]:
    manifest = json.loads(MANIFEST.read_text())
    commit = manifest["commit"]
    paths: dict[str, Path] = {}
    for entry in manifest["files"]:
        path = entry["path"]
        data = urllib.request.urlopen(
            BASE.format(commit=commit, path=path), timeout=30
        ).read()
        actual = sha256(data)
        if actual != entry["sha256"]:
            raise RuntimeError(f"checksum mismatch for {path}: {actual}")
        destination = directory / Path(path).name
        destination.write_bytes(data)
        paths[path] = destination

    config_path = "zpu/hdl/zpu4/core/zpu_config.vhd"
    config = paths[config_path].read_bytes()
    old = b"constant DontCareValue   : std_logic                    := 'X';"
    new = b"constant DontCareValue   : std_logic                    := '0';"
    if config.count(old) != 1:
        raise RuntimeError("pinned DontCareValue token was not found exactly once")
    derived = config.replace(old, new)
    if sha256(derived) != manifest["derived_config_sha256"]:
        raise RuntimeError("derived zero-valued configuration checksum mismatch")
    paths[config_path].write_bytes(derived)
    return manifest, paths


def parse_vcd(path: Path) -> list[Boundary]:
    symbols: dict[str, str] = {}
    scope: list[str] = []
    lines = path.open(encoding="ascii")
    for line in lines:
        text = line.strip()
        if text.startswith("$scope"):
            scope.append(text.split()[2])
        elif text.startswith("$upscope"):
            scope.pop()
        elif text.startswith("$var"):
            fields = text.split()
            symbols[fields[3]] = ".".join(scope + [fields[4].split("[")[0]])
        elif text == "$enddefinitions $end":
            break

    wanted_names = {
        "oracle_control.case_number",
        "zpu_oracle_tb.core.begin_inst",
        "zpu_oracle_tb.core.trace_opcode",
        "zpu_oracle_tb.core.trace_pc",
        "zpu_oracle_tb.core.trace_sp",
        "zpu_oracle_tb.core.trace_topofstack",
        "zpu_oracle_tb.core.trace_topofstackb",
        "zpu_oracle_tb.core.idim_flag",
        "zpu_oracle_tb.core.ininterrupt",
        "zpu_oracle_tb.core.break",
        "zpu_oracle_tb.core.pc",
        "zpu_oracle_tb.core.sp",
    }
    by_name = {name: symbol for symbol, name in symbols.items() if name in wanted_names}
    missing = wanted_names - by_name.keys()
    if missing:
        raise RuntimeError(f"VCD omitted required signals: {sorted(missing)}")

    values: dict[str, str] = {}
    boundaries: list[Boundary] = []
    current_time: int | None = None
    changes: list[tuple[str, str]] = []

    def binary(name: str) -> int:
        value = values[by_name[name]]
        if any(bit not in "01" for bit in value):
            raise RuntimeError(f"non-binary VCD value for {name}: {value}")
        return int(value, 2)

    def scalar(name: str) -> bool:
        value = values[by_name[name]]
        if value not in ("0", "1"):
            raise RuntimeError(f"non-binary VCD scalar for {name}: {value}")
        return value == "1"

    def finish_time(time_fs: int, time_changes: list[tuple[str, str]]) -> None:
        old_begin = values.get(by_name["zpu_oracle_tb.core.begin_inst"])
        for symbol, value in time_changes:
            values[symbol] = value
        new_begin = values.get(by_name["zpu_oracle_tb.core.begin_inst"])
        if new_begin == "1" and old_begin != "1":
            boundaries.append(
                Boundary(
                    time_fs=time_fs,
                    case_number=binary("oracle_control.case_number"),
                    raw=binary("zpu_oracle_tb.core.trace_opcode"),
                    pc=binary("zpu_oracle_tb.core.trace_pc"),
                    sp=binary("zpu_oracle_tb.core.trace_sp") << 2,
                    top=binary("zpu_oracle_tb.core.trace_topofstack"),
                    next_word=binary("zpu_oracle_tb.core.trace_topofstackb"),
                    idim_after=scalar("zpu_oracle_tb.core.idim_flag"),
                    in_interrupt_after=scalar("zpu_oracle_tb.core.ininterrupt"),
                    break_after=scalar("zpu_oracle_tb.core.break"),
                    executed_pc_after=binary("zpu_oracle_tb.core.pc"),
                    executed_sp_after=binary("zpu_oracle_tb.core.sp") << 2,
                )
            )

    for line in lines:
        text = line.strip()
        if not text:
            continue
        if text.startswith("#"):
            if current_time is not None:
                finish_time(current_time, changes)
            current_time = int(text[1:])
            changes = []
        elif text[0] in "01xXzZuU":
            changes.append((text[1:], text[0]))
        elif text[0] == "b":
            value, symbol = text[1:].split()
            changes.append((symbol, value))
    if current_time is not None:
        finish_time(current_time, changes)
    return boundaries


REPORT = re.compile(
    r"@(?P<time>\d+)(?P<unit>fs|ps|ns|us|ms):\(report note\): "
    r"(?P<kind>ZPU_RAM_WRITE|ZPU_BUS_READ|ZPU_BUS_WRITE) "
    r"(?P<case>\d+) (?P<body>.*)$"
)
UNITS = {
    "fs": 1,
    "ps": 1_000,
    "ns": 1_000_000,
    "us": 1_000_000_000,
    "ms": 1_000_000_000_000,
}


def parse_reports(log: str) -> dict[int, list[dict[str, int | str]]]:
    events: dict[int, list[dict[str, int | str]]] = {}
    for line in log.splitlines():
        match = REPORT.search(line)
        if not match:
            continue
        event: dict[str, int | str] = {
            "time_fs": int(match["time"]) * UNITS[match["unit"]],
            "kind": match["kind"],
        }
        fields = match["body"].split()
        if match["kind"] == "ZPU_RAM_WRITE":
            port, word_index, value = fields
            event.update(port=port, address=int(word_index) * 4, value=int(value, 16))
        else:
            address, value = fields
            event.update(address=int(address, 16) & 0xFFFC, value=int(value, 16))
        events.setdefault(int(match["case"]), []).append(event)
    return events


def validate_log(log: str) -> None:
    if log.count("ZPU_ORACLE_PASS") != 1:
        raise RuntimeError("oracle did not emit exactly one PASS sentinel")
    done = [int(value) for value in re.findall(r"ZPU_CASE_DONE (\d+)", log)]
    if done != list(range(260)):
        raise RuntimeError("oracle did not finish every case in order")
    for line in log.splitlines():
        lower = line.lower()
        if "assertion warning" in lower:
            if "@0ms" not in lower or "numeric_std.to_integer: metavalue" not in lower:
                raise RuntimeError(f"unexpected oracle warning: {line}")
        if "report failure" in lower and "break instruction encountered" not in lower:
            raise RuntimeError(f"unexpected oracle failure diagnostic: {line}")
        if "error:" in lower or "oracle dual-port write collision" in lower:
            raise RuntimeError(f"unexpected oracle diagnostic: {line}")


def mnemonic(raw: int) -> str:
    if raw >= 0x80:
        return "IM"
    if raw >= 0x60:
        return f"LOADSP {((raw & 0x0F) | ((~raw) & 0x10))}"
    if raw >= 0x40:
        return f"STORESP {((raw & 0x0F) | ((~raw) & 0x10))}"
    if raw >= 0x20:
        return f"EMULATE {raw & 0x1F}"
    if raw >= 0x10:
        return f"ADDSP {raw & 0x0F}"
    return {
        0x00: "BREAK",
        0x02: "PUSHSP",
        0x04: "POPPC",
        0x05: "ADD",
        0x06: "AND",
        0x07: "OR",
        0x08: "LOAD",
        0x09: "NOT",
        0x0A: "FLIP",
        0x0C: "STORE",
        0x0D: "POPSP",
    }.get(raw, "NOP")


def initial_words(raw: int, scenario: int) -> list[int]:
    words = [0x0B0B0B0B] * 8192
    for offset in range(32):
        words[(8190 + offset) % 8192] = 0x40000000 + offset
    words[64] = 0xA5A55A5A
    words[8190] = 0x00000100
    words[8191] = 0x11223344
    words[0] = (raw << 24) | 0x000B0B0B
    if scenario in (1, 2):
        words[8190] = 0x00008000
    elif scenario in (3, 4):
        words[0] = 0x81820B0B
    return words


def memory_digest(words: list[int]) -> str:
    data = b"".join(word.to_bytes(4, "big") for word in words)
    return sha256(data)


def generate(ghdl: Path) -> dict[str, object]:
    with tempfile.TemporaryDirectory(prefix="zpu-oracle-") as temporary:
        directory = Path(temporary)
        manifest, paths = fetch_sources(directory)
        for support in ("oracle_control.vhd", "dualport_ram.vhd", "zpu_oracle_tb.vhd"):
            shutil.copy2(ORACLE / support, directory / support)
        analyze = run(
            [
                str(ghdl),
                "-a",
                "--std=08",
                paths["zpu/hdl/zpu4/core/zpu_config.vhd"].name,
                paths["zpu/hdl/zpu4/core/zpupkg.vhd"].name,
                "oracle_control.vhd",
                "dualport_ram.vhd",
                paths["zpu/hdl/zpu4/core/zpu_core_small.vhd"].name,
                "zpu_oracle_tb.vhd",
            ],
            directory,
        )
        analysis_diagnostics = analyze.stdout + analyze.stderr
        if "error:" in analysis_diagnostics.lower():
            raise RuntimeError(analysis_diagnostics)
        run([str(ghdl), "-e", "--std=08", "zpu_oracle_tb"], directory)
        simulation = run(
            [
                str(ghdl),
                "-r",
                "--std=08",
                "zpu_oracle_tb",
                "--assert-level=none",
                "--vcd=oracle.vcd",
            ],
            directory,
        )
        log = simulation.stdout + simulation.stderr
        validate_log(log)
        boundaries = parse_vcd(directory / "oracle.vcd")
        reports = parse_reports(log)

    grouped: dict[int, list[Boundary]] = {}
    for boundary in boundaries:
        grouped.setdefault(boundary.case_number, []).append(boundary)
    if sorted(grouped) != list(range(260)):
        raise RuntimeError("VCD did not contain every oracle case")

    isolated: list[dict[str, object]] = []
    for raw in range(256):
        observed = grouped[raw]
        if len(observed) < 2:
            raise RuntimeError(f"case {raw} has fewer than two boundaries")
        first, second = observed[:2]
        if first.raw != raw or first.pc != 0:
            raise RuntimeError(f"case {raw} fetched the wrong first opcode")
        post = first if raw == 0 else second
        interval_events = (
            []
            if raw == 0
            else [
                event
                for event in reports.get(raw, [])
                if first.time_fs < int(event["time_fs"]) < second.time_fs
            ]
        )
        words = initial_words(raw, 0)
        ram_writes: list[dict[str, int | str]] = []
        bus_transaction: dict[str, int | str] | None = None
        for event in interval_events:
            kind = str(event["kind"])
            if kind == "ZPU_RAM_WRITE":
                words[int(event["address"]) // 4] = int(event["value"])
                ram_writes.append(
                    {
                        "port": str(event["port"]),
                        "address": int(event["address"]),
                        "value": int(event["value"]),
                    }
                )
            else:
                if bus_transaction is not None:
                    raise RuntimeError(f"case {raw} emitted multiple bus transactions")
                bus_transaction = {
                    "kind": "read" if kind.endswith("READ") else "write",
                    "address": int(event["address"]),
                    "value": int(event["value"]),
                }
        isolated.append(
            {
                "raw": f"{raw:02x}",
                "mnemonic": mnemonic(raw),
                "pc_before": first.pc,
                "sp_before": first.sp,
                "top_before": first.top,
                "next_before": first.next_word,
                "pc_after": first.executed_pc_after if raw == 0 else second.pc,
                "sp_after": first.executed_sp_after if raw == 0 else post.sp,
                "top_after": post.top,
                "next_after": post.next_word,
                "idim_after": first.idim_after,
                "in_interrupt_after": first.in_interrupt_after,
                "break_after": first.break_after,
                "ram_writes": ram_writes,
                "bus_transaction": bus_transaction,
                "memory_sha256_after": memory_digest(words),
            }
        )

    scenario_names = {
        256: "external-load",
        257: "external-store",
        258: "continued-immediate",
        259: "immediate-interrupt-deferral",
    }
    scenarios = []
    for case_number, name in scenario_names.items():
        raw = 0x08 if case_number == 256 else 0x0C if case_number == 257 else 0x81
        scenarios.append(
            {
                "case": case_number,
                "name": name,
                "initial_memory_sha256": memory_digest(
                    initial_words(raw, case_number - 255)
                ),
                "boundaries": [boundary.as_json() for boundary in grouped[case_number]],
                "events": [
                    {key: value for key, value in event.items() if key != "time_fs"}
                    | {"time_ns": int(event["time_fs"]) // 1_000_000}
                    for event in reports.get(case_number, [])
                ],
            }
        )

    return {
        "schema": "coding-adventures.zpu4-small-ghdl.v1",
        "source_commit": manifest["commit"],
        "source_sha256": {
            entry["path"]: entry["sha256"] for entry in manifest["files"]
        },
        "derived_config_sha256": manifest["derived_config_sha256"],
        "initial_external_read_value": 0x89ABCDEF,
        "isolated": isolated,
        "scenarios": scenarios,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--ghdl", required=True, type=Path)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    arguments = parser.parse_args()
    corpus = (
        json.dumps(generate(arguments.ghdl.resolve()), indent=2, sort_keys=True) + "\n"
    )
    if arguments.check:
        if not arguments.output.exists() or arguments.output.read_text() != corpus:
            raise SystemExit(
                f"frozen corpus differs from regeneration: {arguments.output}"
            )
        print(f"verified {arguments.output}")
    else:
        arguments.output.parent.mkdir(parents=True, exist_ok=True)
        arguments.output.write_text(corpus)
        print(f"wrote {arguments.output}")


if __name__ == "__main__":
    main()
