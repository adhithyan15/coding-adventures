"""
starlark_evaluator.py -- Evaluate Starlark BUILD Files
=======================================================

This module bridges the gap between Starlark BUILD files and the build tool's
executor. Traditional BUILD files in this monorepo are shell scripts -- each
line is a command run sequentially. Starlark BUILD files are *programs* that
declare targets with explicit sources, dependencies, and build metadata.

Why Starlark BUILD files?
-------------------------

Shell BUILD files have three limitations:

1. **No change detection metadata.** The build tool guesses which files
   matter based on file extensions, not explicit declarations.
2. **No dependency declarations.** Dependencies are parsed from
   language-specific config files (pyproject.toml, go.mod) with heuristic
   matching.
3. **No validation.** A typo in a shell BUILD file only surfaces at build
   time.

Starlark BUILD files solve all three. They're real programs that declare
targets with explicit ``srcs``, ``deps``, and build metadata. The build tool
evaluates them using the Python starlark-interpreter package and extracts
the declared targets.

How evaluation works
--------------------

The evaluation pipeline has five steps:

1. **Read** the BUILD file contents from disk.
2. **Create** a Starlark interpreter with a file resolver rooted at the
   repo root (so ``load()`` statements can find shared rule definitions).
3. **Execute** the BUILD file through the interpreter pipeline:
   ``source -> lexer -> parser -> compiler -> VM -> result``
4. **Extract** the ``_targets`` list from the result's variables. Each
   rule function (like ``py_library()``) appends a dict to ``_targets``.
5. **Convert** each target dict to a ``Target`` dataclass.

Detecting Starlark vs shell BUILD files
----------------------------------------

We use a simple heuristic: scan the first non-comment, non-blank line.
If it starts with ``load(``, ``def ``, or a known rule call like
``py_library(``, it's Starlark. Otherwise it's shell.

This works because shell BUILD files start with commands like ``uv pip
install ...`` or ``go build ./...``, which never look like Starlark
function calls.

Generating shell commands from targets
---------------------------------------

Once we have a ``Target``, we convert it into shell commands that the
build tool's executor can run. Each rule type maps to a standard set of
commands:

+-------------------+-------------------------------------------+
| Rule              | Commands                                  |
+-------------------+-------------------------------------------+
| py_library        | uv pip install + pytest                   |
| go_library        | go build + go test + go vet               |
| ruby_library      | bundle install + rake test                |
| ts_library        | npm install + vitest                      |
| rust_library      | cargo build + cargo test                  |
| elixir_library    | mix deps.get + mix test                   |
+-------------------+-------------------------------------------+

The same mapping is used for ``_binary`` variants of each rule.
"""

from __future__ import annotations

import os
import platform
from dataclasses import dataclass, field
from pathlib import Path


# =========================================================================
# Target Dataclass
# =========================================================================
#
# A Target represents a single build target declared in a Starlark BUILD
# file. Each call to py_library(), go_library(), etc. produces one Target.
# This is the Python equivalent of the Go ``Target`` struct in
# evaluator.go.

@dataclass
class Target:
    """A single build target extracted from a Starlark BUILD file.

    Attributes:
        rule: The rule type, e.g. "py_library", "go_binary".
        name: The target name, e.g. "starlark-vm", "build-tool".
        srcs: Declared source file patterns for change detection.
        deps: Dependencies as "language/package-name" strings.
        test_runner: Test framework: "pytest", "vitest", "minitest", etc.
        entry_point: Binary entry point: "main.py", "src/index.ts", etc.
    """

    rule: str = ""
    name: str = ""
    srcs: list[str] = field(default_factory=list)
    deps: list[str] = field(default_factory=list)
    test_runner: str = ""
    entry_point: str = ""
    commands: list[dict] = field(default_factory=list)


# =========================================================================
# BuildResult Dataclass
# =========================================================================
#
# Holds all targets extracted from evaluating a single BUILD file.
# Most BUILD files declare exactly one target, but the data model
# supports multiple (e.g., a library + a binary in the same package).

@dataclass
class BuildResult:
    """The result of evaluating a Starlark BUILD file.

    Attributes:
        targets: All targets declared by the BUILD file.
    """

    targets: list[Target] = field(default_factory=list)


# =========================================================================
# Starlark Detection
# =========================================================================
#
# The build tool needs to know whether a BUILD file contains Starlark code
# or shell commands, because the two formats are processed completely
# differently. Shell BUILD files are split into lines and executed as-is.
# Starlark BUILD files are parsed, evaluated, and then converted to
# shell commands.
#
# The detection heuristic is simple and fast: look at the first
# non-blank, non-comment line. Starlark files always start with one of:
#   - load("...") -- importing rule definitions
#   - def name(   -- function definition
#   - py_library( -- a known rule call (also go_library, ruby_library, etc.)
#
# Shell files start with commands like:
#   - uv pip install ...
#   - go build ./...
#   - bundle install --quiet
#
# These two sets never overlap, so the heuristic is reliable.

# Known rule function prefixes. If the first significant line starts with
# any of these, the BUILD file is Starlark.
# Schema version for the _ctx build context dict.
# Bump this when making breaking changes to the _ctx structure.
CTX_SCHEMA_VERSION = 1

# OS name normalization: platform.system() -> runtime.GOOS equivalents.
_OS_MAP = {"Darwin": "darwin", "Linux": "linux", "Windows": "windows"}

KNOWN_RULES: tuple[str, ...] = (
    "py_library(",
    "py_binary(",
    "go_library(",
    "go_binary(",
    "ruby_library(",
    "ruby_binary(",
    "ts_library(",
    "ts_binary(",
    "rust_library(",
    "rust_binary(",
    "elixir_library(",
    "elixir_binary(",
)


def is_starlark_build(content: str) -> bool:
    """Detect whether BUILD file content is Starlark or shell.

    Scans the first non-comment, non-blank line for Starlark indicators:
    ``load()``, ``def``, or known rule calls.

    Parameters
    ----------
    content : str
        The raw text content of a BUILD file.

    Returns
    -------
    bool
        True if the content appears to be Starlark, False if shell.

    Examples
    --------
    >>> is_starlark_build('load("rules/python.star", "py_library")\\n')
    True
    >>> is_starlark_build('uv pip install -e ".[dev]"\\n')
    False
    >>> is_starlark_build('# comment\\npy_library(name = "foo")\\n')
    True
    """
    for line in content.splitlines():
        trimmed = line.strip()

        # Skip blank lines and comments.
        if not trimmed or trimmed.startswith("#"):
            continue

        # Check for Starlark patterns.
        if trimmed.startswith("load("):
            return True
        if trimmed.startswith("def "):
            return True
        for rule in KNOWN_RULES:
            if trimmed.startswith(rule):
                return True

        # If we've seen a non-comment, non-blank line that doesn't match
        # any Starlark pattern, it's probably shell. Stop checking.
        # This avoids false positives from shell commands that happen to
        # contain "load" or "def" somewhere in the middle.
        break

    return False


# =========================================================================
# Target Extraction
# =========================================================================
#
# After the Starlark interpreter evaluates a BUILD file, the result's
# variables dict contains a ``_targets`` list. Each element is a dict
# with keys like "rule", "name", "srcs", "deps", "test_runner", and
# "entry_point".
#
# The extraction helpers below safely pull values out of these dicts,
# handling missing keys and wrong types gracefully. This defensive style
# prevents crashes when a BUILD file has a malformed target declaration.


def _get_string(d: dict, key: str) -> str:
    """Safely extract a string from a dict.

    Returns ``""`` if the key is missing or the value is not a string.
    This is intentionally lenient -- we'd rather have a missing field
    than crash the entire build.

    Parameters
    ----------
    d : dict
        The dictionary to read from.
    key : str
        The key to look up.

    Returns
    -------
    str
        The string value, or ``""`` if not found.
    """
    value = d.get(key)
    if isinstance(value, str):
        return value
    return ""


def _get_string_list(d: dict, key: str) -> list[str]:
    """Safely extract a list of strings from a dict.

    Returns ``[]`` if the key is missing or the value is not a list.
    Non-string elements in the list are silently skipped.

    Parameters
    ----------
    d : dict
        The dictionary to read from.
    key : str
        The key to look up.

    Returns
    -------
    list[str]
        The list of strings, or ``[]`` if not found.
    """
    value = d.get(key)
    if not isinstance(value, list):
        return []
    return [item for item in value if isinstance(item, str)]


def extract_targets(variables: dict) -> list[Target]:
    """Convert the ``_targets`` list from Starlark result variables into Targets.

    Each element in ``_targets`` should be a dict with keys:
    ``rule``, ``name``, ``srcs``, ``deps``, and optionally ``test_runner``
    and ``entry_point``.

    Parameters
    ----------
    variables : dict
        The variables dict from a Starlark interpreter result.

    Returns
    -------
    list[Target]
        Extracted targets, or an empty list if ``_targets`` is not present.

    Raises
    ------
    TypeError
        If ``_targets`` exists but is not a list, or if an element is not a dict.

    Examples
    --------
    >>> extract_targets({"_targets": [{"rule": "py_library", "name": "foo"}]})
    [Target(rule='py_library', name='foo', srcs=[], deps=[], test_runner='', entry_point='')]
    """
    raw_targets = variables.get("_targets")
    if raw_targets is None:
        # No _targets variable -- the BUILD file didn't declare any targets.
        # This is valid (e.g., a BUILD file that only defines helper functions).
        return []

    if not isinstance(raw_targets, list):
        raise TypeError(
            f"_targets is not a list (got {type(raw_targets).__name__})"
        )

    targets: list[Target] = []
    for i, raw in enumerate(raw_targets):
        if not isinstance(raw, dict):
            raise TypeError(
                f"_targets[{i}] is not a dict (got {type(raw).__name__})"
            )

        targets.append(
            Target(
                rule=_get_string(raw, "rule"),
                name=_get_string(raw, "name"),
                srcs=_get_string_list(raw, "srcs"),
                deps=_get_string_list(raw, "deps"),
                test_runner=_get_string(raw, "test_runner"),
                entry_point=_get_string(raw, "entry_point"),
                commands=_get_dict_list(raw, "commands"),
            )
        )

    return targets


# =========================================================================
# BUILD File Evaluation
# =========================================================================
#
# The core function: read a BUILD file, run it through the Starlark
# interpreter, and extract targets. The interpreter needs two pieces of
# context:
#
# 1. **File resolver** -- how to find files referenced by load() statements.
#    We create a resolver that maps labels to files relative to repo_root.
#    For example, load("code/packages/starlark/library-rules/python.star")
#    resolves to <repo_root>/code/packages/starlark/library-rules/python.star.
#
# 2. **The BUILD file source** -- the actual Starlark code to execute.
#
# The interpreter handles everything else: lexing, parsing, compiling to
# bytecode, and executing on the VM.


def evaluate_build_file(
    build_file_path: str | Path,
    pkg_dir: str | Path,
    repo_root: str | Path,
) -> BuildResult:
    """Evaluate a Starlark BUILD file and extract declared targets.

    Runs the full interpreter pipeline on the BUILD file, then extracts
    the ``_targets`` list from the result's variables.

    Parameters
    ----------
    build_file_path : str or Path
        Path to the BUILD file to evaluate.
    pkg_dir : str or Path
        The package directory (for resolving relative paths, e.g. glob()).
    repo_root : str or Path
        The repository root (for resolving load() paths).

    Returns
    -------
    BuildResult
        The extracted targets from the BUILD file.

    Raises
    ------
    FileNotFoundError
        If the BUILD file does not exist.
    RuntimeError
        If the Starlark interpreter fails to evaluate the file.
    """
    build_file_path = Path(build_file_path)
    repo_root = Path(repo_root)

    # Read the BUILD file first — fail fast if it doesn't exist,
    # before attempting to import the interpreter.
    source = build_file_path.read_text(encoding="utf-8")

    # Import the Starlark interpreter lazily. This module is optional --
    # the build tool can function without it (falling back to shell BUILD
    # files). Lazy import keeps startup fast and avoids hard crashes if
    # the starlark-interpreter package isn't installed.
    from starlark_interpreter import StarlarkInterpreter
    if not source.endswith("\n"):
        source += "\n"

    # Create a file resolver that maps load() labels to file contents.
    #
    # When a BUILD file says:
    #   load("code/packages/starlark/library-rules/python_library.star", "py_library")
    #
    # The resolver reads:
    #   <repo_root>/code/packages/starlark/library-rules/python_library.star
    #
    # This matches the convention used by the Go build tool.
    def file_resolver(label: str) -> str:
        full_path = repo_root / label
        try:
            return full_path.read_text(encoding="utf-8")
        except FileNotFoundError:
            raise FileNotFoundError(
                f'load("{label}"): file not found at {full_path}'
            ) from None

    # Build the _ctx dict — the build context injected into every Starlark
    # scope.  This is how Starlark code (cmd.star, rule files) accesses
    # platform and environment information.  See spec 15 for the full schema.
    ctx_dict = {
        "version": CTX_SCHEMA_VERSION,
        "os": _OS_MAP.get(platform.system(), platform.system().lower()),
        "arch": platform.machine(),
        "cpu_count": os.cpu_count() or 1,
        "ci": os.environ.get("CI", "") != "",
        "repo_root": str(repo_root),
    }

    # Create the interpreter and evaluate.
    interp = StarlarkInterpreter(
        file_resolver=file_resolver,
        globals={"_ctx": ctx_dict},
    )

    try:
        result = interp.interpret(source)
    except Exception as exc:
        raise RuntimeError(
            f"Starlark evaluation failed for {build_file_path}: {exc}"
        ) from exc

    # Extract targets from the result's variables.
    targets = extract_targets(result.variables)
    return BuildResult(targets=targets)


# =========================================================================
# Command Generation
# =========================================================================
#
# Once we have a Target, we need to convert it into the shell commands
# that the build tool's executor will run. This is a straightforward
# mapping from rule type to a set of install + test commands.
#
# The mapping mirrors the Go implementation in evaluator.go exactly,
# so both the Python and Go build tools produce identical commands for
# the same BUILD file.
#
# Truth table for rule -> commands:
#
#   Rule              | Install command               | Test command
#   ------------------|-------------------------------|----------------------------
#   py_library        | uv pip install --system -e .  | python -m pytest ...
#   py_binary         | uv pip install --system -e .  | python -m pytest ...
#   go_library        | go build ./...                | go test + go vet
#   go_binary         | go build ./...                | go test + go vet
#   ruby_library      | bundle install --quiet        | bundle exec rake test
#   ruby_binary       | bundle install --quiet        | bundle exec rake test
#   ts_library        | npm install --silent          | npx vitest run --coverage
#   ts_binary         | npm install --silent          | npx vitest run --coverage
#   rust_library      | cargo build                   | cargo test
#   rust_binary       | cargo build                   | cargo test
#   elixir_library    | mix deps.get                  | mix test --cover
#   elixir_binary     | mix deps.get                  | mix test --cover


def generate_commands(target: Target) -> list[str]:
    """Convert a Target into shell commands for the build executor.

    Maps rule types to standard install + test command sequences.
    The test runner can be overridden via ``target.test_runner``.

    Parameters
    ----------
    target : Target
        The target to generate commands for.

    Returns
    -------
    list[str]
        Shell commands to execute, in order.

    Examples
    --------
    >>> t = Target(rule="py_library", name="foo")
    >>> generate_commands(t)
    ['uv pip install --system -e ".[dev]"', 'python -m pytest --cov --cov-report=term-missing']

    >>> t = Target(rule="go_library", name="bar")
    >>> generate_commands(t)
    ['go build ./...', 'go test ./... -v -cover', 'go vet ./...']
    """
    rule = target.rule

    # --- Python rules ---
    # Python packages use uv for fast installs and pytest for testing.
    # The test runner defaults to pytest but can be overridden to unittest.
    if rule in ("py_library", "py_binary"):
        runner = target.test_runner or "pytest"
        if runner == "pytest":
            return [
                'uv pip install --system -e ".[dev]"',
                "python -m pytest --cov --cov-report=term-missing",
            ]
        return [
            'uv pip install --system -e ".[dev]"',
            "python -m unittest discover tests/",
        ]

    # --- Go rules ---
    # Go packages use the standard go toolchain. We run build, test, and
    # vet to catch compilation errors, test failures, and style issues.
    if rule in ("go_library", "go_binary"):
        return [
            "go build ./...",
            "go test ./... -v -cover",
            "go vet ./...",
        ]

    # --- Ruby rules ---
    # Ruby packages use Bundler for dependency management and Rake for
    # test execution.
    if rule in ("ruby_library", "ruby_binary"):
        return [
            "bundle install --quiet",
            "bundle exec rake test",
        ]

    # --- TypeScript rules ---
    # TypeScript packages use npm for dependencies and Vitest for testing.
    if rule in ("ts_library", "ts_binary"):
        return [
            "npm install --silent",
            "npx vitest run --coverage",
        ]

    # --- Rust rules ---
    # Rust packages use Cargo for everything.
    if rule in ("rust_library", "rust_binary"):
        return [
            "cargo build",
            "cargo test",
        ]

    # --- Elixir rules ---
    # Elixir packages use Mix for dependency management and testing.
    if rule in ("elixir_library", "elixir_binary"):
        return [
            "mix deps.get",
            "mix test --cover",
        ]

    # --- Unknown rules ---
    # If we encounter a rule type we don't recognize, emit a diagnostic
    # echo so the build log makes it clear what happened.
    return [f"echo 'Unknown rule: {rule}'"]


# =========================================================================
# Dict List Extraction
# =========================================================================


def _get_dict_list(d: dict, key: str) -> list[dict]:
    """Safely extract a list of dicts from a dict.

    Returns ``[]`` if the key is missing or the value is not a list.
    Non-dict elements in the list are silently skipped.
    """
    value = d.get(key)
    if not isinstance(value, list):
        return []
    return [item for item in value if isinstance(item, dict)]


# =========================================================================
# Command Rendering
# =========================================================================
#
# When BUILD rules use cmd() from cmd.star, they produce structured command
# dicts like:
#
#   {"type": "cmd", "program": "python", "args": ["-m", "pytest"]}
#
# render_command() turns these into shell strings:
#
#   python -m pytest

# Characters that trigger quoting in shell strings.
_SHELL_META = set(' \t"\'$`\\|&;()<>!#*?[]{}')


def _needs_quoting(arg: str) -> bool:
    """Check whether a shell argument needs quoting."""
    return any(c in _SHELL_META for c in arg)


def _quote_arg(arg: str) -> str:
    """Quote a single shell argument if it contains metacharacters."""
    if not arg:
        return '""'
    if not _needs_quoting(arg):
        return arg
    escaped = arg.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


def render_command(cmd_dict: dict) -> str:
    """Convert a single command dict to a shell string.

    Parameters
    ----------
    cmd_dict : dict
        A command dict with "program" and optional "args" keys.

    Returns
    -------
    str
        A shell-safe command string.
    """
    program = cmd_dict.get("program", "")
    if not isinstance(program, str) or not program:
        raise ValueError(f"command dict missing 'program' key: {cmd_dict}")

    parts = [_quote_arg(program)]

    args = cmd_dict.get("args")
    if args and isinstance(args, list):
        for arg in args:
            parts.append(_quote_arg(str(arg)))

    return " ".join(parts)


def render_commands(cmds: list) -> list[str]:
    """Convert a list of command dicts to shell strings, skipping None entries."""
    result = []
    for cmd in cmds:
        if cmd is None:
            continue
        if isinstance(cmd, dict):
            result.append(render_command(cmd))
    return result
