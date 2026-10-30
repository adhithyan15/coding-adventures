#!/usr/bin/env python3
"""Enforce the repository's monotone third-party dependency inventory."""

from __future__ import annotations

import argparse
import json
import os
import re
import shlex
import subprocess
import sys
import tomllib
from collections.abc import Iterable, Mapping
from pathlib import Path
from typing import Any


DEPENDENCY_TABLES = ("dependencies", "dev-dependencies", "build-dependencies")
NPM_DEPENDENCY_TABLES = (
    "dependencies",
    "devDependencies",
    "peerDependencies",
    "optionalDependencies",
)
DEFAULT_ALLOWLISTS = {
    "cargo": Path("code/specs/data/third-party-dependencies/rust-allowlist.json"),
    "npm": Path("code/specs/data/third-party-dependencies/npm-allowlist.json"),
    "npm-tools": Path(
        "code/specs/data/third-party-dependencies/npm-tool-allowlist.json"
    ),
    "python": Path("code/specs/data/third-party-dependencies/python-allowlist.json"),
    "python-tools": Path(
        "code/specs/data/third-party-dependencies/python-tool-allowlist.json"
    ),
    "ruby": Path("code/specs/data/third-party-dependencies/ruby-allowlist.json"),
    "go": Path("code/specs/data/third-party-dependencies/go-allowlist.json"),
    "go-tools": Path("code/specs/data/third-party-dependencies/go-tool-allowlist.json"),
    "elixir": Path("code/specs/data/third-party-dependencies/elixir-allowlist.json"),
    "elixir-tools": Path(
        "code/specs/data/third-party-dependencies/elixir-tool-allowlist.json"
    ),
}
PYTHON_NAME = re.compile(
    r"([A-Za-z0-9](?:[A-Za-z0-9._-]*[A-Za-z0-9])?)"
    r"(?=\s*(?:\[|@|[<>=!~;(]|$))"
)
RUBY_GEM_NAME = r"[A-Za-z0-9](?:[A-Za-z0-9._-]*[A-Za-z0-9])?"
RUBY_GEM_DECLARATION = re.compile(
    rf"^\s*gem\s+(?P<quote>['\"])(?P<name>{RUBY_GEM_NAME})(?P=quote)(?P<tail>.*)$"
)
RUBY_DEPENDENCY_DECLARATION = re.compile(
    rf"^\s*(?:[A-Za-z_]\w*\.)?add_(?:runtime_|development_)?dependency\s+"
    rf"(?P<quote>['\"])(?P<name>{RUBY_GEM_NAME})(?P=quote)(?P<tail>.*)$"
)
RUBY_GEMSPEC_NAME = re.compile(
    rf"^\s*(?:[A-Za-z_]\w*\.)?name\s*=\s*"
    rf"(?P<quote>['\"])(?P<name>{RUBY_GEM_NAME})(?P=quote)\s*(?:#.*)?$"
)
RUBY_PATH_OPTION = re.compile(
    r"\bpath\s*:\s*(?P<quote>['\"])(?P<path>[^'\"]+)(?P=quote)"
)
GO_TOOL_COMMAND_START = re.compile(
    r"(?:^|[;&|()]|[ \t-]run:[ \t]|=[ \t]*[\"'])[ \t]*(?:-[ \t]+)?"
    r"(?:(?:if|then|do|sudo|command|exec|time|env)[ \t]+|![ \t]*)*"
    r"(?:[A-Za-z_][A-Za-z0-9_]*=[^ \t]+[ \t]+)*"
    r"(?:[A-Za-z0-9_.\\/-]+[\\/])?go[ \t]+(?P<action>install|get|run)\b"
)
MIX_APP_DECLARATION = re.compile(r"\bapp\s*:\s*:(?P<name>[A-Za-z_]\w*)")
MIX_DEPENDENCY_DECLARATION = re.compile(
    r"\{\s*:(?P<name>[A-Za-z_]\w*)\s*,(?P<body>.*?)\}", re.DOTALL
)
MIX_PATH_OPTION = re.compile(
    r"\bpath\s*:\s*(?P<quote>['\"])(?P<path>[^'\"]+)(?P=quote)"
)
MIX_TOOL_COMMAND_START = re.compile(
    r"(?:^|[;&|()]|[ \t-]run:[ \t]|=[ \t]*[\"'])[ \t]*(?:-[ \t]+)?"
    r"(?:(?:if|then|do|sudo|command|exec|time|env)[ \t]+|![ \t]*)*"
    r"(?:[A-Za-z_][A-Za-z0-9_]*=[^ \t]+[ \t]+)*"
    r"(?:[A-Za-z0-9_.\\/-]+[\\/])?mix[ \t]+"
    r"(?P<action>archive\.install|escript\.install|local\.hex|local\.rebar)\b"
)
AUTOMATION_TOOL_SOURCE_PATHS = (
    ":(glob).github/**/*.yml",
    ":(glob).github/**/*.yaml",
    ":(glob)**/BUILD",
    ":(glob)**/BUILD_*",
    ":(glob)**/*.sh",
    ":(glob)**/*.bash",
    ":(glob)**/*.zsh",
    ":(glob)**/*.ps1",
    ":(glob)**/*.bat",
    ":(glob)**/*.cmd",
    ":(glob)**/Makefile",
    ":(glob)**/*.mk",
    ":(glob)**/Dockerfile",
    ":(glob)**/Dockerfile.*",
)
NPM_COMMAND_START = re.compile(
    r"(?:^|[;&|()]|[ \t-]run:[ \t]|=[ \t]*[\"'])[ \t]*(?:-[ \t]+)?"
    r"(?:(?:if|then|do|sudo|command|exec|time|env)[ \t]+|![ \t]*)*"
    r"(?:[A-Za-z_][A-Za-z0-9_]*=[^ \t]+[ \t]+)*"
    r"(?P<command>npx|npm)\b"
)
PYTHON_TOOL_COMMAND_START = re.compile(
    r"(?:^|[;&|()]|[ \t-]run:[ \t]|=[ \t]*[\"'])[ \t]*(?:-[ \t]+)?"
    r"(?:(?:if|then|do|sudo|command|exec|time|env)[ \t]+|![ \t]*)*"
    r"(?:[A-Za-z_][A-Za-z0-9_]*=[^ \t]+[ \t]+)*"
    r"(?:"
    r"(?:(?:[A-Za-z0-9_.\\/-]+[\\/])?(?:python3?|py))[ \t]+-m[ \t]+pip"
    r"[ \t]+(?P<module_pip>install)"
    r"|(?:(?:[A-Za-z0-9_.\\/-]+[\\/])?pip3?)[ \t]+(?P<pip>install)"
    r"|uv[ \t]+pip[ \t]+(?P<uv_pip>install)"
    r"|uv[ \t]+tool[ \t]+(?P<uv_tool>install|run)"
    r"|(?P<uvx>uvx)"
    r"|pipx[ \t]+(?P<pipx>install|run)"
    r")\b"
)
SHELL_SEPARATOR = re.compile(r"[ \t]*(?:&&|\|\||[;|()]|\n)")
SHELL_REDIRECTION = re.compile(r"^(?:\d*>>?|\d*<<?)")
NPM_OPTIONS_WITH_VALUES = {
    "--cache",
    "--call",
    "--include",
    "--install-strategy",
    "--loglevel",
    "--omit",
    "--prefix",
    "--registry",
    "--tag",
    "--userconfig",
    "--workspace",
    "-c",
    "-w",
}
NPX_PACKAGE_OPTIONS = {"--package", "-p"}
PYTHON_INSTALL_OPTIONS_WITH_VALUES = {
    "--abi",
    "--cache-dir",
    "--cert",
    "--client-cert",
    "--constraint",
    "--extra-index-url",
    "--find-links",
    "--implementation",
    "--index-url",
    "--platform",
    "--prefix",
    "--proxy",
    "--python",
    "--python-version",
    "--retries",
    "--root",
    "--src",
    "--target",
    "--timeout",
    "--trusted-host",
    "-c",
    "-f",
    "-t",
}
PYTHON_REQUIREMENT_FILE_OPTIONS = {"--requirement", "-r"}
PYTHON_EDITABLE_OPTIONS = {"--editable", "-e"}
PYTHON_RUNNER_OPTIONS_WITH_VALUES = {
    "--index-url",
    "--python",
    "--python-platform",
    "--python-version",
}
PYTHON_RUNNER_PACKAGE_OPTIONS = {"--from", "--spec", "--with"}
GO_OPTIONS_WITH_VALUES = {
    "-C",
    "-coverpkg",
    "-exec",
    "-mod",
    "-modfile",
    "-overlay",
    "-p",
    "-pkgdir",
    "-tags",
    "-toolexec",
}


class PolicyError(ValueError):
    """Raised when an inventory or allowlist violates the policy contract."""


def unique_json_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    """Build a JSON object while rejecting ambiguous duplicate keys."""

    document: dict[str, Any] = {}
    for key, value in pairs:
        if key in document:
            raise PolicyError(f"duplicate JSON key {key!r}")
        document[key] = value
    return document


def git_cargo_manifests(root: Path) -> list[Path]:
    """Return every tracked Cargo manifest, excluding generated files."""

    result = subprocess.run(
        ["git", "ls-files", "-z", "--", ":(glob)**/Cargo.toml"],
        cwd=root,
        check=True,
        capture_output=True,
    )
    return sorted(
        root / raw_path.decode("utf-8", errors="surrogateescape")
        for raw_path in result.stdout.split(b"\0")
        if raw_path
    )


def git_npm_manifests(root: Path) -> list[Path]:
    """Return every tracked npm manifest, excluding generated files."""

    result = subprocess.run(
        ["git", "ls-files", "-z", "--", ":(glob)**/package.json"],
        cwd=root,
        check=True,
        capture_output=True,
    )
    return sorted(
        root / raw_path.decode("utf-8", errors="surrogateescape")
        for raw_path in result.stdout.split(b"\0")
        if raw_path
    )


def git_python_manifests(root: Path) -> list[Path]:
    """Return every tracked Python project manifest."""

    result = subprocess.run(
        ["git", "ls-files", "-z", "--", ":(glob)**/pyproject.toml"],
        cwd=root,
        check=True,
        capture_output=True,
    )
    return sorted(
        root / raw_path.decode("utf-8", errors="surrogateescape")
        for raw_path in result.stdout.split(b"\0")
        if raw_path
    )


def git_ruby_manifests(root: Path) -> list[Path]:
    """Return every tracked Ruby Gemfile and gemspec."""

    result = subprocess.run(
        [
            "git",
            "ls-files",
            "-z",
            "--",
            ":(glob)**/Gemfile",
            ":(glob)**/Gemfile.*",
            ":(glob)**/*.gemspec",
        ],
        cwd=root,
        check=True,
        capture_output=True,
    )
    return sorted(
        root / raw_path.decode("utf-8", errors="surrogateescape")
        for raw_path in result.stdout.split(b"\0")
        if raw_path and not raw_path.endswith(b".lock")
    )


def git_go_manifests(root: Path) -> list[Path]:
    """Return every tracked Go module manifest."""

    result = subprocess.run(
        ["git", "ls-files", "-z", "--", ":(glob)**/go.mod"],
        cwd=root,
        check=True,
        capture_output=True,
    )
    return sorted(
        root / raw_path.decode("utf-8", errors="surrogateescape")
        for raw_path in result.stdout.split(b"\0")
        if raw_path
    )


def git_elixir_manifests(root: Path) -> list[Path]:
    """Return every tracked Elixir Mix manifest."""

    result = subprocess.run(
        ["git", "ls-files", "-z", "--", ":(glob)**/mix.exs"],
        cwd=root,
        check=True,
        capture_output=True,
    )
    return sorted(
        root / raw_path.decode("utf-8", errors="surrogateescape")
        for raw_path in result.stdout.split(b"\0")
        if raw_path
    )


def git_npm_tool_sources(root: Path) -> list[Path]:
    """Return tracked automation files that can acquire npm tools."""

    result = subprocess.run(
        ["git", "ls-files", "-z", "--", *AUTOMATION_TOOL_SOURCE_PATHS],
        cwd=root,
        check=True,
        capture_output=True,
    )
    return sorted(
        root / raw_path.decode("utf-8", errors="surrogateescape")
        for raw_path in result.stdout.split(b"\0")
        if raw_path
    )


def git_python_tool_sources(root: Path) -> list[Path]:
    """Return tracked automation files that can acquire Python tools."""

    return git_npm_tool_sources(root)


def git_go_tool_sources(root: Path) -> list[Path]:
    """Return tracked automation and go:generate sources."""

    result = subprocess.run(
        [
            "git",
            "ls-files",
            "-z",
            "--",
            *AUTOMATION_TOOL_SOURCE_PATHS,
            ":(glob)**/*.go",
        ],
        cwd=root,
        check=True,
        capture_output=True,
    )
    return sorted(
        root / raw_path.decode("utf-8", errors="surrogateescape")
        for raw_path in result.stdout.split(b"\0")
        if raw_path
    )


def git_elixir_tool_sources(root: Path) -> list[Path]:
    """Return tracked automation files that can acquire Mix tools."""

    return git_npm_tool_sources(root)


def tracked_text_document(path: Path, repository_root: Path) -> str:
    """Read tracked text from disk or, in a sparse checkout, Git's index."""

    return tracked_text_documents([path], repository_root)[path]


def tracked_text_documents(
    paths: Iterable[Path], repository_root: Path
) -> dict[Path, str]:
    """Batch-read tracked text, including files absent from sparse checkouts."""

    documents: dict[Path, str] = {}
    missing: list[Path] = []
    try:
        for path in paths:
            if path.is_file():
                documents[path] = path.read_text(encoding="utf-8")
            else:
                missing.append(path)
        if not missing:
            return documents
        queries = "".join(
            f":{path.relative_to(repository_root).as_posix()}\n" for path in missing
        ).encode()
        result = subprocess.run(
            ["git", "cat-file", "--batch"],
            cwd=repository_root,
            check=True,
            input=queries,
            capture_output=True,
        )
        output = result.stdout
        offset = 0
        for path in missing:
            header_end = output.find(b"\n", offset)
            if header_end == -1:
                raise PolicyError(f"cannot read {path}: truncated Git object header")
            header = output[offset:header_end].split()
            if len(header) != 3 or header[1] != b"blob":
                raise PolicyError(f"cannot read {path}: invalid Git object header")
            size = int(header[2])
            start = header_end + 1
            end = start + size
            if end >= len(output) or output[end : end + 1] != b"\n":
                raise PolicyError(f"cannot read {path}: truncated Git object")
            documents[path] = output[start:end].decode("utf-8")
            offset = end + 1
        return documents
    except (OSError, UnicodeDecodeError, subprocess.CalledProcessError) as error:
        raise PolicyError(f"cannot batch-read tracked text: {error}") from error


def parse_json_manifest_text(text: str, path: Path) -> Mapping[str, Any]:
    """Parse one package manifest while rejecting duplicate keys."""

    try:
        document = json.loads(text, object_pairs_hook=unique_json_object)
    except json.JSONDecodeError as error:
        raise PolicyError(f"cannot parse {path}: {error}") from error
    if not isinstance(document, Mapping):
        raise PolicyError(f"cannot parse {path}: top level must be a JSON object")
    return document


def json_manifest_document(path: Path, repository_root: Path) -> Mapping[str, Any]:
    """Read a JSON manifest from disk or, in a sparse checkout, Git's index."""

    try:
        if path.is_file():
            text = path.read_text(encoding="utf-8")
        else:
            relative_path = path.relative_to(repository_root).as_posix()
            result = subprocess.run(
                ["git", "show", f":{relative_path}"],
                cwd=repository_root,
                check=True,
                capture_output=True,
            )
            text = result.stdout.decode("utf-8")
    except (
        OSError,
        UnicodeDecodeError,
        subprocess.CalledProcessError,
    ) as error:
        raise PolicyError(f"cannot parse {path}: {error}") from error
    return parse_json_manifest_text(text, path)


def npm_alias_target(specifier: str) -> str | None:
    """Return the package targeted by an npm: alias, if present."""

    if not specifier.startswith("npm:"):
        return None
    target = specifier.removeprefix("npm:")
    separator = target.find("@", 1 if target.startswith("@") else 0)
    package = target if separator == -1 else target[:separator]
    if not package:
        raise PolicyError(f"invalid npm alias specifier {specifier!r}")
    return package


def npm_package_name(specifier: str) -> str | None:
    """Normalize an npm install specifier to its exact package name."""

    alias_marker = specifier.find("@npm:", 1)
    if alias_marker != -1:
        specifier = specifier[alias_marker + 1 :]
    if specifier.startswith("npm:"):
        return npm_alias_target(specifier)
    if (
        not specifier
        or specifier in {"(", ")", "{", "}"}
        or ">" in specifier
        or "<" in specifier
        or specifier.startswith((".", "/", "file:"))
    ):
        return None
    if specifier.startswith(
        ("git+", "git:", "http:", "https:", "github:", "gitlab:", "bitbucket:")
    ):
        raise PolicyError(
            f"remote npm tool specifier {specifier!r} must use a package name"
        )
    if specifier.startswith("@"):
        separator = specifier.find("@", 1)
        package = specifier if separator == -1 else specifier[:separator]
    else:
        package = specifier.split("@", 1)[0]
    if not package or (package.startswith("@") and "/" not in package):
        raise PolicyError(f"invalid npm package specifier {specifier!r}")
    return package


def command_arguments(command: str, end: int) -> list[str]:
    """Tokenize one tool command up to the next shell command separator."""

    remainder = command[end:]
    separator = SHELL_SEPARATOR.search(remainder)
    if separator is not None:
        remainder = remainder[: separator.start()]
    remainder = remainder.strip()
    try:
        return shlex.split(remainder, comments=True, posix=True)
    except ValueError as error:
        if remainder.endswith(("'", '"')):
            try:
                return shlex.split(remainder[:-1], comments=True, posix=True)
            except ValueError:
                pass
        raise PolicyError(f"cannot parse tool command: {error}") from error


def npx_package_names(arguments: list[str]) -> set[str]:
    """Return packages acquired by npx or npm exec arguments."""

    explicit: set[str] = set()
    command_name: str | None = None
    index = 0
    while index < len(arguments):
        argument = arguments[index]
        if argument == "--":
            index += 1
            if index < len(arguments):
                command_name = arguments[index]
            break
        if argument in NPX_PACKAGE_OPTIONS:
            index += 1
            if index >= len(arguments):
                raise PolicyError(f"{argument} requires an npm package")
            name = npm_package_name(arguments[index])
            if name is not None:
                explicit.add(name)
        elif any(argument.startswith(f"{option}=") for option in NPX_PACKAGE_OPTIONS):
            name = npm_package_name(argument.split("=", 1)[1])
            if name is not None:
                explicit.add(name)
        elif argument in NPM_OPTIONS_WITH_VALUES:
            index += 1
            if index >= len(arguments):
                raise PolicyError(f"{argument} requires a value")
        elif argument.startswith("-"):
            pass
        else:
            command_name = argument
            break
        index += 1
    if explicit:
        return explicit
    name = npm_package_name(command_name or "")
    return {name} if name is not None else set()


def npm_install_package_names(arguments: list[str]) -> set[str]:
    """Return explicit package names from npm install/i/add arguments."""

    names: set[str] = set()
    index = 0
    while index < len(arguments):
        argument = arguments[index]
        if argument in NPM_OPTIONS_WITH_VALUES:
            index += 1
            if index >= len(arguments):
                raise PolicyError(f"{argument} requires a value")
        elif argument.startswith("-"):
            pass
        else:
            name = npm_package_name(argument)
            if name is not None:
                names.add(name)
        index += 1
    return names


def npm_tool_names(command: str) -> set[str]:
    """Return npm package names acquired by executable command positions."""

    names: set[str] = set()
    normalized = command.replace("\\\n", " ")
    for line in normalized.splitlines():
        if line.lstrip().startswith("#"):
            continue
        for match in NPM_COMMAND_START.finditer(line):
            executable = match.group("command")
            arguments = command_arguments(line, match.end())
            if executable == "npx":
                names.update(npx_package_names(arguments))
                continue
            if not arguments:
                continue
            subcommand, remaining = arguments[0], arguments[1:]
            if subcommand == "exec":
                names.update(npx_package_names(remaining))
            elif subcommand in {"install", "i", "add"}:
                names.update(npm_install_package_names(remaining))
    return names


def npm_third_party_name(
    alias: str,
    specifier: Any,
    *,
    manifest_path: Path,
    repository_root: Path,
    tracked_manifests: set[Path],
    local_package_names: set[str],
) -> str | None:
    """Return an external npm name, or None for a verified local package edge."""

    if not isinstance(specifier, str) or not specifier:
        raise PolicyError(f"dependency {alias!r} has an invalid npm specifier")
    if specifier.startswith("file:"):
        dependency_path = specifier.removeprefix("file:")
        if not dependency_path:
            raise PolicyError(f"dependency {alias!r} has an invalid file path")
        candidate = (manifest_path.parent / dependency_path / "package.json").resolve()
        try:
            candidate.relative_to(repository_root.resolve())
        except ValueError:
            pass
        else:
            if candidate in tracked_manifests:
                return None
    if specifier.startswith("workspace:") and alias in local_package_names:
        return None
    return npm_alias_target(specifier) or alias


def npm_manifest_dependency_names(
    path: Path,
    repository_root: Path,
    tracked_manifests: set[Path],
    local_package_names: set[str],
    document: Mapping[str, Any] | None = None,
) -> set[str]:
    """Parse one package.json and return its third-party package names."""

    if document is None:
        document = json_manifest_document(path, repository_root)
    names: set[str] = set()
    for table_name in NPM_DEPENDENCY_TABLES:
        table = document.get(table_name)
        if table is None:
            continue
        if not isinstance(table, Mapping):
            raise PolicyError(f"{path} {table_name} must be a JSON object")
        for alias, specifier in table.items():
            if not isinstance(alias, str) or not alias:
                raise PolicyError(f"{path} {table_name} has an invalid package name")
            name = npm_third_party_name(
                alias,
                specifier,
                manifest_path=path,
                repository_root=repository_root,
                tracked_manifests=tracked_manifests,
                local_package_names=local_package_names,
            )
            if name is not None:
                names.add(name)
    return names


def dependency_tables(document: Mapping[str, Any]) -> Iterable[Mapping[str, Any]]:
    """Yield Cargo dependency tables, including workspace and target scopes."""

    for table_name in DEPENDENCY_TABLES:
        table = document.get(table_name)
        if isinstance(table, Mapping):
            yield table

    workspace = document.get("workspace")
    if isinstance(workspace, Mapping):
        table = workspace.get("dependencies")
        if isinstance(table, Mapping):
            yield table

    targets = document.get("target")
    if isinstance(targets, Mapping):
        for target in targets.values():
            if not isinstance(target, Mapping):
                continue
            for table_name in DEPENDENCY_TABLES:
                table = target.get(table_name)
                if isinstance(table, Mapping):
                    yield table


def third_party_name(
    alias: str,
    declaration: Any,
    *,
    manifest_path: Path | None = None,
    repository_root: Path | None = None,
    tracked_manifests: set[Path] | None = None,
) -> str | None:
    """Return the registry/git package name, or None for a local/workspace edge."""

    if isinstance(declaration, str):
        return alias
    if not isinstance(declaration, Mapping):
        raise PolicyError(f"dependency {alias!r} has an unsupported declaration")
    dependency_path = declaration.get("path")
    if dependency_path is not None:
        if not isinstance(dependency_path, str) or not dependency_path:
            raise PolicyError(f"dependency {alias!r} has an invalid path")
        if (
            manifest_path is None
            or repository_root is None
            or tracked_manifests is None
        ):
            raise PolicyError(f"dependency {alias!r} path cannot be verified")
        candidate = (manifest_path.parent / dependency_path / "Cargo.toml").resolve()
        try:
            candidate.relative_to(repository_root.resolve())
        except ValueError:
            pass
        else:
            if candidate in tracked_manifests:
                return None
    if declaration.get("workspace") is True and not any(
        key in declaration for key in ("version", "git", "registry", "package")
    ):
        # The governing declaration is counted in [workspace.dependencies].
        return None
    package = declaration.get("package", alias)
    if not isinstance(package, str) or not package:
        raise PolicyError(f"dependency {alias!r} has an invalid package name")
    return package


def manifest_dependency_names(
    path: Path,
    repository_root: Path | None = None,
    tracked_manifests: set[Path] | None = None,
) -> set[str]:
    """Parse one Cargo manifest and return its third-party package names."""

    try:
        if path.is_file():
            with path.open("rb") as manifest:
                document = tomllib.load(manifest)
        elif repository_root is not None:
            relative_path = path.relative_to(repository_root).as_posix()
            result = subprocess.run(
                ["git", "show", f":{relative_path}"],
                cwd=repository_root,
                check=True,
                capture_output=True,
            )
            document = tomllib.loads(result.stdout.decode("utf-8"))
        else:
            raise FileNotFoundError(path)
    except (
        OSError,
        UnicodeDecodeError,
        subprocess.CalledProcessError,
        tomllib.TOMLDecodeError,
    ) as error:
        raise PolicyError(f"cannot parse {path}: {error}") from error

    names: set[str] = set()
    for table in dependency_tables(document):
        for alias, declaration in table.items():
            name = third_party_name(
                alias,
                declaration,
                manifest_path=path,
                repository_root=repository_root,
                tracked_manifests=tracked_manifests,
            )
            if name is not None:
                names.add(name)
    return names


def build_inventory(root: Path, manifests: Iterable[Path]) -> dict[str, list[str]]:
    """Map each third-party package name to the manifests declaring it."""

    inventory: dict[str, list[str]] = {}
    manifest_list = list(manifests)
    tracked_manifests = {manifest.resolve() for manifest in manifest_list}
    for manifest in manifest_list:
        try:
            display_path = manifest.relative_to(root).as_posix()
        except ValueError:
            display_path = manifest.as_posix()
        for name in manifest_dependency_names(manifest, root, tracked_manifests):
            inventory.setdefault(name, []).append(display_path)
    return {name: sorted(paths) for name, paths in sorted(inventory.items())}


def build_npm_inventory(root: Path, manifests: Iterable[Path]) -> dict[str, list[str]]:
    """Map each third-party npm name to the manifests declaring it."""

    inventory: dict[str, list[str]] = {}
    manifest_list = list(manifests)
    tracked_manifests = {manifest.resolve() for manifest in manifest_list}
    documents = {
        manifest: json_manifest_document(manifest, root) for manifest in manifest_list
    }
    local_package_names = {
        name
        for document in documents.values()
        if isinstance((name := document.get("name")), str) and name
    }
    for manifest in manifest_list:
        try:
            display_path = manifest.relative_to(root).as_posix()
        except ValueError:
            display_path = manifest.as_posix()
        for name in npm_manifest_dependency_names(
            manifest,
            root,
            tracked_manifests,
            local_package_names,
            documents[manifest],
        ):
            inventory.setdefault(name, []).append(display_path)
    return {name: sorted(paths) for name, paths in sorted(inventory.items())}


def build_npm_tool_inventory(
    root: Path,
    sources: Iterable[Path],
    manifests: Iterable[Path],
) -> dict[str, list[str]]:
    """Map npm tools acquired by tracked automation to their source files."""

    source_list = list(sources)
    manifest_list = list(manifests)
    documents = tracked_text_documents([*source_list, *manifest_list], root)
    inventory: dict[str, set[str]] = {}
    for source in source_list:
        display_path = source.relative_to(root).as_posix()
        try:
            names = npm_tool_names(documents[source])
        except PolicyError as error:
            raise PolicyError(f"cannot parse {display_path}: {error}") from error
        for name in names:
            inventory.setdefault(name, set()).add(display_path)
    for manifest in manifest_list:
        document = parse_json_manifest_text(documents[manifest], manifest)
        scripts = document.get("scripts")
        if scripts is None:
            continue
        if not isinstance(scripts, Mapping):
            raise PolicyError(f"{manifest} scripts must be a JSON object")
        display_path = manifest.relative_to(root).as_posix()
        for script_name, command in scripts.items():
            if not isinstance(script_name, str) or not isinstance(command, str):
                raise PolicyError(f"{manifest} scripts must map strings to strings")
            try:
                names = npm_tool_names(command)
            except PolicyError as error:
                raise PolicyError(
                    f"cannot parse {display_path} script {script_name!r}: {error}"
                ) from error
            for name in names:
                inventory.setdefault(name, set()).add(display_path)
    return {name: sorted(paths) for name, paths in sorted(inventory.items())}


def python_tool_package_name(specifier: str) -> str | None:
    """Normalize a Python tool specifier, excluding local path installs."""

    specifier = specifier.strip()
    if (
        not specifier
        or specifier.startswith((".", "/", "\\", "file:"))
        or re.match(r"^[A-Za-z]:[\\/]", specifier)
    ):
        return None
    if specifier.startswith(("git+", "git:", "http:", "https:")):
        raise PolicyError(
            f"remote Python tool specifier {specifier!r} must use a package name"
        )
    return python_requirement_name(specifier, "Python tool command")


def python_install_package_names(arguments: list[str]) -> set[str]:
    """Return package names acquired by pip or uv pip install."""

    names: set[str] = set()
    index = 0
    while index < len(arguments):
        argument = arguments[index]
        if SHELL_REDIRECTION.match(argument):
            break
        if argument in PYTHON_REQUIREMENT_FILE_OPTIONS or any(
            argument.startswith(f"{option}=")
            for option in PYTHON_REQUIREMENT_FILE_OPTIONS
        ):
            raise PolicyError(
                "Python tool installs must declare packages inline, not by file"
            )
        if argument in PYTHON_EDITABLE_OPTIONS:
            index += 1
            if index >= len(arguments):
                raise PolicyError(f"{argument} requires a Python package or path")
            name = python_tool_package_name(arguments[index])
            if name is not None:
                names.add(name)
        elif any(
            argument.startswith(f"{option}=") for option in PYTHON_EDITABLE_OPTIONS
        ):
            name = python_tool_package_name(argument.split("=", 1)[1])
            if name is not None:
                names.add(name)
        elif argument in PYTHON_INSTALL_OPTIONS_WITH_VALUES:
            index += 1
            if index >= len(arguments):
                raise PolicyError(f"{argument} requires a value")
        elif any(
            argument.startswith(f"{option}=")
            for option in PYTHON_INSTALL_OPTIONS_WITH_VALUES
        ):
            pass
        elif argument.startswith("-"):
            pass
        else:
            name = python_tool_package_name(argument)
            if name is not None:
                names.add(name)
        index += 1
    return names


def python_runner_package_names(arguments: list[str]) -> set[str]:
    """Return packages acquired by pipx, uv tool, or uvx commands."""

    names: set[str] = set()
    command_name: str | None = None
    primary_override = False
    index = 0
    while index < len(arguments):
        argument = arguments[index]
        matching_package_option = next(
            (
                option
                for option in PYTHON_RUNNER_PACKAGE_OPTIONS
                if argument == option or argument.startswith(f"{option}=")
            ),
            None,
        )
        if matching_package_option is not None:
            if argument == matching_package_option:
                index += 1
                if index >= len(arguments):
                    raise PolicyError(f"{argument} requires a Python package")
                specifier = arguments[index]
            else:
                specifier = argument.split("=", 1)[1]
            name = python_tool_package_name(specifier)
            if name is not None:
                names.add(name)
            if matching_package_option in {"--from", "--spec"}:
                primary_override = True
        elif argument in {"--with-requirements"} or argument.startswith(
            "--with-requirements="
        ):
            raise PolicyError(
                "Python tool installs must declare packages inline, not by file"
            )
        elif argument in PYTHON_RUNNER_OPTIONS_WITH_VALUES:
            index += 1
            if index >= len(arguments):
                raise PolicyError(f"{argument} requires a value")
        elif any(
            argument.startswith(f"{option}=")
            for option in PYTHON_RUNNER_OPTIONS_WITH_VALUES
        ):
            pass
        elif argument == "--":
            index += 1
            if index < len(arguments):
                command_name = arguments[index]
            break
        elif argument.startswith("-"):
            pass
        else:
            command_name = argument
            break
        index += 1
    if command_name is not None and not primary_override:
        name = python_tool_package_name(command_name)
        if name is not None:
            names.add(name)
    return names


def python_tool_names(command: str) -> set[str]:
    """Return Python package names acquired by executable command positions."""

    names: set[str] = set()
    normalized = command.replace("\\\n", " ")
    for line in normalized.splitlines():
        if line.lstrip().startswith("#"):
            continue
        for match in PYTHON_TOOL_COMMAND_START.finditer(line):
            arguments = command_arguments(line, match.end())
            if any(match.group(group) for group in ("module_pip", "pip", "uv_pip")):
                names.update(python_install_package_names(arguments))
            else:
                names.update(python_runner_package_names(arguments))
    return names


def build_python_tool_inventory(
    root: Path,
    sources: Iterable[Path],
    manifests: Iterable[Path],
) -> dict[str, list[str]]:
    """Map Python tools acquired by tracked automation to source files."""

    source_list = list(sources)
    manifest_list = list(manifests)
    documents = tracked_text_documents([*source_list, *manifest_list], root)
    inventory: dict[str, set[str]] = {}
    for source in source_list:
        display_path = source.relative_to(root).as_posix()
        try:
            names = python_tool_names(documents[source])
        except PolicyError as error:
            raise PolicyError(f"cannot parse {display_path}: {error}") from error
        for name in names:
            inventory.setdefault(name, set()).add(display_path)
    for manifest in manifest_list:
        document = parse_json_manifest_text(documents[manifest], manifest)
        scripts = document.get("scripts")
        if scripts is None:
            continue
        if not isinstance(scripts, Mapping):
            raise PolicyError(f"{manifest} scripts must be a JSON object")
        display_path = manifest.relative_to(root).as_posix()
        for script_name, command in scripts.items():
            if not isinstance(script_name, str) or not isinstance(command, str):
                raise PolicyError(f"{manifest} scripts must map strings to strings")
            try:
                names = python_tool_names(command)
            except PolicyError as error:
                raise PolicyError(
                    f"cannot parse {display_path} script {script_name!r}: {error}"
                ) from error
            for name in names:
                inventory.setdefault(name, set()).add(display_path)
    return {name: sorted(paths) for name, paths in sorted(inventory.items())}


def canonical_python_name(name: str) -> str:
    """Return the PEP 503 normalized Python distribution name."""

    return re.sub(r"[-_.]+", "-", name).lower()


def python_requirement_name(requirement: Any, label: str) -> str:
    """Extract and normalize the distribution name from a PEP 508 string."""

    if not isinstance(requirement, str):
        raise PolicyError(f"{label} must contain requirement strings")
    match = PYTHON_NAME.match(requirement.strip())
    if match is None:
        raise PolicyError(f"{label} has an invalid requirement {requirement!r}")
    return canonical_python_name(match.group())


def python_requirement_groups(
    document: Mapping[str, Any], path: Path
) -> Iterable[tuple[str, list[Any]]]:
    """Yield standard Python requirement arrays with descriptive labels."""

    build_system = document.get("build-system")
    if build_system is not None:
        if not isinstance(build_system, Mapping):
            raise PolicyError(f"{path} build-system must be a TOML table")
        requires = build_system.get("requires", [])
        if not isinstance(requires, list):
            raise PolicyError(f"{path} build-system.requires must be an array")
        yield "build-system.requires", requires

    project = document.get("project")
    if project is not None:
        if not isinstance(project, Mapping):
            raise PolicyError(f"{path} project must be a TOML table")
        dependencies = project.get("dependencies", [])
        if not isinstance(dependencies, list):
            raise PolicyError(f"{path} project.dependencies must be an array")
        yield "project.dependencies", dependencies
        optional = project.get("optional-dependencies", {})
        if not isinstance(optional, Mapping):
            raise PolicyError(
                f"{path} project.optional-dependencies must be a TOML table"
            )
        for group, requirements in optional.items():
            if not isinstance(group, str) or not isinstance(requirements, list):
                raise PolicyError(
                    f"{path} project.optional-dependencies must map names to arrays"
                )
            yield f"project.optional-dependencies.{group}", requirements

    dependency_groups = document.get("dependency-groups", {})
    if not isinstance(dependency_groups, Mapping):
        raise PolicyError(f"{path} dependency-groups must be a TOML table")
    for group, requirements in dependency_groups.items():
        if not isinstance(group, str) or not isinstance(requirements, list):
            raise PolicyError(f"{path} dependency-groups must map names to arrays")
        strings: list[Any] = []
        for entry in requirements:
            if isinstance(entry, Mapping):
                if (
                    set(entry) != {"include-group"}
                    or not isinstance(entry["include-group"], str)
                    or not entry["include-group"]
                ):
                    raise PolicyError(
                        f"{path} dependency-groups.{group} has an invalid include"
                    )
                continue
            strings.append(entry)
        yield f"dependency-groups.{group}", strings


def python_local_source_names(
    root: Path,
    documents: Mapping[Path, Mapping[str, Any]],
) -> dict[Path, set[str]]:
    """Map projects to names explicitly sourced from tracked local projects."""

    tracked_manifests = {path.resolve() for path in documents}
    local_names = {path.resolve(): set() for path in documents}
    for manifest, document in documents.items():
        tool = document.get("tool", {})
        if not isinstance(tool, Mapping):
            raise PolicyError(f"{manifest} tool must be a TOML table")
        uv = tool.get("uv", {})
        if not isinstance(uv, Mapping):
            raise PolicyError(f"{manifest} tool.uv must be a TOML table")
        workspace = uv.get("workspace", {})
        if not isinstance(workspace, Mapping):
            raise PolicyError(f"{manifest} tool.uv.workspace must be a TOML table")
        members = workspace.get("members", [])
        if not isinstance(members, list) or not all(
            isinstance(member, str) and member for member in members
        ):
            raise PolicyError(f"{manifest} tool.uv.workspace.members must be strings")
        workspace_names: dict[str, Path] = {}
        for member in members:
            candidate = (manifest.parent / member / "pyproject.toml").resolve()
            if candidate not in tracked_manifests:
                raise PolicyError(
                    f"{manifest} workspace member {member!r} is not a tracked project"
                )
            member_project = documents[candidate].get("project", {})
            member_name = (
                member_project.get("name")
                if isinstance(member_project, Mapping)
                else None
            )
            if not isinstance(member_name, str) or not member_name:
                raise PolicyError(f"{candidate} has no valid project.name")
            workspace_names[canonical_python_name(member_name)] = candidate

        sources = uv.get("sources", {})
        if not isinstance(sources, Mapping):
            raise PolicyError(f"{manifest} tool.uv.sources must be a TOML table")
        for source_name, source in sources.items():
            if not isinstance(source_name, str) or not isinstance(source, Mapping):
                raise PolicyError(f"{manifest} tool.uv.sources has an invalid source")
            if PYTHON_NAME.fullmatch(source_name) is None:
                raise PolicyError(
                    f"{manifest} tool.uv.sources has an invalid distribution name"
                )
            normalized_name = canonical_python_name(source_name)
            dependency_path = source.get("path")
            if dependency_path is not None:
                if not isinstance(dependency_path, str) or not dependency_path:
                    raise PolicyError(
                        f"{manifest} source {source_name!r} has an invalid path"
                    )
                candidate = (
                    manifest.parent / dependency_path / "pyproject.toml"
                ).resolve()
                try:
                    candidate.relative_to(root.resolve())
                except ValueError:
                    continue
                if set(source).issubset({"path", "editable"}) and (
                    candidate in tracked_manifests
                ):
                    local_names[manifest.resolve()].add(normalized_name)
            elif source == {"workspace": True}:
                if normalized_name not in workspace_names:
                    raise PolicyError(
                        f"{manifest} source {source_name!r} is not a workspace member"
                    )
                local_names[manifest.resolve()].add(normalized_name)
                for member_manifest in workspace_names.values():
                    local_names[member_manifest].add(normalized_name)
    return local_names


def build_python_inventory(
    root: Path, manifests: Iterable[Path]
) -> dict[str, list[str]]:
    """Map third-party Python names to the manifests declaring them."""

    manifest_list = list(manifests)
    texts = tracked_text_documents(manifest_list, root)
    documents: dict[Path, Mapping[str, Any]] = {}
    for manifest, text in texts.items():
        try:
            document = tomllib.loads(text)
        except tomllib.TOMLDecodeError as error:
            raise PolicyError(f"cannot parse {manifest}: {error}") from error
        documents[manifest.resolve()] = document

    local_names = python_local_source_names(root, documents)
    inventory: dict[str, list[str]] = {}
    for manifest in manifest_list:
        resolved = manifest.resolve()
        display_path = manifest.relative_to(root).as_posix()
        for label, requirements in python_requirement_groups(
            documents[resolved], manifest
        ):
            for requirement in requirements:
                name = python_requirement_name(requirement, f"{manifest} {label}")
                if name not in local_names[resolved]:
                    inventory.setdefault(name, []).append(display_path)
    return {name: sorted(set(paths)) for name, paths in sorted(inventory.items())}


def ruby_gemspec_names(documents: Mapping[Path, str]) -> dict[Path, str]:
    """Return the literal package identity of every tracked gemspec."""

    names: dict[Path, str] = {}
    for manifest, text in documents.items():
        if manifest.suffix != ".gemspec":
            continue
        declarations = [
            match.group("name")
            for line in text.splitlines()
            if (match := RUBY_GEMSPEC_NAME.fullmatch(line)) is not None
        ]
        if len(declarations) != 1:
            raise PolicyError(
                f"{manifest} must contain exactly one literal gemspec name"
            )
        names[manifest.resolve()] = declarations[0]
    return names


def ruby_manifest_declarations(
    manifest: Path, text: str
) -> list[tuple[str, str | None]]:
    """Parse dependency names and literal path options without executing Ruby."""

    declarations: list[tuple[str, str | None]] = []
    for line_number, line in enumerate(text.splitlines(), 1):
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        gem_match = RUBY_GEM_DECLARATION.fullmatch(line)
        dependency_match = RUBY_DEPENDENCY_DECLARATION.fullmatch(line)
        if gem_match is not None:
            tail = gem_match.group("tail")
            path_matches = list(RUBY_PATH_OPTION.finditer(tail))
            if re.search(r"\bpath\s*:", tail) and len(path_matches) != 1:
                raise PolicyError(
                    f"{manifest}:{line_number} has an unsupported gem path declaration"
                )
            declarations.append(
                (
                    gem_match.group("name"),
                    path_matches[0].group("path") if path_matches else None,
                )
            )
            continue
        if dependency_match is not None:
            declarations.append((dependency_match.group("name"), None))
            continue
        if re.match(r"^\s*gem\b", line) or re.match(
            r"^\s*(?:[A-Za-z_]\w*\.)?"
            r"add_(?:runtime_|development_)?dependency\b",
            line,
        ):
            raise PolicyError(
                f"{manifest}:{line_number} has an unsupported Ruby dependency declaration"
            )
    return declarations


def build_ruby_inventory(root: Path, manifests: Iterable[Path]) -> dict[str, list[str]]:
    """Map third-party Ruby gem names to the manifests declaring them."""

    manifest_list = list(manifests)
    documents = tracked_text_documents(manifest_list, root)
    gemspec_names = ruby_gemspec_names(documents)
    local_names = set(gemspec_names.values())
    gemspecs_by_directory: dict[Path, dict[str, Path]] = {}
    for path, name in gemspec_names.items():
        gemspecs_by_directory.setdefault(path.parent, {})[name] = path

    inventory: dict[str, list[str]] = {}
    for manifest in manifest_list:
        display_path = manifest.relative_to(root).as_posix()
        for name, dependency_path in ruby_manifest_declarations(
            manifest, documents[manifest]
        ):
            if manifest.suffix == ".gemspec" and name in local_names:
                continue
            if dependency_path is not None:
                candidate_directory = (manifest.parent / dependency_path).resolve()
                try:
                    candidate_directory.relative_to(root.resolve())
                except ValueError:
                    pass
                else:
                    if name in gemspecs_by_directory.get(candidate_directory, {}):
                        continue
            inventory.setdefault(name, []).append(display_path)
    return {name: sorted(set(paths)) for name, paths in sorted(inventory.items())}


def go_mod_tokens(text: str, manifest: Path) -> list[list[str]]:
    """Tokenize the dependency-bearing subset of go.mod syntax."""

    statements: list[list[str]] = []
    block: str | None = None
    for line_number, line in enumerate(text.splitlines(), 1):
        source = line.split("//", 1)[0].strip()
        if not source:
            continue
        if source == ")":
            if block is None:
                raise PolicyError(f"{manifest}:{line_number} closes no block")
            block = None
            continue
        try:
            tokens = shlex.split(source, comments=False, posix=True)
        except ValueError as error:
            raise PolicyError(
                f"cannot parse {manifest}:{line_number}: {error}"
            ) from error
        if tokens in (["require", "("], ["replace", "("]):
            if block is not None:
                raise PolicyError(f"{manifest}:{line_number} nests a block")
            block = tokens[0]
            continue
        if block is not None:
            tokens.insert(0, block)
        statements.append(tokens)
    if block is not None:
        raise PolicyError(f"{manifest} has an unterminated {block} block")
    return statements


def parse_go_mod(
    text: str, manifest: Path
) -> tuple[str, list[tuple[str, str]], dict[tuple[str, str | None], list[str]]]:
    """Return a module name, requirements, and replacements from go.mod text."""

    module_names: list[str] = []
    requirements: list[tuple[str, str]] = []
    replacements: dict[tuple[str, str | None], list[str]] = {}
    for tokens in go_mod_tokens(text, manifest):
        if not tokens:
            continue
        directive = tokens[0]
        if directive == "module":
            if len(tokens) != 2:
                raise PolicyError(f"{manifest} has an invalid module directive")
            module_names.append(tokens[1])
        elif directive == "require":
            if len(tokens) != 3:
                raise PolicyError(f"{manifest} has an invalid require directive")
            requirements.append((tokens[1], tokens[2]))
        elif directive == "replace":
            try:
                arrow = tokens.index("=>")
            except ValueError as error:
                raise PolicyError(
                    f"{manifest} has a replace directive without =>"
                ) from error
            left = tokens[1:arrow]
            right = tokens[arrow + 1 :]
            if len(left) not in {1, 2} or len(right) not in {1, 2}:
                raise PolicyError(f"{manifest} has an invalid replace directive")
            key = (left[0], left[1] if len(left) == 2 else None)
            if key in replacements:
                raise PolicyError(f"{manifest} replaces {left!r} more than once")
            replacements[key] = right
    if len(module_names) != 1:
        raise PolicyError(f"{manifest} must contain exactly one module directive")
    return module_names[0], requirements, replacements


def build_go_inventory(root: Path, manifests: Iterable[Path]) -> dict[str, list[str]]:
    """Map third-party Go module names to the manifests declaring them."""

    manifest_list = list(manifests)
    documents = tracked_text_documents(manifest_list, root)
    parsed = {
        manifest.resolve(): parse_go_mod(documents[manifest], manifest)
        for manifest in manifest_list
    }
    tracked_modules: dict[Path, str] = {
        manifest: document[0] for manifest, document in parsed.items()
    }
    inventory: dict[str, list[str]] = {}
    for manifest in manifest_list:
        _, requirements, replacements = parsed[manifest.resolve()]
        display_path = manifest.relative_to(root).as_posix()
        for name, version in requirements:
            replacement = replacements.get((name, version)) or replacements.get(
                (name, None)
            )
            dependency_name = name
            if replacement is not None and len(replacement) == 1:
                replacement_path = replacement[0]
                if replacement_path in {".", ".."} or replacement_path.startswith(
                    ("./", "../")
                ):
                    candidate = (
                        manifest.parent / replacement_path / "go.mod"
                    ).resolve()
                    try:
                        candidate.relative_to(root.resolve())
                    except ValueError:
                        pass
                    else:
                        if tracked_modules.get(candidate) == name:
                            continue
                elif not replacement_path.startswith(("/", "\\")):
                    dependency_name = replacement_path
            elif replacement is not None and len(replacement) == 2:
                dependency_name = replacement[0]
            inventory.setdefault(dependency_name, []).append(display_path)
    return {name: sorted(set(paths)) for name, paths in sorted(inventory.items())}


def go_tool_target_name(target: str) -> str | None:
    """Return the import path acquired by one Go tool target."""

    if (
        not target
        or target in {".", ".."}
        or target.startswith(("./", "../", "/", "\\"))
        or target.endswith(".go")
        or re.match(r"^[A-Za-z]:[\\/]", target)
    ):
        return None
    if any(marker in target for marker in ("$", "`", "{", "}")):
        raise PolicyError(f"dynamic Go tool target {target!r} is not allowed")
    if target.startswith(("http:", "https:", "git:")):
        raise PolicyError(f"remote Go tool target {target!r} must use an import path")
    name = target.rsplit("@", 1)[0]
    if not name or "/" not in name:
        raise PolicyError(f"invalid Go tool import path {target!r}")
    return name


def go_tool_names(command: str) -> set[str]:
    """Return package names acquired by executable Go command positions."""

    names: set[str] = set()
    normalized = command.replace("\\\n", " ")
    for line in normalized.splitlines():
        if line.lstrip().startswith("#"):
            continue
        for match in GO_TOOL_COMMAND_START.finditer(line):
            arguments = command_arguments(line, match.end())
            targets: list[str] = []
            index = 0
            while index < len(arguments):
                argument = arguments[index]
                if SHELL_REDIRECTION.match(argument):
                    break
                if argument in GO_OPTIONS_WITH_VALUES:
                    index += 1
                    if index >= len(arguments):
                        raise PolicyError(f"{argument} requires a value")
                elif any(
                    argument.startswith(f"{option}=")
                    for option in GO_OPTIONS_WITH_VALUES
                ):
                    pass
                elif argument.startswith("-"):
                    pass
                else:
                    targets.append(argument)
                    if match.group("action") == "run":
                        break
                index += 1
            for target in targets:
                name = go_tool_target_name(target)
                if name is not None:
                    names.add(name)
    return names


def build_go_tool_inventory(
    root: Path,
    sources: Iterable[Path],
    manifests: Iterable[Path],
) -> dict[str, list[str]]:
    """Map Go tools acquired by tracked automation to their source files."""

    source_list = list(sources)
    manifest_list = list(manifests)
    documents = tracked_text_documents([*source_list, *manifest_list], root)
    inventory: dict[str, set[str]] = {}
    for source in source_list:
        display_path = source.relative_to(root).as_posix()
        text = documents[source]
        if source.suffix == ".go":
            text = "\n".join(
                line.removeprefix("//go:generate").lstrip()
                for line in text.splitlines()
                if line.startswith("//go:generate")
            )
        try:
            names = go_tool_names(text)
        except PolicyError as error:
            raise PolicyError(f"cannot parse {display_path}: {error}") from error
        for name in names:
            inventory.setdefault(name, set()).add(display_path)
    for manifest in manifest_list:
        document = parse_json_manifest_text(documents[manifest], manifest)
        scripts = document.get("scripts")
        if scripts is None:
            continue
        if not isinstance(scripts, Mapping):
            raise PolicyError(f"{manifest} scripts must be a JSON object")
        display_path = manifest.relative_to(root).as_posix()
        for script_name, command in scripts.items():
            if not isinstance(script_name, str) or not isinstance(command, str):
                raise PolicyError(f"{manifest} scripts must map strings to strings")
            try:
                names = go_tool_names(command)
            except PolicyError as error:
                raise PolicyError(
                    f"cannot parse {display_path} script {script_name!r}: {error}"
                ) from error
            for name in names:
                inventory.setdefault(name, set()).add(display_path)
    return {name: sorted(paths) for name, paths in sorted(inventory.items())}


def elixir_without_comments(text: str) -> str:
    """Remove line comments while preserving quoted dependency values."""

    output: list[str] = []
    for line in text.splitlines():
        quote: str | None = None
        escaped = False
        kept: list[str] = []
        for character in line:
            if escaped:
                kept.append(character)
                escaped = False
                continue
            if character == "\\" and quote is not None:
                kept.append(character)
                escaped = True
                continue
            if character in {"'", '"'}:
                if quote == character:
                    quote = None
                elif quote is None:
                    quote = character
                kept.append(character)
                continue
            if character == "#" and quote is None:
                break
            kept.append(character)
        output.append("".join(kept))
    return "\n".join(output)


def parse_mix_manifest(
    text: str, manifest: Path
) -> tuple[str, list[tuple[str, str | None]]]:
    """Return the literal app identity and dependency declarations."""

    source = elixir_without_comments(text)
    app_names = [match.group("name") for match in MIX_APP_DECLARATION.finditer(source)]
    if len(app_names) != 1:
        raise PolicyError(f"{manifest} must contain exactly one literal app name")
    dependencies: list[tuple[str, str | None]] = []
    for match in MIX_DEPENDENCY_DECLARATION.finditer(source):
        body = match.group("body")
        path_matches = list(MIX_PATH_OPTION.finditer(body))
        if re.search(r"\bpath\s*:", body) and len(path_matches) != 1:
            raise PolicyError(f"{manifest} has an unsupported Mix path declaration")
        dependencies.append(
            (
                match.group("name"),
                path_matches[0].group("path") if path_matches else None,
            )
        )
    return app_names[0], dependencies


def build_elixir_inventory(
    root: Path, manifests: Iterable[Path]
) -> dict[str, list[str]]:
    """Map third-party Hex package names to Mix manifests."""

    manifest_list = list(manifests)
    documents = tracked_text_documents(manifest_list, root)
    parsed = {
        manifest.resolve(): parse_mix_manifest(documents[manifest], manifest)
        for manifest in manifest_list
    }
    tracked_apps = {manifest: document[0] for manifest, document in parsed.items()}
    inventory: dict[str, list[str]] = {}
    for manifest in manifest_list:
        _, dependencies = parsed[manifest.resolve()]
        display_path = manifest.relative_to(root).as_posix()
        for name, dependency_path in dependencies:
            if dependency_path is not None:
                candidate = (manifest.parent / dependency_path / "mix.exs").resolve()
                try:
                    candidate.relative_to(root.resolve())
                except ValueError:
                    pass
                else:
                    if tracked_apps.get(candidate) == name:
                        continue
            inventory.setdefault(name, []).append(display_path)
    return {name: sorted(set(paths)) for name, paths in sorted(inventory.items())}


def mix_tool_package_name(arguments: list[str]) -> str | None:
    """Return the named Hex package acquired by a Mix installer."""

    operands = [argument for argument in arguments if not argument.startswith("-")]
    if not operands:
        raise PolicyError("Mix installer requires a package source")
    source = operands[0]
    if any(marker in source for marker in ("$", "`", "{", "}")):
        raise PolicyError(f"dynamic Mix tool source {source!r} is not allowed")
    if source in {"git", "github"} or source.startswith(("git:", "http:", "https:")):
        raise PolicyError("Mix tool installs must use a named Hex package")
    if source.startswith((".", "/", "\\")):
        return None
    if source == "hex":
        if len(operands) < 2:
            raise PolicyError("Mix hex installer requires a package name")
        source = operands[1]
    source = source.removeprefix(":")
    if re.fullmatch(r"[a-z][A-Za-z0-9_-]*", source) is None:
        raise PolicyError(f"invalid Mix tool package {source!r}")
    return source


def mix_tool_names(command: str) -> set[str]:
    """Return packages acquired by executable Mix installer commands."""

    names: set[str] = set()
    normalized = command.replace("\\\n", " ")
    for line in normalized.splitlines():
        if line.lstrip().startswith("#"):
            continue
        for match in MIX_TOOL_COMMAND_START.finditer(line):
            action = match.group("action")
            if action == "local.hex":
                names.add("hex")
                continue
            if action == "local.rebar":
                names.add("rebar3")
                continue
            name = mix_tool_package_name(command_arguments(line, match.end()))
            if name is not None:
                names.add(name)
    return names


def build_elixir_tool_inventory(
    root: Path,
    sources: Iterable[Path],
    manifests: Iterable[Path],
) -> dict[str, list[str]]:
    """Map Mix tools acquired by tracked automation to their source files."""

    source_list = list(sources)
    manifest_list = list(manifests)
    documents = tracked_text_documents([*source_list, *manifest_list], root)
    inventory: dict[str, set[str]] = {}
    for source in source_list:
        display_path = source.relative_to(root).as_posix()
        try:
            names = mix_tool_names(documents[source])
        except PolicyError as error:
            raise PolicyError(f"cannot parse {display_path}: {error}") from error
        for name in names:
            inventory.setdefault(name, set()).add(display_path)
    for manifest in manifest_list:
        document = parse_json_manifest_text(documents[manifest], manifest)
        scripts = document.get("scripts")
        if scripts is None:
            continue
        if not isinstance(scripts, Mapping):
            raise PolicyError(f"{manifest} scripts must be a JSON object")
        display_path = manifest.relative_to(root).as_posix()
        for script_name, command in scripts.items():
            if not isinstance(script_name, str) or not isinstance(command, str):
                raise PolicyError(f"{manifest} scripts must map strings to strings")
            try:
                names = mix_tool_names(command)
            except PolicyError as error:
                raise PolicyError(
                    f"cannot parse {display_path} script {script_name!r}: {error}"
                ) from error
            for name in names:
                inventory.setdefault(name, set()).add(display_path)
    return {name: sorted(paths) for name, paths in sorted(inventory.items())}


def validate_allowlist(
    document: Any, label: str, ecosystem: str = "cargo"
) -> list[str]:
    """Structurally validate an allowlist JSON document."""

    if not isinstance(document, Mapping):
        raise PolicyError(f"{label} must contain a JSON object")
    if document.get("schema_version") != 1 or document.get("ecosystem") != ecosystem:
        raise PolicyError(f"{label} must be a schema_version 1 {ecosystem} allowlist")
    names = document.get("allowed_dependency_names")
    if not isinstance(names, list) or not all(
        isinstance(name, str) and name for name in names
    ):
        raise PolicyError(f"{label} allowed_dependency_names must be strings")
    if names != sorted(set(names)):
        raise PolicyError(f"{label} allowed_dependency_names must be sorted and unique")
    return names


def load_allowlist(path: Path, ecosystem: str = "cargo") -> list[str]:
    """Load and structurally validate the checked dependency-name allowlist."""

    try:
        document = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=unique_json_object,
        )
    except (OSError, json.JSONDecodeError) as error:
        raise PolicyError(f"cannot read {path}: {error}") from error
    return validate_allowlist(document, str(path), ecosystem)


def load_base_allowlist(
    root: Path, ref: str, path: Path, ecosystem: str = "cargo"
) -> list[str] | None:
    """Load the allowlist at a trusted Git ref, if that ref predates the gate."""

    revision = subprocess.run(
        ["git", "rev-parse", "--verify", ref],
        cwd=root,
        check=False,
        capture_output=True,
        text=True,
    )
    if revision.returncode != 0:
        raise PolicyError(f"cannot resolve dependency-policy base ref {ref!r}")
    listing = subprocess.run(
        ["git", "ls-tree", "--name-only", ref, "--", path.as_posix()],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    if not listing.stdout.strip():
        return None
    result = subprocess.run(
        ["git", "show", f"{ref}:{path.as_posix()}"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    try:
        document = json.loads(
            result.stdout,
            object_pairs_hook=unique_json_object,
        )
    except json.JSONDecodeError as error:
        raise PolicyError(f"cannot read {ref}:{path}: {error}") from error
    return validate_allowlist(document, f"{ref}:{path}", ecosystem)


def policy_differences(
    inventory: Mapping[str, list[str]], allowed_names: Iterable[str]
) -> tuple[list[str], list[str]]:
    """Return newly introduced and stale allowlist names."""

    present = set(inventory)
    allowed = set(allowed_names)
    return sorted(present - allowed), sorted(allowed - present)


def render_failure(
    inventory: Mapping[str, list[str]],
    unknown: list[str],
    stale: list[str],
    allowlist_growth: list[str],
    ecosystem: str = "cargo",
) -> str:
    lines = [f"{ecosystem} third-party dependency policy failed."]
    if unknown:
        lines.append("New dependency names require explicit review:")
        for name in unknown:
            lines.append(f"  {name}: {', '.join(inventory[name])}")
    if stale:
        lines.append("Removed dependency names must be deleted from the allowlist:")
        lines.extend(f"  {name}" for name in stale)
    if allowlist_growth:
        lines.append("The approved dependency set may only shrink:")
        lines.extend(f"  {name}" for name in allowlist_growth)
    return "\n".join(lines)


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("check", "inventory"))
    parser.add_argument(
        "--ecosystem",
        choices=(
            "cargo",
            "npm",
            "npm-tools",
            "python",
            "python-tools",
            "ruby",
            "go",
            "go-tools",
            "elixir",
            "elixir-tools",
        ),
        default="cargo",
    )
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[2]
    )
    parser.add_argument("--allowlist", type=Path)
    parser.add_argument(
        "--base-ref",
        default=os.environ.get("THIRD_PARTY_DEPENDENCY_BASE_REF"),
        help="trusted Git ref whose allowlist is the monotone upper bound",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    root = args.root.resolve()
    try:
        if args.ecosystem == "cargo":
            manifests = git_cargo_manifests(root)
            inventory = build_inventory(root, manifests)
            input_count = len(manifests)
            input_label = "manifests"
        elif args.ecosystem == "npm":
            manifests = git_npm_manifests(root)
            inventory = build_npm_inventory(root, manifests)
            input_count = len(manifests)
            input_label = "manifests"
        elif args.ecosystem == "npm-tools":
            sources = git_npm_tool_sources(root)
            manifests = git_npm_manifests(root)
            inventory = build_npm_tool_inventory(root, sources, manifests)
            input_count = len(sources) + len(manifests)
            input_label = "automation inputs"
        elif args.ecosystem == "python":
            manifests = git_python_manifests(root)
            inventory = build_python_inventory(root, manifests)
            input_count = len(manifests)
            input_label = "manifests"
        elif args.ecosystem == "python-tools":
            sources = git_python_tool_sources(root)
            manifests = git_npm_manifests(root)
            inventory = build_python_tool_inventory(root, sources, manifests)
            input_count = len(sources) + len(manifests)
            input_label = "automation inputs"
        elif args.ecosystem == "ruby":
            manifests = git_ruby_manifests(root)
            inventory = build_ruby_inventory(root, manifests)
            input_count = len(manifests)
            input_label = "manifests"
        elif args.ecosystem == "go":
            manifests = git_go_manifests(root)
            inventory = build_go_inventory(root, manifests)
            input_count = len(manifests)
            input_label = "manifests"
        elif args.ecosystem == "go-tools":
            sources = git_go_tool_sources(root)
            manifests = git_npm_manifests(root)
            inventory = build_go_tool_inventory(root, sources, manifests)
            input_count = len(sources) + len(manifests)
            input_label = "automation inputs"
        elif args.ecosystem == "elixir":
            manifests = git_elixir_manifests(root)
            inventory = build_elixir_inventory(root, manifests)
            input_count = len(manifests)
            input_label = "manifests"
        else:
            sources = git_elixir_tool_sources(root)
            manifests = git_npm_manifests(root)
            inventory = build_elixir_tool_inventory(root, sources, manifests)
            input_count = len(sources) + len(manifests)
            input_label = "automation inputs"
    except (PolicyError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        return 1
    if args.command == "inventory":
        print(json.dumps({"dependency_names": sorted(inventory)}, indent=2))
        return 0

    allowlist_path = args.allowlist or root / DEFAULT_ALLOWLISTS[args.ecosystem]
    try:
        allowed_names = load_allowlist(allowlist_path, args.ecosystem)
        unknown, stale = policy_differences(inventory, allowed_names)
        baseline = None
        if args.base_ref:
            try:
                baseline_path = allowlist_path.resolve().relative_to(root)
            except ValueError as error:
                raise PolicyError(
                    "the allowlist must be inside the repository when --base-ref is used"
                ) from error
            baseline = load_base_allowlist(
                root, args.base_ref, baseline_path, args.ecosystem
            )
    except PolicyError as error:
        print(error, file=sys.stderr)
        return 1
    allowlist_growth = (
        sorted(set(allowed_names) - set(baseline)) if baseline is not None else []
    )
    if unknown or stale or allowlist_growth:
        print(
            render_failure(inventory, unknown, stale, allowlist_growth, args.ecosystem),
            file=sys.stderr,
        )
        return 1
    print(
        f"{args.ecosystem} dependency inventory matches "
        f"{len(allowed_names)} approved names "
        f"across {input_count} {input_label}."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
