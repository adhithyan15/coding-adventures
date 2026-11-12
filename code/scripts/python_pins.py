#!/usr/bin/env python3
"""Pin every third-party Python dependency in the monorepo, with hashes.

The problem
===========

Hundreds of ``pyproject.toml`` files say things like ``pytest>=8.0``.  A range
resolves to whatever is newest on the day CI runs, so two CI runs of the same
commit can install different code.  We want the opposite: one version of each
third-party package, everywhere, verified by hash, and a CI failure the moment
the pins go stale.

The design (one file, applied from the environment)
===================================================

::

    pyproject.toml x ~470  ─┐
    BUILD / workflow tools ─┼─►  python_pins.py generate  ─►  python-constraints.txt
    build-system.requires  ─┘                             └─►  python-build-constraints.txt

* ``python-constraints.txt`` pins everything installed *into* a package venv:
  runtime and dev dependencies plus tools such as ruff and pytest.
* ``python-build-constraints.txt`` pins everything installed into uv's
  *isolated build environments*: hatchling, setuptools, maturin and their
  dependencies.

CI points uv at both files through ``UV_CONSTRAINT`` and
``UV_BUILD_CONSTRAINT`` (see ``.github/workflows/ci.yml``).  No BUILD file
needs to change, because uv reads those variables on every ``uv pip install``.

Two uv behaviours shape the checks below, and both are easy to get wrong:

1. **A name that is missing from the constraints file floats silently.**
   A constraint only *restricts* a package that is being resolved anyway; it
   never complains about packages it has not heard of.  So "the file is
   consistent" is not enough: ``check`` must also prove the file is
   *complete* for every third-party name the repo uses.
2. **``UV_REQUIRE_HASHES`` is deliberately NOT set.**  It would reject every
   editable install (``-e ../sibling``), which is how the monorepo wires its
   own packages together.  Hashes that *are* listed in a constraints file are
   still verified, so a tampered or substituted wheel fails to install.

What counts as third-party
==========================

Every requirement named by a tracked ``pyproject.toml`` (``dependencies``,
``optional-dependencies``, ``dependency-groups``, ``build-system.requires``)
plus tools installed straight from a BUILD file or workflow
(``uv pip install ruff``).  Names that are projects *inside this repo* -- some
tracked ``pyproject.toml`` declares ``[project] name = <that name>`` -- are
excluded: they are installed from source with ``-e`` and PyPI must never be
consulted for them.  Names are compared after PEP 503 normalisation
(``Foo_Bar`` == ``foo-bar``).

Subcommands
===========

``requirements``  print the derived requirement sets and where each came from.
``generate``      compile both sets with uv and write the two constraints files.
``check``         recompile with the existing files as constraints and fail if
                  anything would resolve differently, or is missing.

Only the standard library is imported (uv is run as a subprocess).
"""

from __future__ import annotations

import argparse
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from collections.abc import Callable, Iterable, Mapping
from pathlib import Path

# The policy script already knows how to find tracked manifests, how to read
# requirement arrays out of every pyproject.toml table, and how to recognise a
# tool installed by a shell command.  Reusing it keeps the two inventories
# from ever disagreeing about what "third-party" means.
sys.path.insert(0, str(Path(__file__).resolve().parent))
import third_party_dependency_policy as policy

REPO_ROOT = Path(__file__).resolve().parents[2]
DATA_DIR = Path("code/specs/data/third-party-dependencies")
RUNTIME_FILE = DATA_DIR / "python-constraints.txt"
BUILD_FILE = DATA_DIR / "python-build-constraints.txt"

# The interpreter CI uses.  Pins are compiled "universally" (every platform and
# marker at once) for this Python so one file serves Linux, macOS and Windows.
PYTHON_VERSION = "3.13"

REGENERATE_COMMAND = "python3 code/scripts/python_pins.py generate"

# Matches one `name==version` line of uv's compiled output.  Universal output
# may carry an environment marker after a `;` (`colorama==0.4.6 ; sys_platform
# == 'win32'`), and hashes follow on continuation lines; we only need the
# first two things.
PIN_LINE = re.compile(r"^(?P<name>[A-Za-z0-9][A-Za-z0-9._-]*)==(?P<version>[^\s;\\]+)")


class PinsError(Exception):
    """A user-facing failure; its message is printed verbatim."""


# ---------------------------------------------------------------------------
# Step 1: derive the requirement sets from the repository
# ---------------------------------------------------------------------------


def reject_unsafe_requirement(spec: str, source: str) -> None:
    """Refuse requirement strings that could steer `uv pip compile` elsewhere.

    The requirement lines are written, one per line, into a requirements file
    that uv parses in CI.  A requirement that smuggles in a newline, starts with
    ``-`` (``--index-url ...``, ``--extra-index-url ...``), names a direct URL
    (``pkg @ https://...``) or contains ``://`` would be read as a requirements
    -file option or a non-registry source instead of a plain package, so it is
    an error naming the offending manifest.
    """

    reasons = []
    if "\n" in spec or "\r" in spec:
        reasons.append("contains a line break")
    if spec.lstrip().startswith("-"):
        reasons.append("starts with '-' (looks like a requirements-file option)")
    if " @ " in spec or re.search(r"@\s*\w+://", spec):
        reasons.append("is a direct reference ('name @ url')")
    if "://" in spec:
        reasons.append("contains a URL")
    if reasons:
        raise PinsError(
            f"{source}: unsafe requirement {spec!r}: " + "; ".join(reasons) + "."
        )


class Requirements:
    """Requirement strings per normalised name, with where each came from.

    ``specs`` keeps the *verbatim* requirement strings (extras, ranges and
    markers included) because uv needs them to resolve correctly;
    ``sources`` is only for humans reading ``requirements`` output.
    """

    def __init__(self) -> None:
        self.specs: dict[str, set[str]] = {}
        self.sources: dict[str, set[str]] = {}

    def add(self, name: str, spec: str, source: str) -> None:
        reject_unsafe_requirement(spec, source)
        self.specs.setdefault(name, set()).add(spec)
        self.sources.setdefault(name, set()).add(source)

    def names(self) -> set[str]:
        return set(self.specs)

    def lines(self) -> list[str]:
        """Sorted, de-duplicated requirement lines for a uv ``.in`` file."""
        return sorted({spec for specs in self.specs.values() for spec in specs})


FIRST_PARTY_PREFIX = "coding-adventures-"


def in_repo_project_names(documents: Mapping[Path, Mapping[str, object]]) -> set[str]:
    """Normalised ``[project] name`` of every tracked pyproject.toml.

    These are installed from source, never from PyPI, so they are not
    third-party no matter who depends on them.
    """

    names: set[str] = set()
    for document in documents.values():
        project = document.get("project")
        if isinstance(project, Mapping):
            name = project.get("name")
            if isinstance(name, str) and name:
                names.add(policy.canonical_python_name(name))
    return names


def is_first_party(name: str, local: set[str]) -> bool:
    """True when ``name`` must never be resolved from PyPI.

    Three ways to be first-party: a tracked project declares the name; the
    name is in the repo's own ``coding-adventures-`` namespace (so a dangling
    dependency such as ``coding-adventures-pipeline``, which has no project,
    cannot be satisfied by someone squatting the name on PyPI -- it simply
    fails to resolve); or the name is redirected to a tracked directory by
    ``[tool.uv.sources]`` (``matrix = { path = ... }``), handled by the caller
    through ``local``.
    """

    return name in local or name.startswith(FIRST_PARTY_PREFIX)


def derive_requirements(
    root: Path,
    documents: Mapping[Path, Mapping[str, object]],
    tool_names: Mapping[str, Iterable[str]],
) -> tuple[Requirements, Requirements]:
    """Split the repo's third-party requirements into (runtime, build).

    ``documents`` maps an absolute pyproject.toml path to its parsed TOML;
    ``tool_names`` maps a display path to package names installed directly by
    that automation file.  ``build-system.requires`` goes to the *build* set
    because it is installed into uv's isolated build environments, which
    ``UV_BUILD_CONSTRAINT`` governs; everything else goes to the *runtime* set.
    """

    resolved = {path.resolve(): document for path, document in documents.items()}
    sourced = policy.python_local_source_names(root, resolved)
    global_local = in_repo_project_names(documents)
    runtime, build = Requirements(), Requirements()
    for path, document in sorted(documents.items()):
        display = path.relative_to(root).as_posix()
        local = global_local | sourced[path.resolve()]
        for label, requirements in policy.python_requirement_groups(document, path):
            target = build if label == "build-system.requires" else runtime
            for requirement in requirements:
                name = policy.python_requirement_name(
                    requirement, f"{display} {label}"
                )
                if not is_first_party(name, local):
                    target.add(name, requirement.strip(), display)
    for display, names in sorted(tool_names.items()):
        for raw in names:
            name = policy.canonical_python_name(raw)
            if not is_first_party(name, global_local):
                runtime.add(name, name, display)
    return runtime, build


def load_repository(root: Path) -> tuple[Requirements, Requirements]:
    """Read the tracked files under ``root`` and derive both requirement sets."""

    manifests = policy.git_python_manifests(root)
    texts = policy.tracked_text_documents(manifests, root)
    documents: dict[Path, Mapping[str, object]] = {}
    for manifest in manifests:
        try:
            documents[manifest] = tomllib.loads(texts[manifest])
        except tomllib.TOMLDecodeError as error:
            raise PinsError(f"cannot parse {manifest}: {error}") from error

    sources = policy.git_python_tool_sources(root)
    source_texts = policy.tracked_text_documents(sources, root)
    tool_names: dict[str, set[str]] = {}
    for source in sources:
        display = source.relative_to(root).as_posix()
        try:
            names = policy.python_tool_names(source_texts[source])
        except policy.PolicyError as error:
            raise PinsError(f"cannot parse {display}: {error}") from error
        if names:
            tool_names[display] = names
    return derive_requirements(root, documents, tool_names)


# ---------------------------------------------------------------------------
# Step 2: ask uv to compile a requirement set
# ---------------------------------------------------------------------------

# A runner takes a uv argument list and returns its stdout.  Tests replace it
# with a fake so no network or uv binary is needed.
Runner = Callable[[list[str]], str]


def find_uv() -> str:
    """Locate uv on PATH, or fail with an actionable message."""

    uv = shutil.which("uv")
    if uv is None:
        raise PinsError(
            "uv was not found on PATH. `python_pins.py` shells out to "
            "`uv pip compile`; install uv (https://docs.astral.sh/uv/) and retry."
        )
    return uv


def run_uv(arguments: list[str]) -> str:
    """Run uv for real and return stdout, surfacing uv's own error text."""

    command = [find_uv(), *arguments]
    # CI exports UV_CONSTRAINT / UV_BUILD_CONSTRAINT for the whole workflow.
    # Left in place they would silently pre-constrain `generate` (so it could
    # never move a pin) and double up on `check`, so compile hermetically.
    environment = {
        key: value
        for key, value in os.environ.items()
        if key not in ("UV_CONSTRAINT", "UV_BUILD_CONSTRAINT")
    }
    result = subprocess.run(
        command, capture_output=True, text=True, check=False, env=environment
    )
    if result.returncode != 0:
        raise PinsError(
            f"`uv {' '.join(arguments[:2])}` failed (exit {result.returncode}):\n"
            f"{result.stderr.strip()}"
        )
    return result.stdout


def compile_requirements(
    requirements: Requirements,
    *,
    hashes: bool,
    constraint: Path | None = None,
    runner: Runner = run_uv,
) -> str:
    """Compile ``requirements`` to a pinned list with ``uv pip compile``.

    ``--universal`` resolves for every platform at once, so Windows-only
    packages such as colorama appear (with markers) in the same file macOS and
    Linux read.  ``--no-annotate`` drops the ``# via`` comments, which would
    otherwise name the temporary input file.
    """

    with tempfile.TemporaryDirectory() as scratch:
        source = Path(scratch) / "requirements.in"
        source.write_text("\n".join(requirements.lines()) + "\n", encoding="utf-8")
        arguments = [
            "pip",
            "compile",
            str(source),
            "--universal",
            "--python-version",
            PYTHON_VERSION,
            "--no-header",
            "--no-annotate",
            "--quiet",
        ]
        if hashes:
            arguments.append("--generate-hashes")
        if constraint is not None:
            arguments += ["--constraint", str(constraint)]
        return runner(arguments)


def parse_pins(text: str) -> set[tuple[str, str]]:
    """Return the ``(normalised name, version)`` pairs in a constraints file.

    Only versions are compared, never hashes: hash lists legitimately grow
    when a package publishes another wheel for a version already pinned.
    Universal output may list one name twice at different versions under
    different markers, so this is a set of pairs rather than a mapping.
    """

    pins: set[tuple[str, str]] = set()
    for line in text.splitlines():
        match = PIN_LINE.match(line.strip())
        if match:
            pins.add(
                (policy.canonical_python_name(match["name"]), match["version"])
            )
    return pins


# One pin line: `name==version`, optional ` ; marker`, optional trailing ` \`.
PIN_BLOCK_LINE = re.compile(
    r"^[A-Za-z0-9][A-Za-z0-9._-]*==[A-Za-z0-9][A-Za-z0-9._+!-]*"
    r"(?: ; [^\\\r\n]+?)?(?: \\)?$"
)
HASH_LINE = re.compile(r"^    --hash=sha256:[0-9a-f]{64}(?: \\)?$")


def validate_constraints_text(text: str) -> list[str]:
    """Check a constraints file line by line; return problems (empty = ok).

    The file is read by uv with authority, so beyond `name==version` pins and
    `--hash=sha256:` continuations nothing may appear except comments and blank
    lines: no `--index-url`, no URLs, no extra requirement-file options.  Every
    pin must also carry at least one hash, or hash verification would be a no-op
    for that package.
    """

    problems: list[str] = []
    current: tuple[int, str] | None = None
    hashes = 0

    def close() -> None:
        if current is not None and hashes == 0:
            problems.append(f"line {current[0]}: pin {current[1]!r} has no hash.")

    for number, line in enumerate(text.splitlines(), 1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if HASH_LINE.match(line):
            if current is None:
                problems.append(f"line {number}: hash with no preceding pin.")
            hashes += 1
        elif PIN_BLOCK_LINE.match(line):
            close()
            current, hashes = (number, line.split("==")[0]), 0
        else:
            problems.append(f"line {number}: unexpected content {line!r}.")
    close()
    return problems


# ---------------------------------------------------------------------------
# Step 3: the generated files and the staleness check
# ---------------------------------------------------------------------------

HEADER = """\
# GENERATED FILE - DO NOT EDIT BY HAND.
#
# {what}
#
# Every version below is pinned with sha256 hashes, compiled universally
# (all platforms) for Python {python}.  CI applies it through the
# {variable} environment variable, so no BUILD file refers to it.
# Packages that live in this repository are intentionally absent.
#
# Regenerate after changing any third-party requirement:
#
#     {command}
#
# CI runs `python3 code/scripts/python_pins.py check` and fails when this file
# is stale or incomplete.
"""

RUNTIME_WHAT = (
    "Pins for everything installed into package virtualenvs "
    "(runtime and dev dependencies, plus tools such as ruff and pytest)."
)
BUILD_WHAT = (
    "Pins for uv's isolated build environments "
    "(hatchling, setuptools, maturin and their dependencies)."
)


def render_file(compiled: str, what: str, variable: str) -> str:
    """Prefix uv's output with our generated-file header."""

    header = HEADER.format(
        what=what,
        python=PYTHON_VERSION,
        variable=variable,
        command=REGENERATE_COMMAND,
    )
    return header + "\n" + compiled.strip() + "\n"


def generate(root: Path, runner: Runner = run_uv) -> list[Path]:
    """Compile both requirement sets and write the two constraints files."""

    runtime, build = load_repository(root)
    written = []
    for requirements, path, what, variable in (
        (runtime, RUNTIME_FILE, RUNTIME_WHAT, "UV_CONSTRAINT"),
        (build, BUILD_FILE, BUILD_WHAT, "UV_BUILD_CONSTRAINT"),
    ):
        compiled = compile_requirements(requirements, hashes=True, runner=runner)
        target = root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(render_file(compiled, what, variable), encoding="utf-8")
        written.append(target)
    return written


def check_one(
    requirements: Requirements,
    constraint_file: Path,
    display: Path,
    runner: Runner = run_uv,
) -> list[str]:
    """Return human-readable problems with one constraints file (empty = ok)."""

    if not constraint_file.is_file():
        return [f"{display} does not exist."]
    existing = parse_pins(constraint_file.read_text(encoding="utf-8"))
    pinned_names = {name for name, _ in existing}
    problems: list[str] = [
        f"{display}: {problem}"
        for problem in validate_constraints_text(
            constraint_file.read_text(encoding="utf-8")
        )
    ]

    # Completeness first.  uv lets an unlisted name float without a word, so a
    # name missing here is the dangerous failure and gets its own message.
    for name in sorted(requirements.names() - pinned_names):
        where = ", ".join(sorted(requirements.sources[name])[:3])
        problems.append(f"{display}: no pin for {name!r} (required by {where}).")

    # Consistency second.  Recompile with the file as a constraint and no
    # hashes.  Pinned names keep their versions; anything unpinned, newly
    # required, or no longer required changes the resulting name==version set.
    compiled = compile_requirements(
        requirements, hashes=False, constraint=constraint_file, runner=runner
    )
    fresh = parse_pins(compiled)
    for name, version in sorted(fresh - existing):
        problems.append(f"{display}: would add {name}=={version}.")
    for name, version in sorted(existing - fresh):
        problems.append(f"{display}: {name}=={version} is no longer needed.")
    return problems


def check(root: Path, runner: Runner = run_uv) -> list[str]:
    """Check both constraints files against the repository."""

    runtime, build = load_repository(root)
    problems = check_one(runtime, root / RUNTIME_FILE, RUNTIME_FILE, runner)
    problems += check_one(build, root / BUILD_FILE, BUILD_FILE, runner)
    return problems


# ---------------------------------------------------------------------------
# Command line
# ---------------------------------------------------------------------------


def print_requirements(root: Path) -> None:
    runtime, build = load_repository(root)
    for title, requirements in (
        ("Runtime / tool requirements (python-constraints.txt)", runtime),
        ("Build-backend requirements (python-build-constraints.txt)", build),
    ):
        print(f"{title}: {len(requirements.names())} names")
        for name in sorted(requirements.names()):
            sources = sorted(requirements.sources[name])
            extra = f" +{len(sources) - 1} more" if len(sources) > 1 else ""
            print(f"  {name:28} {sources[0]}{extra}")
        print()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("command", choices=("requirements", "generate", "check"))
    parser.add_argument("--root", type=Path, default=REPO_ROOT)
    args = parser.parse_args(argv)
    root = args.root.resolve()
    try:
        if args.command == "requirements":
            print_requirements(root)
        elif args.command == "generate":
            for path in generate(root):
                print(f"wrote {path.relative_to(root)}")
        else:
            problems = check(root)
            if problems:
                print("Python dependency pins are stale or incomplete:\n")
                for problem in problems:
                    print(f"  - {problem}")
                print(f"\nFix: run `{REGENERATE_COMMAND}` and commit the result.")
                return 1
            print("Python dependency pins are up to date.")
    except PinsError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
