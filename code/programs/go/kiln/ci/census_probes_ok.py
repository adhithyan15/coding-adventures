#!/usr/bin/env python3
"""Require that every installed toolchain probed OK inside the sandbox.

Usage: census_probes_ok.py <pre/probes.json> <shard-index>

`kiln record prepare` runs a toolchain probe per tool (`ghc --version`, ...) in
the SAME sandbox the build steps run in (internal/record/probe_toolchain.go),
with the sandbox's PATH, roots and pins, and seals the results in pre/probes.json
(recfmt: ProbeReport.toolchains, a sorted list of {tool, status[, detail]}).
status is one of ok, absent, refused, mismatch, timeout.

The host-side check in the workflow (each tool resolves under a declared root)
cannot see a PATH or pin mismatch, or a root that is a symlink the sandbox does
not follow.  Without this check a `home: runner` census in which every non-Go
step still fails "not found" INSIDE the sandbox would exit 0 and be labelled the
PR8 gate, which is the fresh-HOME census again.  So in runner mode each of the
installed tools below must have probed ok.

The file is parsed strictly: a missing, unreadable, symlinked or oversized (over
MAX_BYTES) file, a non-object, an entry with a missing or unknown key, a
malformed or duplicated entry, an unknown status, or a missing required tool all
fail.  Top-level keys other than `toolchains` are ignored: the gate rests on the
entries alone.  No string from the file is echoed except the fixed statuses.  It prints one Markdown line for $GITHUB_STEP_SUMMARY and
exits 0 only when every required tool is ok, 1 otherwise.
"""

import json
import os
import sys

# The seven installed toolchains, by the tool names toolchains.json probes:
# python3, ruby, go, node, cargo and rustc, dotnet, ghc.
REQUIRED = ("cargo", "dotnet", "ghc", "go", "node", "python3", "ruby", "rustc")
# Installed too (uv, bundler, cabal) and probed: reported, but a pin mismatch in
# a companion tool does not by itself void the gate.
COMPANIONS = ("bundle", "cabal", "uv")
STATUSES = {"ok", "absent", "refused", "mismatch", "timeout"}
# probes.json is a few KiB; anything past this is not a recorder's file.
MAX_BYTES = 1 << 20


class BadProbes(Exception):
    pass


def load(path: str) -> dict:
    """tool name -> status, from a strictly parsed probes.json."""
    try:
        # O_NOFOLLOW: the sealed file is a regular file, never a link.  The read
        # is bounded: one byte past MAX_BYTES fails rather than being parsed.
        fd = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
        with os.fdopen(fd, "rb") as f:
            raw = f.read(MAX_BYTES + 1)
        if len(raw) > MAX_BYTES:
            raise BadProbes(f"{path} is larger than {MAX_BYTES} bytes")
        doc = json.loads(raw.decode("utf-8"))
    except BadProbes:
        raise
    except Exception as e:  # noqa: BLE001 - only the type name is reported
        raise BadProbes(f"{path} unreadable ({type(e).__name__})") from None
    if not isinstance(doc, dict) or not isinstance(doc.get("toolchains"), list):
        raise BadProbes("no toolchains list")
    out: dict = {}
    for i, e in enumerate(doc["toolchains"]):
        if not isinstance(e, dict) or not {"tool", "status"} <= set(e) or set(e) - {"tool", "status", "detail"}:
            raise BadProbes(f"toolchains[{i}] is not a {{tool, status}} object")
        tool, status = e["tool"], e["status"]
        if not isinstance(tool, str) or not tool or not isinstance(status, str) or status not in STATUSES:
            raise BadProbes(f"toolchains[{i}] has a bad tool or status")
        if tool in out:
            raise BadProbes(f"toolchains[{i}] repeats a tool")
        out[tool] = status
    return out


def failures(tools: dict) -> list:
    return [f"{t}: {tools.get(t, 'not probed')}" for t in REQUIRED if tools.get(t) != "ok"]


def main(argv: list) -> int:
    path, shard = argv[1], argv[2]
    try:
        tools = load(path)
    except BadProbes as e:
        print(f"- shard {shard}: **toolchain probes not OK**: {e}\n")
        return 1
    bad = failures(tools)
    if bad:
        print(f"- shard {shard}: **toolchain probes not OK in the sandbox, not the PR8 gate**: {'; '.join(bad)}\n")
        return 1
    notes = [f"{t}: {tools[t]}" for t in COMPANIONS if t in tools and tools[t] != "ok"]
    extra = f" (companion tools not ok: {'; '.join(notes)})" if notes else ""
    print(f"- shard {shard}: every installed toolchain probed OK in the sandbox ({', '.join(REQUIRED)}){extra}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
