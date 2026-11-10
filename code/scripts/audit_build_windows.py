#!/usr/bin/env python3
"""Audit every package's Windows build front for drift from its Linux BUILD.

WHY THIS EXISTS
===============

Almost no pull request runs the Windows shard of CI.  The build tool only
schedules ``build (windows-latest)`` when a change happens to pull a package
that needs it into the affected set.  So a ``BUILD_windows`` that has quietly
drifted away from its ``BUILD`` sits unnoticed until some unrelated PR
rebuilds the package as a dependent -- and then *that* PR fails on code it
never touched.  This script finds the drift without waiting for that.

HOW THE BUILD TOOL PICKS A BUILD FILE
=====================================

Read from ``code/programs/go/build-tool/internal/discovery/discovery.go``
(``buildFileCandidatesForPlatform``), first existing file wins::

    darwin   BUILD_mac,   BUILD_mac_and_linux, BUILD
    linux    BUILD_linux, BUILD_mac_and_linux, BUILD
    windows  BUILD_windows,                     BUILD

Two consequences drive the checks below:

* ``BUILD_mac_and_linux`` is never used on Windows.  A package with only that
  file has *no* Windows build (fine, deliberately skipped).
* A package with a plain ``BUILD`` and no ``BUILD_windows`` runs that ``BUILD``
  on Windows, and the executor runs every line as ``cmd /C <line>``.  A
  ``BUILD`` written for ``sh`` therefore breaks (or worse, silently skips).

THE DRIFT CLASSES
=================

    id  name                      what it looks like
    --  ------------------------  --------------------------------------------
    A   vitest-drift              BUILD_windows runs ``vitest`` but the package
                                  migrated to ``node --test`` (no vitest in
                                  package.json), so ``npx vitest`` downloads or
                                  fails instead of running the suite.
    B   perl5lib-set              ``set PERL5LIB=..\\x\\lib;%PERL5LIB% && prove``.
                                  CI's MSYS perl splits PERL5LIB on ``:``, so
                                  the entry is never found.  Use
                                  ``prove -I..\\x\\lib -l -v t``.
    C   ruby-standardrb-eol       A Ruby front that runs ``standardrb`` without
                                  ``--except Layout/EndOfLine``.  On Windows the
                                  cop wants CRLF; the repo forces LF through
                                  ``.gitattributes``.
    D   runner-mismatch           BUILD and BUILD_windows run different test
                                  runners (e.g. node --test vs vitest).
    E   windows-posix-fallback    No BUILD_windows, and the BUILD that Windows
                                  falls back to uses POSIX-only shell syntax.

Class D is about test *runners* only.  A ``BUILD_windows`` that is an
intentional skip stub (it runs no runner at all) is not a mismatch, and a
runner set that is merely a subset or superset of BUILD's (cargo skipped on
Windows, say) is reported as *advisory*: it never fails ``--check``.  Only a
swapped, disjoint runner set blocks.

Class (d) from the original incident notes -- Ruby library code passing sizes
to native methods that overflow a 32-bit C ``long`` on Windows -- needs a
human reading the code, so it is deliberately not audited here.

USAGE
=====

    python3 code/scripts/audit_build_windows.py            # print a report
    python3 code/scripts/audit_build_windows.py --check    # exit 1 on findings
    python3 code/scripts/audit_build_windows.py --root DIR # audit another tree

The script uses only the standard library.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path

# ---------------------------------------------------------------------------
# Build-file selection (mirrors discovery.go; see the module docstring)
# ---------------------------------------------------------------------------

WINDOWS_CANDIDATES = ("BUILD_windows", "BUILD")
LINUX_CANDIDATES = ("BUILD_linux", "BUILD_mac_and_linux", "BUILD")


def effective_build_file(package: Path, candidates: tuple[str, ...]) -> Path | None:
    """Return the first candidate that exists, exactly as the build tool does."""
    for name in candidates:
        candidate = package / name
        if candidate.is_file():
            return candidate
    return None


def find_packages(root: Path) -> list[Path]:
    """Every ``code/packages/*/*`` and ``code/programs/*/*`` directory that has
    at least one BUILD file."""
    packages: list[Path] = []
    for bucket in ("packages", "programs"):
        base = root / "code" / bucket
        if not base.is_dir():
            continue
        for language in sorted(p for p in base.iterdir() if p.is_dir()):
            for package in sorted(p for p in language.iterdir() if p.is_dir()):
                if any(package.glob("BUILD*")):
                    packages.append(package)
    return packages


def read_command_lines(path: Path) -> list[str]:
    """The lines the executor would run: no blanks, no ``#`` comments, no
    ``rem`` lines."""
    lines: list[str] = []
    for raw in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = raw.strip().lstrip("﻿")
        if not line or line.startswith("#") or re.match(r"(?i)^@?rem(\s|$)", line):
            continue
        lines.append(line)
    return lines


def is_starlark(path: Path) -> bool:
    """A Starlark BUILD is evaluated, not run line by line."""
    text = path.read_text(encoding="utf-8", errors="replace")
    return bool(re.search(r"(?m)^[a-z_]+\(", text)) and "&&" not in text


# ---------------------------------------------------------------------------
# Findings
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Finding:
    cls: str
    package: str
    file: str
    detail: str
    # Advisory findings are reported but never fail ``--check``: they are
    # differences a human has to judge (e.g. a runner deliberately skipped on
    # Windows), not drift with a single correct repair.
    advisory: bool = False


CLASS_TITLES = {
    "A": "vitest-drift (BUILD_windows runs vitest, package no longer uses it)",
    "B": "perl5lib-set (set PERL5LIB=... is ignored by MSYS perl; use -I)",
    "C": "ruby-standardrb-eol (standardrb without --except Layout/EndOfLine)",
    "D": "runner-mismatch (BUILD and BUILD_windows run different test runners)",
    "E": "windows-posix-fallback (no BUILD_windows; BUILD uses POSIX-only syntax)",
}


# ---------------------------------------------------------------------------
# Class A -- vitest left behind in BUILD_windows
# ---------------------------------------------------------------------------


def package_json(package: Path) -> dict:
    path = package / "package.json"
    if not path.is_file():
        return {}
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {}


def uses_vitest(package: Path) -> bool:
    """True when package.json still depends on vitest (any dependency table)."""
    data = package_json(package)
    for table in (
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ):
        if "vitest" in (data.get(table) or {}):
            return True
    return False


def check_vitest_drift(package: Path, rel: str) -> list[Finding]:
    windows = package / "BUILD_windows"
    if not windows.is_file() or not (package / "package.json").is_file():
        return []
    if uses_vitest(package):
        return []
    findings = []
    for line in read_command_lines(windows):
        if re.search(r"\bvitest\b", line):
            findings.append(
                Finding(
                    "A",
                    rel,
                    "BUILD_windows",
                    f"runs vitest but package.json has none: {line}",
                )
            )
    return findings


# ---------------------------------------------------------------------------
# Class B -- `set PERL5LIB=` in BUILD_windows
# ---------------------------------------------------------------------------

PERL5LIB_SET = re.compile(r'(?i)\bset\s+"?PERL5LIB=')


def check_perl5lib(package: Path, rel: str) -> list[Finding]:
    windows = package / "BUILD_windows"
    if not windows.is_file():
        return []
    return [
        Finding("B", rel, "BUILD_windows", f"sets PERL5LIB: {line}")
        for line in read_command_lines(windows)
        if PERL5LIB_SET.search(line)
    ]


# ---------------------------------------------------------------------------
# Class C -- standardrb without the EndOfLine exemption
# ---------------------------------------------------------------------------


def check_ruby_standardrb(package: Path, rel: str) -> list[Finding]:
    """Look at the file Windows will actually run (BUILD_windows, else BUILD)."""
    if package.parent.name != "ruby":
        return []
    effective = effective_build_file(package, WINDOWS_CANDIDATES)
    if effective is None:
        return []
    findings = []
    for line in read_command_lines(effective):
        if "standardrb" in line and "Layout/EndOfLine" not in line:
            if effective.name == "BUILD":
                detail = f"no BUILD_windows, so Windows runs BUILD with full standardrb: {line}"
            else:
                detail = f"standardrb lacks --except Layout/EndOfLine: {line}"
            findings.append(Finding("C", rel, effective.name, detail))
    return findings


# ---------------------------------------------------------------------------
# Class D -- different test runners
# ---------------------------------------------------------------------------

# (runner name, pattern).  Order does not matter; every match is collected.
# `perl -I.. t/x.t` and `prove` are both TAP producers, so they share a name.
RUNNER_PATTERNS = [
    ("vitest", re.compile(r"\bvitest\b")),
    ("node-test", None),  # matched by _is_node_test below, not a regex
    ("pytest", re.compile(r"\bpytest\b")),
    ("unittest", re.compile(r"-m\s+unittest\b")),
    ("tap", re.compile(r"\bprove\b|\bperl\b[^&|]*\bt[/\\][^\s&|]*\.t\b")),
    ("rspec", re.compile(r"\brspec\b")),
    ("rake-test", re.compile(r"\brake\s+test\b")),
    ("mix-test", re.compile(r"\bmix\s+test\b")),
    ("cargo-test", re.compile(r"\bcargo\s+(?:\S+\s+)*test\b")),
    ("go-test", re.compile(r"\bgo\s+test\b")),
    ("dotnet-test", re.compile(r"\bdotnet\s+test\b")),
    ("swift-test", re.compile(r"\bswift\s+test\b")),
    ("busted", re.compile(r"\bbusted\b")),
    ("gradle-test", re.compile(r"\b(?:gradlew?|gradle\.bat)\b[^&|]*\btest\b")),
    ("ctest", re.compile(r"\bctest\b")),
    ("cabal-test", re.compile(r"\b(?:cabal|stack)\s+test\b")),
    ("dart-test", re.compile(r"\bdart\s+test\b")),
]


# `node [flags...] --test ...` is matched on whitespace-split tokens rather
# than with a regex. The obvious regex, `\bnode\b(?:\s+--?\S+)*\s+--test\b`,
# nests a quantified group whose pieces can each match the same characters,
# so a hostile line such as "node -!\t-!\t-!..." makes it backtrack
# exponentially (CodeQL py/redos). A token scan is linear and says exactly
# what we mean: a `node` executable, then only flag tokens, then `--test`.
def _is_node_test(text: str) -> bool:
    tokens = text.split()
    for i, token in enumerate(tokens):
        name = token.replace("\\", "/").rsplit("/", 1)[-1].lower()
        if name not in ("node", "node.exe"):
            continue
        for follower in tokens[i + 1 :]:
            if follower == "--test":
                return True
            if not follower.startswith("-"):
                break
    return False


NPM_RUN = re.compile(r"\bnpm\s+run\s+(?:--silent\s+)?([\w:.-]+)|\bnpm\s+(test)\b")


def expand_npm(line: str, scripts: dict[str, str]) -> str:
    """Replace ``npm run X`` / ``npm test`` with the script body, one level, so
    ``npm run test:coverage`` is recognised as ``node --test`` or ``vitest``."""

    def repl(match: re.Match) -> str:
        name = match.group(1) or match.group(2)
        return scripts.get(name, match.group(0))

    return NPM_RUN.sub(repl, line)


def runners_in(path: Path, package: Path) -> set[str]:
    scripts = {
        k: v
        for k, v in (package_json(package).get("scripts") or {}).items()
        if isinstance(v, str)
    }
    found: set[str] = set()
    for line in read_command_lines(path):
        text = expand_npm(line, scripts)
        for name, pattern in RUNNER_PATTERNS:
            matched = _is_node_test(text) if pattern is None else pattern.search(text)
            if matched:
                found.add(name)
    return found


def check_runner_mismatch(package: Path, rel: str) -> list[Finding]:
    build = package / "BUILD"
    windows = package / "BUILD_windows"
    if not (build.is_file() and windows.is_file()):
        return []
    if is_starlark(build) or is_starlark(windows):
        return []
    build_runners = runners_in(build, package)
    windows_runners = runners_in(windows, package)
    # An intentional skip stub runs no runner; that is not drift.
    if not build_runners or not windows_runners or build_runners == windows_runners:
        return []
    # Disjoint sets mean one runner was swapped for another (vitest left behind
    # after a move to node --test).  A strict subset or superset means Windows
    # deliberately runs fewer or extra runners (cargo skipped, say) -- advisory.
    disjoint = not (build_runners & windows_runners)
    return [
        Finding(
            "D",
            rel,
            "BUILD_windows",
            f"BUILD runs {sorted(build_runners)} but BUILD_windows runs {sorted(windows_runners)}",
            advisory=not disjoint,
        )
    ]


# ---------------------------------------------------------------------------
# Class E -- POSIX-only syntax in the BUILD Windows falls back to
# ---------------------------------------------------------------------------

# The same table the build tool's `windows-posix-syntax` rule uses
# (validator/windows_posix_syntax.go), minus nothing: this audit just applies
# it to the *fallback* BUILD, which that rule does not read.
WORD_START = r"(?:^|[\s(!;])"
POSIX_CONSTRUCTS = [
    ("/dev/null", re.compile(r"/dev/null")),
    ("command -v", re.compile(WORD_START + r"command\s+-v(?:\s|$)")),
    ("$(...)", re.compile(r"\$\(")),
    ("${...}", re.compile(r"\$\{")),
    ("[ ... ] test", re.compile(WORD_START + r"\[\[?(?:\s|$)")),
    ("; then/fi/do/done/esac", re.compile(r";\s*(?:then|fi|do|done|esac)(?:[\s;)]|$)")),
]


def strip_double_quoted(text: str) -> str:
    """cmd hands double-quoted text to the program untouched, so it is the
    program's business, not the shell's."""
    return re.sub(r'"[^"]*"', '""', text)


def posix_construct(line: str) -> str:
    visible = strip_double_quoted(line)
    for name, pattern in POSIX_CONSTRUCTS:
        if pattern.search(visible):
            return name
    return ""


# Packages whose fallback BUILD is POSIX-flavoured today and that have no
# unambiguous Windows translation yet.  They are reported as advisory so the
# check can block *new* drift without first demanding these be solved.  Remove
# an entry when its package gains a BUILD_windows.
KNOWN_POSIX_FALLBACKS = {
    # `luarocks show X >/dev/null 2>&1 || (cd ../dep && sh BUILD)`: the cmd
    # redirect fails, so the `||` branch always runs and `sh BUILD` rebuilds the
    # dependency chain.  Noisy but working; a faithful BUILD_windows needs a
    # per-dependency translation (`call BUILD_windows` vs an inline
    # `luarocks make`) that the neighbouring Lua packages do not agree on.
    "packages/lua/nib_ir_compiler",
    "packages/lua/nib_type_checker",
    "packages/lua/nib_wasm_compiler",
    # Copies a cargo-built .dylib/.so/.dll with `2>/dev/null ||` fallbacks;
    # needs a Windows-specific copy of the native addon (a design decision).
    "packages/typescript/matrix-rust-napi",
}


def check_windows_posix_fallback(package: Path, rel: str) -> list[Finding]:
    if (package / "BUILD_windows").is_file():
        return []
    build = package / "BUILD"
    if not build.is_file() or is_starlark(build):
        return []
    findings = []
    for line in read_command_lines(build):
        construct = posix_construct(line)
        if construct:
            findings.append(
                Finding(
                    "E",
                    rel,
                    "BUILD",
                    f"Windows runs this BUILD under cmd; {construct}: {line}",
                    advisory=rel in KNOWN_POSIX_FALLBACKS,
                )
            )
    return findings


# ---------------------------------------------------------------------------
# Driver
# ---------------------------------------------------------------------------

CHECKS = (
    check_vitest_drift,
    check_perl5lib,
    check_ruby_standardrb,
    check_runner_mismatch,
    check_windows_posix_fallback,
)


def audit(root: Path) -> list[Finding]:
    findings: list[Finding] = []
    for package in find_packages(root):
        rel = package.relative_to(root / "code").as_posix()
        for check in CHECKS:
            findings.extend(check(package, rel))
    return sorted(findings, key=lambda f: (f.cls, f.package, f.file, f.detail))


def render(findings: list[Finding]) -> str:
    out: list[str] = []
    for cls, title in CLASS_TITLES.items():
        group = [f for f in findings if f.cls == cls]
        out.append(f"[{cls}] {title}: {len(group)}")
        for f in group:
            tag = " (advisory)" if f.advisory else ""
            out.append(f"    {f.package}/{f.file}{tag}: {f.detail}")
    blocking = [f for f in findings if not f.advisory]
    out.append(f"total findings: {len(findings)} ({len(blocking)} blocking)")
    return "\n".join(out)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[2],
        help="repository root (default: this checkout)",
    )
    parser.add_argument(
        "--check", action="store_true", help="exit 1 if anything is found"
    )
    args = parser.parse_args(argv)

    findings = audit(args.root)
    print(render(findings))
    if args.check and any(not f.advisory for f in findings):
        print(
            "\nBuild-front drift found; see the module docstring for what each class means.",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
