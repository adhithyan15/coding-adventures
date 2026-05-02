"""Command-line runner for Prolog programs compiled onto the Logic VM."""

from __future__ import annotations

import sys
from collections.abc import Sequence
from dataclasses import dataclass
from importlib import resources
from pathlib import Path
from typing import Literal, TextIO, cast

from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult
from logic_engine import Term

from prolog_vm_compiler.compiler import (
    PrologAnswer,
    PrologVMBackend,
    query_prolog_file,
    query_prolog_file_values,
    query_prolog_project_file,
    query_prolog_project_file_values,
    query_prolog_source,
    query_prolog_source_values,
    run_prolog_file_query,
    run_prolog_file_query_answers,
    run_prolog_project_file_query,
    run_prolog_project_file_query_answers,
    run_prolog_source_query,
    run_prolog_source_query_answers,
)

type CliDialect = Literal["swi", "iso"]

_PROGRAM_NAME = "prolog-vm"
_SPEC_RESOURCE = "prolog_vm_cli.json"


@dataclass(frozen=True, slots=True)
class CliArgs:
    """Validated CLI options after CLI Builder has parsed argv."""

    files: tuple[Path, ...]
    source: str | None
    query: str | None
    source_query_index: int
    query_module: str | None
    limit: int | None
    dialect: CliDialect
    backend: PrologVMBackend
    values: bool
    initialize: bool


def main(argv: Sequence[str] | None = None) -> int:
    """Run the Prolog VM command-line interface."""

    try:
        parsed = _parse_argv(argv)
    except ParseErrors as error:
        print(error, file=sys.stderr)
        return 2

    if isinstance(parsed, HelpResult):
        print(parsed.text)
        return 0
    if isinstance(parsed, VersionResult):
        print(parsed.version)
        return 0

    try:
        args = _cli_args_from_result(parsed)
    except ValueError as error:
        print(f"{_PROGRAM_NAME}: {error}", file=sys.stderr)
        return 2

    try:
        results = _run_cli(args)
    except Exception as error:  # noqa: BLE001 - CLI should render failures plainly.
        print(f"{_PROGRAM_NAME}: {error}", file=sys.stderr)
        return 1

    _print_results(results, values=args.values)
    return 0 if results else 1


def _parse_argv(
    argv: Sequence[str] | None,
) -> ParseResult | HelpResult | VersionResult:
    actual_argv = list(sys.argv if argv is None else (_PROGRAM_NAME, *argv))
    spec_path = resources.files("prolog_vm_compiler").joinpath(_SPEC_RESOURCE)
    return Parser(str(spec_path), actual_argv).parse()


def _cli_args_from_result(result: ParseResult) -> CliArgs:
    flags = result.flags
    files = tuple(
        Path(value)
        for value in cast("list[Path | str]", result.arguments.get("files", []))
    )
    source = _optional_string(flags["source"])
    query = _optional_string(flags["query"])
    query_module = _optional_string(flags["query-module"])
    limit = _optional_int(flags["limit"])
    source_query_index = _required_int(
        flags["source-query-index"],
        name="--source-query-index",
    )

    if limit is not None and limit < 0:
        msg = "--limit must be non-negative"
        raise ValueError(msg)
    if source is not None and files:
        msg = "--source cannot be combined with file paths"
        raise ValueError(msg)
    if source is None and not files:
        msg = "provide --source or at least one Prolog file"
        raise ValueError(msg)

    return CliArgs(
        files=files,
        source=source,
        query=query,
        source_query_index=source_query_index,
        query_module=query_module,
        limit=limit,
        dialect=_dialect(_required_string(flags["dialect"], name="--dialect")),
        backend=_backend(_required_string(flags["backend"], name="--backend")),
        values=bool(flags["values"]),
        initialize=not bool(flags["no-initialize"]),
    )


def _optional_string(value: object) -> str | None:
    if value is None:
        return None
    if isinstance(value, str):
        return value
    msg = f"expected string flag value, got {type(value).__name__}"
    raise ValueError(msg)


def _required_string(value: object, *, name: str) -> str:
    parsed = _optional_string(value)
    if parsed is None:
        msg = f"{name} is required"
        raise ValueError(msg)
    return parsed


def _optional_int(value: object) -> int | None:
    if value is None:
        return None
    if isinstance(value, int):
        return value
    msg = f"expected integer flag value, got {type(value).__name__}"
    raise ValueError(msg)


def _required_int(value: object, *, name: str) -> int:
    parsed = _optional_int(value)
    if parsed is None:
        msg = f"{name} is required"
        raise ValueError(msg)
    return parsed


def _run_cli(
    args: CliArgs,
) -> list[PrologAnswer] | list[Term | tuple[Term, ...]]:
    if args.query is not None:
        return _run_ad_hoc_query(args)
    return _run_source_query(args)


def _run_ad_hoc_query(
    args: CliArgs,
) -> list[PrologAnswer] | list[Term | tuple[Term, ...]]:
    query = args.query
    if query is None:
        msg = "ad-hoc query runner requires --query"
        raise ValueError(msg)

    if args.source is not None:
        if args.values:
            return query_prolog_source_values(
                args.source,
                query,
                limit=args.limit,
                dialect=args.dialect,
                initialize=args.initialize,
                backend=args.backend,
            )
        return query_prolog_source(
            args.source,
            query,
            limit=args.limit,
            dialect=args.dialect,
            initialize=args.initialize,
            backend=args.backend,
        )

    if len(args.files) == 1:
        if args.values:
            return query_prolog_file_values(
                args.files[0],
                query,
                limit=args.limit,
                dialect=args.dialect,
                initialize=args.initialize,
                backend=args.backend,
            )
        return query_prolog_file(
            args.files[0],
            query,
            limit=args.limit,
            dialect=args.dialect,
            initialize=args.initialize,
            backend=args.backend,
        )

    if args.values:
        return query_prolog_project_file_values(
            *args.files,
            query_source=query,
            limit=args.limit,
            dialect=args.dialect,
            query_module=args.query_module,
            initialize=args.initialize,
            backend=args.backend,
        )
    return query_prolog_project_file(
        *args.files,
        query_source=query,
        limit=args.limit,
        dialect=args.dialect,
        query_module=args.query_module,
        initialize=args.initialize,
        backend=args.backend,
    )


def _run_source_query(
    args: CliArgs,
) -> list[PrologAnswer] | list[Term | tuple[Term, ...]]:
    if args.source is not None:
        if args.values:
            return run_prolog_source_query(
                args.source,
                source_query_index=args.source_query_index,
                limit=args.limit,
                dialect=args.dialect,
                initialize=args.initialize,
                backend=args.backend,
            )
        return run_prolog_source_query_answers(
            args.source,
            source_query_index=args.source_query_index,
            limit=args.limit,
            dialect=args.dialect,
            initialize=args.initialize,
            backend=args.backend,
        )

    if len(args.files) == 1:
        if args.values:
            return run_prolog_file_query(
                args.files[0],
                source_query_index=args.source_query_index,
                limit=args.limit,
                dialect=args.dialect,
                initialize=args.initialize,
                backend=args.backend,
            )
        return run_prolog_file_query_answers(
            args.files[0],
            source_query_index=args.source_query_index,
            limit=args.limit,
            dialect=args.dialect,
            initialize=args.initialize,
            backend=args.backend,
        )

    if args.values:
        return run_prolog_project_file_query(
            *args.files,
            source_query_index=args.source_query_index,
            limit=args.limit,
            dialect=args.dialect,
            initialize=args.initialize,
            backend=args.backend,
        )
    return run_prolog_project_file_query_answers(
        *args.files,
        source_query_index=args.source_query_index,
        limit=args.limit,
        dialect=args.dialect,
        initialize=args.initialize,
        backend=args.backend,
    )


def _dialect(value: str) -> CliDialect:
    if value == "swi":
        return "swi"
    if value == "iso":
        return "iso"
    msg = f"unsupported dialect {value!r}"
    raise ValueError(msg)


def _backend(value: str) -> PrologVMBackend:
    if value == "structured":
        return "structured"
    if value == "bytecode":
        return "bytecode"
    msg = f"unsupported backend {value!r}"
    raise ValueError(msg)


def _print_results(
    results: list[PrologAnswer] | list[Term | tuple[Term, ...]],
    *,
    values: bool,
    stdout: TextIO | None = None,
) -> None:
    output = sys.stdout if stdout is None else stdout
    if not results:
        print("false.", file=output)
        return
    for result in results:
        if values:
            print(f"{_format_value(result)}.", file=output)
        else:
            print(f"{_format_answer(result)}.", file=output)


def _format_answer(answer: object) -> str:
    if not isinstance(answer, PrologAnswer):
        return _format_value(answer)
    bindings = answer.as_dict()
    if not bindings:
        return "true"
    return ", ".join(
        f"{name} = {_format_value(value)}"
        for name, value in sorted(bindings.items())
    )


def _format_value(value: object) -> str:
    if isinstance(value, tuple):
        if not value:
            return "true"
        return "(" + ", ".join(_format_value(item) for item in value) + ")"
    return str(value)


if __name__ == "__main__":
    raise SystemExit(main())
