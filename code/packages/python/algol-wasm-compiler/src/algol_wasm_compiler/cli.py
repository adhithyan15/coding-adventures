"""Command-line entry point for compiling ALGOL 60 sources to WebAssembly."""

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

from algol_wasm_compiler.compiler import (
    MAX_SOURCE_LENGTH,
    AlgolWasmCompiler,
    AlgolWasmError,
)

_PROGRAM_NAME = "algol60-wasm"
_SPEC_RESOURCE = "algol60_wasm_cli.json"


def main(argv: Sequence[str] | None = None) -> int:
    """Compile a source file from the command line."""

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
    spec_resource = files("algol_wasm_compiler").joinpath(_SPEC_RESOURCE)
    with as_file(spec_resource) as spec_path:
        return Parser(str(spec_path), cli_argv).parse()


def _compile_from_parse_result(parsed: ParseResult) -> int:
    source_path = Path(str(parsed.arguments["source"]))
    output_value = parsed.flags["output"]
    output_path = (
        Path(str(output_value))
        if output_value is not None
        else source_path.with_suffix(".wasm")
    )

    try:
        source = _read_source_file(source_path)
        AlgolWasmCompiler().write_wasm_file(source, output_path)
    except AlgolWasmError as exc:
        print(f"{_PROGRAM_NAME}: {exc}", file=sys.stderr)
        return 1

    if not bool(parsed.flags["quiet"]):
        print(output_path)
    return 0


def _read_source_file(source_path: Path) -> str:
    try:
        source_size = source_path.stat().st_size
    except OSError as exc:
        raise AlgolWasmError("read", str(exc), exc) from exc

    if source_size > MAX_SOURCE_LENGTH:
        raise AlgolWasmError(
            "source",
            "ALGOL source file size "
            f"{source_size} exceeds configured limit {MAX_SOURCE_LENGTH}",
        )

    try:
        return source_path.read_text(encoding="utf-8")
    except UnicodeDecodeError as exc:
        raise AlgolWasmError("read", str(exc), exc) from exc
    except OSError as exc:
        raise AlgolWasmError("read", str(exc), exc) from exc


if __name__ == "__main__":
    raise SystemExit(main())
