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
- Run common Prolog list predicates, including finite `length/2`, `sort/2`,
  `msort/2`, `nth0/3`, `nth1/3`, `nth0/4`, and `nth1/4`, through the VM path
  by adapting them to `logic-stdlib` relations.

## 0.1.0

- Add the first Prolog-to-Logic-VM compiler package.
- Compile loaded Prolog clauses, dynamic declarations, initialization goals,
  and source queries into `logic-instructions`.
- Add helpers for loading compiled instruction streams into `logic-vm` and
  running source-level queries.
