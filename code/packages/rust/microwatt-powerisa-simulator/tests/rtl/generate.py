#!/usr/bin/env python3
"""Generate deterministic one-instruction Microwatt RTL oracle rows.

This script intentionally has no third-party Python dependencies.  It accepts
an already-fetched exact Microwatt checkout and a checksum-verified GHDL 6.0.0
binary.  The source checkout may be disposable: the checked-in testbench is
copied into it before compilation, but no source-controlled RTL file is
changed.
"""

from __future__ import annotations

import argparse
import hashlib
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import tempfile

MICROWATT_COMMIT = "f341bed306a9a7281bf65200df8cc1f0a002617d"
GHDL_VERSION = "GHDL 6.0.0 (6.0.0.r0.ge589c698c) [Dunoon edition]"
GHDL_RELEASE_URL = (
    "https://github.com/ghdl/ghdl/releases/download/v6.0.0/"
    "ghdl-llvm-6.0.0-macos15-aarch64.tar.gz"
)
GHDL_RELEASE_SHA256 = "69beb5913a490b5971980bd045af6a63b6f52e861b444fa990bffac436587478"
SEED = 0x07BD_5A17_2026_1026
MEMORY_SIZE = 64 * 1024
CONSTANT_BASE = 0x1000
DATA_BASE = 0x2000


def d(op: int, rt: int, ra: int, imm: int) -> int:
    return (op << 26) | (rt << 21) | (ra << 16) | (imm & 0xFFFF)


def x(rs: int, ra: int, rb: int, xo: int, rc: bool = False) -> int:
    return (31 << 26) | (rs << 21) | (ra << 16) | (rb << 11) | (xo << 1) | int(rc)


def xo(rt: int, ra: int, rb: int, oe: bool, selector: int, rc: bool = False) -> int:
    return (
        (31 << 26)
        | (rt << 21)
        | (ra << 16)
        | (rb << 11)
        | (int(oe) << 10)
        | (selector << 1)
        | int(rc)
    )


def xfx(rs: int, spr: int, selector: int) -> int:
    encoded = ((spr & 0x1F) << 5) | ((spr >> 5) & 0x1F)
    return (31 << 26) | (rs << 21) | (encoded << 11) | (selector << 1)


def xl(bo: int, bi: int, bh: int, selector: int, lk: bool = False) -> int:
    return (19 << 26) | (bo << 21) | (bi << 16) | (bh << 11) | (selector << 1) | int(lk)


def md(rs: int, ra: int, sh: int, mb: int, selector: int, rc: bool = False) -> int:
    return (
        (30 << 26)
        | (rs << 21)
        | (ra << 16)
        | ((sh & 31) << 11)
        | ((mb & 31) << 6)
        | (mb & 32)
        | ((selector & 7) << 2)
        | ((sh & 32) >> 4)
        | int(rc)
    )


def mds(rs: int, ra: int, rb: int, mb: int, selector: int, rc: bool = False) -> int:
    return (
        (30 << 26)
        | (rs << 21)
        | (ra << 16)
        | (rb << 11)
        | ((mb & 63) << 6)
        | ((selector & 15) << 1)
        | int(rc)
    )


def row_definitions(
    target_cia: int,
) -> list[tuple[str, int, dict[int, int], dict[str, int]]]:
    rows: list[tuple[str, int, dict[int, int], dict[str, int]]] = []

    def add(
        name: str,
        raw: int,
        regs: dict[int, int] | None = None,
        special: dict[str, int] | None = None,
    ) -> None:
        rows.append((name, raw, regs or {}, special or {}))

    for name, raw in [
        ("mulli", d(7, 5, 3, -7)),
        ("subfic", d(8, 5, 3, -7)),
        ("cmplwi", d(10, 0, 3, 9)),
        ("cmpwi", d(11, 0, 3, -9)),
        ("addic", d(12, 5, 3, -7)),
        ("addic_record", d(13, 5, 3, -7)),
        ("addi", d(14, 5, 3, -7)),
        ("addis", d(15, 5, 3, -7)),
        ("ori", d(24, 3, 5, 0x1234)),
        ("oris", d(25, 3, 5, 0x1234)),
        ("xori", d(26, 3, 5, 0x1234)),
        ("xoris", d(27, 3, 5, 0x1234)),
        ("andi_record", d(28, 3, 5, 0x1234)),
        ("andis_record", d(29, 3, 5, 0x1234)),
    ]:
        add(name, raw)
    next_cia = target_cia + 4
    add("b_relative", (18 << 26) | 4)
    add("b_absolute_link", (18 << 26) | (next_cia & 0x03FF_FFFC) | 3)
    add("bc_relative", (16 << 26) | (20 << 21) | 4)
    add("bc_absolute_link", (16 << 26) | (20 << 21) | (next_cia & 0xFFFC) | 3)
    add("bclr", xl(20, 0, 0, 16), special={"lr": next_cia})
    add("bclrl", xl(20, 0, 0, 16, True), special={"lr": next_cia})
    add("bcctr", xl(20, 0, 0, 528), special={"ctr": next_cia})
    add("bcctrl", xl(20, 0, 0, 528, True), special={"ctr": next_cia})
    add("mcrf", (19 << 26) | (4 << 21) | (8 << 16))
    for selector, name in [
        (33, "crnor"),
        (129, "crandc"),
        (193, "crxor"),
        (225, "crnand"),
        (257, "crand"),
        (289, "creqv"),
        (417, "crorc"),
        (449, "cror"),
    ]:
        add(name, (19 << 26) | (1 << 21) | (2 << 16) | (3 << 11) | (selector << 1))
    add("isync", (19 << 26) | (150 << 1))
    add("rlwimi", (20 << 26) | (3 << 21) | (5 << 16) | (7 << 11) | (4 << 6) | (20 << 1))
    add("rlwinm", (21 << 26) | (3 << 21) | (5 << 16) | (7 << 11) | (4 << 6) | (20 << 1))
    add("rlwnm", (23 << 26) | (3 << 21) | (5 << 16) | (4 << 11) | (4 << 6) | (20 << 1))
    for name, raw in [
        ("rldicl", md(3, 5, 7, 9, 0)),
        ("rldicr", md(3, 5, 7, 9, 1)),
        ("rldic", md(3, 5, 7, 9, 2)),
        ("rldimi", md(3, 5, 7, 9, 3)),
        ("rldcl", mds(3, 5, 4, 9, 8)),
        ("rldcr", mds(3, 5, 4, 9, 9)),
    ]:
        add(name, raw)
    for op, name in [
        (32, "lwz"),
        (33, "lwzu"),
        (34, "lbz"),
        (35, "lbzu"),
        (36, "stw"),
        (37, "stwu"),
        (38, "stb"),
        (39, "stbu"),
        (40, "lhz"),
        (41, "lhzu"),
        (42, "lha"),
        (43, "lhau"),
        (44, "sth"),
        (45, "sthu"),
    ]:
        add(name, d(op, 3, 4, 8), {4: DATA_BASE})
    for name, raw in [
        ("ld", (58 << 26) | (3 << 21) | (4 << 16) | 8),
        ("ldu", (58 << 26) | (3 << 21) | (4 << 16) | 9),
        ("lwa", (58 << 26) | (3 << 21) | (4 << 16) | 10),
        ("std", (62 << 26) | (3 << 21) | (4 << 16) | 8),
        ("stdu", (62 << 26) | (3 << 21) | (4 << 16) | 9),
    ]:
        add(name, raw, {4: DATA_BASE})
    for selector, name in [
        (8, "subfc"),
        (10, "addc"),
        (40, "subf"),
        (104, "neg"),
        (136, "subfe"),
        (138, "adde"),
        (200, "subfze"),
        (202, "addze"),
        (232, "subfme"),
        (233, "mulld"),
        (234, "addme"),
        (235, "mullw"),
        (266, "add"),
        (457, "divdu"),
        (459, "divwu"),
        (489, "divd"),
        (491, "divw"),
    ]:
        add(name, xo(5, 3, 4, False, selector, True))
    for selector, name in [
        (9, "mulhdu"),
        (11, "mulhwu"),
        (73, "mulhd"),
        (75, "mulhw"),
        (24, "slw"),
        (27, "sld"),
        (28, "and"),
        (60, "andc"),
        (124, "nor"),
        (284, "eqv"),
        (316, "xor"),
        (412, "orc"),
        (444, "or"),
        (476, "nand"),
        (536, "srw"),
        (539, "srd"),
        (792, "sraw"),
        (794, "srad"),
        (26, "cntlzw"),
        (58, "cntlzd"),
        (538, "cnttzw"),
        (570, "cnttzd"),
        (922, "extsh"),
        (954, "extsb"),
        (986, "extsw"),
    ]:
        add(name, x(3, 5, 4, selector, True))
    for selector, name in [
        (122, "popcntb"),
        (154, "prtyw"),
        (186, "prtyd"),
        (378, "popcntw"),
        (506, "popcntd"),
    ]:
        add(name, x(3, 5, 4, selector))
    add("srawi", x(3, 5, 7, 824, True))
    add("sradi_0", x(3, 5, 7, 826, True))
    add("sradi_1", x(3, 5, 7, 827, True))
    add("cmpw", x(0, 3, 4, 0))
    add("cmplw", x(0, 3, 4, 32))
    add("mfcr", x(5, 0, 0, 19))
    add("mtcrf", (31 << 26) | (3 << 21) | (0xFF << 12) | (144 << 1))
    for spr, name in [(1, "mfxer"), (8, "mflr"), (9, "mfctr")]:
        add(name, xfx(5, spr, 339))
    for spr, name in [(1, "mtxer"), (8, "mtlr"), (9, "mtctr")]:
        add(name, xfx(3, spr, 467))
    for selector, name in [
        (21, "ldx"),
        (53, "ldux"),
        (23, "lwzx"),
        (55, "lwzux"),
        (87, "lbzx"),
        (119, "lbzux"),
        (149, "stdx"),
        (181, "stdux"),
        (151, "stwx"),
        (183, "stwux"),
        (215, "stbx"),
        (247, "stbux"),
        (279, "lhzx"),
        (311, "lhzux"),
        (341, "lwax"),
        (373, "lwaux"),
        (343, "lhax"),
        (375, "lhaux"),
        (407, "sthx"),
        (439, "sthux"),
        (532, "ldbrx"),
        (534, "lwbrx"),
        (660, "stdbrx"),
        (662, "stwbrx"),
        (790, "lhbrx"),
        (918, "sthbrx"),
    ]:
        add(name, x(3, 4, 5, selector), {4: DATA_BASE, 5: 8})
    add("sync", (31 << 26) | (598 << 1))
    add("lwsync", (31 << 26) | (1 << 21) | (598 << 1))
    add("eieio", (31 << 26) | (854 << 1))
    add("attn", 0x0000_0200)
    # Required source-lock boundary matrices.
    add(
        "addo_ov32_so0",
        xo(5, 3, 4, True, 266, True),
        {3: 0x7FFF_FFFF, 4: 1},
        {"xer": 0},
    )
    add(
        "addo_ov32_so1",
        xo(5, 3, 4, True, 266, True),
        {3: 0x7FFF_FFFF, 4: 1},
        {"xer": 1 << 31},
    )
    add(
        "mullwo_no_overflow",
        xo(5, 3, 4, True, 235, True),
        {3: 0x7FFF, 4: 2},
        {"xer": 0},
    )
    add(
        "mullwo_positive_overflow",
        xo(5, 3, 4, True, 235, True),
        {3: 0x7FFF_FFFF, 4: 2},
        {"xer": 0},
    )
    add(
        "mullwo_negative_overflow",
        xo(5, 3, 4, True, 235, True),
        {3: 0x8000_0000, 4: 2},
        {"xer": 0},
    )
    add("mulhwu_shape", x(5, 3, 4, 11, True), {3: 0xFFFF_FFFF, 4: 0xFFFF_FFFF})
    add("mulhw_shape", x(5, 3, 4, 75, True), {3: 0x8000_0000, 4: 2})
    add(
        "divw_shape",
        xo(5, 3, 4, False, 491, True),
        {3: (-10) & 0xFFFF_FFFF_FFFF_FFFF, 4: 3},
    )
    add("divwu_shape", xo(5, 3, 4, False, 459, True), {3: 0xFFFF_FFFF, 4: 3})
    add("sraw_sign_extend_1", x(3, 5, 4, 792, True), {3: 0x8000_0001, 4: 1})
    add("sraw_sign_extend_32", x(3, 5, 4, 792, True), {3: 0x8000_0001, 4: 32})
    add("srawi_sign_extend_31", x(3, 5, 31, 824, True), {3: 0x8000_0001})
    add("prtyw_byte_lsb", x(3, 5, 0, 154), {3: 0x0102_0408_0102_0304})
    add("prtyd_byte_lsb", x(3, 5, 0, 186), {3: 0x2})
    # Normative Power ISA 1.3.3 slash-reserved fields are ignored by the
    # processor.  These rows execute the pinned RTL with each important decode
    # shape nonzero rather than treating slash fields as validity guards.
    add("cmpw_slash", x(0, 3, 4, 0, True) | (1 << 22))
    add("mcrf_slash", (19 << 26) | (4 << 21) | (8 << 16) | (3 << 11) | 1)
    add("bclr_bh_slash", xl(20, 0, 0x1c, 16), special={"lr": next_cia})
    add("isync_slash", (19 << 26) | (0x7FFF << 11) | (150 << 1) | 1)
    add("popcntb_slash", x(3, 5, 31, 122, True))
    add("mfcr_slash", x(5, 0, 0, 19, True) | (0x1FF << 11))
    add("mulhwu_oe_slash", x(3, 5, 4, 11, True) | (1 << 10))
    add("sync_slash", (31 << 26) | (7 << 23) | (0x3FF << 11) | (598 << 1) | 1)
    add("eieio_slash", (31 << 26) | (0x7FFF << 11) | (854 << 1) | 1)
    return rows


def splitmix64(value: int) -> int:
    value = (value + 0x9E3779B97F4A7C15) & 0xFFFF_FFFF_FFFF_FFFF
    value = ((value ^ (value >> 30)) * 0xBF58476D1CE4E5B9) & 0xFFFF_FFFF_FFFF_FFFF
    value = ((value ^ (value >> 27)) * 0x94D049BB133111EB) & 0xFFFF_FFFF_FFFF_FFFF
    return value ^ (value >> 31)


def encode_image(
    raw: int, overrides: dict[int, int], special: dict[str, int]
) -> tuple[bytes, list[int], dict[str, int], int]:
    gprs = [splitmix64(SEED + index) for index in range(32)]
    for index, value in overrides.items():
        gprs[index] = value & 0xFFFF_FFFF_FFFF_FFFF
    state = {
        "cr": 0xA5A5_5A5A,
        "xer": (1 << 31) | (1 << 29) | (1 << 18),
        "lr": 0x1234,
        "ctr": 0x5678,
    }
    state.update(special)
    words = [d(58, index, 0, CONSTANT_BASE + index * 8) for index in range(32)]
    words += [
        d(58, 30, 0, 0x1100),
        (31 << 26) | (30 << 21) | (0xFF << 12) | (144 << 1),
        d(58, 30, 0, 0x1108),
        xfx(30, 1, 467),
        d(58, 30, 0, 0x1110),
        xfx(30, 8, 467),
        d(58, 30, 0, 0x1118),
        xfx(30, 9, 467),
        d(58, 30, 0, CONSTANT_BASE + 30 * 8),
    ]
    target_cia = len(words) * 4
    words += [raw, 0x0000_0200]
    memory = bytearray(MEMORY_SIZE)
    for index, word in enumerate(words):
        struct.pack_into("<I", memory, index * 4, word)
    for index, value in enumerate(gprs):
        struct.pack_into("<Q", memory, CONSTANT_BASE + index * 8, value)
    for offset, key in [(0, "cr"), (8, "xer"), (16, "lr"), (24, "ctr")]:
        struct.pack_into("<Q", memory, 0x1100 + offset, state[key])
    for index in range(256):
        memory[DATA_BASE + index] = (index * 73 + 19) & 0xFF
    return bytes(memory), gprs, state, target_cia


def parse_result(
    output: str, initial_memory: bytes
) -> tuple[list[int], dict[str, int], str, str]:
    gprs = [None] * 32
    special: dict[str, int] = {}
    for match in re.finditer(r"GPR(\d+) ([0-9A-F]{16})", output):
        gprs[int(match.group(1))] = int(match.group(2), 16)
    for key in ("LR", "CTR", "CR", "XER"):
        matches = re.findall(rf"{key} ([0-9A-F]{{16}})", output)
        if matches:
            special[key.lower()] = int(matches[-1], 16)
    if any(value is None for value in gprs) or set(special) != {
        "lr",
        "ctr",
        "cr",
        "xer",
    }:
        raise RuntimeError("incomplete Microwatt state dump")
    memory = bytearray(initial_memory)
    writes = []
    for data_hex, address_hex, select_hex in re.findall(
        r"RAM writing ([0-9A-F]{16}) to ([0-9A-F]+) sel:([0-9A-F]+)", output
    ):
        data, address, select = (
            int(data_hex, 16),
            int(address_hex, 16),
            int(select_hex, 16),
        )
        if address < MEMORY_SIZE:
            for byte in range(8):
                if select & (1 << byte):
                    memory[address + byte] = (data >> (byte * 8)) & 0xFF
            writes.append(f"{address:04x}:{data:016x}:{select:02x}")
    if "end of test" not in output:
        raise RuntimeError("Microwatt did not reach ATTN state dump")
    return (
        [int(v) for v in gprs],
        special,
        hashlib.sha256(memory).hexdigest(),
        ",".join(writes) or "-",
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--microwatt", required=True, type=Path)
    parser.add_argument("--ghdl", required=True, type=Path)
    parser.add_argument(
        "--output", type=Path, default=Path(__file__).with_name("vectors.tsv")
    )
    args = parser.parse_args()
    source = args.microwatt.resolve()
    ghdl = args.ghdl.resolve()
    commit = subprocess.check_output(
        ["git", "-C", str(source), "rev-parse", "HEAD"], text=True
    ).strip()
    if commit != MICROWATT_COMMIT:
        raise SystemExit(f"wrong Microwatt commit: {commit}")
    if (
        subprocess.check_output([str(ghdl), "--version"], text=True).splitlines()[0]
        != GHDL_VERSION
    ):
        raise SystemExit("wrong GHDL build")
    harness = Path(__file__).with_name("microwatt_oracle_tb.vhdl")
    staged = source / "core_tb.vhdl"
    original_harness = staged.read_bytes()
    shutil.copyfile(harness, staged)
    env = os.environ.copy()
    env["PATH"] = f"{ghdl.parent}:{env['PATH']}"
    try:
        subprocess.run(
            ["make", "-C", str(source), "-B", "-j4", "core_tb"],
            env=env,
            check=True,
            timeout=300,
        )
    finally:
        staged.write_bytes(original_harness)
    executable = source / "core_tb"
    _, _, _, target_cia = encode_image(0, {}, {})
    rows = row_definitions(target_cia)
    lines = [
        "name\traw\tcia_before\tcia_after\tbefore_gprs\tafter_gprs\tbefore_lr\tafter_lr\tbefore_ctr\tafter_ctr\tbefore_cr\tafter_cr\tbefore_xer\tafter_xer\tmemory_sha256\twrites"
    ]
    for index, (name, raw, overrides, special_overrides) in enumerate(rows, 1):
        image, before_gprs, before_special, actual_cia = encode_image(
            raw, overrides, special_overrides
        )
        if actual_cia != target_cia:
            raise AssertionError("target CIA drift")
        with tempfile.TemporaryDirectory(prefix="microwatt-rtl-row-") as run_dir:
            Path(run_dir, "main_ram.bin").write_bytes(image)
            result = subprocess.run(
                [str(executable)],
                cwd=run_dir,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                check=False,
                timeout=60,
            )
        after_gprs, after_special, memory_hash, writes = parse_result(
            result.stdout, image
        )
        cia_after = target_cia + 4
        before_regs = ",".join(f"{value:016x}" for value in before_gprs)
        after_regs = ",".join(f"{value:016x}" for value in after_gprs)
        fields = [
            name,
            f"{raw:08x}",
            f"{target_cia:016x}",
            f"{cia_after:016x}",
            before_regs,
            after_regs,
            f"{before_special['lr']:016x}",
            f"{after_special['lr']:016x}",
            f"{before_special['ctr']:016x}",
            f"{after_special['ctr']:016x}",
            f"{before_special['cr']:08x}",
            f"{after_special['cr']:08x}",
            f"{before_special['xer']:08x}",
            f"{after_special['xer']:08x}",
            memory_hash,
            writes,
        ]
        lines.append("\t".join(fields))
        print(f"[{index}/{len(rows)}] {name}")
    content = ("\n".join(lines) + "\n").encode()
    args.output.write_bytes(content)
    digest = hashlib.sha256(content).hexdigest()
    manifest = args.output.with_name("manifest.txt")
    manifest.write_text(
        "\n".join(
            [
                f"microwatt_commit={MICROWATT_COMMIT}",
                f"ghdl_version={GHDL_VERSION}",
                f"ghdl_release_url={GHDL_RELEASE_URL}",
                f"ghdl_release_sha256={GHDL_RELEASE_SHA256}",
                f"harness_sha256={hashlib.sha256(harness.read_bytes()).hexdigest()}",
                f"generator_sha256={hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}",
                "command=python3 tests/rtl/generate.py --microwatt <exact-checkout> --ghdl <verified-ghdl>",
                f"seed=0x{SEED:016x}",
                f"row_count={len(rows)}",
                f"vectors_sha256={digest}",
            ]
        )
        + "\n"
    )


if __name__ == "__main__":
    main()
