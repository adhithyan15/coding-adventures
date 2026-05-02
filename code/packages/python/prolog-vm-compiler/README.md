# prolog-vm-compiler

`prolog-vm-compiler` lowers loaded Prolog artifacts into standardized
`logic-instructions` programs that can run through `logic-vm`.

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

## Quick Start

```python
from logic_engine import atom
from prolog_vm_compiler import compile_swi_prolog_source, run_compiled_prolog_query

compiled = compile_swi_prolog_source(
    """
    parent(homer, bart).
    parent(homer, lisa).
    ancestor(X, Y) :- parent(X, Y).
    ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).

    ?- ancestor(homer, Who).
    """,
)

assert run_compiled_prolog_query(compiled) == [atom("bart"), atom("lisa")]
```

Use the generic dialect-routed entry point when the caller should choose the
frontend policy explicitly:

```python
from logic_engine import atom
from prolog_vm_compiler import compile_prolog_source, run_compiled_prolog_query

compiled = compile_prolog_source(
    """
    parent(homer, bart).
    ?- parent(homer, Who).
    """,
    dialect="iso",
)

assert run_compiled_prolog_query(compiled) == [atom("bart")]
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

## Layer Position

```text
Prolog dialect parser
    ↓
prolog-loader
    ↓
prolog-vm-compiler
    ↓
logic-instructions
    ↓
logic-vm
```

## Development

```bash
bash BUILD
```
