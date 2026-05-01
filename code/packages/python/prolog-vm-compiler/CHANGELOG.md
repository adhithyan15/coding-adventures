# Changelog

## Unreleased

- Add end-to-end stress coverage for recursive search, modules, DCGs,
  arithmetic, collections, dynamic initialization, named answers, and expansion.
- Add named answer helpers for source-level query results.
- Add initialized query helpers that run compiled initialization slots before
  executing source queries.
- Add a stateful SWI Prolog VM runtime for repeated ad-hoc query strings,
  initialized dynamic state, optional query commits, and raw query values.
- Add file-backed Prolog VM compile/runtime helpers for loading `.pl` files and
  linked project file graphs through `prolog-loader`.
- Add module-aware ad-hoc query rewriting for project runtimes via
  `query_module=...`.
- Run Prolog `->/2` and `(If -> Then ; Else)` control constructs through the
  VM path with committed condition semantics.
- Run Prolog `between/3` through the VM path for finite integer generation and
  validation.
- Run Prolog `integer/1` and `succ/2` through the VM path for integer type
  checks and successor relations.
- Run callable CLP(FD) forms through the VM path, including finite domains,
  arithmetic equality constraints, all-different, and labeling.
- Run natural SWI CLP(FD) infix syntax through the VM path, including `1..N`
  range domains and arithmetic equality constraints.
- Run nested additive CLP(FD) equality expressions through the VM path by
  lowering them to finite-domain sum constraints.
- Run supported CLP(FD) `labeling/2` option lists through the VM path,
  including descending value order.
- Run common Prolog list predicates, including finite `length/2`, `sort/2`,
  `msort/2`, `nth0/3`, `nth1/3`, `nth0/4`, and `nth1/4`, through the VM path
  by adapting them to `logic-stdlib` relations.
- Run module-imported apply-family closures through the VM path so higher-order
  list predicates can target predicates from linked modules.
- Run Prolog term equality predicates `=/2`, `\\=/2`, `==/2`, and `\\==/2`
  through the VM path.
- Run Prolog `dif/2` through the VM path as a delayed disequality constraint.
- Expose residual delayed disequality constraints on named `PrologAnswer`
  values produced by compiled source queries and stateful runtimes.
- Add end-to-end stress coverage for negation-as-failure, `once/1`,
  `forall/2`, `bagof/3`, and `setof/3` through the Prolog VM path.
- Raise structured Prolog runtime errors for source-level arithmetic
  instantiation, type, and zero-divisor evaluation failures.
- Run Prolog `throw/1` and `catch/3` exception control through the VM path,
  including recovery from structured arithmetic runtime errors.
- Run Prolog `term_variables/2` through the VM path for source-level
  metaprogramming over reified terms.
- Run Prolog `current_prolog_flag/2` through the VM path for read-only runtime
  flag introspection.
- Run Prolog `set_prolog_flag/2` through the VM path with branch-local
  backtracking semantics.
- Run Prolog `=@=/2`, `\\=@=/2`, and `subsumes_term/2` through the VM path for
  source-level term generality checks.
- Run Prolog text conversion predicates through the VM path, including
  `atom_chars/2`, `atom_codes/2`, `number_chars/2`, `number_codes/2`,
  `char_code/2`, `string_chars/2`, and `string_codes/2`.
- Run Prolog atom composition predicates through the VM path, including
  `atom_concat/3`, `atomic_list_concat/2`, `atomic_list_concat/3`, and
  `number_string/2`.
- Run Prolog text inspection predicates through the VM path, including
  `atom_length/2`, `string_length/2`, `sub_atom/5`, and `sub_string/5`.
- Run Prolog term text I/O predicates through the VM path, including
  `term_to_atom/2` and `atom_to_term/3`.
- Run Prolog term read/write option predicates through the VM path, including
  `read_term_from_atom/3` and `write_term_to_atom/3`.
- Run Prolog `numbervars/3` and `write_term_to_atom/3` `numbervars(true)`
  rendering through the VM path.
- Run Prolog `compound_name_arguments/3` and `compound_name_arity/3` through
  the VM path for compound-only term reflection.
- Run Prolog `acyclic_term/1` and `cyclic_term/1` through the VM path for
  source-level term-shape checks.
- Run Prolog `unifiable/3` and `unify_with_occurs_check/2` through the VM path
  for explicit finite unification.
- Run Prolog `term_hash/2` and `term_hash/4` through the VM path for
  deterministic structural term hashes.

## 0.1.0

- Add the first Prolog-to-Logic-VM compiler package.
- Compile loaded Prolog clauses, dynamic declarations, initialization goals,
  and source queries into `logic-instructions`.
- Add helpers for loading compiled instruction streams into `logic-vm` and
  running source-level queries.
