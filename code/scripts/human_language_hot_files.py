#!/usr/bin/env python3
"""Measure recent human-language file contention without creating an aggregate.

The report deliberately asks Git for NUL-delimited records. Repository paths are
data, not shell syntax: spaces, quotes, tabs, and newlines must survive exactly,
and no filename is ever interpolated into a command string. See HL42.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Sequence


SCHEMA_VERSION = 1
DEFAULT_COMMIT_COUNT = 200
DEFAULT_PATH_SCOPE = (
    "code/learning/human-languages",
    "code/packages/typescript/human-language-data",
    "code/programs/typescript/language-ladder",
    "code/packages/typescript/script-ductus",
)
COMMIT_ID = re.compile(r"[0-9a-f]{40,64}\Z", re.ASCII)
STATUS = re.compile(r"([ACDMRT])(\d{1,3})?\Z", re.ASCII)
STATUS_ORDER = {status: index for index, status in enumerate("ACDMRT")}


class HotFileReportError(RuntimeError):
    """A concise, content-free error suitable for the command line."""


@dataclass(frozen=True)
class Change:
    """One destination-oriented path change from one commit.

    Two-path records carry a source as well as a destination. The destination
    is the owner that receives the touch; the source remains provenance rather
    than a second touch. This is what prevents a rename from looking like two
    hot files.
    """

    status: str
    path: str
    source_path: str | None = None
    similarity: int | None = None


def positive_commit_count(value: str) -> int:
    """Argparse converter whose domain is the positive integers."""

    try:
        parsed = int(value, 10)
    except ValueError as error:
        raise ValueError("commit count must be a positive integer") from error
    if parsed <= 0:
        raise ValueError("commit count must be a positive integer")
    return parsed


def normalize_scope(value: str) -> str:
    """Return one safe repository-relative POSIX path scope.

    Git accepts a pathspec language far richer than this tool needs. HL42 uses
    literal directory/file roots, so rejecting metasyntax and platform-specific
    separators makes the measurement's population visible from the JSON alone.
    Whitespace remains legal because real repository paths may contain it.
    """

    if not value or "\0" in value or "\\" in value:
        raise HotFileReportError("path scope must be a non-empty POSIX repository path")
    path = PurePosixPath(value)
    if path.is_absolute() or value.endswith("/"):
        raise HotFileReportError("path scope must be repository-relative without a trailing slash")
    if any(part in {"", ".", ".."} for part in value.split("/")):
        raise HotFileReportError("path scope must not contain empty or dot segments")
    # A leading colon invokes Git's pathspec magic. Brackets, stars, and
    # question marks are glob metacharacters. The scope is literal by contract.
    if value.startswith(":") or any(character in value for character in "*?["):
        raise HotFileReportError("path scope must be a literal repository path")
    return path.as_posix()


def _diagnostic(stderr: bytes) -> str:
    """Render one escaped stderr line without trusting the ambient encoding."""

    decoded = stderr.decode("utf-8", errors="backslashreplace").strip()
    first_line = decoded.splitlines()[0] if decoded else "no diagnostic"
    return json.dumps(first_line[:300], ensure_ascii=True)


def _git(root: Path, arguments: Sequence[str]) -> bytes:
    """Run Git without a shell and return its exact stdout bytes."""

    result = subprocess.run(
        ["git", *arguments],
        cwd=root,
        check=False,
        capture_output=True,
    )
    if result.returncode != 0:
        operation = arguments[0] if arguments else "command"
        raise HotFileReportError(
            f"git {operation} failed with exit {result.returncode}: {_diagnostic(result.stderr)}"
        )
    return result.stdout


def _decode_utf8(value: bytes, label: str) -> str:
    try:
        return value.decode("utf-8", errors="strict")
    except UnicodeDecodeError as error:
        raise HotFileReportError(f"{label} is not valid UTF-8") from error


def _repository_root(root: Path) -> Path:
    output = _decode_utf8(
        _git(root, ("rev-parse", "--show-toplevel")),
        "repository root",
    ).strip()
    if not output:
        raise HotFileReportError("git returned an empty repository root")
    return Path(output).resolve()


def _resolve_revision(root: Path, revision: str) -> str:
    if not revision or "\0" in revision:
        raise HotFileReportError("revision must be non-empty and contain no NUL")
    try:
        output = _git(
            root,
            ("rev-parse", "--verify", "--end-of-options", f"{revision}^{{commit}}"),
        )
    except HotFileReportError as error:
        raise HotFileReportError(f"revision could not be resolved: {error}") from error
    commit = _decode_utf8(output, "resolved revision").strip()
    if not COMMIT_ID.fullmatch(commit):
        raise HotFileReportError("git returned a malformed resolved revision")
    return commit


def _selected_commits(
    root: Path,
    revision: str,
    commit_count: int,
    path_scope: Sequence[str],
) -> list[str]:
    output = _decode_utf8(
        _git(
            root,
            (
                "rev-list",
                "--first-parent",
                f"--max-count={commit_count}",
                revision,
                "--",
                *(_literal_pathspec(scope) for scope in path_scope),
            ),
        ),
        "commit list",
    )
    commits = output.splitlines()
    if any(not COMMIT_ID.fullmatch(commit) for commit in commits):
        raise HotFileReportError("git returned a malformed commit list")
    return commits


def _path_token(tokens: Sequence[bytes], index: int, commit: str) -> str:
    if index >= len(tokens):
        raise HotFileReportError(f"malformed name/status stream for commit {commit}")
    return _decode_utf8(tokens[index], f"path in commit {commit}")


def parse_name_status(data: bytes, commit: str) -> list[Change]:
    """Parse `git diff-tree --name-status -z` output, failing closed."""

    if not data:
        return []
    if not data.endswith(b"\0"):
        raise HotFileReportError(f"malformed name/status stream for commit {commit}")
    tokens = data[:-1].split(b"\0")
    changes: list[Change] = []
    index = 0
    while index < len(tokens):
        try:
            status_token = tokens[index].decode("ascii", errors="strict")
        except UnicodeDecodeError as error:
            raise HotFileReportError(
                f"malformed name/status stream for commit {commit}"
            ) from error
        index += 1
        match = STATUS.fullmatch(status_token)
        if match is None:
            raise HotFileReportError(
                f"unsupported git status in name/status stream for commit {commit}"
            )
        status, score_text = match.groups()
        if status in {"R", "C"}:
            if score_text is None:
                raise HotFileReportError(
                    f"rename/copy status has no similarity for commit {commit}"
                )
            similarity = int(score_text, 10)
            if not 0 <= similarity <= 100:
                raise HotFileReportError(
                    f"rename/copy similarity is out of range for commit {commit}"
                )
            source = _path_token(tokens, index, commit)
            destination = _path_token(tokens, index + 1, commit)
            index += 2
            changes.append(Change(status, destination, source, similarity))
        else:
            if score_text is not None:
                raise HotFileReportError(
                    f"unexpected status score in commit {commit}"
                )
            path = _path_token(tokens, index, commit)
            index += 1
            changes.append(Change(status, path))
    return changes


def _changes_for_commit(
    root: Path,
    commit: str,
) -> list[Change]:
    return parse_name_status(
        _git(
            root,
            (
                "diff-tree",
                "--root",
                "--first-parent",
                "--no-commit-id",
                "--name-status",
                "-z",
                "-r",
                "-M",
                commit,
            ),
        ),
        commit,
    )


def _literal_pathspec(scope: str) -> str:
    """Return a top-anchored literal pathspec for a validated scope."""

    return f":(top,literal){scope}"


def _path_in_scope(path: str, path_scope: Sequence[str]) -> bool:
    return any(path == scope or path.startswith(f"{scope}/") for scope in path_scope)


def _change_in_scope(change: Change, path_scope: Sequence[str]) -> bool:
    return _path_in_scope(change.path, path_scope) or (
        change.source_path is not None and _path_in_scope(change.source_path, path_scope)
    )


def _paths_at_revision(
    root: Path,
    revision: str,
    path_scope: Sequence[str],
) -> set[str]:
    """Return exact scoped path membership from the resolved tree."""

    output = _git(
        root,
        (
            "ls-tree",
            "-r",
            "-z",
            "--name-only",
            revision,
            "--",
            *(_literal_pathspec(scope) for scope in path_scope),
        ),
    )
    if not output:
        return set()
    if not output.endswith(b"\0"):
        raise HotFileReportError("malformed tree path stream")
    paths = {
        _decode_utf8(token, "path in resolved tree")
        for token in output[:-1].split(b"\0")
    }
    if any(not _path_in_scope(path, path_scope) for path in paths):
        raise HotFileReportError("git returned a tree path outside the requested scope")
    return paths


def _owner_family(path: str) -> str | None:
    parts = path.split("/")
    for index, part in enumerate(parts[:-1]):
        if part.endswith(".d"):
            return "/".join(parts[: index + 1])
    return None


def _is_metadata_owner(path: str) -> bool:
    name = path.rsplit("/", 1)[-1]
    return name == "_meta" or name.startswith("_meta.")


def _ordered_status_counts(counts: Counter[str]) -> dict[str, int]:
    return {
        status: counts[status]
        for status in sorted(counts, key=lambda item: (STATUS_ORDER[item], item))
    }


def build_report(
    root: Path,
    *,
    revision: str,
    commit_count: int,
    path_scope: Sequence[str],
) -> dict[str, object]:
    """Build the HL42 report from a real Git repository."""

    if commit_count <= 0:
        raise HotFileReportError("commit count must be positive")
    scopes = tuple(sorted({normalize_scope(value) for value in path_scope}))
    if not scopes:
        raise HotFileReportError("at least one path scope is required")

    repository = _repository_root(Path(root))
    resolved = _resolve_revision(repository, revision)
    commits = _selected_commits(repository, resolved, commit_count, scopes)
    present_paths = _paths_at_revision(repository, resolved, scopes)

    file_commits: dict[str, list[str]] = defaultdict(list)
    file_statuses: dict[str, Counter[str]] = defaultdict(Counter)
    family_commits: dict[str, set[str]] = defaultdict(set)
    family_members: dict[str, set[str]] = defaultdict(set)
    family_member_touches: dict[str, Counter[str]] = defaultdict(Counter)
    family_metadata_touches: Counter[str] = Counter()
    renames: list[dict[str, object]] = []

    for commit in commits:
        changes = [
            change
            for change in _changes_for_commit(repository, commit)
            if _change_in_scope(change, scopes)
        ]
        by_path: dict[str, Change] = {}
        for change in changes:
            if not _path_in_scope(change.path, scopes):
                if change.status == "R":
                    renames.append(
                        {
                            "commit": commit,
                            "old_path": change.source_path,
                            "new_path": change.path,
                            "similarity": change.similarity,
                        }
                    )
                continue
            previous = by_path.get(change.path)
            if previous is not None and previous != change:
                raise HotFileReportError(
                    f"git returned conflicting changes for one path in commit {commit}"
                )
            by_path[change.path] = change

        for path in sorted(by_path):
            change = by_path[path]
            file_commits[path].append(commit)
            file_statuses[path][change.status] += 1
            family = _owner_family(path)
            if family is not None:
                family_commits[family].add(commit)
                family_members[family].add(path)
                family_member_touches[family][path] += 1
                if _is_metadata_owner(path):
                    family_metadata_touches[family] += 1
            if change.status == "R":
                renames.append(
                    {
                        "commit": commit,
                        "old_path": change.source_path,
                        "new_path": change.path,
                        "similarity": change.similarity,
                    }
                )

    files = [
        {
            "path": path,
            "commits_touching": len(file_commits[path]),
            "status_counts": _ordered_status_counts(file_statuses[path]),
            "commits": file_commits[path],
            "present_at_revision": path in present_paths,
        }
        for path in file_commits
    ]
    files.sort(key=lambda row: (-int(row["commits_touching"]), str(row["path"])))

    owner_families: list[dict[str, object]] = []
    for family in family_commits:
        member_counts = family_member_touches[family]
        maximum = max(member_counts.values())
        owner_families.append(
            {
                "path": family,
                "commits_touching": len(family_commits[family]),
                "total_member_touches": sum(member_counts.values()),
                "members_touched": len(family_members[family]),
                "metadata_touches": family_metadata_touches[family],
                "max_member_touches": maximum,
                "hottest_members": sorted(
                    path for path, count in member_counts.items() if count == maximum
                ),
            }
        )
    owner_families.sort(
        key=lambda row: (
            -int(row["commits_touching"]),
            -int(row["total_member_touches"]),
            str(row["path"]),
        )
    )

    return {
        "schema_version": SCHEMA_VERSION,
        "repository": str(repository),
        "revision": {"requested": revision, "resolved": resolved},
        "window": {
            "requested_commits": commit_count,
            "effective_commits": len(commits),
            "traversal": "first-parent",
            "newest_commit": commits[0] if commits else None,
            "oldest_commit": commits[-1] if commits else None,
        },
        "path_scope": list(scopes),
        "files": files,
        "owner_families": owner_families,
        "renames": renames,
    }


def render_report(report: dict[str, object]) -> str:
    """Serialize one report deterministically and terminate it with a newline."""

    return json.dumps(report, ensure_ascii=True, indent=2, separators=(",", ": ")) + "\n"


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Emit a deterministic JSON report of recent human-language hot files."
    )
    parser.add_argument(
        "--repo",
        type=Path,
        default=Path.cwd(),
        help="path inside the Git worktree (default: current directory)",
    )
    parser.add_argument(
        "--revision",
        default="HEAD",
        help="commit-ish to measure (default: HEAD)",
    )
    parser.add_argument(
        "--commits",
        type=positive_commit_count,
        default=DEFAULT_COMMIT_COUNT,
        help=f"number of scoped first-parent commits (default: {DEFAULT_COMMIT_COUNT})",
    )
    parser.add_argument(
        "--path",
        action="append",
        dest="path_scope",
        help="literal repository-relative scope; repeat to replace the defaults",
    )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = _parser()
    arguments = parser.parse_args(argv)
    try:
        report = build_report(
            arguments.repo,
            revision=arguments.revision,
            commit_count=arguments.commits,
            path_scope=arguments.path_scope or DEFAULT_PATH_SCOPE,
        )
    except HotFileReportError as error:
        print(f"human-language-hot-files: {error}", file=sys.stderr)
        return 2
    sys.stdout.write(render_report(report))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
