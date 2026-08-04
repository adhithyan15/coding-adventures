"""Audit the repository TypeScript compiler-path portability contracts.

The repository intentionally keeps one extendable TypeScript base config. A
plain relative path in that file is anchored to the base file, not to a child
project. TypeScript 5.5's ``${configDir}`` template is the portable way to say
"the directory of the project being compiled". This module keeps that small
but high-leverage build invariant executable without requiring Node or an npm
install in the CI detection job.

Standalone configs have a second boundary: an emit-capable build must direct
generated files away from tracked source and test trees. Those projects either
opt out with ``noEmit: true`` or declare a non-empty ``outDir``.

Compiler inputs that use Node.js modules or globals have a third boundary:
their package must directly own ``@types/node`` as a development dependency,
and the checked-in npm lock must agree. A small lexer keeps this audit
independent of ``node_modules`` while ignoring comments and string prose.
"""

from __future__ import annotations

import argparse
import json
import re
# The visibility query below uses a fixed Git argument vector and never a shell.
import subprocess  # nosec B404
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path

SHARED_BASE = Path("code/packages/typescript/tsconfig.base.json")
TYPESCRIPT_AREAS = (
    Path("code/packages/typescript"),
    Path("code/programs/typescript"),
)
PORTABLE_PATHS = {
    "rootDir": "${configDir}/src",
    "outDir": "${configDir}/dist",
}
MINIMUM_CONFIG_DIR_VERSION = (5, 5, 0)
VERSION_RE = re.compile(r"^(\d+)\.(\d+)\.(\d+)")
NODE_BUILTINS = frozenset(
    """
    assert async_hooks buffer child_process cluster console crypto dgram
    diagnostics_channel dns domain events fs http http2 https module net os
    path perf_hooks process punycode querystring readline repl stream
    string_decoder sys timers tls trace_events tty url util v8 vm wasi
    worker_threads zlib
    """.split()
)
NODE_LOCK_EXCEPTIONS = {
    Path("code/packages/typescript/matrix-rust-napi"):
        "native N-API workspace regenerates its platform lock per build",
}


@dataclass(frozen=True)
class Issue:
    """One stable repository-contract diagnostic."""

    code: str
    path: str
    message: str


@dataclass(frozen=True)
class AuditSummary:
    """Counts and diagnostics emitted by one repository audit."""

    total_projects: int
    shared_projects: int
    inherited_root_dir: int
    inherited_out_dir: int
    standalone_emit_projects: int
    isolated_standalone_projects: int
    rooted_projects: int
    bounded_root_projects: int
    unbounded_root_projects: int
    outside_root_inputs: int
    node_api_projects: int
    node_provider_projects: int
    missing_node_provider_projects: int
    stale_node_provider_locks: int
    node_lock_exemptions: int
    locked_compilers: int
    issues: tuple[Issue, ...]


class PortabilityError(ValueError):
    """Raised when the shared TypeScript config contract is not portable."""


def _display_path(root: Path, path: Path) -> str:
    try:
        return path.relative_to(root).as_posix()
    except ValueError:
        return path.as_posix()


def _git_visible_files(root: Path, patterns: Iterable[str]) -> list[Path] | None:
    """Return Git-visible paths when ``root`` is a checkout.

    Unit tests use synthetic directories without Git metadata, so callers fall
    back to a bounded filesystem walk when this returns ``None``.
    """

    if not (root / ".git").exists():
        return None
    # The executable and option vector are fixed; root and patterns are arguments.
    result = subprocess.run(  # nosec B603 B607
        [
            "git",
            "-C",
            str(root),
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            *patterns,
        ],
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    if result.returncode != 0:
        return None
    return [root / line for line in result.stdout.splitlines() if line]


def _area_files(root: Path, filename: str) -> list[Path]:
    patterns = [f"{area.as_posix()}/**/{filename}" for area in TYPESCRIPT_AREAS]
    tracked = _git_visible_files(root, patterns)
    if tracked is not None:
        return sorted(tracked)

    found: list[Path] = []
    for relative_area in TYPESCRIPT_AREAS:
        area = root / relative_area
        if not area.exists():
            continue
        found.extend(
            path
            for path in area.rglob(filename)
            if "node_modules" not in path.parts
        )
    return sorted(found)


def _typescript_files(root: Path) -> list[Path]:
    patterns = [
        f"{area.as_posix()}/**/*{suffix}"
        for area in TYPESCRIPT_AREAS
        for suffix in (".ts", ".tsx", ".mts", ".cts")
    ]
    tracked = _git_visible_files(root, patterns)
    if tracked is not None:
        return sorted(tracked)

    found: list[Path] = []
    for relative_area in TYPESCRIPT_AREAS:
        area = root / relative_area
        if not area.exists():
            continue
        found.extend(
            path
            for path in area.rglob("*")
            if path.is_file()
            and path.suffix in {".ts", ".tsx", ".mts", ".cts"}
            and "node_modules" not in path.parts
        )
    return sorted(found)


def _read_json(root: Path, path: Path, issues: list[Issue]) -> object | None:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        issues.append(
            Issue(
                "JSON_INVALID",
                _display_path(root, path),
                f"cannot read strict UTF-8 JSON: {error}",
            )
        )
        return None


def _compiler_options(document: object) -> dict[str, object]:
    if not isinstance(document, dict):
        return {}
    options = document.get("compilerOptions")
    return options if isinstance(options, dict) else {}


def _extends_shared_base(root: Path, config_path: Path, document: object) -> bool:
    if not isinstance(document, dict):
        return False
    extends = document.get("extends")
    if not isinstance(extends, str) or not extends:
        return False
    candidate = (config_path.parent / extends).resolve()
    return candidate == (root / SHARED_BASE).resolve()


def _effective_root_dir(
    root: Path,
    config_path: Path,
    document: object,
    options: dict[str, object],
) -> Path | None:
    raw_root = options.get("rootDir")
    if not isinstance(raw_root, str) and _extends_shared_base(
        root, config_path, document
    ):
        raw_root = PORTABLE_PATHS["rootDir"]
    if not isinstance(raw_root, str) or not raw_root.strip():
        return None

    expanded = raw_root.replace("${configDir}", str(config_path.parent))
    candidate = Path(expanded)
    if not candidate.is_absolute():
        candidate = config_path.parent / candidate
    return candidate.resolve()


def _has_input_boundary(document: object) -> bool:
    return isinstance(document, dict) and any(
        field in document for field in ("include", "files", "exclude")
    )


def _is_within(path: Path, directory: Path) -> bool:
    try:
        path.relative_to(directory)
    except ValueError:
        return False
    return True


@dataclass(frozen=True)
class _Token:
    kind: str
    value: str


def _lex_typescript(source: str) -> tuple[_Token, ...]:
    """Return the tokens needed by the Node API audit.

    This deliberately is not a TypeScript parser. It recognizes identifiers,
    quoted strings, and punctuation while skipping comments and template
    literal prose. Those token classes are sufficient for module specifiers
    and the unqualified Node globals covered by this contract.
    """

    tokens: list[_Token] = []
    index = 0
    length = len(source)

    def scan_template() -> None:
        nonlocal index
        index += 1
        while index < length:
            if source[index] == "\\" and index + 1 < length:
                index += 2
            elif source[index] == "`":
                index += 1
                return
            elif source.startswith("${", index):
                index += 2
                scan_code(stop_at_template_brace=True)
            else:
                index += 1

    def scan_code(*, stop_at_template_brace: bool = False) -> None:
        nonlocal index
        brace_depth = 0
        while index < length:
            char = source[index]
            if char.isspace():
                index += 1
                continue
            if source.startswith("//", index):
                newline = source.find("\n", index + 2)
                index = length if newline < 0 else newline + 1
                continue
            if source.startswith("/*", index):
                closing = source.find("*/", index + 2)
                index = length if closing < 0 else closing + 2
                continue
            if char in {"'", '"'}:
                quote = char
                index += 1
                value: list[str] = []
                while index < length:
                    char = source[index]
                    if char == "\\" and index + 1 < length:
                        value.append(source[index + 1])
                        index += 2
                        continue
                    if char == quote:
                        index += 1
                        break
                    value.append(char)
                    index += 1
                tokens.append(_Token("string", "".join(value)))
                continue
            if char == "`":
                scan_template()
                continue
            if stop_at_template_brace and char == "}":
                if brace_depth == 0:
                    index += 1
                    return
                brace_depth -= 1
            elif stop_at_template_brace and char == "{":
                brace_depth += 1
            if char.isalpha() or char in {"_", "$"}:
                start = index
                index += 1
                while index < length and (
                    source[index].isalnum() or source[index] in {"_", "$"}
                ):
                    index += 1
                tokens.append(_Token("identifier", source[start:index]))
                continue
            tokens.append(_Token("punctuation", char))
            index += 1

    scan_code()
    return tuple(tokens)


def _node_builtin(specifier: str) -> str | None:
    normalized = specifier.removeprefix("node:")
    root = normalized.split("/", 1)[0]
    return root if root in NODE_BUILTINS else None


def _node_api_evidence(source: str) -> tuple[str, ...]:
    tokens = _lex_typescript(source)
    evidence: set[str] = set()
    for index, token in enumerate(tokens):
        previous = tokens[index - 1] if index else None
        following = tokens[index + 1] if index + 1 < len(tokens) else None
        after_following = tokens[index + 2] if index + 2 < len(tokens) else None

        if token.kind == "identifier" and token.value == "from":
            if following is not None and following.kind == "string":
                builtin = _node_builtin(following.value)
                if builtin is not None:
                    evidence.add(f"module:{builtin}")
        elif token.kind == "identifier" and token.value == "import":
            candidate = None
            if following is not None and following.kind == "string":
                candidate = following.value
            elif (
                following is not None
                and following.value == "("
                and after_following is not None
                and after_following.kind == "string"
            ):
                candidate = after_following.value
            if candidate is not None:
                builtin = _node_builtin(candidate)
                if builtin is not None:
                    evidence.add(f"module:{builtin}")
        elif (
            token.kind == "identifier"
            and token.value == "require"
            and (previous is None or previous.value != ".")
            and following is not None
            and following.value == "("
            and after_following is not None
            and after_following.kind == "string"
        ):
            builtin = _node_builtin(after_following.value)
            if builtin is not None:
                evidence.add(f"module:{builtin}")
        elif (
            token.kind == "identifier"
            and token.value == "process"
            and (previous is None or previous.value != ".")
            and following is not None
            and following.value in {".", "["}
        ):
            evidence.add("global:process")
        elif (
            token.kind == "identifier"
            and token.value == "Buffer"
            and (previous is None or previous.value != ".")
            and following is not None
            and following.value in {".", "(", "[", "<"}
        ):
            evidence.add("global:Buffer")
        elif (
            token.kind == "identifier"
            and token.value == "NodeJS"
            and (previous is None or previous.value != ".")
            and following is not None
            and following.value == "."
        ):
            evidence.add("global:NodeJS")
        elif (
            token.kind == "identifier"
            and token.value in {"__dirname", "__filename"}
            and (previous is None or previous.value != ".")
        ):
            evidence.add(f"global:{token.value}")
    return tuple(sorted(evidence))


def _compiler_owned_files(
    effective_root: Path | None,
    project_files: Iterable[Path],
) -> list[Path]:
    if effective_root is None:
        return list(project_files)
    return [path for path in project_files if _is_within(path, effective_root)]


def _node_lock_matches(
    lock: object,
    expected_range: str,
) -> bool:
    if not isinstance(lock, dict):
        return False
    packages = lock.get("packages")
    if not isinstance(packages, dict):
        return False
    root_package = packages.get("")
    if not isinstance(root_package, dict):
        return False
    dev_dependencies = root_package.get("devDependencies")
    if not isinstance(dev_dependencies, dict):
        return False
    if dev_dependencies.get("@types/node") != expected_range:
        return False
    provider = packages.get("node_modules/@types/node")
    return isinstance(provider, dict) and isinstance(provider.get("version"), str)


def _node_lock_exception_valid(root: Path, project: Path) -> bool:
    try:
        relative_project = project.relative_to(root)
    except ValueError:
        return False
    if relative_project not in NODE_LOCK_EXCEPTIONS:
        return False
    ignore_path = project / ".gitignore"
    try:
        ignored_entries = {
            line.strip()
            for line in ignore_path.read_text(encoding="utf-8").splitlines()
            if line.strip() and not line.lstrip().startswith("#")
        }
    except (OSError, UnicodeError):
        return False
    return "package-lock.json" in ignored_entries


def _parse_version(version: object) -> tuple[int, int, int] | None:
    if not isinstance(version, str):
        return None
    match = VERSION_RE.match(version)
    if match is None:
        return None
    return tuple(int(component) for component in match.groups())  # type: ignore[return-value]


def audit_repository(root: Path) -> AuditSummary:
    """Inspect one checkout and return its shared-config portability summary."""

    root = root.resolve()
    issues: list[Issue] = []
    base_path = root / SHARED_BASE
    base_document = _read_json(root, base_path, issues)
    base_options = _compiler_options(base_document)
    for option, expected in PORTABLE_PATHS.items():
        actual = base_options.get(option)
        if actual != expected:
            issues.append(
                Issue(
                    "SHARED_PATH_NOT_PORTABLE",
                    SHARED_BASE.as_posix(),
                    f"compilerOptions.{option} must be {expected!r}, got {actual!r}",
                )
            )

    total_projects = 0
    shared_projects = 0
    inherited_root_dir = 0
    inherited_out_dir = 0
    standalone_emit_projects = 0
    isolated_standalone_projects = 0
    rooted_projects = 0
    bounded_root_projects = 0
    unbounded_root_projects = 0
    outside_root_inputs = 0
    node_api_projects = 0
    node_provider_projects = 0
    missing_node_provider_projects = 0
    stale_node_provider_locks = 0
    node_lock_exemptions = 0
    typescript_files = _typescript_files(root)
    manifest_paths = _area_files(root, "package.json")
    files_by_project: dict[Path, list[Path]] = {
        manifest_path.parent: [] for manifest_path in manifest_paths
    }
    project_directories = set(files_by_project)
    for source_path in typescript_files:
        directory = source_path.parent
        while directory != root:
            if directory in project_directories:
                files_by_project[directory].append(source_path)
                break
            directory = directory.parent

    for manifest_path in manifest_paths:
        manifest = _read_json(root, manifest_path, issues)
        if not isinstance(manifest, dict):
            continue
        scripts = manifest.get("scripts")
        if not isinstance(scripts, dict) or not scripts.get("build"):
            continue
        config_path = manifest_path.with_name("tsconfig.json")
        if not config_path.is_file():
            continue
        config = _read_json(root, config_path, issues)
        if config is None:
            continue
        total_projects += 1
        options = _compiler_options(config)
        if not _extends_shared_base(root, config_path, config):
            if options.get("noEmit") is not True:
                standalone_emit_projects += 1
                out_dir = options.get("outDir")
                if isinstance(out_dir, str) and out_dir.strip():
                    isolated_standalone_projects += 1
                else:
                    issues.append(
                        Issue(
                            "STANDALONE_OUTPUT_NOT_ISOLATED",
                            _display_path(root, config_path),
                            "emit-capable standalone config requires noEmit: true "
                            "or a non-empty compilerOptions.outDir",
                        )
                    )
        else:
            shared_projects += 1
            if "rootDir" not in options:
                inherited_root_dir += 1
            if "outDir" not in options:
                inherited_out_dir += 1

        effective_root = _effective_root_dir(root, config_path, config, options)
        project_files = files_by_project[manifest_path.parent]
        compiler_files = _compiler_owned_files(effective_root, project_files)
        if effective_root is not None:
            rooted_projects += 1
            if _has_input_boundary(config):
                bounded_root_projects += 1
            else:
                outside_files = [
                    path
                    for path in project_files
                    if not _is_within(path, effective_root)
                ]
                if outside_files:
                    unbounded_root_projects += 1
                    outside_root_inputs += len(outside_files)
                    examples = ", ".join(
                        _display_path(root, path) for path in outside_files[:3]
                    )
                    issues.append(
                        Issue(
                            "INPUT_BOUNDARY_MISSING",
                            _display_path(root, config_path),
                            f"effective rootDir excludes {len(outside_files)} tracked "
                            f"TypeScript file(s), including {examples}; declare a "
                            "top-level include, files, or exclude boundary",
                        )
                    )

        evidence_by_file: list[tuple[Path, tuple[str, ...]]] = []
        for source_path in compiler_files:
            try:
                evidence = _node_api_evidence(source_path.read_text(encoding="utf-8"))
            except (OSError, UnicodeError) as error:
                issues.append(
                    Issue(
                        "TYPESCRIPT_SOURCE_INVALID",
                        _display_path(root, source_path),
                        f"cannot read strict UTF-8 TypeScript source: {error}",
                    )
                )
                continue
            if evidence:
                evidence_by_file.append((source_path, evidence))
        if not evidence_by_file:
            continue

        node_api_projects += 1
        dev_dependencies = manifest.get("devDependencies")
        provider_range = (
            dev_dependencies.get("@types/node")
            if isinstance(dev_dependencies, dict)
            else None
        )
        if not isinstance(provider_range, str) or not provider_range.strip():
            missing_node_provider_projects += 1
            example_path, example_evidence = evidence_by_file[0]
            issues.append(
                Issue(
                    "NODE_TYPES_NOT_OWNED",
                    _display_path(root, manifest_path),
                    "compiler input uses Node APIs without direct "
                    f"devDependencies ownership; {_display_path(root, example_path)} "
                    f"uses {', '.join(example_evidence)}",
                )
            )
            continue

        node_provider_projects += 1
        if _node_lock_exception_valid(root, manifest_path.parent):
            node_lock_exemptions += 1
            continue
        lock_path = manifest_path.with_name("package-lock.json")
        lock = _read_json(root, lock_path, issues) if lock_path.is_file() else None
        if not _node_lock_matches(lock, provider_range):
            stale_node_provider_locks += 1
            issues.append(
                Issue(
                    "NODE_TYPES_LOCK_MISMATCH",
                    _display_path(root, lock_path),
                    "root devDependencies and resolved node_modules/@types/node "
                    f"must agree with manifest range {provider_range!r}",
                )
            )

    locked_compilers = 0
    for lock_path in _area_files(root, "package-lock.json"):
        lock = _read_json(root, lock_path, issues)
        if not isinstance(lock, dict):
            continue
        packages = lock.get("packages")
        if not isinstance(packages, dict):
            continue
        compiler = packages.get("node_modules/typescript")
        if not isinstance(compiler, dict) or "version" not in compiler:
            continue
        locked_compilers += 1
        raw_version = compiler.get("version")
        version = _parse_version(raw_version)
        if version is None:
            issues.append(
                Issue(
                    "TYPESCRIPT_VERSION_INVALID",
                    _display_path(root, lock_path),
                    f"cannot parse locked TypeScript version {raw_version!r}",
                )
            )
        elif version < MINIMUM_CONFIG_DIR_VERSION:
            issues.append(
                Issue(
                    "TYPESCRIPT_TOO_OLD",
                    _display_path(root, lock_path),
                    "${configDir} requires TypeScript 5.5 or newer; "
                    f"lock contains {raw_version}",
                )
            )

    return AuditSummary(
        total_projects=total_projects,
        shared_projects=shared_projects,
        inherited_root_dir=inherited_root_dir,
        inherited_out_dir=inherited_out_dir,
        standalone_emit_projects=standalone_emit_projects,
        isolated_standalone_projects=isolated_standalone_projects,
        rooted_projects=rooted_projects,
        bounded_root_projects=bounded_root_projects,
        unbounded_root_projects=unbounded_root_projects,
        outside_root_inputs=outside_root_inputs,
        node_api_projects=node_api_projects,
        node_provider_projects=node_provider_projects,
        missing_node_provider_projects=missing_node_provider_projects,
        stale_node_provider_locks=stale_node_provider_locks,
        node_lock_exemptions=node_lock_exemptions,
        locked_compilers=locked_compilers,
        issues=tuple(issues),
    )


def validate_repository(root: Path) -> AuditSummary:
    """Return a clean audit or raise one stable combined diagnostic."""

    summary = audit_repository(root)
    if summary.issues:
        details = "\n".join(
            f"{issue.code}: {issue.path}: {issue.message}" for issue in summary.issues
        )
        raise PortabilityError(f"TypeScript tsconfig portability failed:\n{details}")
    return summary


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate shared TypeScript tsconfig path portability."
    )
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args()

    try:
        summary = validate_repository(args.root)
    except PortabilityError as error:
        print(error)
        return 1

    print(
        "TypeScript tsconfig portability passed: "
        f"projects={summary.total_projects} "
        f"shared={summary.shared_projects} "
        f"inherited_rootDir={summary.inherited_root_dir} "
        f"inherited_outDir={summary.inherited_out_dir} "
        f"standalone_emit={summary.standalone_emit_projects} "
        f"standalone_isolated={summary.isolated_standalone_projects} "
        f"rooted={summary.rooted_projects} "
        f"bounded_root={summary.bounded_root_projects} "
        f"unbounded_root={summary.unbounded_root_projects} "
        f"outside_root_inputs={summary.outside_root_inputs} "
        f"node_api_projects={summary.node_api_projects} "
        f"node_providers={summary.node_provider_projects} "
        f"missing_node_providers={summary.missing_node_provider_projects} "
        f"stale_node_locks={summary.stale_node_provider_locks} "
        f"node_lock_exceptions={summary.node_lock_exemptions} "
        f"compiler_locks={summary.locked_compilers}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
