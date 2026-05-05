# Changelog

## Unreleased

- adapt source-level `exists_file/1`, `read_file_to_string/2`, and
  `read_file_to_codes/2` into the executable logic builtin layer
- adapt source-level bounded file stream predicates including `open/3`,
  `close/1`, `read_string/3`, `read_line_to_string/2`, `get_char/2`,
  `at_end_of_stream/1`, `write/2`, and `nl/1`
- preserve source query variables across `goal_expansion/2` rewrites
- adapt Prolog `call/2..8` into variadic callable-term execution
- preserve extra arguments while rewriting module-qualified `call/N` meta-goals
- adapt Prolog term equality predicates `=/2`, `\\=/2`, `==/2`, and `\\==/2`
  into engine and builtin goals
- adapt Prolog `dif/2` into delayed disequality constraints
- rewrite module-qualified callable arguments inside `findall/3`, `bagof/3`,
  `setof/3`, and `forall/2`
- rewrite module-qualified and imported apply-family closures for `maplist/N`,
  `convlist/3`, `include/3`, `exclude/3`, `partition/4`, `foldl/N`, and
  `scanl/N`
- expose `rewrite_loaded_prolog_query(...)` for ad-hoc queries that need a
  linked project's module/import context
- adapt Prolog `->/2` and `(If -> Then ; Else)` control constructs into the
  executable logic builtin layer
- adapt Prolog `between/3` into the logic builtin layer for finite integer
  generation and validation
- adapt Prolog `integer/1` and `succ/2` into the logic builtin layer for
  integer type checks and successor relations
- adapt callable CLP(FD) forms (`in/2`, `ins/2`, `#=/2`, `#\=/2`, `#</2`,
  `#=</2`, `#>/2`, `#>=/2`, `all_different/1`, `all_distinct/1`, `label/1`,
  and `labeling/2`) into the finite-domain builtin layer
- flatten nested additive CLP(FD) equality expressions such as
  `Z #= X + Y + 1` into n-ary finite-domain sum constraints
- preserve supported `labeling/2` options (`leftmost`, `ff`, `up`, and `down`)
  when adapting Prolog CLP(FD) queries into finite-domain goals
- adapt common Prolog list predicates (`member/2`, `append/3`, `select/3`,
  `permutation/2`, `reverse/2`, `last/2`, `length/2`, `sort/2`, `msort/2`,
  `nth0/3`, `nth1/3`, `nth0/4`, `nth1/4`, and `is_list/1`) into the
  relational standard library
- adapt higher-order Prolog list predicates (`maplist/2..5`, `convlist/3`,
  `include/3`, `exclude/3`, `partition/4`, `foldl/4..7`, and `scanl/4..7`)
  into callable-term-backed logic builtins
- expand builtin adaptation for truth/failure/cut, arithmetic, collections,
  `forall/2`, and `copy_term/2`
- adapt source-level arithmetic predicates to strict Prolog runtime errors for
  instantiation, type, and zero-divisor evaluation failures
- adapt Prolog `throw/1` and `catch/3` into the executable exception-control
  builtin layer
- adapt Prolog `term_variables/2` into the term-metaprogramming builtin layer
- adapt Prolog `current_prolog_flag/2` into read-only runtime flag
  introspection
- adapt Prolog `set_prolog_flag/2` into branch-local runtime flag updates
- adapt Prolog `=@=/2`, `\\=@=/2`, and `subsumes_term/2` into non-binding
  term generality checks
- adapt Prolog text conversion predicates `atom_chars/2`, `atom_codes/2`,
  `number_chars/2`, `number_codes/2`, `char_code/2`, `string_chars/2`, and
  `string_codes/2` into finite conversion goals
- adapt Prolog atom composition predicates `atom_concat/3`,
  `atomic_list_concat/2`, `atomic_list_concat/3`, and `number_string/2` into
  finite text goals
- adapt Prolog text inspection predicates `atom_length/2`, `string_length/2`,
  `sub_atom/5`, and `sub_string/5` into finite text goals
- adapt Prolog term text I/O predicates `term_to_atom/2` and `atom_to_term/3`
  through the parser boundary
- adapt Prolog term read/write predicates `read_term_from_atom/3` and
  `write_term_to_atom/3` with finite option-list support
- adapt Prolog `numbervars/3` and render `'$VAR'(N)` placeholders through
  `write_term_to_atom/3` when `numbervars(true)` is present
- adapt Prolog `compound_name_arguments/3` and `compound_name_arity/3` into
  compound-only term reflection goals
- adapt Prolog `acyclic_term/1` and `cyclic_term/1` into term-shape goals
- adapt Prolog `unifiable/3` and `unify_with_occurs_check/2` into explicit
  finite unification goals
- adapt Prolog `term_hash/2` and `term_hash/4` into deterministic structural
  hash goals
- preserve raw collection goal scopes for `bagof/3` and `setof/3`, enabling
  source-level free-variable grouping and `^/2` existential quantification

## 0.1.0

- add `LoadedPrologSource` as a shared loader result over dialect parser outputs
- add `load_iso_prolog_source(...)` and `load_swi_prolog_source(...)`
- add explicit `run_initialization_goals(...)` execution with ordered
  `initialization/1` handling
- support optional goal adaptation so parsed initialization goals can be mapped
  into richer runtime or builtin goals before execution
- add `adapt_prolog_goal(...)` as a shared builtin adapter for parsed Prolog
  goals
- add `run_prolog_initialization_goals(...)` so loader callers can execute
  `call/1`, `dynamic/1`, `assertz/1`, `predicate_property/2`, and related
  builtins without writing custom Python adapters
- add `phrase/2` and `phrase/3` builtin adaptation for DCG-backed grammar calls
- add structured module/import metadata on loaded sources
- add multi-source project linking with namespace-aware `module/2` and
  `use_module/1,2` resolution
- add loader-time rewriting for explicit `module:goal` qualification, including
  linked queries, initialization goals, and common meta-goal wrappers
- add file-backed SWI project loading with recursive `consult/1`,
  `ensure_loaded/1`, and relative `use_module/1,2` resolution
- add `include/1` source splicing for file-backed SWI loader flows
- add pluggable `SourceResolver` hooks so callers can resolve `library(...)`
  and other custom source references during dependency loading
- add explicit `term_expansion/2` and `goal_expansion/2` load-time rewriting
- add `PrologExpansionError` for invalid or non-converging loader expansions
