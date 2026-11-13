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
  file has *no* Windows build, which is a skip like any other (class F below):
  it needs a real ``BUILD_windows`` or an allowlist entry.
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
    F   windows-skip              The recipe Windows runs does nothing: every
                                  command is an ``echo``/``exit 0``/``if ...
                                  echo`` no-op, it is empty, or the package has
                                  no Windows build file at all.  Blocking unless
                                  the package is on the W6 allowlist.
    G   skip-allowlist            The W6 allowlist itself is wrong: an entry is
                                  stale (the package is no longer a skip, or no
                                  longer exists), malformed, or the list grew
                                  past its cap.  Blocking.
    H   conditional-skip          A real recipe that falls back to
                                  ``echo ... skipping`` when the toolchain is
                                  missing (``where swift && swift test || echo
                                  ... skipping``).  Advisory: it still passes
                                  without testing anything.

Class D is about test *runners* only.  A ``BUILD_windows`` that is an
intentional skip stub (it runs no runner at all) is not a mismatch, and a
runner set that is merely a subset or superset of BUILD's (cargo skipped on
Windows, say) is reported as *advisory*: it never fails ``--check``.  Only a
swapped, disjoint runner set blocks.

THE W6 GUARDRAIL (classes F and G)
==================================

BUILD-002 sets the goal that every package runs real tests on Linux, macOS and
Windows.  An echo-only ``BUILD_windows`` is a gap, not a form of support, so the
audit fails on one unless the package is listed, with a reason, in
``code/specs/data/windows-skip-allowlist.json``.  The list can only shrink:

* every entry carries a ``category`` (``intrinsic-apple``,
  ``intrinsic-platform``, ``no-tests-by-design`` or ``tracked-follow-up``), a
  ``reason`` and ``evidence`` (a spec section or file that backs it);
* an entry whose package is no longer a skip is *stale* and fails the audit
  (class G), so converting a package forces its line out of the list and the
  list cannot rot;
* ``ALLOWLIST_CAPS`` below fixes the number of entries per category.  A new
  entry fails the audit until that cap is raised in the same diff, which makes
  growth a visible, reviewed decision.  Caps are only ever lowered, to match a
  shrinking list; raising one needs an owner decision recorded in BUILD-002.

``intrinsic-apple`` is for what genuinely needs Apple frameworks (Metal,
Objective-C); the audit checks that the package itself says so.  Everything
else must be ``tracked-follow-up`` (a gap with a plan) or a reasoned
``intrinsic-platform`` / ``no-tests-by-design`` case.

What counts as a skip is decided by ``windows_skip_shape``: every effective
line, split on ``&``/``&&``/``||``/``else``, must be a no-op (``echo``,
``exit``/``exit /b 0``, ``cmd /c exit|echo``, ``type nul``, ``ver >nul``,
``call echo``, ``goto :eof``, a PowerShell ``Write-Host``), optionally behind
``if [not] exist|defined|errorlevel|a==b``.  A line that also runs any real
command (``where swift && swift test || echo skipping``) is not a skip; that
shape is the advisory class H.  Comments (``#``, ``REM``, ``::``) and blank
lines do not count as commands.

Class (d) from the original incident notes -- Ruby library code passing sizes
to native methods that overflow a 32-bit C ``long`` on Windows -- needs a
human reading the code, so it is deliberately not audited here.

USAGE
=====

    python3 code/scripts/audit_build_windows.py            # print a report
    python3 code/scripts/audit_build_windows.py --check    # exit 1 on findings
    python3 code/scripts/audit_build_windows.py --root DIR # audit another tree
    python3 code/scripts/audit_build_windows.py --verbose  # list every advisory line

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
    ``rem`` lines, no ``::`` label-comments."""
    lines: list[str] = []
    for raw in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = raw.strip().lstrip("﻿")
        if (
            not line
            or line.startswith(("#", "::"))
            or re.match(r"(?i)^@?rem(\s|$)", line)
        ):
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
    "F": "windows-skip (Windows recipe runs nothing and the package is not allowlisted)",
    "G": "skip-allowlist (stale, malformed or over-cap W6 allowlist entry)",
    "H": "conditional-skip (recipe falls back to `echo ... skipping` if the toolchain is missing)",
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
    # A command run from a POSIX virtualenv: `.venv/bin/python`, or the same
    # reached through `../`. Windows virtualenvs put executables in
    # `.venv\Scripts`, so cmd reports "'.venv' is not recognized". This
    # pattern was missing, and 19 Python packages with no BUILD_windows
    # failed the first time the dependency-pinning work rebuilt them on
    # Windows. Each path segment is `[\w.-]+/` (which also covers `../`), and
    # a segment cannot contain `/`, so the match is unambiguous and linear.
    # A separate `(?:\.\./)*` group would overlap it and backtrack
    # quadratically on a long `../../..` line.
    (".venv/bin/", re.compile(r"(?:^|[\s(&|;=])(?:[\w.-]+/)*\.venv/bin/")),
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
    # Runs its dependency's POSIX BUILD via `bash BUILD` and then that
    # package's `.venv/bin/python`; a Windows version has to call conduit's
    # BUILD_windows and its `.venv\Scripts\python` instead.
    "programs/python/conduit-hello",
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
# Class F -- a Windows recipe that runs nothing (BUILD-002 W6)
# ---------------------------------------------------------------------------

# A line longer than this is never analysed as a no-op: every real recipe line
# is far shorter, and the bound keeps the regular expressions below linear in
# practice on a hostile file (CodeQL py/redos).
MAX_SKIP_LINE = 2000

# `if [/i] [not] <condition> ` -- the conditions cmd offers.  Quoted operands
# are matched as `"[^"]*"`, bare ones as `\S+`; no group is nested inside a
# quantifier, so there is nothing to backtrack exponentially.
IF_PREFIX = re.compile(
    r'(?i)^if\s+(?:/i\s+)?(?:not\s+)?'
    r'(?:exist\s+(?:"[^"]*"|\S+)'
    r"|defined\s+\S+"
    r"|errorlevel\s+\d+"
    r"|cmdextversion\s+\d+"
    r'|(?:"[^"]*"|\S+)\s*==\s*(?:"[^"]*"|\S+)'
    r'|(?:"[^"]*"|\S+)\s+(?:equ|neq|lss|leq|gtr|geq)\s+(?:"[^"]*"|\S+)'
    r")\s*"
)

# Commands that do nothing useful for a build: they print, exit cleanly or are
# a placeholder.  `exit 1` is deliberately absent -- that is a hard failure,
# the opposite of a skip.  `echo` and `cmd /c echo` take any text; the rest must
# match whole (optionally followed by a redirection such as `>nul 2>&1`).
ECHO_COMMAND = re.compile(r"(?i)^(?:cmd(?:\.exe)?\s+/[ck]\s+|call\s+)?echo(?![A-Za-z0-9_-])")
EXACT_NOOP = re.compile(
    r"(?i)^(?:"
    r"(?:cmd(?:\.exe)?\s+/[ck]\s+|call\s+)?exit(?:\s+/b)?(?:\s+0)?"
    r"|type\s+nul"
    r"|ver"
    r"|goto\s+:?eof"
    r")(?:\s*\d?>+\s*(?:nul|&\d)){0,2}\s*$"
)
POWERSHELL_PRINT = ("write-host", "write-output", "write-warning", "echo")

ELSE_SPLIT = re.compile(r"\)\s*else\b\s*", re.IGNORECASE)


def split_segments(line: str) -> list[str]:
    """Split a cmd line into the commands it chains.

    Separators are ``&&``, ``||``, ``&`` and ``) else``, and only outside
    double quotes (cmd hands quoted text to the program untouched).  Parentheses
    are not separators, because ``C:\\Program Files (x86)\\tool.exe`` is one
    command; they are trimmed from each segment's ends instead.
    """
    pieces: list[str] = []
    current: list[str] = []
    in_quote = False
    i = 0
    while i < len(line):
        ch = line[i]
        if ch == '"':
            in_quote = not in_quote
            current.append(ch)
        elif not in_quote and ch in "&|":
            # `&&`, `||` and a lone `&` all end a command; a lone `|` is a pipe
            # whose right side is a real command, so keep it in the segment.
            if line[i : i + 2] in ("&&", "||"):
                pieces.append("".join(current))
                current = []
                i += 1
            elif ch == "&":
                pieces.append("".join(current))
                current = []
            else:
                current.append(ch)
        else:
            current.append(ch)
        i += 1
    pieces.append("".join(current))
    segments: list[str] = []
    for piece in pieces:
        segments.extend(ELSE_SPLIT.split(piece) if not _has_quote(piece) else [piece])
    return [seg.strip() for seg in segments if seg.strip()]


def _has_quote(text: str) -> bool:
    return '"' in text


def _is_powershell_print(segment: str) -> bool:
    """``powershell -Command "Write-Host skipping"`` and nothing else."""
    tokens = segment.split()
    if not tokens or tokens[0].lower().removesuffix(".exe") not in ("powershell", "pwsh"):
        return False
    lowered = [t.lower() for t in tokens]
    for flag in ("-command", "-c"):
        if flag in lowered:
            rest = " ".join(tokens[lowered.index(flag) + 1 :]).lstrip("\"'").lower()
            return rest.startswith(POWERSHELL_PRINT) and ";" not in rest and "&" not in rest
    return False


def is_noop_segment(segment: str) -> bool:
    """True when one chained command does nothing for the build."""
    text = segment.strip().lstrip("@").strip()
    for _ in range(4):  # `if a if b echo x` style nesting, bounded
        text = text.strip().lstrip("(").rstrip(")").strip().lstrip("@").strip()
        match = IF_PREFIX.match(text)
        if not match:
            break
        text = text[match.end() :]
    text = text.strip().lstrip("(").rstrip(")").strip().lstrip("@").strip()
    if not text:
        return True  # `if exist x ()` -- an empty branch
    if ECHO_COMMAND.match(text):
        # `echo y | cargo publish` feeds a real command; split_segments keeps a
        # lone `|` inside its segment so it shows up here.
        return "|" not in strip_double_quoted(text)
    return bool(EXACT_NOOP.match(text)) or _is_powershell_print(text)


def is_noop_line(line: str) -> bool:
    if len(line) > MAX_SKIP_LINE:
        return False
    segments = split_segments(line)
    return bool(segments) and all(is_noop_segment(seg) for seg in segments)


def windows_skip_shape(package: Path) -> tuple[str, str] | None:
    """If the recipe Windows runs for ``package`` does nothing, return
    ``(file, shape)``; otherwise ``None``.

    ``file`` is ``BUILD_windows`` or the ``BUILD`` Windows falls back to, or
    ``(none)`` when no candidate exists.  Starlark BUILD files are evaluated
    rather than run line by line, so they are never a skip.
    """
    effective = effective_build_file(package, WINDOWS_CANDIDATES)
    if effective is None:
        if any(package.glob("BUILD*")):
            return ("(none)", "no Windows build file (only BUILD_linux/BUILD_mac*)")
        return None
    if is_starlark(effective):
        return None
    lines = read_command_lines(effective)
    if not lines:
        return (effective.name, "empty recipe (no commands)")
    if not all(is_noop_line(line) for line in lines):
        return None
    if all(re.match(r"(?i)^@?echo(?![A-Za-z0-9_-])", line) for line in lines):
        return (effective.name, f"echo-only: {lines[0]}")
    return (effective.name, f"no-op only (echo/exit/if-echo): {lines[0]}")


def check_conditional_skip(package: Path, rel: str) -> list[Finding]:
    """Class H: a real command with an ``echo ... skipping`` fallback."""
    effective = effective_build_file(package, WINDOWS_CANDIDATES)
    if effective is None or effective.name != "BUILD_windows" or is_starlark(effective):
        return []
    if windows_skip_shape(package) is not None:
        return []  # a plain skip is class F's business
    findings = []
    for line in read_command_lines(effective):
        if len(line) > MAX_SKIP_LINE:
            continue
        segments = split_segments(line)
        has_real = any(not is_noop_segment(seg) for seg in segments)
        prints_skip = any(
            re.match(r"(?i)^[(@\s]*(?:if\s.*?[(\s])?echo\b.*\bskipping\b", seg) for seg in segments
        )
        if has_real and prints_skip:
            findings.append(
                Finding(
                    "H",
                    rel,
                    effective.name,
                    f"passes without testing when the toolchain is missing: {line}",
                    advisory=True,
                )
            )
    return findings


# ---------------------------------------------------------------------------
# Class G -- the W6 allowlist (data file + caps)
# ---------------------------------------------------------------------------

ALLOWLIST_PATH = ("code", "specs", "data", "windows-skip-allowlist.json")

CATEGORIES = (
    "intrinsic-apple",  # needs Apple frameworks (Metal, Objective-C)
    "intrinsic-platform",  # bound to a platform API with no equivalent here
    "no-tests-by-design",  # nothing to run: rule definitions, shell packages
    "tracked-follow-up",  # a real gap with a plan; must only shrink
)

# How many entries each category may hold.  This is the "can only shrink"
# rule's teeth: adding an entry to the JSON without raising a cap here fails
# the audit, so growth always shows up as a reviewed change to this table.
# Lower a number when an entry is removed; raise one only with an owner decision
# recorded in BUILD-002 (see the module docstring).
ALLOWLIST_CAPS = {
    "intrinsic-apple": 11,
    "intrinsic-platform": 1,
    "no-tests-by-design": 7,
    "tracked-follow-up": 1,
}

# An `intrinsic-apple` entry must be backed by the package itself: its name, its
# BUILD or its Windows recipe has to mention an Apple framework or platform.
APPLE_EVIDENCE = re.compile(r"(?i)metal|objc|darwin|macos|apple")


def load_allowlist(root: Path) -> tuple[dict[str, dict], list[str]]:
    """Read the allowlist; return ``(entries, problems)``.  A missing file is an
    empty list (a throwaway tree, or a repo that has none)."""
    path = root.joinpath(*ALLOWLIST_PATH)
    if not path.is_file():
        return {}, []
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        return {}, [f"{path.name} is not valid JSON: {error}"]
    entries = data.get("entries") if isinstance(data, dict) else None
    if not isinstance(entries, dict):
        return {}, [f"{path.name} must be an object with an `entries` object"]
    return entries, []


def validate_entry(root: Path, rel: str, entry: object) -> list[str]:
    """Everything wrong with one entry's shape, independent of the package."""
    if not isinstance(entry, dict):
        return ["entry must be an object"]
    problems = []
    category = entry.get("category")
    if category not in CATEGORIES:
        problems.append(f"category must be one of {', '.join(CATEGORIES)}")
    for key in ("reason", "evidence"):
        value = entry.get(key)
        if not isinstance(value, str) or len(value.strip()) < 10:
            problems.append(f"`{key}` must be a real sentence (at least 10 characters)")
    if category == "intrinsic-apple":
        package = root / "code" / rel
        haystack = rel
        for name in ("BUILD", "BUILD_windows"):
            if (package / name).is_file():
                haystack += (package / name).read_text(encoding="utf-8", errors="replace")
        if not APPLE_EVIDENCE.search(haystack):
            problems.append(
                "intrinsic-apple, but nothing in the package names Metal/ObjC/Darwin/macOS"
            )
    return problems


def check_skip_allowlist(root: Path, skips: dict[str, tuple[str, str]]) -> list[Finding]:
    """Class F (an unlisted skip) and class G (a bad or stale entry)."""
    entries, problems = load_allowlist(root)
    findings = [Finding("G", "(allowlist)", ALLOWLIST_PATH[-1], p) for p in problems]
    shown = "/".join(ALLOWLIST_PATH)

    for rel, (file, shape) in sorted(skips.items()):
        if rel not in entries:
            findings.append(
                Finding(
                    "F",
                    rel,
                    file,
                    f"{shape}.  Give it a real recipe; if it truly cannot run on "
                    f"Windows, add a reasoned entry to {shown} (BUILD-002 W6)",
                )
            )

    counts: dict[str, int] = {}
    for rel, entry in sorted(entries.items()):
        for problem in validate_entry(root, rel, entry):
            findings.append(Finding("G", rel, ALLOWLIST_PATH[-1], problem))
        category = entry.get("category") if isinstance(entry, dict) else None
        if category in CATEGORIES:
            counts[category] = counts.get(category, 0) + 1
        if rel not in skips:
            gone = not (root / "code" / rel).is_dir()
            why = "the package no longer exists" if gone else "it now runs a real Windows recipe"
            findings.append(
                Finding(
                    "G",
                    rel,
                    ALLOWLIST_PATH[-1],
                    f"stale entry: {why}.  Delete it from the allowlist (and lower "
                    "its category cap in ALLOWLIST_CAPS)",
                )
            )
    for category, count in sorted(counts.items()):
        cap = ALLOWLIST_CAPS.get(category, 0)
        if count > cap:
            findings.append(
                Finding(
                    "G",
                    "(allowlist)",
                    ALLOWLIST_PATH[-1],
                    f"{count} `{category}` entries but the cap is {cap}.  The "
                    "allowlist may only shrink; raising ALLOWLIST_CAPS needs an "
                    "owner decision recorded in BUILD-002",
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
    check_conditional_skip,
)


def audit(root: Path) -> list[Finding]:
    findings: list[Finding] = []
    skips: dict[str, tuple[str, str]] = {}
    for package in find_packages(root):
        rel = package.relative_to(root / "code").as_posix()
        for check in CHECKS:
            findings.extend(check(package, rel))
        skip = windows_skip_shape(package)
        if skip is not None:
            skips[rel] = skip
    findings.extend(check_skip_allowlist(root, skips))
    return sorted(findings, key=lambda f: (f.cls, f.package, f.file, f.detail))


def render(findings: list[Finding], verbose: bool = True) -> str:
    out: list[str] = []
    for cls, title in CLASS_TITLES.items():
        group = [f for f in findings if f.cls == cls]
        out.append(f"[{cls}] {title}: {len(group)}")
        if cls == "H" and group and not verbose:
            # Class H is an inventory of ~130 advisory lines; summarise it per
            # language and leave the full list to --verbose.
            by_language: dict[str, int] = {}
            for f in group:
                language = f.package.split("/")[1]
                by_language[language] = by_language.get(language, 0) + 1
            summary = ", ".join(f"{k}: {v}" for k, v in sorted(by_language.items()))
            out.append(f"    by language: {summary}  (--verbose lists each)")
            continue
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
    parser.add_argument(
        "--verbose", action="store_true", help="list every advisory class H line"
    )
    args = parser.parse_args(argv)

    findings = audit(args.root)
    print(render(findings, verbose=args.verbose))
    if args.check and any(not f.advisory for f in findings):
        print(
            "\nBuild-front drift found; see the module docstring for what each class means.",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
