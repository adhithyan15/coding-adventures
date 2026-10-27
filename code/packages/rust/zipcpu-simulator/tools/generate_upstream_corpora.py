#!/usr/bin/env python3
"""Reproduce the bounded ZipCPU C++ and RTL evidence corpora offline.

The caller supplies a checkout of the public upstream repository at the pinned
commit.  This script verifies every source hash before compiling either
harness.  It does not download sources and it never invokes the Rust model.
"""

from __future__ import annotations

import argparse
import hashlib
import pathlib
import shutil
import subprocess
import tempfile


PINNED_COMMIT = "42606d2d6ef55df313772232977621b2d72f0159"
HASHES = {
    "rtl/core/zipcore.v": "b0db25e2017c2e6cb6715d8901806c3bdd12ae98879e0723ff5b58b750c7938e",
    "rtl/core/idecode.v": "f8e29e5e5d387a7fefc73dd456448bcbd41d65ed4d80e5d23d8541ae4e507c4e",
    "rtl/core/cpuops.v": "54707430127001eb7e716eb14ff3b733ecfeac437a4a1cec6906bb7befa48a7e",
    "rtl/core/div.v": "ca220c1db020a4b3365d8c29fe9cdb24dfbc93a5db82827cb818b888d834abf5",
    "rtl/core/mpyop.v": "2b831b42a8b14671925c2284ce2e838378f3d738b1293eca74ede4c861aed5a4",
    "rtl/core/slowmpy.v": "449527da756405fab2b30ad7121609a0a0d7555c7fba68a6d0c81eb0f90502d4",
    "rtl/core/memops.v": "b2d0f99a20263acc860d91c835f05df6ad0b2713a4a03ac1acb96da47946ed61",
    "sim/cpp/zsim.cpp": "cecb8e72d741c81a69e68982673021d273259da0d680dfce5158b881b9374e1b",
    "sim/cpp/twoc.cpp": "55466d21033429bd06cc28a075c96588db5e07973a04284652d7f7e6a521764a",
    "sim/cpp/twoc.h": "32110672ffd716f340da93ac2532ba485a64cd0f147d1f94798db938b723b1cb",
    "sim/cpp/zipelf.h": "4cecdc651d2d7f5235f10fe19fb2ac361e304b096dec7fac5a3e5ba5351e7bfa",
}


def digest(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify(upstream: pathlib.Path) -> None:
    commit = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=upstream,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    if commit != PINNED_COMMIT:
        raise SystemExit(f"expected upstream {PINNED_COMMIT}, found {commit}")
    for relative, expected in HASHES.items():
        actual = digest(upstream / relative)
        if actual != expected:
            raise SystemExit(f"hash mismatch for {relative}: {actual}")


ZSIM_HARNESS = r'''
#define main zipcpu_original_main
#include "{zsim}"
#undef main

bool iself(const char *) {{ return false; }}
void elfread(const char *, uint32_t &, ELFSECTION **&) {{}}

int main() {{
    const uint32_t values[] = {{0u, 1u, 2u, 7u, 15u, 0x1234u,
        0x7fffffffu, 0x80000000u, 0xfffffffeu, 0xffffffffu}};
    const unsigned ops[] = {{0, 1, 2, 3, 4, 8, 9, 12, 13, 16, 17}};
    const uint32_t initial_flags[] = {{0u, 0xau}};
    std::puts("word,a,b,flags_in,result,flags_out,pc");
    for (unsigned op : ops)
    for (uint32_t a : values)
    for (uint32_t b : values)
    for (uint32_t flags : initial_flags) {{
        // zsim predates the current RTL's overflow-adjusted N relation and
        // uses a shorter overflow test. Keep only their common arithmetic.
        uint32_t arithmetic_result = (op == 2) ? a + b : a - b;
        if ((op == 0 || op == 2 || op == 16)
                && (((a ^ arithmetic_result) & 0x80000000u) != 0))
            continue;
        ZIPMACHINE machine;
        machine.init(nullptr);
        machine.m_r[1] = a;
        machine.m_r[2] = b;
        machine.m_r[14] = flags;
        machine.m_r[15] = 0x100;
        uint32_t word = (1u << 27) | (op << 22)
            | ((op == 13 ? 0u : 1u) << 18) | (2u << 14);
        machine.execute(word);
        std::printf("%08x,%08x,%08x,%x,%08x,%x,%08x\n", word, a, b,
            flags, machine.m_r[1], machine.m_r[14] & 15, machine.m_r[15]);
    }}
}}
'''


RTL_HARNESS = r'''
#include "Vcpuops.h"
#include "verilated.h"
#include <cstdint>
#include <cstdio>

static void tick(Vcpuops &top) {
    top.i_clk = 0; top.eval();
    top.i_clk = 1; top.eval();
}

int main(int argc, char **argv) {
    Verilated::commandArgs(argc, argv);
    const uint32_t values[] = {0u, 1u, 2u, 7u, 31u, 32u, 63u,
        0x12345678u, 0x7fffffffu, 0x80000000u, 0xfffffffeu, 0xffffffffu};
    std::puts("op,a,b,result,flags");
    for (unsigned op = 0; op <= 13; ++op)
    for (uint32_t a : values)
    for (uint32_t b : values) {
        Vcpuops top;
        top.i_reset = 1; top.i_stb = 0; top.i_a = 0; top.i_b = 0; top.i_op = 0;
        tick(top);
        top.i_reset = 0; top.i_stb = 1; top.i_a = a; top.i_b = b; top.i_op = op;
        tick(top);
        top.i_stb = 0;
        for (unsigned clocks = 0; clocks < 80 && !top.o_valid; ++clocks)
            tick(top);
        if (!top.o_valid)
            return 2;
        std::printf("%x,%08x,%08x,%08x,%x\n", op, a, b, top.o_c,
            static_cast<unsigned>(top.o_f));
    }
}
'''


def run(command: list[str], cwd: pathlib.Path) -> None:
    subprocess.run(command, cwd=cwd, check=True)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--upstream", required=True, type=pathlib.Path)
    parser.add_argument(
        "--output-dir",
        type=pathlib.Path,
        default=pathlib.Path(__file__).parents[1] / "tests" / "fixtures",
    )
    args = parser.parse_args()
    upstream = args.upstream.resolve()
    output = args.output_dir.resolve()
    verify(upstream)
    for tool in ("g++", "verilator"):
        if shutil.which(tool) is None:
            raise SystemExit(f"required developer tool not found: {tool}")

    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="zipcpu-corpora-") as temporary:
        temp = pathlib.Path(temporary)
        zsim_source = temp / "zsim_harness.cpp"
        zsim_source.write_text(
            ZSIM_HARNESS.format(zsim=(upstream / "sim/cpp/zsim.cpp").as_posix())
        )
        zsim_binary = temp / "zsim_harness"
        run(
            [
                "g++",
                "-std=c++17",
                "-O2",
                "-Wno-deprecated-declarations",
                "-I",
                str(upstream / "sim/cpp"),
                str(zsim_source),
                str(upstream / "sim/cpp/twoc.cpp"),
                "-o",
                str(zsim_binary),
            ],
            upstream,
        )
        zsim = subprocess.run([str(zsim_binary)], check=True, capture_output=True).stdout
        (output / "zsim-rev3-common.csv").write_bytes(zsim)

        rtl_source = temp / "rtl_harness.cpp"
        rtl_source.write_text(RTL_HARNESS)
        object_dir = temp / "obj"
        run(
            [
                "verilator",
                "--cc",
                str(upstream / "rtl/core/cpuops.v"),
                str(upstream / "rtl/core/mpyop.v"),
                str(upstream / "rtl/core/slowmpy.v"),
                "--top-module",
                "cpuops",
                "-GOPT_MPY=36",
                "-GOPT_SHIFTS=1",
                "-GOPT_LOWPOWER=0",
                "--exe",
                str(rtl_source),
                "--Mdir",
                str(object_dir),
                "--build",
                "-j",
                "2",
            ],
            upstream,
        )
        rtl = subprocess.run(
            [str(object_dir / "Vcpuops")], check=True, capture_output=True
        ).stdout
        (output / "rtl-rev3-cpuops.csv").write_bytes(rtl)

    print(f"wrote zsim rows={len(zsim.splitlines()) - 1}")
    print(f"wrote rtl rows={len(rtl.splitlines()) - 1}")


if __name__ == "__main__":
    main()
