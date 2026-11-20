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

The file is parsed strictly: a missing or unreadable file, a non-object, an
unknown key, a malformed or duplicated entry, an unknown status, or a missing
required tool all fail.  It prints one Markdown line for $GITHUB_STEP_SUMMARY and
exits 0 only when every required tool is ok, 1 otherwise.
"""

import json
import sys

# The seven installed toolchains, by the tool names toolchains.json probes:
# python3, ruby, go, node, cargo and rustc, dotnet, ghc.
REQUIRED = ("cargo", "dotnet", "ghc", "go", "node", "python3", "ruby", "rustc")
# Installed too (uv, bundler, cabal) and probed: reported, but a pin mismatch in
# a companion tool does not by itself void the gate.
COMPANIONS = ("bundle", "cabal", "uv")
STATUSES = {"ok", "absent", "refused", "mismatch", "timeout"}


class BadProbes(Exception):
    pass


def load(path: str) -> dict:
    """tool name -> status, from a strictly parsed probes.json."""
    try:
        with open(path, encoding="utf-8") as f:
            doc = json.load(f)
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
            raise BadProbes(f"toolchains[{i}] repeats the tool {tool!r}")
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
