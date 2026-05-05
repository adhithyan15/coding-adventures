# prolog-vm-compiler

`prolog-vm-compiler` lowers loaded Prolog artifacts into standardized
`logic-instructions` programs that can run through `logic-vm` or be lowered
again into compact `logic-bytecode` for `logic-bytecode-vm`.

It is the bridge between the Prolog frontend stack and the Logic VM:

- parse and load source with `prolog-loader`
- route ISO/Core and SWI-compatible source through shared dialect profiles via
  `compile_prolog_source(...)` and `create_prolog_source_vm_runtime(...)`
- collect relation declarations, dynamic declarations, clauses, and queries
- compile ground Prolog facts to `FACT`
- compile Prolog rules and variable-bearing facts to `RULE`
- preserve source queries as VM `QUERY` instructions
- optionally adapt supported Prolog builtins into executable logic builtin goals
- run source queries with named answer bindings
- run initialization query slots before later source queries when callers want
  stateful dynamic startup behavior
- execute the same compiled Prolog program through the bytecode VM path with
  `compile_prolog_to_bytecode(...)` and
  `run_compiled_prolog_bytecode_query(...)`
- select either the structured VM or bytecode VM through shared APIs with
  `backend="structured"` or `backend="bytecode"`
- run source text directly through one-shot helpers such as
  `run_prolog_source_query(...)` when callers do not need to keep the compiled
  program around
- run `.pl` files and linked projects directly through one-shot helpers such as
  `run_swi_prolog_file_query_answers(...)` and
  `run_swi_prolog_project_file_query_answers(...)`
- answer ad-hoc top-level query strings directly through helpers such as
  `query_swi_prolog_source_values(...)` and
  `query_swi_prolog_project_file(...)`

## Quick Start

```python
from logic_engine import atom
from prolog_vm_compiler import run_swi_prolog_source_query

answers = run_swi_prolog_source_query(
    """
    parent(homer, bart).
    parent(homer, lisa).
    ancestor(X, Y) :- parent(X, Y).
    ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).

    ?- ancestor(homer, Who).
    """,
)

assert answers == [atom("bart"), atom("lisa")]
```

Use the generic dialect-routed entry point when the caller should choose the
frontend policy explicitly:

```python
from logic_engine import atom
from prolog_vm_compiler import run_prolog_source_query

answers = run_prolog_source_query(
    """
    parent(homer, bart).
    ?- parent(homer, Who).
    """,
    dialect="iso",
)

assert answers == [atom("bart")]
```

When you want to reuse a compiled program, keep using
`compile_swi_prolog_source(...)` plus `run_compiled_prolog_query(...)`.

When the query should come from the Python caller rather than a source-level
`?-` directive, use the top-level query helpers:

```python
from logic_engine import atom
from prolog_vm_compiler import query_swi_prolog_source_values

answers = query_swi_prolog_source_values(
    """
    parent(homer, bart).
    parent(homer, lisa).
    """,
    "parent(homer, Who)",
    backend="bytecode",
)

assert answers == [atom("bart"), atom("lisa")]
```

The one-shot helpers also cover files and linked projects:

```python
from logic_engine import atom
from prolog_vm_compiler import run_swi_prolog_file_query_answers

answers = run_swi_prolog_file_query_answers("family.pl", backend="bytecode")

assert [answer.as_dict() for answer in answers] == [
    {"Who": atom("bart")},
    {"Who": atom("lisa")},
]
```

Use `run_swi_prolog_project_query(...)` for linked source strings and
`run_swi_prolog_project_file_query(...)` for linked file graphs. All one-shot
runner variants accept `backend="structured"` or `backend="bytecode"` and
`initialize=True` when initialization directives should run before the selected
source query.

For ad-hoc queries over linked modules, use `query_swi_prolog_project(...)` or
`query_swi_prolog_project_file(...)` with `query_module=...` so imports are
resolved the same way stateful runtimes resolve later top-level queries.

## Bytecode VM Path

Use the `backend` selector when you want the Prolog frontend stack to converge
on the lower opcode runtime instead of stopping at `logic-instructions`:

```python
from logic_engine import atom
from prolog_vm_compiler import run_swi_prolog_source_query

answers = run_swi_prolog_source_query(
    """
    parent(homer, bart).
    parent(homer, lisa).

    ?- parent(homer, Who).
    """,
    backend="bytecode",
)

assert answers == [atom("bart"), atom("lisa")]
```

Stateful bytecode runtimes mirror the structured VM runtime helpers:

```python
from logic_engine import atom
from prolog_vm_compiler import create_swi_prolog_vm_runtime

runtime = create_swi_prolog_vm_runtime(
    """
    :- dynamic(memo/1).
    parent(homer, bart).
    """,
    backend="bytecode",
)

runtime.query("assertz(memo(saved))", commit=True)

assert runtime.query_values("parent(homer, Who)") == [atom("bart")]
assert runtime.query_values("memo(Value)") == [atom("saved")]
```

## Named Answers And Initialization

```python
from logic_engine import atom
from prolog_vm_compiler import (
    compile_swi_prolog_source,
    run_initialized_compiled_prolog_query_answers,
)

compiled = compile_swi_prolog_source(
    """
    :- initialization(dynamic(seen/1)).
    :- initialization(assertz(seen(alpha))).

    ?- seen(Name).
    """,
)

answers = run_initialized_compiled_prolog_query_answers(compiled)

assert [answer.as_dict() for answer in answers] == [{"Name": atom("alpha")}]
```

Named `PrologAnswer` values also expose `residual_constraints` so delayed goals
such as `dif(X, tea)` stay visible when the answer still contains unresolved
logic variables.

## Stateful Query Runtime

Use `create_swi_prolog_vm_runtime(...)` when you want to consult a program once
and ask many top-level queries later:

```python
from logic_engine import atom
from prolog_vm_compiler import create_swi_prolog_vm_runtime

runtime = create_swi_prolog_vm_runtime(
    """
    :- dynamic(memo/1).
    parent(homer, bart).
    parent(homer, lisa).
    """,
)

assert [answer.as_dict() for answer in runtime.query("parent(homer, Who)")] == [
    {"Who": atom("bart")},
    {"Who": atom("lisa")},
]

runtime.query("assertz(memo(saved))", commit=True)

assert [answer.as_dict() for answer in runtime.query("memo(Value)")] == [
    {"Value": atom("saved")},
]
```

## File Runtime

Use `create_swi_prolog_file_runtime(...)` to load a `.pl` file once and query it
through the same stateful VM runtime:

```python
from logic_engine import atom
from prolog_vm_compiler import create_swi_prolog_file_runtime

runtime = create_swi_prolog_file_runtime("app.pl")

assert [answer.as_dict() for answer in runtime.query("ancestor(homer, Who)")] == [
    {"Who": atom("bart")},
    {"Who": atom("lisa")},
]
```

For linked file graphs, use `create_swi_prolog_project_file_runtime(...)`.
Pass `query_module=...` when later ad-hoc queries should resolve through a
module's imports:

```python
from logic_engine import atom
from prolog_vm_compiler import create_swi_prolog_project_file_runtime

runtime = create_swi_prolog_project_file_runtime("app.pl", query_module="app")

assert [answer.as_dict() for answer in runtime.query("ancestor(homer, Who)")] == [
    {"Who": atom("bart")},
    {"Who": atom("lisa")},
]
```

## Capability Manifest

The core Prolog-on-Logic-VM implementation track covers PR00 through PR77. The
package exposes that as a machine-readable manifest so downstream tools and
future implementation work can distinguish completed core functionality from
deliberately deferred advanced dialect emulation:

```python
from prolog_vm_compiler import prolog_vm_capability_manifest

manifest = prolog_vm_capability_manifest()

assert manifest.status == "core-complete"
assert manifest.dialects == ("iso", "swi")
assert manifest.backends == ("structured", "bytecode")
```

The completed core batches cover frontend loading, directives, modules, file
graphs, expansion, structured and bytecode VM execution, source/file/project
runners, top-level query APIs, the CLI, CLP(FD), dynamic database behavior,
exceptions, control, collections, term/text predicates, and reflection. The
manifest also names the remaining advanced-dialect work that is intentionally
outside the PR00-PR77 core: full external dialect emulation, tabling and
well-founded negation, generalized attributed-variable/coroutining services,
non-FD constraint domains, rich streams/I/O, foreign predicates, engines, and
concurrency.

## CLI

The package also exposes a `prolog-vm` command backed by the repo's declarative
`cli-builder` package. It can run inline source, a single `.pl` file, or a
linked project file graph through either VM backend:

```bash
prolog-vm \
  --source "parent(homer, bart). parent(homer, lisa)." \
  --query "parent(homer, Who)" \
  --backend bytecode
```

Use `--dump-capabilities` without source input to inspect the same support
manifest from scripts or CI:

```bash
prolog-vm --dump-capabilities --format json
```

For linked module projects, pass all entry files and the module context for
ad-hoc or interactive top-level queries:

```bash
prolog-vm app.pl family.pl \
  --query "ancestor(homer, Who)" \
  --query-module app \
  --backend bytecode
```

`--query-module` only applies to project file graphs with `--query` or
`--interactive`; it is rejected for inline source, single files, source-query
execution, and compile-only diagnostics because those paths do not consume a
module context.

Use `--values` to print raw answer values, omit `--query` to run a source-level
`?-` directive by index, and use `--dialect iso` when the ISO parser profile is
the desired frontend. `--source-query-index` only applies to that single stored
source-query mode and is rejected when another mode would ignore it.

Use `--source-stdin` when editor integrations or shell pipelines should provide
the source through stdin while query selection still comes from flags:

```bash
cat family.pl | prolog-vm --source-stdin --query "parent(homer, Who)"
```

Use `--check` to parse, load, compile, and initialize source without running a
query. This is intended for CI and editor integrations that need to validate a
Prolog file graph even when it does not contain embedded `?-` directives.

Use `--dump-instructions` to compile source into the structured Logic VM
instruction stream without executing queries. This is the best first diagnostic
when checking what the Prolog loader emitted before bytecode lowering:

```bash
prolog-vm \
  --source "parent(homer, bart). ?- parent(homer, Who)." \
  --dump-instructions
```

Use `--dump-bytecode` to compile source into Logic bytecode and print the
loader-bytecode disassembly without executing queries. This is useful when
diagnosing convergence between the structured VM and the bytecode VM:

```bash
prolog-vm --source "parent(homer, bart). ?- parent(homer, Who)." --dump-bytecode
```

Use `--dump-source-metadata` to compile source without executing it and emit a
small manifest: dialect, initialization/source query counts, total VM queries,
instruction count, and visible variables plus VM query indexes for each embedded
source query. This is intended for editor and CI integrations that need a stable
source manifest before selecting a query to run.

Use `--list-source-queries` to inspect embedded `?-` directives before choosing
what to run. Text output lists the zero-based source query index, VM query
index, and visible variables; JSON output emits the same metadata as a stable
`source_queries` record.

Use `--all-source-queries` when a source file contains several embedded `?-`
queries and the CLI should run them as a script. The command prints or emits
one result record per source query, sets `source_query_index` in JSON formats,
and exits nonzero if any embedded query has no answers.

Use `--limit` on query-running modes to cap the number of answers printed per
query. It is rejected for compile-only diagnostic modes because those modes do
not enumerate answers.

Use `--values` on query-running modes to print raw answer values instead of
named bindings. It is rejected for compile-only diagnostic modes because those
modes do not emit answer records.

Use `--no-initialize` with `--check` or query-running modes when initialization
directives should not run. It is rejected for dump and source-query listing
diagnostics because those modes never run initialization directives.

Use `--commit` only with one or more ad-hoc `--query` flags. It persists the
first answer state from those query flags into later query flags or an
interactive loop, but it is rejected when no ad-hoc query is supplied.

Use `--summary` on non-interactive query runs when scripts, CI jobs, or editor
integrations need compact totals. Text output appends one summary line, JSONL
appends a summary record, and JSON wraps result records with summary metadata.
Because it summarizes query execution, `--summary` is rejected for compile-only
diagnostic modes such as `--check`, `--dump-instructions`, `--dump-bytecode`,
`--dump-source-metadata`, and `--list-source-queries`.

Use `--format json` for a machine-readable result object, or `--format jsonl`
when repeated or interactive queries should stream one result record per line.
Interactive mode rejects `--format json` because it cannot emit a single
complete JSON document while reading an open-ended query stream. JSON answers
preserve named bindings, raw values, compound terms, variables, and residual
constraints without requiring callers to scrape the human text format:

```bash
prolog-vm \
  --source "parent(homer, bart)." \
  --query "parent(homer, Who)" \
  --format json
```

The same JSON formats also structure CLI diagnostics on stderr. Parse,
validation, and runtime failures emit a `success: false` object with an error
`type`, a human `message`, and parser detail records when argument parsing
fails.

Repeated `--query` flags run in one stateful runtime. Add `--commit` when the
first answer state from each query should persist into the next query, which is
useful for dynamic database updates:

```bash
prolog-vm \
  --source ":- dynamic(memo/1)." \
  --query "assertz(memo(saved))" \
  --query "memo(Value)" \
  --commit
```

Use `--interactive` to consult source once and then stream top-level queries
from stdin until `halt.`, `:q`, `:quit`, or EOF:

```bash
prolog-vm family.pl --interactive --backend bytecode
```

## Stress Coverage

The package includes end-to-end stress tests for:

- recursive graph/path search
- linked modules and imported predicates
- DCG expansion with `phrase/3`
- arithmetic evaluation and comparison
- structured runtime errors for source-level arithmetic instantiation, type,
  and zero-divisor failures
- exception control with `throw/1`, `catch/3`, and catchable runtime errors
- term metaprogramming with `term_variables/2`, `=@=/2`, `\=@=/2`, and
  `subsumes_term/2`
- text conversion with `atom_chars/2`, `atom_codes/2`, `number_chars/2`,
  `number_codes/2`, `atom_number/2`, `char_code/2`, `string_chars/2`, and
  `string_codes/2`
- atom composition with `atom_concat/3`, `atomic_list_concat/2`,
  `atomic_list_concat/3`, and `number_string/2`
- text inspection with `atom_length/2`, `string_length/2`, `sub_atom/5`, and
  `sub_string/5`
- term text I/O with `term_to_atom/2`, `atom_to_term/3`,
  `read_term_from_atom/3`, and `write_term_to_atom/3`
- numbered term variables with `numbervars/3` and `write_term_to_atom/3`
  `numbervars(true)` rendering
- compound reflection with `compound_name_arguments/3` and
  `compound_name_arity/3`
- term-shape checks with `acyclic_term/1` and `cyclic_term/1`
- explicit unifiability checks with `unifiable/3` and
  `unify_with_occurs_check/2`
- structural term hashing with `term_hash/2` and `term_hash/4`
- runtime flag introspection and branch-local updates with
  `current_prolog_flag/2` and `set_prolog_flag/2`
- finite integer builtins such as `integer/1`, `between/3`, and `succ/2`
- negation and control builtins such as `\+/1`, `once/1`, and `forall/2`
- cleanup control with `call_cleanup/2` and `setup_call_cleanup/3`
- callable, natural infix, and nested additive CLP(FD) forms for finite-domain
  puzzles
- CLP(FD) labeling options such as descending value order
- collection predicates such as `findall/3`, `bagof/3`, and `setof/3`
- Prolog-style `bagof/3` and `setof/3` grouping by free variables, including
  `^/2` existential scopes
- common list predicates such as `member/2`, `append/3`, `length/2`,
  `sort/2`, `msort/2`, `nth0/3`, `nth1/3`, `nth0/4`, and `nth1/4`
- dynamic predicates seeded by initialization directives
- named Python answer bindings
- residual delayed `dif/2` constraints on named answers
- loader term/goal expansion before VM compilation
- bytecode VM parity for recursive search, modules, DCGs, dynamic
  initialization, control/cleanup, grouped collections, higher-order
  predicates, list stdlib predicates, term/text metadata, flags, and CLP(FD)
  modeling globals

## Layer Position

```text
Prolog dialect parser
    ↓
prolog-loader
    ↓
prolog-vm-compiler
    ↓
logic-instructions
    ├── logic-vm
    ↓
logic-bytecode
    ↓
logic-bytecode-vm
```

## Development

```bash
bash BUILD
```
