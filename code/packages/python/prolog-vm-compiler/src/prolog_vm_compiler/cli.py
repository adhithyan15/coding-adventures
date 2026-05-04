"""Command-line runner for Prolog programs compiled onto the Logic VM."""

from __future__ import annotations

import json
import sys
from collections.abc import Iterator, Sequence
from dataclasses import dataclass
from importlib import resources
from pathlib import Path
from typing import Literal, TextIO, cast

from cli_builder import (
    HelpResult,
    ParseError,
    ParseErrors,
    Parser,
    ParseResult,
    VersionResult,
)
from logic_engine import Atom, Compound, Disequality, LogicVar, Number, String, Term

from prolog_vm_compiler.compiler import (
    CompiledPrologVMProgram,
    PrologAnswer,
    PrologVMBackend,
    PrologVMRuntime,
    compile_prolog_file,
    compile_prolog_project_from_files,
    compile_prolog_source,
    create_prolog_file_runtime,
    create_prolog_project_file_runtime,
    create_prolog_source_vm_runtime,
    run_compiled_prolog_query,
    run_compiled_prolog_query_answers,
    run_initialized_compiled_prolog_query,
    run_initialized_compiled_prolog_query_answers,
    run_prolog_file_query,
    run_prolog_file_query_answers,
    run_prolog_project_file_query,
    run_prolog_project_file_query_answers,
    run_prolog_source_query,
    run_prolog_source_query_answers,
)

type CliDialect = Literal["swi", "iso"]
type CliOutputFormat = Literal["text", "json", "jsonl"]

_PROGRAM_NAME = "prolog-vm"
_SPEC_RESOURCE = "prolog_vm_cli.json"


@dataclass(frozen=True, slots=True)
class CliArgs:
    """Validated CLI options after CLI Builder has parsed argv."""

    files: tuple[Path, ...]
    source: str | None
    queries: tuple[str, ...]
    source_query_index: int
    all_source_queries: bool
    query_module: str | None
    limit: int | None
    dialect: CliDialect
    backend: PrologVMBackend
    values: bool
    output_format: CliOutputFormat
    commit: bool
    interactive: bool
    initialize: bool


def main(argv: Sequence[str] | None = None) -> int:
    """Run the Prolog VM command-line interface."""

    try:
        parsed = _parse_argv(argv)
    except ParseErrors as error:
        _print_parse_error(error, output_format=_requested_output_format(argv))
        return 2

    if isinstance(parsed, HelpResult):
        print(parsed.text)
        return 0
    if isinstance(parsed, VersionResult):
        print(parsed.version)
        return 0

    output_format = _output_format_from_result(parsed)
    try:
        args = _cli_args_from_result(parsed)
    except ValueError as error:
        _print_error(
            error_type="validation_error",
            message=str(error),
            output_format=output_format,
        )
        return 2

    try:
        status = _run_cli(args)
    except Exception as error:  # noqa: BLE001 - CLI should render failures plainly.
        _print_error(
            error_type="runtime_error",
            message=str(error),
            output_format=args.output_format,
        )
        return 1

    return status


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
    queries = _string_tuple(flags["query"])
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
    all_source_queries = bool(flags["all-source-queries"])
    if all_source_queries and queries:
        msg = "--all-source-queries cannot be combined with --query"
        raise ValueError(msg)
    if all_source_queries and bool(flags["interactive"]):
        msg = "--all-source-queries cannot be combined with --interactive"
        raise ValueError(msg)

    return CliArgs(
        files=files,
        source=source,
        queries=queries,
        source_query_index=source_query_index,
        all_source_queries=all_source_queries,
        query_module=query_module,
        limit=limit,
        dialect=_dialect(_required_string(flags["dialect"], name="--dialect")),
        backend=_backend(_required_string(flags["backend"], name="--backend")),
        values=bool(flags["values"]),
        output_format=_output_format(
            _required_string(flags["format"], name="--format"),
        ),
        commit=bool(flags["commit"]),
        interactive=bool(flags["interactive"]),
        initialize=not bool(flags["no-initialize"]),
    )


def _requested_output_format(argv: Sequence[str] | None) -> CliOutputFormat:
    raw_argv = sys.argv[1:] if argv is None else argv
    for index, token in enumerate(raw_argv):
        if token == "--format" and index + 1 < len(raw_argv):
            value = raw_argv[index + 1]
        elif token.startswith("--format="):
            value = token.partition("=")[2]
        else:
            continue
        if value in {"text", "json", "jsonl"}:
            return cast("CliOutputFormat", value)
    return "text"


def _output_format_from_result(result: ParseResult) -> CliOutputFormat:
    value = result.flags.get("format")
    if isinstance(value, str):
        return _output_format(value)
    return "text"


def _print_parse_error(
    error: ParseErrors,
    *,
    output_format: CliOutputFormat,
    stderr: TextIO | None = None,
) -> None:
    if output_format == "text":
        print(error, file=sys.stderr if stderr is None else stderr)
        return
    _print_error(
        error_type="parse_error",
        message="invalid command-line arguments",
        output_format=output_format,
        stderr=stderr,
        details={
            "errors": [
                _parse_error_detail(parse_error)
                for parse_error in error.errors
            ],
        },
    )


def _parse_error_detail(error: ParseError) -> dict[str, object]:
    detail: dict[str, object] = {
        "type": error.error_type,
        "message": error.message,
    }
    if error.suggestion is not None:
        detail["suggestion"] = error.suggestion
    if error.context:
        detail["context"] = error.context
    return detail


def _print_error(
    *,
    error_type: str,
    message: str,
    output_format: CliOutputFormat,
    stderr: TextIO | None = None,
    details: dict[str, object] | None = None,
) -> None:
    output = sys.stderr if stderr is None else stderr
    if output_format == "text":
        print(f"{_PROGRAM_NAME}: {message}", file=output)
        return

    error: dict[str, object] = {"type": error_type, "message": message}
    if details is not None:
        error.update(details)
    print(json.dumps({"success": False, "error": error}, sort_keys=True), file=output)


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


def _string_tuple(value: object) -> tuple[str, ...]:
    if value is None:
        return ()
    if isinstance(value, str):
        return (value,)
    if isinstance(value, list):
        return tuple(_required_string(item, name="--query") for item in value)
    msg = f"expected string flag value, got {type(value).__name__}"
    raise ValueError(msg)


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


def _run_cli(args: CliArgs) -> int:
    if args.queries or args.interactive:
        runtime = _create_runtime(args)
        saw_failure = False
        if args.queries:
            saw_failure = _run_ad_hoc_queries(
                args,
                runtime=runtime,
                streaming=args.interactive,
            )
        if args.interactive:
            saw_failure = _run_interactive(args, runtime=runtime) or saw_failure
        return 1 if saw_failure else 0

    if args.all_source_queries:
        return _run_all_source_queries(args)

    results = _run_source_query(args)
    _print_result_records(
        [
            _result_record(
                results,
                values=args.values,
                source_query_index=args.source_query_index,
            ),
        ],
        args=args,
    )
    return 0 if results else 1


def _run_all_source_queries(args: CliArgs) -> int:
    compiled_program = _compile_source_program(args)
    if compiled_program.source_query_count == 0:
        msg = "source contains no ?- queries"
        raise ValueError(msg)

    records: list[dict[str, object]] = []
    saw_failure = False
    for index in range(compiled_program.source_query_count):
        results = _run_compiled_source_query(args, compiled_program, index)
        records.append(
            _result_record(
                results,
                values=args.values,
                source_query_index=index,
            ),
        )
        saw_failure = saw_failure or not results

    _print_result_records(records, args=args)
    return 1 if saw_failure else 0


def _compile_source_program(args: CliArgs) -> CompiledPrologVMProgram:
    if args.source is not None:
        return compile_prolog_source(args.source, dialect=args.dialect)
    if len(args.files) == 1:
        return compile_prolog_file(args.files[0], dialect=args.dialect)
    return compile_prolog_project_from_files(*args.files, dialect=args.dialect)


def _run_compiled_source_query(
    args: CliArgs,
    compiled_program: CompiledPrologVMProgram,
    source_query_index: int,
) -> list[PrologAnswer] | list[Term | tuple[Term, ...]]:
    if args.values:
        if args.initialize:
            return run_initialized_compiled_prolog_query(
                compiled_program,
                source_query_index,
                args.limit,
                backend=args.backend,
            )
        return run_compiled_prolog_query(
            compiled_program,
            source_query_index,
            args.limit,
            backend=args.backend,
        )
    if args.initialize:
        return run_initialized_compiled_prolog_query_answers(
            compiled_program,
            source_query_index,
            args.limit,
            backend=args.backend,
        )
    return run_compiled_prolog_query_answers(
        compiled_program,
        source_query_index,
        args.limit,
        backend=args.backend,
    )


def _run_ad_hoc_queries(
    args: CliArgs,
    *,
    runtime: PrologVMRuntime,
    streaming: bool = False,
) -> bool:
    saw_failure = False
    records: list[dict[str, object]] = []
    for query in args.queries:
        results = _query_runtime(runtime, query, args=args)
        record = _result_record(results, values=args.values, query=query)
        if streaming:
            _print_result_records([record], args=args, streaming=True)
        else:
            records.append(record)
        saw_failure = saw_failure or not results
    if records:
        _print_result_records(records, args=args)
    return saw_failure


def _create_runtime(args: CliArgs) -> PrologVMRuntime:
    if args.source is not None:
        return create_prolog_source_vm_runtime(
            args.source,
            dialect=args.dialect,
            initialize=args.initialize,
            backend=args.backend,
        )
    if len(args.files) == 1:
        return create_prolog_file_runtime(
            args.files[0],
            dialect=args.dialect,
            initialize=args.initialize,
            backend=args.backend,
        )
    return create_prolog_project_file_runtime(
        *args.files,
        dialect=args.dialect,
        query_module=args.query_module,
        initialize=args.initialize,
        backend=args.backend,
    )


def _run_interactive(args: CliArgs, *, runtime: PrologVMRuntime) -> bool:
    saw_failure = False

    for query in _iter_interactive_queries(sys.stdin, stdout=sys.stdout):
        results = _query_runtime(runtime, query, args=args)
        _print_result_records(
            [_result_record(results, values=args.values, query=query)],
            args=args,
            streaming=True,
        )
        saw_failure = saw_failure or not results

    return saw_failure


def _iter_interactive_queries(
    stdin: TextIO,
    *,
    stdout: TextIO,
) -> Iterator[str]:
    while True:
        if stdin.isatty():
            print("?- ", end="", file=stdout, flush=True)
        line = stdin.readline()
        if not line:
            return
        query = line.strip()
        if not query or query.startswith("%"):
            continue
        if query in {"halt", "halt.", ":q", ":quit"}:
            return
        yield query


def _query_runtime(
    runtime: PrologVMRuntime,
    query: str,
    *,
    args: CliArgs,
) -> list[PrologAnswer] | list[Term | tuple[Term, ...]]:
    if args.values:
        return runtime.query_values(query, limit=args.limit, commit=args.commit)
    return runtime.query(query, limit=args.limit, commit=args.commit)


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


def _output_format(value: str) -> CliOutputFormat:
    if value == "text":
        return "text"
    if value == "json":
        return "json"
    if value == "jsonl":
        return "jsonl"
    msg = f"unsupported output format {value!r}"
    raise ValueError(msg)


def _print_result_records(
    records: list[dict[str, object]],
    *,
    args: CliArgs,
    streaming: bool = False,
    stdout: TextIO | None = None,
) -> None:
    output = sys.stdout if stdout is None else stdout
    if args.output_format == "text":
        for record in records:
            _print_results(
                cast(
                    "list[PrologAnswer] | list[Term | tuple[Term, ...]]",
                    record["results"],
                ),
                values=args.values,
                stdout=output,
            )
        return

    json_records = [_json_record(record) for record in records]
    if args.output_format == "jsonl" or streaming:
        for record in json_records:
            print(json.dumps(record, sort_keys=True), file=output)
        return

    payload: object = json_records[0] if len(json_records) == 1 else json_records
    print(json.dumps(payload, sort_keys=True), file=output)


def _result_record(
    results: list[PrologAnswer] | list[Term | tuple[Term, ...]],
    *,
    values: bool,
    query: str | None = None,
    source_query_index: int | None = None,
) -> dict[str, object]:
    return {
        "query": query,
        "source_query_index": source_query_index,
        "success": bool(results),
        "values": values,
        "results": results,
    }


def _json_record(record: dict[str, object]) -> dict[str, object]:
    query = record["query"]
    source_query_index = record["source_query_index"]
    results = cast(
        "list[PrologAnswer] | list[Term | tuple[Term, ...]]",
        record["results"],
    )
    values = bool(record["values"])
    payload: dict[str, object] = {
        "success": bool(record["success"]),
        "answer_count": len(results),
        "answers": [_json_answer(result, values=values) for result in results],
    }
    if query is not None:
        payload["query"] = query
    if source_query_index is not None:
        payload["source_query_index"] = source_query_index
    return payload


def _json_answer(answer: object, *, values: bool) -> dict[str, object]:
    if values:
        return {"value": _json_value(answer)}
    if isinstance(answer, PrologAnswer):
        return {
            "bindings": {
                name: _json_value(value)
                for name, value in sorted(answer.as_dict().items())
            },
            "residual_constraints": [
                _json_disequality(constraint)
                for constraint in answer.residual_constraints
            ],
        }
    return {"value": _json_value(answer)}


def _json_disequality(constraint: Disequality) -> dict[str, object]:
    return {
        "left": _json_value(constraint.left),
        "right": _json_value(constraint.right),
    }


def _json_value(value: object) -> object:
    if isinstance(value, Atom):
        return {"type": "atom", "value": str(value.symbol)}
    if isinstance(value, Number):
        return {"type": "number", "value": value.value}
    if isinstance(value, String):
        return {"type": "string", "value": value.value}
    if isinstance(value, LogicVar):
        payload: dict[str, object] = {"type": "variable", "id": value.id}
        if value.display_name is not None:
            payload["name"] = str(value.display_name)
        return payload
    if isinstance(value, Compound):
        return {
            "type": "compound",
            "functor": str(value.functor),
            "args": [_json_value(argument) for argument in value.args],
        }
    if isinstance(value, tuple):
        return {
            "type": "tuple",
            "items": [_json_value(item) for item in value],
        }
    return {"type": "host", "value": str(value)}


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
