# PR77 - Prolog VM CLI

## Overview

Expose the Prolog-on-Logic-VM runtime as an installable command-line tool
without introducing a separate language runtime path. The CLI is a thin layer
over `prolog-vm-compiler` and uses the repo's declarative `cli-builder`
package for argument parsing.

The goal is practical usability:

- run inline Prolog source through the structured VM or bytecode VM
- run Prolog source piped through stdin
- run a single `.pl` file or linked file graph
- check that source parses, loads, compiles, and initializes without a query
- dump structured Logic VM instructions for compile-only diagnostics
- dump Logic bytecode disassembly for compile-only diagnostics
- dump source metadata for editor and CI manifest integrations
- list embedded source-level `?-` query indexes and visible variables
- run stored source-level `?-` queries by index
- run all stored source-level `?-` queries as an executable script
- run one or more ad-hoc top-level queries
- preserve state across repeated queries when explicitly requested
- emit opt-in run summaries for scripts and CI
- provide a small stdin-driven interactive loop

## Public Command

```bash
prolog-vm [OPTIONS] [FILE...]
```

Important options:

- `--source SOURCE` loads inline source instead of files.
- `--source-stdin` reads source text from stdin instead of `--source` or
  files.
- `--query QUERY` runs an ad-hoc top-level query. It may be repeated.
- `--check` parses, loads, compiles, and initializes without running a query.
- `--dump-instructions` compiles to structured Logic VM instructions and emits
  them without executing queries.
- `--dump-bytecode` compiles to Logic bytecode and emits disassembly without
  executing queries.
- `--dump-source-metadata` emits dialect, query counts, instruction count, and
  embedded source-query variable metadata without executing queries.
- `--list-source-queries` lists embedded `?-` query indexes and visible
  variables without running them.
- `--source-query-index INDEX` selects a stored source-level query when no
  ad-hoc query is provided.
- `--all-source-queries` runs every stored source-level query in order when no
  ad-hoc query is provided.
- `--query-module MODULE` resolves ad-hoc or interactive project queries
  through a module's imports.
- `--backend structured|bytecode` selects the VM backend.
- `--dialect swi|iso` selects the frontend dialect profile.
- `--limit N` caps the number of answers emitted per executed query.
- `--values` prints raw result values instead of named bindings for executed
  queries.
- `--summary` appends compact run totals for non-interactive query execution.
- `--format text|json|jsonl` selects human text, one JSON document, or
  newline-delimited JSON records.
- `--commit` persists the first answer state from each ad-hoc query into the
  next query and is only valid when at least one `--query` is supplied.
- `--interactive` starts a small top-level query loop after loading the source.
- `--no-initialize` skips initialization directives.

## Stateful Query Scripts

Repeated `--query` flags run against one stateful `PrologVMRuntime`.
Without `--commit`, each query sees the runtime state at the start of that
query. With `--commit`, the first successful answer state persists into later
queries, which makes dynamic database updates usable from scripts:

```bash
prolog-vm \
  --source ":- dynamic(memo/1)." \
  --query "assertz(memo(saved))" \
  --query "memo(Value)" \
  --commit
```

Expected output:

```text
true.
Value = saved.
```

## Pipeline Source Input

`--source-stdin` reads Prolog source text from stdin before selecting the query
mode. This lets editors, generated source pipelines, and shell scripts avoid
temporary files while keeping execution on the same compiled VM path:

```bash
cat family.pl | prolog-vm --source-stdin --query "parent(homer, Who)"
```

Because `--interactive` also consumes stdin, the two modes are intentionally
mutually exclusive.

## Machine-Readable Output

`--format text` is the default human-facing output. `--format json` emits one
JSON document for non-interactive scripts: a single result object for one query
or a list of result objects for repeated queries. `--format jsonl` emits one
JSON object per query result.

When `--check` is set, JSON formats emit one success object with `mode:
"check"`, `source_query_count`, `initialization_query_count`, `backend`, and
whether initialization ran.

When `--dump-instructions` is set, text output emits indexed structured
instruction lines. JSON formats emit one success object with `mode:
"instructions"`, `instruction_count`, and per-instruction records containing
`index`, `opcode`, and rendered `text`. This mode is compile-only and mutually
exclusive with query execution modes.

When `--dump-bytecode` is set, text output emits bytecode disassembly lines.
JSON formats emit one success object with `mode: "bytecode"`, pool counts, an
`instruction_count`, and structured disassembly `lines`. This mode is
compile-only and mutually exclusive with query execution modes.

When `--dump-source-metadata` is set, JSON formats emit one success object with
`mode: "source_metadata"`, `dialect`, `dialect_display_name`, query counts,
`instruction_count`, and a `source_queries` list containing zero-based `index`
visible `variables`, and `vm_query_index` metadata. This mode is compile-only
and mutually exclusive with query execution modes.

When `--list-source-queries` is set, JSON formats emit one success object with
`mode: "source_queries"`, `source_query_count`, and a `queries` list containing
zero-based `index`, `vm_query_index`, and visible `variables` metadata for each
embedded query.

When `--all-source-queries` is set, each embedded `?-` query produces one result
object with its `source_query_index`. The process exits with status 1 if any
source query has no answers, matching repeated ad-hoc query scripts.
`--source-query-index` only applies to single stored source-query execution and
is rejected with ad-hoc, all-source-query, interactive, and compile-only modes
rather than being ignored.

`--query-module` only applies to project file graphs in ad-hoc or interactive
top-level query modes. Inline source, single-file input, source-query modes, and
compile-only diagnostics reject it rather than silently ignoring the module
context.

When `--summary` is set for non-interactive query execution, text output
appends one compact line with query, success, failure, and answer totals. JSONL
appends a final `mode: "summary"` record. JSON wraps result records in an object
with `results`, `summary`, and aggregate `success` fields so downstream tools
can consume both details and totals without re-counting. Compile-only modes
reject query-execution modifiers such as `--limit`, `--values`, and `--summary`
rather than silently ignoring them.

Each result object includes:

- `success`: whether the query produced at least one answer
- `answer_count`: the number of emitted answers
- `query`: the ad-hoc query text when available
- `source_query_index`: the selected source query when running stored `?-`
  directives
- `answers`: binding/value records

Term records are typed so downstream tools do not need to parse rendered Prolog
text:

```json
{
  "bindings": {
    "Who": {"type": "atom", "value": "bart"}
  },
  "residual_constraints": []
}
```

Residual disequality constraints are preserved as typed `left`/`right` term
pairs on named-answer records.

Machine-readable modes also apply to diagnostics on stderr. Parse failures,
validation failures, and runtime exceptions emit a `success: false` object with
an error `type` of `parse_error`, `validation_error`, or `runtime_error` plus a
human `message`. Parse errors additionally include an `errors` list containing
the underlying `cli-builder` error type, message, suggestion, and context when
available.

## Interactive Loop

`--interactive` consults the source once and reads top-level queries from
stdin until EOF, `halt.`, `halt`, `:q`, or `:quit`.

```bash
prolog-vm family.pl --interactive --backend bytecode
```

When stdin is a TTY, the loop prints a `?- ` prompt. Non-TTY input is quiet so
the mode can be tested and scripted cleanly.

If `--query` and `--interactive` are both supplied, the repeated query script
runs first in the same runtime. With `--commit`, this gives callers a simple
setup phase before entering the top-level loop.

## Validation

The CLI test coverage should prove:

- inline source ad-hoc queries through bytecode
- stdin source ad-hoc and stored source queries
- query-free compile/check mode
- structured instruction dump mode
- bytecode disassembly dump mode
- source metadata dump mode
- source query listing without execution
- stored source-level queries from files
- all source-level queries from files and inline source
- linked project file graphs with module-context queries
- declarative help generated by `cli-builder`
- no-solution exit behavior
- JSON and JSONL machine-readable output
- JSON and JSONL machine-readable diagnostics
- opt-in query run summaries
- repeated query scripts with committed runtime state
- interactive stdin query handling
- committed setup queries before interactive mode

## Non-goals

- No full terminal UX beyond a minimal query loop.
- No readline/history dependency.
- No parser syntax changes; the CLI uses existing parser and loader packages.
- No separate execution semantics; all behavior must flow through
  `PrologVMRuntime` or the existing one-shot helpers.
