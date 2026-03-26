"""grammar-tools CLI — validate and compile .tokens and .grammar files.

This program wraps the grammar_tools library behind a user-friendly command-line
interface built with cli_builder. It is the Python counterpart of the Elixir
escript, Ruby bin, Go binary, Rust binary, and TypeScript CLI — all of which
produce identical output so CI scripts can use any implementation.

Usage
-----

    grammar-tools validate <file.tokens> <file.grammar>
    grammar-tools validate-tokens <file.tokens>
    grammar-tools validate-grammar <file.grammar>
    grammar-tools compile-tokens <file.tokens> [-o <output.py>]
    grammar-tools compile-grammar <file.grammar> [-o <output.py>]
    grammar-tools --help

Exit codes
----------

0  All checks passed / compilation succeeded.
1  One or more validation errors found / compile error.
2  Usage error (wrong number of arguments, unknown command).

Why cli_builder?
----------------

Using cli_builder gives us ``--help``, ``--version``, and consistent argument
parsing "for free". The grammar-tools commands themselves are very simple
(a positional COMMAND and 1–2 file paths), so the spec JSON is small, but
wiring in cli_builder ensures the tool behaves consistently with all other
CLI tools in this repo.

Compile commands
----------------

The compile commands convert .tokens and .grammar files into Python source
code that embeds the grammar as native data structures. This eliminates
runtime file I/O and parsing in downstream packages.

    grammar-tools compile-tokens json.tokens -o json_tokens.py
    grammar-tools compile-grammar json.grammar -o json_parser.py

Without ``-o``, the generated code is printed to stdout.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Resolve the repo root so we can find the spec JSON and packages.
#
# Walk up from this file's directory until we find code/specs/grammar-tools.json.
# This lets the program be run from any directory inside the repo.
# ---------------------------------------------------------------------------

def _find_root() -> Path:
    current = Path(__file__).resolve().parent
    for _ in range(20):
        if (current / "code" / "specs" / "grammar-tools.json").exists():
            return current
        parent = current.parent
        if parent == current:
            break
        current = parent
    return Path(__file__).resolve().parent.parent.parent.parent


ROOT = _find_root()

# Add library packages to sys.path for monorepo development.
# Install order matters: leaf → root (directed-graph before state-machine
# before cli-builder, grammar-tools has no deps).
sys.path.insert(0, str(ROOT / "code" / "packages" / "python" / "grammar-tools" / "src"))
sys.path.insert(0, str(ROOT / "code" / "packages" / "python" / "directed-graph" / "src"))
sys.path.insert(0, str(ROOT / "code" / "packages" / "python" / "state-machine" / "src"))
sys.path.insert(0, str(ROOT / "code" / "packages" / "python" / "cli-builder" / "src"))

from cli_builder import Parser, ParseResult, HelpResult, VersionResult, ParseErrors  # noqa: E402
from grammar_tools.compiler import compile_parser_grammar, compile_token_grammar  # noqa: E402
from grammar_tools.cross_validator import cross_validate  # noqa: E402
from grammar_tools.parser_grammar import (  # noqa: E402
    ParserGrammarError,
    parse_parser_grammar,
    validate_parser_grammar,
)
from grammar_tools.token_grammar import (  # noqa: E402
    TokenGrammarError,
    parse_token_grammar,
    validate_token_grammar,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _count_errors(issues: list[str]) -> int:
    """Count issues that are actual errors, not warnings.

    Issues starting with "Warning:" are informational and do not cause the
    tool to exit non-zero. Everything else counts as an error.
    """
    return sum(1 for issue in issues if not issue.startswith("Warning:"))


def _print_issues(issues: list[str], indent: str = "  ") -> None:
    """Print issues with leading indentation so they stand out."""
    for issue in issues:
        print(f"{indent}{issue}")


def _print_usage() -> None:
    """Print a short usage summary."""
    print("Usage: grammar-tools <command> [args...]")
    print()
    print("Commands:")
    print("  validate <file.tokens> <file.grammar>      Validate a token/grammar pair")
    print("  validate-tokens <file.tokens>               Validate just a .tokens file")
    print("  validate-grammar <file.grammar>             Validate just a .grammar file")
    print("  compile-tokens <file.tokens> [-o <out.py>] Compile tokens to Python")
    print("  compile-grammar <file.grammar> [-o <out.py>] Compile grammar to Python")
    print()
    print("Run 'grammar-tools --help' for full help text.")


# ---------------------------------------------------------------------------
# validate — cross-validate a .tokens/.grammar pair
# ---------------------------------------------------------------------------


def validate_command(tokens_path: str, grammar_path: str) -> int:
    """Validate a .tokens and .grammar file pair.

    Runs three checks in sequence:
    1. Parse and validate the .tokens file (syntax, duplicates, bad regexes).
    2. Parse and validate the .grammar file (undefined references, duplicates).
    3. Cross-validate the two for consistency (missing/extra token definitions).

    Returns 0 on success, 1 if any errors are found.
    """
    total_issues = 0

    # Step 1 — .tokens file
    tokens_file = Path(tokens_path)
    if not tokens_file.exists():
        print(f"Error: File not found: {tokens_path}", file=sys.stderr)
        return 1

    print(f"Validating {tokens_file.name} ...", end=" ")
    try:
        token_grammar = parse_token_grammar(tokens_file.read_text())
    except TokenGrammarError as e:
        print("PARSE ERROR")
        print(f"  {e}")
        return 1

    token_issues = validate_token_grammar(token_grammar)
    n_tokens = len(token_grammar.definitions)
    n_skip = len(token_grammar.skip_definitions)
    n_error = len(token_grammar.error_definitions)
    token_errors = _count_errors(token_issues)

    if token_errors:
        print(f"{token_errors} error(s)")
        _print_issues(token_issues)
        total_issues += token_errors
    else:
        parts = [f"{n_tokens} tokens"]
        if n_skip:
            parts.append(f"{n_skip} skip")
        if n_error:
            parts.append(f"{n_error} error")
        print(f"OK ({', '.join(parts)})")

    # Step 2 — .grammar file
    grammar_file = Path(grammar_path)
    if not grammar_file.exists():
        print(f"Error: File not found: {grammar_path}", file=sys.stderr)
        return 1

    print(f"Validating {grammar_file.name} ...", end=" ")
    try:
        parser_grammar = parse_parser_grammar(grammar_file.read_text())
    except ParserGrammarError as e:
        print("PARSE ERROR")
        print(f"  {e}")
        return 1

    parser_issues = validate_parser_grammar(
        parser_grammar,
        token_names=token_grammar.token_names(),
    )
    n_rules = len(parser_grammar.rules)
    parser_errors = _count_errors(parser_issues)

    if parser_errors:
        print(f"{parser_errors} error(s)")
        _print_issues(parser_issues)
        total_issues += parser_errors
    else:
        print(f"OK ({n_rules} rules)")

    # Step 3 — cross-validation
    print("Cross-validating ...", end=" ")
    cross_issues = cross_validate(token_grammar, parser_grammar)
    cross_errors = _count_errors(cross_issues)
    cross_warnings = len(cross_issues) - cross_errors

    if cross_errors:
        print(f"{cross_errors} error(s)")
        _print_issues(cross_issues)
        total_issues += cross_errors
    elif cross_warnings:
        print(f"OK ({cross_warnings} warning(s))")
        _print_issues(cross_issues)
    else:
        print("OK")

    print()
    if total_issues:
        print(f"Found {total_issues} error(s). Fix them and try again.")
        return 1
    print("All checks passed.")
    return 0


# ---------------------------------------------------------------------------
# validate-tokens — validate just a .tokens file
# ---------------------------------------------------------------------------


def validate_tokens_only(tokens_path: str) -> int:
    """Validate just a .tokens file (no grammar file needed).

    Returns 0 on success, 1 if any errors are found.
    """
    tokens_file = Path(tokens_path)
    if not tokens_file.exists():
        print(f"Error: File not found: {tokens_path}", file=sys.stderr)
        return 1

    print(f"Validating {tokens_file.name} ...", end=" ")
    try:
        token_grammar = parse_token_grammar(tokens_file.read_text())
    except TokenGrammarError as e:
        print("PARSE ERROR")
        print(f"  {e}")
        return 1

    issues = validate_token_grammar(token_grammar)
    n_tokens = len(token_grammar.definitions)
    n_skip = len(token_grammar.skip_definitions)
    n_error = len(token_grammar.error_definitions)
    errors = _count_errors(issues)

    if errors:
        print(f"{errors} error(s)")
        _print_issues(issues)
        print()
        print(f"Found {errors} error(s). Fix them and try again.")
        return 1

    parts = [f"{n_tokens} tokens"]
    if n_skip:
        parts.append(f"{n_skip} skip")
    if n_error:
        parts.append(f"{n_error} error")
    print(f"OK ({', '.join(parts)})")
    print()
    print("All checks passed.")
    return 0


# ---------------------------------------------------------------------------
# validate-grammar — validate just a .grammar file
# ---------------------------------------------------------------------------


def validate_grammar_only(grammar_path: str) -> int:
    """Validate just a .grammar file (no tokens file needed).

    Without a tokens file only rule-level checks run. Token reference
    checks are skipped because there is nothing to check against.

    Returns 0 on success, 1 if any errors are found.
    """
    grammar_file = Path(grammar_path)
    if not grammar_file.exists():
        print(f"Error: File not found: {grammar_path}", file=sys.stderr)
        return 1

    print(f"Validating {grammar_file.name} ...", end=" ")
    try:
        parser_grammar = parse_parser_grammar(grammar_file.read_text())
    except ParserGrammarError as e:
        print("PARSE ERROR")
        print(f"  {e}")
        return 1

    issues = validate_parser_grammar(parser_grammar)
    n_rules = len(parser_grammar.rules)
    errors = _count_errors(issues)

    if errors:
        print(f"{errors} error(s)")
        _print_issues(issues)
        print()
        print(f"Found {errors} error(s). Fix them and try again.")
        return 1

    print(f"OK ({n_rules} rules)")
    print()
    print("All checks passed.")
    return 0


# ---------------------------------------------------------------------------
# compile-tokens — compile a .tokens file to Python source code
# ---------------------------------------------------------------------------


def compile_tokens_command(tokens_path: str, output_path: str | None) -> int:
    """Parse and compile a .tokens file into Python source code.

    The generated Python file embeds the ``TokenGrammar`` as native data
    structures, eliminating runtime file I/O and parsing in downstream
    packages.

    Writes to *output_path* if given, otherwise prints to stdout.

    Returns 0 on success, 1 if the file cannot be parsed or is invalid.
    """
    tokens_file = Path(tokens_path)
    if not tokens_file.exists():
        print(f"Error: File not found: {tokens_path}", file=sys.stderr)
        return 1

    print(f"Compiling {tokens_file.name} ...", end=" ", file=sys.stderr)
    try:
        token_grammar = parse_token_grammar(tokens_file.read_text())
    except TokenGrammarError as e:
        print("PARSE ERROR", file=sys.stderr)
        print(f"  {e}", file=sys.stderr)
        return 1

    issues = validate_token_grammar(token_grammar)
    errors = _count_errors(issues)
    if errors:
        print(f"{errors} error(s)", file=sys.stderr)
        _print_issues(issues)
        return 1

    code = compile_token_grammar(token_grammar, tokens_file.name)

    if output_path:
        Path(output_path).write_text(code)
        print(f"OK → {output_path}", file=sys.stderr)
    else:
        print("OK", file=sys.stderr)
        print(code, end="")

    return 0


# ---------------------------------------------------------------------------
# compile-grammar — compile a .grammar file to Python source code
# ---------------------------------------------------------------------------


def compile_grammar_command(grammar_path: str, output_path: str | None) -> int:
    """Parse and compile a .grammar file into Python source code.

    The generated Python file embeds the ``ParserGrammar`` as native data
    structures, eliminating runtime file I/O and parsing in downstream
    packages.

    Writes to *output_path* if given, otherwise prints to stdout.

    Returns 0 on success, 1 if the file cannot be parsed or is invalid.
    """
    grammar_file = Path(grammar_path)
    if not grammar_file.exists():
        print(f"Error: File not found: {grammar_path}", file=sys.stderr)
        return 1

    print(f"Compiling {grammar_file.name} ...", end=" ", file=sys.stderr)
    try:
        parser_grammar = parse_parser_grammar(grammar_file.read_text())
    except ParserGrammarError as e:
        print("PARSE ERROR", file=sys.stderr)
        print(f"  {e}", file=sys.stderr)
        return 1

    issues = validate_parser_grammar(parser_grammar)
    errors = _count_errors(issues)
    if errors:
        print(f"{errors} error(s)", file=sys.stderr)
        _print_issues(issues)
        return 1

    code = compile_parser_grammar(parser_grammar, grammar_file.name)

    if output_path:
        Path(output_path).write_text(code)
        print(f"OK → {output_path}", file=sys.stderr)
    else:
        print("OK", file=sys.stderr)
        print(code, end="")

    return 0


# ---------------------------------------------------------------------------
# dispatch — map command → function
# ---------------------------------------------------------------------------


def dispatch(command: str, files: list[str], output: str | None = None) -> int:
    """Dispatch a parsed command to the appropriate function.

    Returns an exit code (0, 1, or 2).
    """
    if command == "validate":
        if len(files) != 2:
            print(
                "Error: 'validate' requires two arguments: <tokens> <grammar>",
                file=sys.stderr,
            )
            print(file=sys.stderr)
            _print_usage()
            return 2
        return validate_command(files[0], files[1])

    if command == "validate-tokens":
        if len(files) != 1:
            print(
                "Error: 'validate-tokens' requires one argument: <tokens>",
                file=sys.stderr,
            )
            print(file=sys.stderr)
            _print_usage()
            return 2
        return validate_tokens_only(files[0])

    if command == "validate-grammar":
        if len(files) != 1:
            print(
                "Error: 'validate-grammar' requires one argument: <grammar>",
                file=sys.stderr,
            )
            print(file=sys.stderr)
            _print_usage()
            return 2
        return validate_grammar_only(files[0])

    if command == "compile-tokens":
        if len(files) != 1:
            print(
                "Error: 'compile-tokens' requires one argument: <tokens>",
                file=sys.stderr,
            )
            print(file=sys.stderr)
            _print_usage()
            return 2
        return compile_tokens_command(files[0], output)

    if command == "compile-grammar":
        if len(files) != 1:
            print(
                "Error: 'compile-grammar' requires one argument: <grammar>",
                file=sys.stderr,
            )
            print(file=sys.stderr)
            _print_usage()
            return 2
        return compile_grammar_command(files[0], output)

    print(f"Error: Unknown command '{command}'", file=sys.stderr)
    print(file=sys.stderr)
    _print_usage()
    return 2


# ---------------------------------------------------------------------------
# main — parse argv with cli_builder, then dispatch
# ---------------------------------------------------------------------------


def main() -> int:
    """Main entry point.

    Uses cli_builder to handle --help, --version, and argument parsing.
    The COMMAND and FILES positional arguments are extracted from the
    ParseResult and passed to dispatch().
    """
    spec_path = str(ROOT / "code" / "specs" / "grammar-tools.json")

    try:
        parser = Parser(spec_path, sys.argv)
        result = parser.parse()
    except ParseErrors as e:
        print(str(e), file=sys.stderr)
        return 2
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        return 2

    if isinstance(result, HelpResult):
        print(result.text)
        return 0

    if isinstance(result, VersionResult):
        print(result.version)
        return 0

    # ParseResult — extract command, files, and optional --output flag.
    flags = result.flags
    args = result.arguments

    command = args.get("command", "")
    files_raw = args.get("files", [])
    if isinstance(files_raw, str):
        files = [files_raw]
    elif files_raw is None:
        files = []
    else:
        files = list(files_raw)

    output = flags.get("output") if flags else None

    return dispatch(command, files, output)


if __name__ == "__main__":
    sys.exit(main())
