#!/usr/bin/env python3
"""Keep every toolchain version exact, and keep CI agreeing with mise.toml.

Why this exists
===============

A CI result is only worth caching (or trusting at all) if the same commit
always builds with the same compiler. A workflow that says
``python-version: '3.13'`` or ``toolchain: stable`` silently takes whatever
upstream released last night, so a commit that was green yesterday can turn red
today with no change in the repository. The cure is boring: every version is
exact, and there is one place to change it.

The one place
=============

::

    mise.toml [tools]                      what a developer installs locally
    code/specs/data/toolchain-pins.json    CI-only tools mise does not manage,
                                           and the few deliberate exceptions

Every ``with:`` input in ``.github/workflows/*.yml`` that names a toolchain
version is mapped to one of those entries:

====================================  =====================  ===============
workflow input                        action                 canonical name
====================================  =====================  ===============
``go-version``                        actions/setup-go       ``go``
``python-version``                    actions/setup-python   ``python``
``node-version``                      actions/setup-node     ``node``
``ruby-version``                      ruby/setup-ruby        ``ruby``
``ghc-version`` / ``cabal-version``   haskell-actions/setup  ``ghc`` / ``cabal``
``sdk``                               dart-lang/setup-dart   ``dart``
``version``                           astral-sh/setup-uv     ``uv``
``toolchain``                         dtolnay/rust-toolchain ``rust``
``elixir-version`` / ``otp-version``  erlef/setup-beam       pins.json
``flutter-version``                   subosito/flutter-action pins.json
``dotnet-version``                    actions/setup-dotnet   pins.json
====================================  =====================  ===============

A value that differs from its canonical version must be listed, with a reason,
in ``deviations`` in the pins file (for example a matrix that deliberately tests
Python 3.10, or a release workflow that ships with an older Rust). A deviation
must still be exact, and an entry that no longer matches anything is an error,
so the list cannot quietly rot.

Shell steps are checked too. ``rustup run stable``, ``+stable``, an undated
``nightly`` and an unversioned ``gem install bundler`` all float, so they are
rejected in workflows, in ``code/scripts/**/*.sh`` and in every BUILD file.

The parser is a deliberately small line scanner rather than a YAML library:
the contracts job runs with the standard library only, and line numbers are
what a reviewer needs in the error.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tomllib
from collections.abc import Iterable, Iterator
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
MISE_TOML = Path("mise.toml")
PINS_JSON = Path("code/specs/data/toolchain-pins.json")
WORKFLOWS = Path(".github/workflows")

# (workflow input key, action that owns it or None for "any") -> canonical name.
# `None` covers inputs that also appear in a job's `strategy.matrix`, where the
# value is not inside the step that consumes it.
INPUTS: dict[tuple[str, str | None], str] = {
    ("go-version", None): "go",
    ("python-version", None): "python",
    ("node-version", None): "node",
    ("ruby-version", None): "ruby",
    ("ghc-version", None): "ghc",
    ("cabal-version", None): "cabal",
    ("elixir-version", None): "elixir",
    ("otp-version", None): "erlang",
    ("flutter-version", None): "flutter",
    ("dotnet-version", None): "dotnet",
    ("sdk", "dart-lang/setup-dart"): "dart",
    ("version", "astral-sh/setup-uv"): "uv",
    ("toolchain", "dtolnay/rust-toolchain"): "rust",
}

# Actions that install a toolchain and float unless given an explicit version.
REQUIRED_INPUT = {
    "dtolnay/rust-toolchain": "toolchain",
    "astral-sh/setup-uv": "version",
}

# An exact version: dotted numbers (two or more parts), optionally a
# pre-release/OTP suffix mise uses, e.g. `1.18.4-otp-27`.
EXACT = re.compile(r"^\d+(?:\.\d+)+(?:-otp-\d+)?$")
# Tools whose upstream publishes x.y.z releases: x.y alone is a range.
NEEDS_PATCH = {"go", "python", "node", "ruby", "rust", "dart", "ghc", "uv",
               "lua", "elixir", "erlang", "flutter", "dotnet", "kotlin"}
DATED_NIGHTLY = re.compile(r"^nightly-\d{4}-\d{2}-\d{2}$")

FLOATING_SHELL = [
    (re.compile(r"rustup\s+run\s+stable\b"),
     ("`rustup run stable` runs the floating stable channel; call `cargo` "
     "(the pinned default toolchain) instead")),
    (re.compile(r"(?:cargo|rustc|rustup)\s+\+stable\b"),
     "`+stable` selects the floating stable channel"),
    (re.compile(r"\+nightly(?!-\d{4}-\d{2}-\d{2})\b"),
     "`+nightly` floats; name the dated nightly from toolchain-pins.json"),
    (re.compile(r"rustup\s+toolchain\s+install\s+(?:stable|nightly(?!-\d))\b"),
     "installing an undated channel floats; install the pinned version"),
    (re.compile(r"gem\s+install\s+bundler(?![^\n]*(?:--version|-v)\s+\d)"),
     "`gem install bundler` without `--version` floats"),
]

KEY_VALUE = re.compile(
    r"^(?P<indent>\s*)(?:-\s+)?(?P<key>[A-Za-z_][\w-]*):\s*"
    r"(?P<quote>['\"]?)(?P<value>[^'\"#]*?)(?P=quote)\s*(?:#.*)?$"
)
USES = re.compile(r"^\s*(?:-\s+)?uses:\s*(?P<action>[^@\s]+)@")
LIST_ITEM = re.compile(r"^(?P<indent>\s*)-\s")


class PinError(Exception):
    """The repository's toolchain pins are inconsistent."""


@dataclass(frozen=True)
class Finding:
    path: str
    line: int
    message: str

    def __str__(self) -> str:
        return f"{self.path}:{self.line}: {self.message}"


@dataclass(frozen=True)
class Pins:
    canonical: dict[str, str]
    nightly: str
    bundler: str
    deviations: frozenset[tuple[str, str, str]]


# ── Loading the sources of truth ─────────────────────────────────────


def load_pins(root: Path) -> tuple[Pins, list[Finding]]:
    """Merge mise.toml and toolchain-pins.json into one canonical table."""

    findings: list[Finding] = []
    mise = tomllib.loads((root / MISE_TOML).read_text(encoding="utf-8"))
    tools = mise.get("tools", {})
    canonical: dict[str, str] = {}
    for name, value in tools.items():
        if not isinstance(value, str):
            findings.append(Finding(str(MISE_TOML), 0, f"{name}: version must be a string"))
            continue
        problem = exactness_problem(name, value)
        if problem:
            findings.append(Finding(str(MISE_TOML), 0, f"{name} = {value!r}: {problem}"))
        canonical[name] = value

    data = json.loads((root / PINS_JSON).read_text(encoding="utf-8"))
    if data.get("schema_version") != 1:
        raise PinError(f"{PINS_JSON}: unsupported schema_version")
    for name, value in data.get("ci_only", {}).items():
        if name in canonical:
            findings.append(Finding(str(PINS_JSON), 0,
                                    f"{name} is pinned in both mise.toml and ci_only; keep one"))
            continue
        problem = exactness_problem(name, value)
        if problem:
            findings.append(Finding(str(PINS_JSON), 0, f"{name} = {value!r}: {problem}"))
        canonical[name] = value

    nightly = data.get("rust_nightly", "")
    if not DATED_NIGHTLY.match(nightly):
        findings.append(Finding(str(PINS_JSON), 0, "rust_nightly must be nightly-YYYY-MM-DD"))
    bundler = data.get("bundler", "")
    if not EXACT.match(bundler):
        findings.append(Finding(str(PINS_JSON), 0, "bundler must be an exact version"))

    deviations = set()
    for entry in data.get("deviations", []):
        if not entry.get("reason"):
            findings.append(Finding(str(PINS_JSON), 0,
                                    f"deviation {entry!r} needs a reason"))
        deviations.add((entry["file"], entry["input"], entry["value"]))
    return Pins(canonical, nightly, bundler, frozenset(deviations)), findings


def exactness_problem(name: str, value: str) -> str | None:
    if not EXACT.match(value):
        return "not an exact version (no latest/stable/lts or ranges)"
    if name in NEEDS_PATCH and value.split("-")[0].count(".") < 2:
        return "missing the patch component, so it is a range"
    return None


def canonical_for(name: str, value: str, pins: Pins) -> str | None:
    """Map a canonical name to the form a workflow input writes it in."""

    wanted = pins.canonical.get(name)
    if wanted is None:
        return None
    if name == "elixir":
        # mise spells it `1.18.4-otp-27`; setup-beam takes `1.18.4`.
        return wanted.split("-otp-")[0]
    return wanted


# ── Scanning workflows ───────────────────────────────────────────────


@dataclass(frozen=True)
class Input:
    line: int
    key: str
    value: str
    action: str | None


@dataclass
class Step:
    action: str | None
    line: int
    inputs: list[Input]


def scan_workflow(text: str) -> tuple[list[Input], list[Step]]:
    """Return every key/value line with the action of its enclosing step.

    A step is a YAML list item; ``uses:`` anywhere in it names its action. The
    action is attached to every key in the item, which is what lets
    ``version:`` mean "uv version" only inside ``astral-sh/setup-uv``.
    """

    inputs: list[Input] = []
    steps: list[Step] = []
    stack: list[tuple[int, Step]] = []
    for number, raw in enumerate(text.splitlines(), start=1):
        if not raw.strip() or raw.lstrip().startswith("#"):
            continue
        indent = len(raw) - len(raw.lstrip())
        while stack and indent <= stack[-1][0]:
            stack.pop()
        item = LIST_ITEM.match(raw)
        if item:
            step = Step(None, number, [])
            steps.append(step)
            stack.append((len(item.group("indent")), step))
        uses = USES.match(raw)
        if uses and stack:
            stack[-1][1].action = uses.group("action")
        match = KEY_VALUE.match(raw)
        if match and match.group("value"):
            action = stack[-1][1].action if stack else None
            entry = Input(number, match.group("key"), match.group("value").strip(), action)
            inputs.append(entry)
            if stack:
                stack[-1][1].inputs.append(entry)
    # The action of a step is known only once its `uses:` line is read, which
    # may come after some of its keys; rebuild inputs with the final action.
    resolved: list[Input] = []
    owner = {id(i): s for s in steps for i in s.inputs}
    for entry in inputs:
        step = owner.get(id(entry))
        resolved.append(Input(entry.line, entry.key, entry.value,
                              step.action if step else entry.action))
    return resolved, steps


def check_workflow(path: str, text: str, pins: Pins,
                   seen_deviations: set[tuple[str, str, str]]) -> Iterator[Finding]:
    inputs, steps = scan_workflow(text)
    for entry in inputs:
        name = INPUTS.get((entry.key, entry.action)) or INPUTS.get((entry.key, None))
        if name is None:
            continue
        if entry.value.startswith("${{"):
            # Resolved from a matrix or env value; those literals are checked
            # where they are written (matrix keys share the input's name, and
            # *_VERSION env values are checked below).
            continue
        if name == "rust" and entry.value.startswith("nightly"):
            if entry.value != pins.nightly:
                yield Finding(path, entry.line,
                              f"rust nightly {entry.value!r} is not {pins.nightly!r}")
            continue
        yield from check_value(path, entry, name, pins, seen_deviations)

    for step in steps:
        required = REQUIRED_INPUT.get(step.action or "")
        if required and not any(i.key == required for i in step.inputs):
            yield Finding(path, step.line,
                          f"{step.action} without `{required}:` installs a floating version")

    # Workflow-level env such as `RUST_VERSION: "1.97.0"` feeds `${{ env.* }}`.
    for entry in inputs:
        if entry.key == "RUST_VERSION":
            yield from check_value(path, entry, "rust", pins, seen_deviations)


def check_value(path: str, entry: Input, name: str, pins: Pins,
                seen_deviations: set[tuple[str, str, str]]) -> Iterator[Finding]:
    problem = exactness_problem(name, entry.value)
    if problem:
        yield Finding(path, entry.line, f"{entry.key}: {entry.value!r} is {problem}")
        return
    wanted = canonical_for(name, entry.value, pins)
    if wanted is None or entry.value == wanted:
        return
    key = (path, entry.key, entry.value)
    if key in pins.deviations:
        seen_deviations.add(key)
        return
    yield Finding(path, entry.line,
                  f"{entry.key}: {entry.value!r} differs from {name} = {wanted!r} "
                  f"(change it, or record a reasoned deviation in {PINS_JSON})")


def check_shell(path: str, text: str) -> Iterator[Finding]:
    for number, line in enumerate(text.splitlines(), start=1):
        stripped = line.lstrip()
        if stripped.startswith(("#", "::")) or stripped.lower().startswith("rem "):
            continue
        for pattern, message in FLOATING_SHELL:
            if pattern.search(line):
                yield Finding(path, number, message)


# ── Driver ───────────────────────────────────────────────────────────


def tracked_files(root: Path, patterns: Iterable[str]) -> list[str]:
    result = subprocess.run(
        ["git", "ls-files", "-z", "--", *patterns],
        cwd=root, check=True, capture_output=True,
    )
    return [p for p in result.stdout.decode("utf-8").split("\0") if p]


def check(root: Path) -> list[Finding]:
    pins, findings = load_pins(root)
    seen: set[tuple[str, str, str]] = set()
    workflows = sorted(tracked_files(root, [f"{WORKFLOWS}/*.yml", f"{WORKFLOWS}/*.yaml"]))
    for rel in workflows:
        text = (root / rel).read_text(encoding="utf-8")
        findings.extend(check_workflow(rel, text, pins, seen))
        findings.extend(check_shell(rel, text))
    shell_files = tracked_files(root, ["*BUILD", "*BUILD_windows", "code/scripts/*.sh",
                                       "code/scripts/**/*.sh"])
    for rel in sorted(set(shell_files)):
        path = root / rel
        if not path.is_file():
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        findings.extend(check_shell(rel, text))
    for stale in sorted(pins.deviations - seen):
        findings.append(Finding(str(PINS_JSON), 0,
                                f"deviation {stale!r} matches nothing; remove it"))
    return findings


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("command", choices=["check"])
    parser.add_argument("--root", type=Path, default=REPO_ROOT)
    args = parser.parse_args(argv)
    findings = check(args.root.resolve())
    for finding in findings:
        print(finding, file=sys.stderr)
    if findings:
        print(f"toolchain_pins: {len(findings)} problem(s)", file=sys.stderr)
        return 1
    print("toolchain_pins: every toolchain version is exact and agrees with mise.toml")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
