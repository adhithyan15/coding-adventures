"""Helpers for classifying CI workflow edits.

The main monorepo CI workflow can change in two very different ways:

1. Toolchain-scoped edits, such as bumping `actions/setup-dotnet` or
   updating the `.NET` verification step. These should only opt the touched
   toolchains into CI verification.
2. Shared CI behavior edits, such as changing the build commands or matrix.
   Those still need a full rebuild because they can affect every package.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import subprocess

CI_WORKFLOW_PATH = ".github/workflows/ci.yml"

_TOOLCHAIN_MARKERS: dict[str, tuple[str, ...]] = {
    "python": (
        "needs_python", "setup-python", "python-version", "setup-uv",
        "python --version", "uv --version", "pytest",
        "set up python", "install uv",
    ),
    "ruby": (
        "needs_ruby", "setup-ruby", "ruby-version", "bundler",
        "gem install bundler", "ruby --version", "bundle --version",
        "set up ruby", "install bundler",
    ),
    "go": (
        "needs_go", "setup-go", "go-version", "go version", "set up go",
    ),
    "typescript": (
        "needs_typescript", "setup-node", "node-version", "npm install -g jest",
        "node --version", "npm --version", "set up node",
    ),
    "rust": (
        "needs_rust", "rust-toolchain", "cargo", "rustc", "tarpaulin",
        "wasm32-unknown-unknown", "set up rust", "install cargo-tarpaulin",
    ),
    "elixir": (
        "needs_elixir", "setup-beam", "elixir-version", "otp-version",
        "elixir --version", "mix --version", "set up elixir",
    ),
    "lua": (
        "needs_lua", "gh-actions-lua", "gh-actions-luarocks", "luarocks",
        "lua -v", "msvc", "set up lua", "set up luarocks",
    ),
    "perl": (
        "needs_perl", "cpanm", "perl --version", "install cpanm",
    ),
    "haskell": (
        "needs_haskell", "haskell-actions/setup", "ghc-version", "cabal-version",
        "ghc --version", "cabal --version", "set up haskell",
    ),
    "java": (
        "needs_java", "setup-java", "java-version", "java --version",
        "temurin", "set up jdk", "set up gradle", "setup-gradle",
        "disable long-lived gradle services",
        "gradle_opts", "org.gradle.daemon", "org.gradle.vfs.watch",
    ),
    "kotlin": (
        "needs_kotlin", "setup-java", "java-version",
        "temurin", "set up jdk", "set up gradle", "setup-gradle",
        "disable long-lived gradle services",
        "gradle_opts", "org.gradle.daemon", "org.gradle.vfs.watch",
    ),
    "dotnet": (
        "needs_dotnet", "setup-dotnet", "dotnet-version", "dotnet --version",
        "set up .net",
    ),
}

_UNSAFE_MARKERS = (
    "./build-tool",
    "build-tool.exe",
    "-detect-languages",
    "-emit-plan",
    "-force",
    "-plan-file",
    "-validate-build-files",
    "actions/checkout",
    "build-plan",
    "cancel-in-progress:",
    "concurrency:",
    "diff-base",
    "download-artifact",
    "event_name",
    "fetch-depth",
    "git fetch origin main",
    "git_ref",
    "is_main",
    "matrix:",
    "permissions:",
    "pr_base_ref",
    "pull_request:",
    "push:",
    "runs-on:",
    "strategy:",
    "upload-artifact",
)


@dataclass(frozen=True)
class CIWorkflowChange:
    toolchains: frozenset[str]
    requires_full_rebuild: bool = False


def analyze_ci_workflow_changes(root: Path, diff_base: str) -> CIWorkflowChange:
    """Read the current ci.yml patch and classify it."""
    return analyze_ci_workflow_patch(_get_file_diff(root, diff_base, CI_WORKFLOW_PATH))


def analyze_ci_workflow_patch(patch: str) -> CIWorkflowChange:
    toolchains: set[str] = set()
    hunk: list[str] = []

    def flush() -> CIWorkflowChange | None:
        nonlocal hunk
        hunk_toolchains, unsafe = _classify_hunk(hunk)
        hunk = []
        if unsafe:
            return CIWorkflowChange(frozenset(), requires_full_rebuild=True)
        toolchains.update(hunk_toolchains)
        return None

    for line in patch.splitlines():
        if line.startswith("@@"):
            result = flush()
            if result is not None:
                return result
            continue
        if line.startswith(("diff --git ", "index ", "--- ", "+++ ")):
            continue
        hunk.append(line)

    result = flush()
    if result is not None:
        return result
    return CIWorkflowChange(frozenset(toolchains))


def sorted_toolchains(toolchains: frozenset[str] | set[str]) -> list[str]:
    return sorted(toolchains)


def _classify_hunk(lines: list[str]) -> tuple[set[str], bool]:
    hunk_toolchains: set[str] = set()
    changed_toolchains: set[str] = set()
    changed_lines: list[str] = []

    for line in lines:
        if not line or not _is_diff_line(line):
            continue

        content = line[1:].strip()
        hunk_toolchains.update(_detect_toolchains(content))

        if not _is_changed_line(line):
            continue
        if not content or content.startswith("#"):
            continue

        changed_lines.append(content)
        changed_toolchains.update(_detect_toolchains(content))

    if not changed_lines:
        return set(), False

    resolved_toolchains = changed_toolchains
    if not resolved_toolchains:
        if len(hunk_toolchains) != 1:
            return set(), True
        resolved_toolchains = set(hunk_toolchains)

    for content in changed_lines:
        if _touches_shared_ci_behavior(content):
            return set(), True
        if _detect_toolchains(content):
            continue
        if _is_toolchain_scoped_structural_line(content):
            continue
        return set(), True

    return resolved_toolchains, False


def _detect_toolchains(content: str) -> set[str]:
    found: set[str] = set()
    normalized = content.lower()
    for toolchain, markers in _TOOLCHAIN_MARKERS.items():
        if any(marker in normalized for marker in markers):
            found.add(toolchain)
    return found


def _touches_shared_ci_behavior(content: str) -> bool:
    normalized = content.lower()
    return any(marker in normalized for marker in _UNSAFE_MARKERS)


def _is_toolchain_scoped_structural_line(content: str) -> bool:
    return content.startswith((
        "if:", "run:", "shell:", "with:", "env:", "{", "}", "else", "fi", "then",
        "printf ", "echo ", "curl ", "powershell ", "call ", "cd ",
    ))


def _is_diff_line(line: str) -> bool:
    return line.startswith(" ") or _is_changed_line(line)


def _is_changed_line(line: str) -> bool:
    return line.startswith(("+", "-"))


def _get_file_diff(root: Path, diff_base: str, relative_path: str) -> str:
    for args in (
        ["git", "diff", "--unified=0", f"{diff_base}...HEAD", "--", relative_path],
        ["git", "diff", "--unified=0", diff_base, "HEAD", "--", relative_path],
    ):
        result = subprocess.run(
            args,
            cwd=root,
            capture_output=True,
            text=True,
            check=False,
        )
        if result.returncode == 0:
            return result.stdout
    return ""
