# Changelog — cas-list-operations (Rust)

## [0.1.0] — 2026-04-27

### Added

- Initial Rust port of the Python `cas-list-operations` package.
- `operations` module with `ListOperationError` error type and `ListResult<T>` alias.
- Helper functions: `as_list(node)` (extract args from a List IR node),
  `make_list(args)` (construct a `List(...)` IR node).
- `length(lst)` — returns `IRNode::Integer(n)`.
- `first(lst)` — first element; error on empty list.
- `rest(lst)` — everything but the first; error on empty list.
- `last(lst)` — last element; error on empty list.
- `reverse(lst)` — reversed list.
- `append(lsts: &[IRNode])` — concatenate a slice of lists (variadic in Python).
- `join(lsts: &[IRNode])` — alias for `append` (Mathematica spelling).
- `part(lst, index: i64)` — 1-based access; negative counts from end; `0` is an error.
- `range_(start, stop: Option<i64>, step: i64)` — generate integer list;
  single-arg form (`stop = None`) produces `[1..start]` per MACSYMA convention.
- `map_(f: IRNode, lst)` — apply `f` to each element as unevaluated `f(a)` nodes.
- `apply_(f: IRNode, lst)` — replace list head with `f`.
- `select(lst, pred: Fn(&IRNode) -> bool)` — filter elements.
- `sort_(lst)` — stable sort by `Debug` representation (consistent with canonical ordering).
- `flatten(lst, depth: i64)` — flatten nested lists; `-1` means unlimited depth.
- IR head-name string constants: `LIST`, `LENGTH`, `FIRST`, `REST`, `LAST`,
  `APPEND`, `REVERSE`, `RANGE`, `MAP`, `APPLY_HEAD`, `SELECT`, `SORT`,
  `PART`, `FLATTEN`, `JOIN`.
- 36 integration tests; all passing.
