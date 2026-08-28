"""Generate deterministic full-state vectors from the Python 8086 oracle.

Run from the repository root with both Python source roots on ``PYTHONPATH``.
The committed JSON is consumed by ``python_differential.rs``; generation is
kept here so every expected hash remains independently reproducible.
"""

from __future__ import annotations

import json
import os

from intel_8086_simulator import X86Simulator


def fnv1a(data: bytes | bytearray) -> str:
    value = 0xCBF29CE484222325
    for byte in data:
        value ^= byte
        value = (value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{value:016x}"


DEFAULT = {
    "ax": 0x1234,
    "bx": 0x0200,
    "cx": 0x0003,
    "dx": 0x0040,
    "si": 0x0010,
    "di": 0x0020,
    "sp": 0x8000,
    "bp": 0x0300,
    "cs": 0,
    "ds": 0x1000,
    "ss": 0x2000,
    "es": 0x3000,
    "ip": 0,
    "cf": True,
    "pf": False,
    "af": True,
    "zf": False,
    "sf": False,
    "tf": False,
    "if": True,
    "df": False,
    "of": False,
}


def case(name: str, program: list[int], *, setup: dict | None = None, steps: int = 1,
         memory: list[list[int]] | None = None, inputs: list[list[int]] | None = None) -> dict:
    return {
        "name": name,
        "program": program,
        "setup": setup or {},
        "steps": steps,
        "memory": memory or [],
        "inputs": inputs or [],
    }


cases: list[dict] = []

# Every first byte, with safe deterministic operand bytes. This classifies the
# complete opcode map and catches accidental byte-length or fallback changes.
for opcode in range(256):
    cases.append(case(f"opcode_{opcode:02x}", [opcode, 0xC0, 0x03, 0x00, 0xF4]))

# Every extension of the densely encoded group opcodes.
for opcode in (0x80, 0x81, 0x82, 0x83, 0xFE, 0xFF, 0xF6, 0xF7, 0xD0, 0xD1, 0xD2, 0xD3):
    for extension in range(8):
        cases.append(case(
            f"group_{opcode:02x}_{extension}",
            [opcode, 0xC0 | (extension << 3), 0x03, 0x00, 0xF4],
            setup={"ax": 0x0102, "dx": 0, "cx": 2},
        ))

# All 24 effective-address forms for byte and word MOV in both directions.
for opcode in (0x88, 0x89, 0x8A, 0x8B):
    for mode in range(3):
        for rm in range(8):
            displacement = [0x05] if mode == 1 else ([0x05, 0x00] if mode == 2 else [])
            # mod=00,r/m=110 consumes a direct 16-bit address.
            if mode == 0 and rm == 6:
                displacement = [0x40, 0x00]
            cases.append(case(
                f"modrm_{opcode:02x}_{mode}_{rm}",
                [opcode, (mode << 6) | (1 << 3) | rm, *displacement, 0xF4],
                memory=[[0x10200 + offset, byte] for offset, byte in enumerate((0x78, 0x56))]
                + [[0x20300 + offset, byte] for offset, byte in enumerate((0xBC, 0x9A))]
                + [[0x10040 + offset, byte] for offset, byte in enumerate((0xEF, 0xBE))],
            ))

# Prefixes, REP string termination, control-flow outcomes, stack, and I/O.
cases.extend([
    case("segment_override", [0x26, 0xA1, 0x20, 0x00], memory=[[0x30020, 0xEF], [0x30021, 0xBE]]),
    case("rep_movsb", [0xF3, 0xA4], setup={"cx": 3}, memory=[[0x10010, 1], [0x10011, 2], [0x10012, 3]]),
    case("repe_cmpsb", [0xF3, 0xA6], setup={"cx": 3}, memory=[[0x10010, 1], [0x10011, 2], [0x10012, 9], [0x30020, 1], [0x30021, 2], [0x30022, 3]]),
    case("repne_scasb", [0xF2, 0xAE], setup={"ax": 3, "cx": 4}, memory=[[0x30020, 1], [0x30021, 2], [0x30022, 3], [0x30023, 4]]),
    case("std_stosw", [0xFD, 0xAB], setup={"di": 0x20}, steps=2),
    case("call_ret", [0xE8, 0x02, 0x00, 0xF4, 0xF4, 0xC3], steps=2),
    case("jz_taken", [0x74, 0x02, 0xF4, 0xF4, 0x90], setup={"zf": True}),
    case("jz_not_taken", [0x74, 0x02, 0x90], setup={"zf": False}),
    case("loop_taken", [0xE2, 0xFE], setup={"cx": 2}),
    case("jcxz_taken", [0xE3, 0x02], setup={"cx": 0}),
    case("in_word_wrap", [0xE5, 0xFF], inputs=[[255, 0x34], [0, 0x12]]),
    case("out_word_wrap", [0xE7, 0xFF], setup={"ax": 0x1234}),
    case("iret", [0xCF], memory=[[0x28000, 0x34], [0x28001, 0x12], [0x28002, 0x78], [0x28003, 0x56], [0x28004, 0xD7], [0x28005, 0x0D]]),
])


def apply_setup(sim: X86Simulator, record: dict) -> None:
    values = DEFAULT | record["setup"]
    for key, value in values.items():
        setattr(sim, f"_{key}", value)
    for address, value in record["memory"]:
        sim._mem[address] = value
    for port, value in record["inputs"]:
        sim._input_ports[port] = value


start = int(os.environ.get("ORACLE_START", "0"))
end = int(os.environ.get("ORACLE_END", str(len(cases))))
output = []
for record in cases[start:end]:
    sim = X86Simulator()
    sim.load(bytes(record["program"]))
    apply_setup(sim, record)
    mnemonics = []
    error = None
    try:
        for _ in range(record["steps"]):
            if sim._halted:
                break
            mnemonics.append(sim.step().mnemonic)
    except (ZeroDivisionError, OverflowError) as exc:
        error = type(exc).__name__
    state = sim.get_state()
    output.append({
        **record,
        "mnemonics": mnemonics,
        "error": error,
        "expected": {
            "ax": state.ax, "bx": state.bx, "cx": state.cx, "dx": state.dx,
            "si": state.si, "di": state.di, "sp": state.sp, "bp": state.bp,
            "cs": state.cs, "ds": state.ds, "ss": state.ss, "es": state.es,
            "ip": state.ip, "flags": state.flags, "halted": state.halted,
            "memory_hash": fnv1a(sim._mem),
            "input_hash": fnv1a(sim._input_ports),
            "output_hash": fnv1a(sim._output_ports),
        },
    })

for record in output:
    print(json.dumps(record, separators=(",", ":")))
