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

import sys
from pathlib import Path

# Provide access to local monorepo packages for standalone execution
monorepo_root = Path(__file__).resolve().parent.parent.parent.parent
sys.path.insert(0, str(monorepo_root / "packages" / "python" / "directed-graph" / "src"))
sys.path.insert(0, str(monorepo_root / "packages" / "python" / "state-machine" / "src"))
sys.path.insert(0, str(monorepo_root / "packages" / "python" / "cli-builder" / "src"))
sys.path.insert(0, str(monorepo_root / "packages" / "python" / "grammar-tools" / "src"))

from cli_builder import Parser, ParseResult, HelpResult, VersionResult, ParseErrors

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
    return sum(1 for issue in issues if not issue.startswith("Warning:"))


def _print_issues(issues: list[str], indent: str = "  ") -> None:
    for issue in issues:
        print(f"{indent}{issue}")


def validate_command(tokens_path: str, grammar_path: str) -> int:
    total_issues = 0

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

    if total_issues:
        print(f"\nFound {total_issues} error(s). Fix them and try again.")
        return 1
    else:
        print("\nAll checks passed.")
        return 0


def validate_tokens_only(tokens_path: str) -> int:
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


def generate_command() -> int:
    has_errors = False
    monorepo_root = Path(__file__).resolve().parent.parent.parent.parent
    grammars_dir = monorepo_root / "grammars"
    lang_dir = monorepo_root / "packages" / "python"
    
    if not grammars_dir.exists():
        print(f"Error: grammars directory not found at {grammars_dir}")
        return 1

    for file_path in grammars_dir.iterdir():
        if not file_path.is_file():
            continue
            
        ext = file_path.suffix
        if ext not in (".tokens", ".grammar"):
            continue
            
        is_tokens = ext == ".tokens"
        kind = "lexer" if is_tokens else "parser"
        gn = file_path.stem
        
        # Determine target package directory in python
        # (could be python/json-parser or python/json_parser)
        possible_dirs = [
            lang_dir / f"{gn}-{kind}",
            lang_dir / f"{gn}_{kind}"
        ]
        target_dir = next((d for d in possible_dirs if d.exists() and d.is_dir()), None)
        
        if not target_dir:
            # Package doesn't exist for this grammar in python
            continue
            
        print(f"Generating for {file_path.name} ...")
        
        pkg = f"{gn}_{kind}"
        fname_base = f"{gn}_tokens" if is_tokens else f"{gn}_grammar"
        out_path = target_dir / "src" / pkg / f"{fname_base}.py"
        
        export_name = "".join(word.title() for word in gn.replace("-", "_").split("_")) + ("Tokens" if is_tokens else "Grammar")
        
        try:
            if is_tokens:
                tg = parse_token_grammar(file_path.read_text())
                issues = validate_token_grammar(tg)
                if _count_errors(issues) > 0:
                    print(f"Error: Cannot compile invalid grammar file {file_path}")
                    _print_issues(issues)
                    has_errors = True
                    continue
                code = compile_tokens_to_python(tg, export_name)
            else:
                pg = parse_parser_grammar(file_path.read_text())
                issues = validate_parser_grammar(pg)
                if _count_errors(issues) > 0:
                    print(f"Error: Cannot compile invalid grammar file {file_path}")
                    _print_issues(issues)
                    has_errors = True
                    continue
                code = compile_parser_to_python(pg, export_name)

            # Strip the "AUTO-GENERATED FILE" header and re-add if compiler doesn't add properly,
            # or just write what's returned.
            out_path.parent.mkdir(parents=True, exist_ok=True)
            out_path.write_text(code)
            print(f"  -> Saved {out_path}")
            
        except Exception as e:
            print(f"Error compiling {file_path}: {e}")
            has_errors = True

    return 1 if has_errors else 0


def main() -> int:
    # Resolve the spec file path
    # We navigate up from programs/python/grammar-tools/main.py -> programs/python/grammar-tools -> programs/python -> programs -> code -> specs/grammar-tools.cli.json
    spec_path = Path(__file__).resolve().parent.parent.parent.parent / "specs" / "grammar-tools.cli.json"
    
    if not spec_path.exists():
        print(f"Error: Cannot find CLI spec file at {spec_path}")
        return 1
        
    try:
        result = Parser(str(spec_path), sys.argv).parse()
    except ParseErrors as e:
        for err in e.errors:
            print(f"Error: {err.message}")
            if err.suggestion:
                print(f"  Did you mean '{err.suggestion}'?")
        return 2
    except Exception as e:
        print(f"Error: {e}")
        return 1

    if isinstance(result, HelpResult):
        print(result.text, end="")
        return 0
    elif isinstance(result, VersionResult):
        print(result.version)
        return 0
        
    command = result.command_path[-1]
    args = result.arguments

    if command == "validate":
        return validate_command(args["tokens_file"], args["grammar_file"])

    elif command == "validate-tokens":
        return validate_tokens_only(args["tokens_file"])

    elif command == "validate-grammar":
        return validate_grammar_only(args["grammar_file"])
        
    elif command == "compile-tokens":
        tokens_file = Path(args["tokens_file"])
        export_name = args["export_name"]
        if not tokens_file.exists():
            print(f"Error: File not found: {tokens_file}")
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
        print(compile_tokens_to_python(tg, export_name), end="")
        return 0

    elif command == "compile-grammar":
        grammar_file = Path(args["grammar_file"])
        export_name = args["export_name"]
        if not grammar_file.exists():
            print(f"Error: File not found: {grammar_file}")
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
        print(compile_parser_to_python(pg, export_name), end="")
        return 0
        
    elif command == "generate":
        return generate_command()

    else:
        print(f"Error: Unknown command '{command}'")
        return 2


if __name__ == "__main__":
    sys.exit(main())
