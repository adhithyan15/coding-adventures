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
- Run common Prolog list predicates through the VM path by adapting them to
  `logic-stdlib` relations.

## 0.1.0

- Add the first Prolog-to-Logic-VM compiler package.
- Compile loaded Prolog clauses, dynamic declarations, initialization goals,
  and source queries into `logic-instructions`.
- Add helpers for loading compiled instruction streams into `logic-vm` and
  running source-level queries.
