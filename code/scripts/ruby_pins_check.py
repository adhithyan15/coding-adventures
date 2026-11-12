#!/usr/bin/env python3
"""Fail when the central Ruby gem lock is stale relative to the repo.

The repository pins every third-party gem with ONE checksummed lock,
``code/packages/ruby/third-party/Gemfile.lock``, served to all packages
through a local gem mirror (see ``ruby_gem_mirror.rb``).  The lock is only
trustworthy if it keeps agreeing with what the 290-odd package Gemfiles and
gemspecs ask for.  This check runs in the cheap contracts job, with no Ruby
and no network, and verifies:

1. every third-party gem any tracked Gemfile or gemspec declares is listed in
   the central Gemfile (in-repo gems -- those named by a tracked gemspec or
   declared with ``path:`` -- are excluded);
2. the central Gemfile spells out every distinct version requirement used,
   so adding a new range anywhere forces a deliberate lock refresh;
3. the version locked for each gem satisfies every requirement string;
4. the central Gemfile only names allowlisted gems;
5. no tracked BUILD file runs `gem install` except for Bundler at an exact
   version (a bare `gem install minitest` would bypass lock and mirror);
6. the lock records checksums for every spec and covers the CI platforms.

Whether the lock itself matches the central Gemfile is checked where Bundler
exists: ``ruby_gem_mirror.rb`` runs ``bundle cache`` under BUNDLE_FROZEN.

Refreshing the lock::

    cd code/packages/ruby/third-party
    bundle lock --add-checksums     # after editing Gemfile
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections.abc import Iterable
from pathlib import Path

import third_party_dependency_policy as policy

THIRD_PARTY_DIR = Path("code/packages/ruby/third-party")
ALLOWLIST = Path("code/specs/data/third-party-dependencies/ruby-allowlist.json")
REQUIRED_PLATFORMS = ("x86_64-linux", "x64-mingw-ucrt", "arm64-darwin", "x86_64-darwin")

# A literal requirement string directly after the gem name: `, "~> 5.0"`.
# Several may follow one another (`"~> 5.0", "< 6"`); options such as
# `require: false` end the run because they are not quoted strings.
_QUOTED_REQUIREMENT = re.compile(r"""\s*,\s*(?P<q>['"])(?P<req>[^'"]*)(?P=q)""")
_REQUIREMENT = re.compile(r"^\s*(?P<op>=|!=|>=|<=|>|<|~>)?\s*(?P<ver>[0-9][0-9A-Za-z.]*)\s*$")
_LOCK_SPEC = re.compile(r"^    (?P<name>\S+) \((?P<ver>[^)]+)\)$")


class RubyPinsError(ValueError):
    """The pinned gem set disagrees with the repository."""


# --- Gem::Version, the part we need ----------------------------------------
#
# RubyGems orders versions by splitting on dots into segments; numeric
# segments compare as integers, and any segment containing a letter marks a
# prerelease that sorts BEFORE the release ("1.0.pre" < "1.0").  Trailing
# zeros are insignificant ("1.0" == "1").


def _segments(version: str) -> list[object]:
    parts: list[object] = []
    for raw in version.replace("-", ".pre.").split("."):
        for piece in re.findall(r"[0-9]+|[A-Za-z]+", raw):
            parts.append(int(piece) if piece.isdigit() else piece)
    while parts and parts[-1] == 0:
        parts.pop()
    return parts


def compare_versions(left: str, right: str) -> int:
    a, b = _segments(left), _segments(right)
    for index in range(max(len(a), len(b))):
        x = a[index] if index < len(a) else 0
        y = b[index] if index < len(b) else 0
        if x == y:
            continue
        if isinstance(x, str) and isinstance(y, int):
            return -1  # a prerelease segment sorts below a number
        if isinstance(x, int) and isinstance(y, str):
            return 1
        return -1 if x < y else 1  # type: ignore[operator]
    return 0


def _pessimistic_upper_bound(version: str) -> str:
    """`~> 1.2.3` allows < 1.3: drop the last segment, bump the one before."""

    segments = [int(s) for s in version.split(".") if s.isdigit()]
    if len(segments) <= 1:
        return str(segments[0] + 1) if segments else "0"
    kept = segments[:-1]
    kept[-1] += 1
    return ".".join(str(s) for s in kept)


def satisfies(version: str, requirement: str) -> bool:
    """Gem::Requirement#satisfied_by? for one requirement string."""

    match = _REQUIREMENT.match(requirement)
    if match is None:
        raise RubyPinsError(f"unsupported Ruby version requirement {requirement!r}")
    operator = match.group("op") or "="
    target = match.group("ver")
    order = compare_versions(version, target)
    if operator == "=":
        return order == 0
    if operator == "!=":
        return order != 0
    if operator == ">":
        return order > 0
    if operator == ">=":
        return order >= 0
    if operator == "<":
        return order < 0
    if operator == "<=":
        return order <= 0
    return order >= 0 and compare_versions(version, _pessimistic_upper_bound(target)) < 0


# --- Reading manifests ------------------------------------------------------


def _declaration(line: str):
    return policy.RUBY_GEM_DECLARATION.fullmatch(
        line
    ) or policy.RUBY_DEPENDENCY_DECLARATION.fullmatch(line)


def requirements_from_tail(tail: str) -> list[str]:
    found: list[str] = []
    position = 0
    while (match := _QUOTED_REQUIREMENT.match(tail, position)) is not None:
        found.append(match.group("req"))
        position = match.end()
    return found


def declared_requirements(
    documents: dict[Path, str], local_names: set[str], skip: Path | None = None
) -> dict[str, dict[str, set[str]]]:
    """Map third-party gem name -> requirement string -> declaring manifests."""

    result: dict[str, dict[str, set[str]]] = {}
    for manifest, text in documents.items():
        if skip is not None and manifest.resolve() == skip.resolve():
            continue
        for line in text.splitlines():
            if line.strip().startswith("#"):
                continue
            match = _declaration(line)
            if match is None:
                continue
            name, tail = match.group("name"), match.group("tail")
            if name in local_names or policy.RUBY_PATH_OPTION.search(tail):
                continue
            per_gem = result.setdefault(name, {})
            # ">= 0" is RubyGems' spelling of "any version": no constraint.
            for requirement in requirements_from_tail(tail) or [""]:
                if requirement.replace(" ", "") == ">=0":
                    requirement = ""
                per_gem.setdefault(requirement, set()).add(manifest.name)
    return result


def parse_lock(text: str) -> dict[str, object]:
    """Pull the sections of a Gemfile.lock this check needs."""

    section = ""
    specs: dict[str, str] = {}
    platforms: list[str] = []
    dependencies: set[str] = set()
    checksums: set[str] = set()
    for line in text.splitlines():
        if line and not line.startswith(" "):
            section = line.strip()
            continue
        stripped = line.strip()
        if not stripped:
            continue
        if section == "GEM":
            match = _LOCK_SPEC.match(line)
            if match is not None:
                version = match.group("ver").split("-")[0]
                specs.setdefault(match.group("name"), version)
        elif section == "PLATFORMS":
            platforms.append(stripped)
        elif section == "DEPENDENCIES":
            dependencies.add(stripped.split(" ")[0].rstrip("!"))
        elif section == "CHECKSUMS":
            checksums.add(stripped.split(" ")[0])
    return {
        "specs": specs,
        "platforms": platforms,
        "dependencies": dependencies,
        "checksums": checksums,
    }


def gemfile_requirements(text: str) -> dict[str, set[str]]:
    declared: dict[str, set[str]] = {}
    for line in text.splitlines():
        match = policy.RUBY_GEM_DECLARATION.fullmatch(line)
        if match is not None:
            declared.setdefault(match.group("name"), set()).update(
                requirements_from_tail(match.group("tail"))
            )
    return declared


# --- BUILD files must not bypass Bundler -------------------------------------
#
# `gem install minitest rake` ignores the Gemfile, the lock and the local
# mirror, so the gem floats.  The only `gem install` a BUILD or workflow-style
# BUILD_windows may run is Bundler itself, at an exact version.

_GEM_INSTALL = re.compile(r"\bgem\s+install\b(?P<args>.*)$")
_BUNDLER_EXACT = re.compile(
    r"^\s*bundler\s+(?:--version|-v)[ =]\s*['\"]?\d+(?:\.\d+)+['\"]?(?:\s+--[\w-]+)*\s*$"
)


def build_file_problems(root: Path) -> list[str]:
    import subprocess

    listed = subprocess.run(
        ["git", "ls-files", "-z", "--", ":(glob)**/BUILD", ":(glob)**/BUILD_*"],
        cwd=root,
        check=True,
        capture_output=True,
    ).stdout.split(b"\0")
    problems: list[str] = []
    for raw in listed:
        if not raw:
            continue
        relative = raw.decode("utf-8", errors="surrogateescape")
        path = root / relative
        if not path.is_file():
            continue
        for number, line in enumerate(
            path.read_text(encoding="utf-8", errors="replace").splitlines(), 1
        ):
            if line.lstrip().startswith("#"):
                continue
            match = _GEM_INSTALL.search(line)
            if match is not None and not _BUNDLER_EXACT.match(match.group("args")):
                problems.append(
                    f"{relative}:{number} runs `gem install` outside Bundler "
                    f"({line.strip()}); use `bundle install` so the pinned mirror applies"
                )
    return problems


# --- The check --------------------------------------------------------------


def check(root: Path, manifests: Iterable[Path] | None = None) -> list[str]:
    """Return a list of human-readable problems (empty means in sync)."""

    third_party = root / THIRD_PARTY_DIR
    central_gemfile = third_party / "Gemfile"
    central_lock = third_party / "Gemfile.lock"
    problems: list[str] = []
    for needed in (central_gemfile, central_lock):
        if not needed.is_file():
            return [f"{needed.relative_to(root)} is missing"]

    manifest_list = (
        list(manifests) if manifests is not None else policy.git_ruby_manifests(root)
    )
    documents = policy.tracked_text_documents(manifest_list, root)
    local_names = set(policy.ruby_gemspec_names(documents).values())
    declared = declared_requirements(documents, local_names, skip=central_gemfile)

    central = gemfile_requirements(central_gemfile.read_text(encoding="utf-8"))
    lock = parse_lock(central_lock.read_text(encoding="utf-8"))
    specs: dict[str, str] = lock["specs"]  # type: ignore[assignment]
    allowlist = set(
        json.loads((root / ALLOWLIST).read_text(encoding="utf-8"))[
            "allowed_dependency_names"
        ]
    )

    for name in sorted(set(central) - allowlist):
        problems.append(f"central Gemfile lists {name}, which is not allowlisted")
    for name, requirements in sorted(declared.items()):
        if name not in central:
            problems.append(
                f"{name} is used by {sorted(next(iter(requirements.values())))[0]} "
                "but missing from the central Gemfile"
            )
            continue
        if name not in lock["dependencies"]:
            problems.append(f"{name} is in the central Gemfile but not the lock")
        locked = specs.get(name)
        for requirement, users in sorted(requirements.items()):
            if requirement and requirement not in central[name]:
                problems.append(
                    f'central Gemfile must list {name} "{requirement}" '
                    f"(used by {sorted(users)[0]})"
                )
            if requirement and locked is not None:
                try:
                    ok = satisfies(locked, requirement)
                except RubyPinsError as error:
                    problems.append(f"{name}: {error}")
                    continue
                if not ok:
                    problems.append(
                        f'{name} is locked at {locked}, which violates "{requirement}" '
                        f"(used by {sorted(users)[0]})"
                    )
        if locked is None:
            problems.append(f"{name} has no locked version")
    for name in sorted(specs):
        if name not in lock["checksums"]:
            problems.append(f"lock has no checksum for {name}")
    for platform in REQUIRED_PLATFORMS:
        if platform not in lock["platforms"]:
            problems.append(f"lock is missing platform {platform}")
    problems.extend(build_file_problems(root))
    return problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    args = parser.parse_args(argv)
    problems = check(args.root)
    if problems:
        print("The pinned Ruby gem set is stale or inconsistent:", file=sys.stderr)
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        print(
            "Edit code/packages/ruby/third-party/Gemfile and run "
            "`bundle lock --add-checksums` there.",
            file=sys.stderr,
        )
        return 1
    print("Ruby gem pins are in sync with every Gemfile and gemspec.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
