#!/usr/bin/env python3
"""Source-independent schema and semantic witness for the frozen ZPU corpus."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / "tests" / "fixtures" / "zpu4_small_oracle.json"


def word(memory: bytearray, address: int) -> int:
    address &= 0x7FFC
    return int.from_bytes(memory[address : address + 4], "big")


def put(memory: bytearray, address: int, value: int) -> None:
    address &= 0x7FFC
    memory[address : address + 4] = (value & 0xFFFF_FFFF).to_bytes(4, "big")


def stack_add(sp: int, words: int) -> int:
    return (((sp >> 2) + words) & 0x1FFF) << 2


def initial(raw: int, scenario: int = 0) -> bytearray:
    memory = bytearray((0x0B, 0x0B, 0x0B, 0x0B) * 8192)
    for offset in range(32):
        put(memory, stack_add(0x7FF8, offset), 0x4000_0000 + offset)
    put(memory, 0x0100, 0xA5A5_5A5A)
    put(memory, 0x7FF8, 0x0000_0100)
    put(memory, 0x7FFC, 0x1122_3344)
    put(memory, 0, (raw << 24) | 0x000B_0B0B)
    if scenario in (1, 2):
        put(memory, 0x7FF8, 0x0000_8000)
    elif scenario in (3, 4):
        put(memory, 0, 0x8182_0B0B)
    return memory


def classify(raw: int) -> str:
    if raw >= 0x80:
        return "IM"
    if raw >= 0x60:
        return f"LOADSP {(raw & 15) | ((~raw) & 16)}"
    if raw >= 0x40:
        return f"STORESP {(raw & 15) | ((~raw) & 16)}"
    if raw >= 0x20:
        return f"EMULATE {raw & 31}"
    if raw >= 0x10:
        return f"ADDSP {raw & 15}"
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


def reverse_bits(value: int) -> int:
    return int(f"{value:032b}"[::-1], 2)


def execute(raw: int) -> dict[str, object]:
    memory = initial(raw)
    sp = 0x7FF8
    pc = 1
    top = word(memory, sp)
    next_word = word(memory, stack_add(sp, 1))
    idim = False
    breaking = False

    if raw >= 0x80:
        sp = stack_add(sp, -1)
        signed = raw & 0x7F
        if signed & 0x40:
            signed -= 0x80
        put(memory, sp, signed)
        idim = True
    elif raw >= 0x60:
        offset = (raw & 15) | ((~raw) & 16)
        value = word(memory, stack_add(sp, offset))
        sp = stack_add(sp, -1)
        put(memory, sp, value)
    elif raw >= 0x40:
        offset = (raw & 15) | ((~raw) & 16)
        put(memory, stack_add(sp, offset), top)
        sp = stack_add(sp, 1)
    elif raw >= 0x20:
        sp = stack_add(sp, -1)
        put(memory, sp, 1)
        pc = (raw & 31) << 5
    elif raw >= 0x10:
        operand = word(memory, stack_add(sp, raw & 15))
        put(memory, sp, top + operand)
    elif raw == 0x00:
        breaking = True
    elif raw == 0x02:
        sp = stack_add(sp, -1)
        put(memory, sp, 0x7FF8)
    elif raw == 0x04:
        pc = top & 0x7FFF
        sp = stack_add(sp, 1)
    elif raw in (0x05, 0x06, 0x07):
        value = (
            next_word + top
            if raw == 0x05
            else next_word & top
            if raw == 0x06
            else next_word | top
        )
        sp = stack_add(sp, 1)
        put(memory, sp, value)
    elif raw == 0x08:
        put(memory, sp, word(memory, top))
    elif raw == 0x09:
        put(memory, sp, ~top)
    elif raw == 0x0A:
        put(memory, sp, reverse_bits(top))
    elif raw == 0x0C:
        put(memory, top, next_word)
        sp = stack_add(sp, 2)
    elif raw == 0x0D:
        sp = top & 0x7FFC

    return {
        "pc_after": pc,
        "sp_after": sp,
        "top_after": word(memory, sp),
        "next_after": word(memory, stack_add(sp, 1)),
        "idim_after": idim,
        "break_after": breaking,
        "memory_sha256_after": hashlib.sha256(memory).hexdigest(),
    }


def main() -> None:
    corpus = json.loads(CORPUS.read_text())
    assert corpus["schema"] == "coding-adventures.zpu4-small-ghdl.v1"
    assert corpus["source_commit"] == "e29aef5f7aa741557d0eb16ecf13e974d55789ff"
    assert len(corpus["source_sha256"]) == 6
    rows = corpus["isolated"]
    assert len(rows) == 256
    assert [row["raw"] for row in rows] == [f"{raw:02x}" for raw in range(256)]

    for raw, row in enumerate(rows):
        assert row["mnemonic"] == classify(raw), f"classification {raw:02x}"
        assert row["pc_before"] == 0 and row["sp_before"] == 0x7FF8
        assert row["top_before"] == 0x100 and row["next_before"] == 0x1122_3344
        assert not row["in_interrupt_after"]
        expected = execute(raw)
        for key, value in expected.items():
            assert row[key] == value, f"{key} for {raw:02x}"

        replay = initial(raw)
        for write in row["ram_writes"]:
            assert write["port"] in ("A", "B")
            put(replay, write["address"], write["value"])
        assert hashlib.sha256(replay).hexdigest() == row["memory_sha256_after"]
        assert row["bus_transaction"] is None

    scenarios = {scenario["name"]: scenario for scenario in corpus["scenarios"]}
    assert set(scenarios) == {
        "external-load",
        "external-store",
        "continued-immediate",
        "immediate-interrupt-deferral",
    }
    load = scenarios["external-load"]
    assert load["boundaries"][0]["top"] == 0x8000
    assert load["boundaries"][1]["top"] == 0x89AB_CDEF
    assert any(event["kind"] == "ZPU_BUS_READ" for event in load["events"])
    store = scenarios["external-store"]
    assert any(
        event["kind"] == "ZPU_BUS_WRITE"
        and event["address"] == 0x8000
        and event["value"] == 0x1122_3344
        for event in store["events"]
    )
    immediate = scenarios["continued-immediate"]["boundaries"]
    assert [row["raw"] for row in immediate[:3]] == ["81", "82", "0b"]
    assert [row["top"] for row in immediate[:3]] == [0x100, 1, 130]
    deferred = scenarios["immediate-interrupt-deferral"]["boundaries"]
    assert [row["in_interrupt_after"] for row in deferred[:4]] == [
        False,
        False,
        False,
        True,
    ]
    assert deferred[4]["pc"] == 0x20 and deferred[4]["top"] == 3
    print("verified 256 isolated vectors and 4 multistep GHDL scenarios")


if __name__ == "__main__":
    main()
