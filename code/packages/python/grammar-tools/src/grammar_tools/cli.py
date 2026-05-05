"""Command-line entry points for compiling grammar files."""

from __future__ import annotations

import sys
from collections.abc import Sequence
from importlib.resources import as_file, files
from pathlib import Path

from cli_builder import (
    HelpResult,
    ParseErrors,
    Parser,
    ParseResult,
    SpecError,
    VersionResult,
)

from grammar_tools.compiler import compile_parser_grammar, compile_token_grammar
from grammar_tools.parser_grammar import ParserGrammarError, parse_parser_grammar
from grammar_tools.token_grammar import TokenGrammarError, parse_token_grammar

_PROGRAM_NAME = "grammar-tools"
_SPEC_RESOURCE = "grammar_tools_cli.json"


def _write_generated(code: str, output: Path | None) -> None:
    if output is None:
        sys.stdout.write(code)
        return
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(code, encoding="utf-8")


def _compile_tokens(source: Path) -> str:
    grammar = parse_token_grammar(source.read_text(encoding="utf-8"))
    return compile_token_grammar(grammar, source.as_posix())


def _compile_grammar(source: Path) -> str:
    grammar = parse_parser_grammar(source.read_text(encoding="utf-8"))
    return compile_parser_grammar(grammar, source.as_posix())


def main(argv: Sequence[str] | None = None) -> int:
    """Run the ``grammar-tools`` command-line interface."""
    try:
        parsed = _parse_cli_builder_args(argv)
    except ParseErrors as exc:
        print(f"{_PROGRAM_NAME}: {exc}", file=sys.stderr)
        return 2
    except SpecError as exc:
        print(f"{_PROGRAM_NAME}: {exc}", file=sys.stderr)
        return 2

    if isinstance(parsed, HelpResult):
        print(parsed.text)
        return 0

    if isinstance(parsed, VersionResult):
        print(parsed.version)
        return 0

    return _compile_from_parse_result(parsed)


def _parse_cli_builder_args(
    argv: Sequence[str] | None,
) -> ParseResult | HelpResult | VersionResult:
    if argv is None:
        cli_argv = [Path(sys.argv[0]).name, *sys.argv[1:]]
    else:
        cli_argv = [_PROGRAM_NAME, *argv]
    spec_resource = files("grammar_tools").joinpath(_SPEC_RESOURCE)
    with as_file(spec_resource) as spec_path:
        return Parser(str(spec_path), cli_argv).parse()


def _compile_from_parse_result(parsed: ParseResult) -> int:
    command = parsed.command_path[-1]
    if command not in {"compile-tokens", "compile-grammar"}:
        print(f"{_PROGRAM_NAME}: expected a command", file=sys.stderr)
        return 2

    source = Path(str(parsed.arguments["source"]))
    output = parsed.flags["output"]
    output_path = Path(str(output)) if output is not None else None

    try:
        if command == "compile-tokens":
            code = _compile_tokens(source)
        else:
            code = _compile_grammar(source)
        _write_generated(code, output_path)
    except (OSError, ParserGrammarError, TokenGrammarError) as exc:
        print(f"{_PROGRAM_NAME}: {exc}", file=sys.stderr)
        return 1

    return 0
