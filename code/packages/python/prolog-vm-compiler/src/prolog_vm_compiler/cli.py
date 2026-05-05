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
from logic_bytecode import LogicBytecodeProgram, disassemble, disassemble_text
from logic_engine import Atom, Compound, Disequality, LogicVar, Number, String, Term
from logic_instructions import (
    DynamicRelationDefInstruction,
    FactInstruction,
    InstructionProgram,
    LogicInstruction,
    QueryInstruction,
    RelationDefInstruction,
    RuleInstruction,
)

from prolog_vm_compiler.compiler import (
    CompiledPrologVMProgram,
    PrologAnswer,
    PrologVMBackend,
    PrologVMRuntime,
    compile_prolog_file,
    compile_prolog_project_from_files,
    compile_prolog_source,
    compile_prolog_to_bytecode,
    create_prolog_file_runtime,
    create_prolog_project_file_runtime,
    create_prolog_source_vm_runtime,
    run_compiled_prolog_initializations,
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
    check: bool
    dump_bytecode: bool
    dump_instructions: bool
    dump_source_metadata: bool
    list_source_queries: bool
    source_query_index: int
    all_source_queries: bool
    query_module: str | None
    limit: int | None
    dialect: CliDialect
    backend: PrologVMBackend
    values: bool
    summary: bool
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
    source_stdin = bool(flags["source-stdin"])
    output_format = _output_format(_required_string(flags["format"], name="--format"))
    source_query_index = _required_int(
        flags["source-query-index"],
        name="--source-query-index",
    )
    interactive = bool(flags["interactive"])

    if limit is not None and limit < 0:
        msg = "--limit must be non-negative"
        raise ValueError(msg)
    if source is not None and files:
        msg = "--source cannot be combined with file paths"
        raise ValueError(msg)
    if source_stdin and source is not None:
        msg = "--source-stdin cannot be combined with --source"
        raise ValueError(msg)
    if source_stdin and files:
        msg = "--source-stdin cannot be combined with file paths"
        raise ValueError(msg)
    if source_stdin and interactive:
        msg = "--source-stdin cannot be combined with --interactive"
        raise ValueError(msg)
    if source_stdin:
        source = sys.stdin.read()
    if source is None and not files:
        msg = "provide --source or at least one Prolog file"
        raise ValueError(msg)
    if query_module is not None and source is not None:
        msg = "--query-module requires a project file graph"
        raise ValueError(msg)
    if query_module is not None and len(files) < 2:
        msg = "--query-module requires a project file graph"
        raise ValueError(msg)
    all_source_queries = bool(flags["all-source-queries"])
    check = bool(flags["check"])
    dump_bytecode = bool(flags["dump-bytecode"])
    dump_instructions = bool(flags["dump-instructions"])
    dump_source_metadata = bool(flags["dump-source-metadata"])
    list_source_queries = bool(flags["list-source-queries"])
    values = bool(flags["values"])
    source_query_index_explicit = "source-query-index" in result.explicit_flags
    no_initialize_explicit = "no-initialize" in result.explicit_flags
    if check and queries:
        msg = "--check cannot be combined with --query"
        raise ValueError(msg)
    if check and limit is not None:
        msg = "--limit cannot be combined with --check"
        raise ValueError(msg)
    if check and values:
        msg = "--values cannot be combined with --check"
        raise ValueError(msg)
    if check and source_query_index_explicit:
        msg = "--source-query-index cannot be combined with --check"
        raise ValueError(msg)
    if check and all_source_queries:
        msg = "--check cannot be combined with --all-source-queries"
        raise ValueError(msg)
    if check and interactive:
        msg = "--check cannot be combined with --interactive"
        raise ValueError(msg)
    if dump_bytecode and queries:
        msg = "--dump-bytecode cannot be combined with --query"
        raise ValueError(msg)
    if dump_bytecode and check:
        msg = "--dump-bytecode cannot be combined with --check"
        raise ValueError(msg)
    if dump_bytecode and limit is not None:
        msg = "--limit cannot be combined with --dump-bytecode"
        raise ValueError(msg)
    if dump_bytecode and values:
        msg = "--values cannot be combined with --dump-bytecode"
        raise ValueError(msg)
    if dump_bytecode and source_query_index_explicit:
        msg = "--source-query-index cannot be combined with --dump-bytecode"
        raise ValueError(msg)
    if dump_bytecode and no_initialize_explicit:
        msg = "--no-initialize cannot be combined with --dump-bytecode"
        raise ValueError(msg)
    if dump_bytecode and list_source_queries:
        msg = "--dump-bytecode cannot be combined with --list-source-queries"
        raise ValueError(msg)
    if dump_bytecode and all_source_queries:
        msg = "--dump-bytecode cannot be combined with --all-source-queries"
        raise ValueError(msg)
    if dump_bytecode and interactive:
        msg = "--dump-bytecode cannot be combined with --interactive"
        raise ValueError(msg)
    if dump_instructions and queries:
        msg = "--dump-instructions cannot be combined with --query"
        raise ValueError(msg)
    if dump_instructions and check:
        msg = "--dump-instructions cannot be combined with --check"
        raise ValueError(msg)
    if dump_instructions and limit is not None:
        msg = "--limit cannot be combined with --dump-instructions"
        raise ValueError(msg)
    if dump_instructions and values:
        msg = "--values cannot be combined with --dump-instructions"
        raise ValueError(msg)
    if dump_instructions and source_query_index_explicit:
        msg = "--source-query-index cannot be combined with --dump-instructions"
        raise ValueError(msg)
    if dump_instructions and no_initialize_explicit:
        msg = "--no-initialize cannot be combined with --dump-instructions"
        raise ValueError(msg)
    if dump_instructions and dump_bytecode:
        msg = "--dump-instructions cannot be combined with --dump-bytecode"
        raise ValueError(msg)
    if dump_instructions and list_source_queries:
        msg = "--dump-instructions cannot be combined with --list-source-queries"
        raise ValueError(msg)
    if dump_instructions and all_source_queries:
        msg = "--dump-instructions cannot be combined with --all-source-queries"
        raise ValueError(msg)
    if dump_instructions and interactive:
        msg = "--dump-instructions cannot be combined with --interactive"
        raise ValueError(msg)
    if dump_source_metadata and queries:
        msg = "--dump-source-metadata cannot be combined with --query"
        raise ValueError(msg)
    if dump_source_metadata and check:
        msg = "--dump-source-metadata cannot be combined with --check"
        raise ValueError(msg)
    if dump_source_metadata and limit is not None:
        msg = "--limit cannot be combined with --dump-source-metadata"
        raise ValueError(msg)
    if dump_source_metadata and values:
        msg = "--values cannot be combined with --dump-source-metadata"
        raise ValueError(msg)
    if dump_source_metadata and source_query_index_explicit:
        msg = "--source-query-index cannot be combined with --dump-source-metadata"
        raise ValueError(msg)
    if dump_source_metadata and no_initialize_explicit:
        msg = "--no-initialize cannot be combined with --dump-source-metadata"
        raise ValueError(msg)
    if dump_source_metadata and dump_bytecode:
        msg = "--dump-source-metadata cannot be combined with --dump-bytecode"
        raise ValueError(msg)
    if dump_source_metadata and dump_instructions:
        msg = "--dump-source-metadata cannot be combined with --dump-instructions"
        raise ValueError(msg)
    if dump_source_metadata and list_source_queries:
        msg = "--dump-source-metadata cannot be combined with --list-source-queries"
        raise ValueError(msg)
    if dump_source_metadata and all_source_queries:
        msg = "--dump-source-metadata cannot be combined with --all-source-queries"
        raise ValueError(msg)
    if dump_source_metadata and interactive:
        msg = "--dump-source-metadata cannot be combined with --interactive"
        raise ValueError(msg)
    if list_source_queries and queries:
        msg = "--list-source-queries cannot be combined with --query"
        raise ValueError(msg)
    if list_source_queries and all_source_queries:
        msg = "--list-source-queries cannot be combined with --all-source-queries"
        raise ValueError(msg)
    if list_source_queries and check:
        msg = "--list-source-queries cannot be combined with --check"
        raise ValueError(msg)
    if list_source_queries and limit is not None:
        msg = "--limit cannot be combined with --list-source-queries"
        raise ValueError(msg)
    if list_source_queries and values:
        msg = "--values cannot be combined with --list-source-queries"
        raise ValueError(msg)
    if list_source_queries and source_query_index_explicit:
        msg = "--source-query-index cannot be combined with --list-source-queries"
        raise ValueError(msg)
    if list_source_queries and no_initialize_explicit:
        msg = "--no-initialize cannot be combined with --list-source-queries"
        raise ValueError(msg)
    if list_source_queries and interactive:
        msg = "--list-source-queries cannot be combined with --interactive"
        raise ValueError(msg)
    summary = bool(flags["summary"])
    if summary and check:
        msg = "--summary cannot be combined with --check"
        raise ValueError(msg)
    if summary and dump_bytecode:
        msg = "--summary cannot be combined with --dump-bytecode"
        raise ValueError(msg)
    if summary and dump_instructions:
        msg = "--summary cannot be combined with --dump-instructions"
        raise ValueError(msg)
    if summary and dump_source_metadata:
        msg = "--summary cannot be combined with --dump-source-metadata"
        raise ValueError(msg)
    if summary and list_source_queries:
        msg = "--summary cannot be combined with --list-source-queries"
        raise ValueError(msg)
    if summary and interactive:
        msg = "--summary cannot be combined with --interactive"
        raise ValueError(msg)
    if interactive and output_format == "json":
        msg = "--format json cannot be combined with --interactive"
        raise ValueError(msg)
    if all_source_queries and queries:
        msg = "--all-source-queries cannot be combined with --query"
        raise ValueError(msg)
    if all_source_queries and source_query_index_explicit:
        msg = "--source-query-index cannot be combined with --all-source-queries"
        raise ValueError(msg)
    if all_source_queries and interactive:
        msg = "--all-source-queries cannot be combined with --interactive"
        raise ValueError(msg)
    if queries and source_query_index_explicit:
        msg = "--source-query-index cannot be combined with --query"
        raise ValueError(msg)
    if interactive and source_query_index_explicit:
        msg = "--source-query-index cannot be combined with --interactive"
        raise ValueError(msg)
    if query_module is not None and not (queries or interactive):
        msg = "--query-module requires --query or --interactive"
        raise ValueError(msg)
    if bool(flags["commit"]) and not queries:
        msg = "--commit requires at least one --query"
        raise ValueError(msg)

    return CliArgs(
        files=files,
        source=source,
        queries=queries,
        check=check,
        dump_bytecode=dump_bytecode,
        dump_instructions=dump_instructions,
        dump_source_metadata=dump_source_metadata,
        list_source_queries=list_source_queries,
        source_query_index=source_query_index,
        all_source_queries=all_source_queries,
        query_module=query_module,
        limit=limit,
        dialect=_dialect(_required_string(flags["dialect"], name="--dialect")),
        backend=_backend(_required_string(flags["backend"], name="--backend")),
        values=values,
        summary=summary,
        output_format=output_format,
        commit=bool(flags["commit"]),
        interactive=interactive,
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
    if args.check:
        return _run_check(args)
    if args.dump_instructions:
        return _run_dump_instructions(args)
    if args.dump_bytecode:
        return _run_dump_bytecode(args)
    if args.dump_source_metadata:
        return _run_dump_source_metadata(args)
    if args.list_source_queries:
        return _run_list_source_queries(args)

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


def _run_check(args: CliArgs) -> int:
    compiled_program = _compile_source_program(args)
    if args.initialize:
        run_compiled_prolog_initializations(compiled_program, backend=args.backend)
    _print_check_record(compiled_program, args=args)
    return 0


def _run_dump_bytecode(args: CliArgs) -> int:
    compiled_program = _compile_source_program(args)
    bytecode = compile_prolog_to_bytecode(compiled_program)
    _print_bytecode_dump(bytecode, args=args)
    return 0


def _run_dump_instructions(args: CliArgs) -> int:
    compiled_program = _compile_source_program(args)
    _print_instruction_dump(compiled_program.instructions, args=args)
    return 0


def _run_dump_source_metadata(args: CliArgs) -> int:
    compiled_program = _compile_source_program(args)
    _print_source_metadata(compiled_program, args=args)
    return 0


def _run_list_source_queries(args: CliArgs) -> int:
    compiled_program = _compile_source_program(args)
    _print_source_query_summary(compiled_program, args=args)
    return 0


def _source_query_records(
    compiled_program: CompiledPrologVMProgram,
) -> list[dict[str, object]]:
    return [
        {
            "index": index,
            "vm_query_index": compiled_program.source_query_vm_index(index),
            "variables": list(compiled_program.source_query_variable_names(index)),
        }
        for index in range(compiled_program.source_query_count)
    ]


def _print_source_metadata(
    compiled_program: CompiledPrologVMProgram,
    *,
    args: CliArgs,
    stdout: TextIO | None = None,
) -> None:
    output = sys.stdout if stdout is None else stdout
    profile = compiled_program.dialect_profile
    dialect_name = args.dialect if profile is None else profile.name
    dialect_display_name = dialect_name if profile is None else profile.display_name
    query_summaries = _source_query_records(compiled_program)

    if args.output_format == "text":
        print(f"dialect: {dialect_display_name}", file=output)
        print(
            f"initialization queries: {compiled_program.initialization_query_count}",
            file=output,
        )
        print(f"source queries: {compiled_program.source_query_count}", file=output)
        print(f"total queries: {compiled_program.query_count}", file=output)
        print(
            f"instructions: {len(compiled_program.instructions.instructions)}",
            file=output,
        )
        for query in query_summaries:
            variables = cast("list[str]", query["variables"])
            variable_text = ", ".join(variables) if variables else "(none)"
            print(
                f"query {query['index']} "
                f"(vm query {query['vm_query_index']}) variables: {variable_text}",
                file=output,
            )
        return

    payload = {
        "success": True,
        "mode": "source_metadata",
        "dialect": dialect_name,
        "dialect_display_name": dialect_display_name,
        "instruction_count": len(compiled_program.instructions.instructions),
        "initialization_query_count": compiled_program.initialization_query_count,
        "source_query_count": compiled_program.source_query_count,
        "query_count": compiled_program.query_count,
        "source_queries": query_summaries,
    }
    print(json.dumps(payload, sort_keys=True), file=output)


def _print_source_query_summary(
    compiled_program: CompiledPrologVMProgram,
    *,
    args: CliArgs,
    stdout: TextIO | None = None,
) -> None:
    output = sys.stdout if stdout is None else stdout
    query_summaries = _source_query_records(compiled_program)

    if args.output_format == "text":
        if not query_summaries:
            print("no source queries.", file=output)
            return
        for query in query_summaries:
            variables = cast("list[str]", query["variables"])
            variable_text = ", ".join(variables) if variables else "(no variables)"
            print(
                f"query {query['index']} "
                f"(vm query {query['vm_query_index']}): {variable_text}",
                file=output,
            )
        return

    payload = {
        "success": True,
        "mode": "source_queries",
        "source_query_count": compiled_program.source_query_count,
        "queries": query_summaries,
    }
    print(json.dumps(payload, sort_keys=True), file=output)


def _print_instruction_dump(
    program: InstructionProgram,
    *,
    args: CliArgs,
    stdout: TextIO | None = None,
) -> None:
    output = sys.stdout if stdout is None else stdout
    records = [
        _instruction_dump_record(index, instruction)
        for index, instruction in enumerate(program.instructions)
    ]

    if args.output_format == "text":
        for record in records:
            print(
                f"{int(record['index']):04d}: {record['opcode']} {record['text']}",
                file=output,
            )
        return

    payload = {
        "success": True,
        "mode": "instructions",
        "instruction_count": len(program.instructions),
        "instructions": records,
    }
    print(json.dumps(payload, sort_keys=True), file=output)


def _instruction_dump_record(
    index: int,
    instruction: LogicInstruction,
) -> dict[str, object]:
    return {
        "index": index,
        "opcode": instruction.opcode.value,
        "text": _instruction_dump_text(instruction),
    }


def _instruction_dump_text(instruction: LogicInstruction) -> str:
    if isinstance(
        instruction,
        RelationDefInstruction | DynamicRelationDefInstruction,
    ):
        return str(instruction.relation)
    if isinstance(instruction, FactInstruction):
        return str(instruction.head)
    if isinstance(instruction, RuleInstruction):
        return f"{instruction.head} :- {instruction.body}"
    if isinstance(instruction, QueryInstruction):
        if instruction.outputs is None:
            return str(instruction.goal)
        outputs = ", ".join(str(output) for output in instruction.outputs)
        return f"{instruction.goal} -> {outputs}"
    msg = f"unsupported instruction type {type(instruction).__name__}"
    raise TypeError(msg)


def _print_bytecode_dump(
    bytecode: LogicBytecodeProgram,
    *,
    args: CliArgs,
    stdout: TextIO | None = None,
) -> None:
    output = sys.stdout if stdout is None else stdout
    if args.output_format == "text":
        text = disassemble_text(bytecode)
        if text:
            print(text, file=output)
        return

    lines = [
        {
            "index": line.index,
            "opcode": line.opcode,
            "operand": line.operand,
            "comment": line.comment,
        }
        for line in disassemble(bytecode)
    ]
    payload = {
        "success": True,
        "mode": "bytecode",
        "instruction_count": len(bytecode.instructions),
        "relation_pool_count": len(bytecode.relation_pool),
        "fact_pool_count": len(bytecode.fact_pool),
        "rule_pool_count": len(bytecode.rule_pool),
        "query_pool_count": len(bytecode.query_pool),
        "lines": lines,
    }
    print(json.dumps(payload, sort_keys=True), file=output)


def _print_check_record(
    compiled_program: CompiledPrologVMProgram,
    *,
    args: CliArgs,
    stdout: TextIO | None = None,
) -> None:
    output = sys.stdout if stdout is None else stdout
    if args.output_format == "text":
        print("ok.", file=output)
        return

    payload = {
        "success": True,
        "mode": "check",
        "backend": args.backend,
        "initialized": args.initialize,
        "initialization_query_count": compiled_program.initialization_query_count,
        "source_query_count": compiled_program.source_query_count,
    }
    print(json.dumps(payload, sort_keys=True), file=output)


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
        if args.summary:
            _print_summary_text(_summary_record(records), stdout=output)
        return

    json_records = [_json_record(record) for record in records]
    if args.output_format == "jsonl" or streaming:
        for record in json_records:
            print(json.dumps(record, sort_keys=True), file=output)
        if args.summary:
            print(json.dumps(_summary_record(records), sort_keys=True), file=output)
        return

    payload: object = json_records[0] if len(json_records) == 1 else json_records
    if args.summary:
        payload = {
            "results": json_records,
            "summary": _summary_record(records),
            "success": all(bool(record["success"]) for record in records),
        }
    print(json.dumps(payload, sort_keys=True), file=output)


def _summary_record(records: Sequence[dict[str, object]]) -> dict[str, object]:
    query_count = len(records)
    failed_query_count = sum(1 for record in records if not bool(record["success"]))
    answer_count = sum(
        len(
            cast(
                "list[PrologAnswer] | list[Term | tuple[Term, ...]]",
                record["results"],
            ),
        )
        for record in records
    )
    return {
        "success": failed_query_count == 0,
        "mode": "summary",
        "query_count": query_count,
        "succeeded_query_count": query_count - failed_query_count,
        "failed_query_count": failed_query_count,
        "answer_count": answer_count,
    }


def _print_summary_text(
    summary: dict[str, object],
    *,
    stdout: TextIO | None = None,
) -> None:
    output = sys.stdout if stdout is None else stdout
    print(
        "summary: "
        f"queries={summary['query_count']}, "
        f"succeeded={summary['succeeded_query_count']}, "
        f"failed={summary['failed_query_count']}, "
        f"answers={summary['answer_count']}.",
        file=output,
    )


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
