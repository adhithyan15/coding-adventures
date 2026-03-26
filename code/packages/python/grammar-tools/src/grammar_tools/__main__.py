"""CLI entry point for grammar-tools validation.

Run with::

    python -m grammar_tools validate tokens.file grammar.file

This validates both files individually and cross-validates them against
each other. If any errors are found, the tool exits with a non-zero
status code.

Why a CLI tool?
--------------

The validation functions (``validate_token_grammar``, ``validate_parser_grammar``,
``cross_validate``) exist as library code, but when you are writing or editing
``.tokens`` and ``.grammar`` files, you need a quick way to check for typos
and consistency errors without writing Python code. This CLI tool is that
quick check.

Think of it like a compiler's ``-fsyntax-only`` flag — it parses and validates
without generating any output, and tells you exactly what is wrong.

What Gets Checked
-----------------

**Token grammar (.tokens file)**:
- Duplicate token names
- Invalid regex patterns (won't compile)
- Non-UPPER_CASE naming conventions
- Invalid aliases
- Unknown lexer modes or escape modes
- Empty patterns

**Parser grammar (.grammar file)**:
- Undefined rule references (typo in a rule name)
- Undefined token references (referencing a token that doesn't exist)
- Duplicate rule names
- Non-lowercase rule names
- Unreachable rules (defined but never referenced)

**Cross-validation (both files together)**:
- Token referenced in grammar but not defined in tokens file
- Token defined in tokens file but never used in grammar

Example Output
--------------

::

    $ python -m grammar_tools validate css.tokens css.grammar
    Validating css.tokens ... OK (39 tokens, 2 skip, 2 error)
    Validating css.grammar ... OK (36 rules)
    Cross-validating ... OK
    All checks passed.

::

    $ python -m grammar_tools validate broken.tokens broken.grammar
    Validating broken.tokens ... 2 issues
      Line 5: Duplicate token name 'IDENT' (first defined on line 3)
      Line 8: Invalid regex pattern for token 'BAD': ...
    Validating broken.grammar ... 1 issue
      Undefined rule reference: 'expresion'
    Cross-validating ... 1 issue
      Error: Grammar references token 'SEMICOL' which is not defined ...
    Found 4 issues. Fix them and try again.
"""

from __future__ import annotations

import sys
from pathlib import Path

from grammar_tools.cross_validator import cross_validate
from grammar_tools.parser_grammar import (
    ParserGrammarError,
    parse_parser_grammar,
    validate_parser_grammar,
)
from grammar_tools.token_grammar import (
    TokenGrammarError,
    parse_token_grammar,
    validate_token_grammar,
)
from grammar_tools.compiler import compile_tokens_to_python, compile_parser_to_python


def _count_errors(issues: list[str]) -> int:
    """Count how many issues are actual errors (not warnings).

    Issues starting with "Warning:" are informational and do not cause
    the tool to fail. Everything else (errors, undefined references, etc.)
    counts as a real error.
    """
    return sum(1 for issue in issues if not issue.startswith("Warning:"))


def _print_issues(issues: list[str], indent: str = "  ") -> None:
    """Print a list of issues with indentation."""
    for issue in issues:
        print(f"{indent}{issue}")


def validate_command(tokens_path: str, grammar_path: str) -> int:
    """Validate a .tokens and .grammar file pair.

    This is the core of the ``validate`` subcommand. It:
    1. Parses the .tokens file and runs ``validate_token_grammar``
    2. Parses the .grammar file and runs ``validate_parser_grammar``
    3. Cross-validates the two with ``cross_validate``

    Args:
        tokens_path: Path to the .tokens file.
        grammar_path: Path to the .grammar file.

    Returns:
        0 if all checks pass, 1 if any issues are found.
    """
    total_issues = 0

    # --- Parse and validate the .tokens file ---
    tokens_file = Path(tokens_path)
    if not tokens_file.exists():
        print(f"Error: File not found: {tokens_path}")
        return 1

    print(f"Validating {tokens_file.name} ...", end=" ")
    try:
        token_grammar = parse_token_grammar(tokens_file.read_text())
    except TokenGrammarError as e:
        print(f"PARSE ERROR")
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

    # --- Parse and validate the .grammar file ---
    grammar_file = Path(grammar_path)
    if not grammar_file.exists():
        print(f"Error: File not found: {grammar_path}")
        return 1

    print(f"Validating {grammar_file.name} ...", end=" ")
    try:
        parser_grammar = parse_parser_grammar(grammar_file.read_text())
    except ParserGrammarError as e:
        print(f"PARSE ERROR")
        print(f"  {e}")
        return 1

    # Pass token names so undefined token references are caught
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

    # --- Cross-validate ---
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

    # --- Summary ---
    if total_issues:
        print(f"\nFound {total_issues} error(s). Fix them and try again.")
        return 1
    else:
        print("\nAll checks passed.")
        return 0


def validate_tokens_only(tokens_path: str) -> int:
    """Validate just a .tokens file (no grammar file needed).

    Args:
        tokens_path: Path to the .tokens file.

    Returns:
        0 if all checks pass, 1 if any issues are found.
    """
    tokens_file = Path(tokens_path)
    if not tokens_file.exists():
        print(f"Error: File not found: {tokens_path}")
        return 1

    print(f"Validating {tokens_file.name} ...", end=" ")
    try:
        token_grammar = parse_token_grammar(tokens_file.read_text())
    except TokenGrammarError as e:
        print(f"PARSE ERROR")
        print(f"  {e}")
        return 1

    issues = validate_token_grammar(token_grammar)
    n_tokens = len(token_grammar.definitions)
    errors = _count_errors(issues)
    if errors:
        print(f"{errors} error(s)")
        _print_issues(issues)
        print(f"\nFound {errors} error(s). Fix them and try again.")
        return 1
    else:
        print(f"OK ({n_tokens} tokens)")
        print("\nAll checks passed.")
        return 0


def validate_grammar_only(grammar_path: str) -> int:
    """Validate just a .grammar file (no tokens file needed).

    Args:
        grammar_path: Path to the .grammar file.

    Returns:
        0 if all checks pass, 1 if any issues are found.
    """
    grammar_file = Path(grammar_path)
    if not grammar_file.exists():
        print(f"Error: File not found: {grammar_path}")
        return 1

    print(f"Validating {grammar_file.name} ...", end=" ")
    try:
        parser_grammar = parse_parser_grammar(grammar_file.read_text())
    except ParserGrammarError as e:
        print(f"PARSE ERROR")
        print(f"  {e}")
        return 1

    # Without a tokens file, we can only check rule-level issues
    issues = validate_parser_grammar(parser_grammar)
    n_rules = len(parser_grammar.rules)
    errors = _count_errors(issues)
    if errors:
        print(f"{errors} error(s)")
        _print_issues(issues)
        print(f"\nFound {errors} error(s). Fix them and try again.")
        return 1
    else:
        print(f"OK ({n_rules} rules)")
        print("\nAll checks passed.")
        return 0


def print_usage() -> None:
    """Print usage information."""
    print("Usage: python -m grammar_tools <command> [args...]")
    print()
    print("Commands:")
    print("  validate <file.tokens> <file.grammar>  Validate a token/grammar pair")
    print("  validate-tokens <file.tokens>           Validate just a .tokens file")
    print("  validate-grammar <file.grammar>         Validate just a .grammar file")
    print("  compile-tokens <file.tokens> <export_name> Compile just a .tokens file to python")
    print("  compile-grammar <file.grammar> <export_name> Compile just a .grammar file to python")
    print()
    print("Examples:")
    print("  python -m grammar_tools validate css.tokens css.grammar")
    print("  python -m grammar_tools validate-tokens css.tokens")
    print("  python -m grammar_tools validate-grammar css.grammar")
    print("  python -m grammar_tools compile-tokens json.tokens JsonTokens")


def main() -> int:
    """Main entry point for the grammar-tools CLI.

    Parses command-line arguments and dispatches to the appropriate
    validation function.

    Returns:
        Exit code: 0 for success, 1 for errors, 2 for usage errors.
    """
    args = sys.argv[1:]

    if not args or args[0] in ("-h", "--help", "help"):
        print_usage()
        return 0

    command = args[0]

    if command == "validate":
        if len(args) != 3:
            print("Error: 'validate' requires two arguments: <tokens> <grammar>")
            print()
            print_usage()
            return 2
        return validate_command(args[1], args[2])

    elif command == "validate-tokens":
        if len(args) != 2:
            print("Error: 'validate-tokens' requires one argument: <tokens>")
            print()
            print_usage()
            return 2
        return validate_tokens_only(args[1])

    elif command == "validate-grammar":
        if len(args) != 2:
            print("Error: 'validate-grammar' requires one argument: <grammar>")
            print()
            print_usage()
            return 2
        return validate_grammar_only(args[1])
        
    elif command == "compile-tokens":
        if len(args) != 3:
            print("Error: 'compile-tokens' requires two arguments: <tokens> <export_name>")
            print()
            print_usage()
            return 2
        tokens_file = Path(args[1])
        if not tokens_file.exists():
            print(f"Error: File not found: {args[1]}")
            return 1
        try:
            tg = parse_token_grammar(tokens_file.read_text())
        except TokenGrammarError as e:
            print("PARSE ERROR")
            print(f"  {e}")
            return 1
        issues = validate_token_grammar(tg)
        if _count_errors(issues) > 0:
            print("Error: Cannot compile invalid grammar file.")
            _print_issues(issues)
            return 1
        print(compile_tokens_to_python(tg, args[2]), end="")
        return 0

    elif command == "compile-grammar":
        if len(args) != 3:
            print("Error: 'compile-grammar' requires two arguments: <grammar> <export_name>")
            print()
            print_usage()
            return 2
        grammar_file = Path(args[1])
        if not grammar_file.exists():
            print(f"Error: File not found: {args[1]}")
            return 1
        try:
            pg = parse_parser_grammar(grammar_file.read_text())
        except ParserGrammarError as e:
            print("PARSE ERROR")
            print(f"  {e}")
            return 1
        issues = validate_parser_grammar(pg)
        if _count_errors(issues) > 0:
            print("Error: Cannot compile invalid grammar file.")
            _print_issues(issues)
            return 1
        print(compile_parser_to_python(pg, args[2]), end="")
        return 0

    else:
        print(f"Error: Unknown command '{command}'")
        print()
        print_usage()
        return 2


if __name__ == "__main__":
    sys.exit(main())
