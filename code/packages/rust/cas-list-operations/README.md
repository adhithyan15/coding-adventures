# cas-list-operations (Rust)

Pure list operations over the symbolic IR — Rust port of the Python `cas-list-operations` package.

## Operations

| Function | Description |
|----------|-------------|
| `length(lst)` | Number of elements |
| `first(lst)` | First element (error on empty) |
| `rest(lst)` | All but first (error on empty) |
| `last(lst)` | Last element (error on empty) |
| `reverse(lst)` | Reversed list |
| `append(lsts)` | Concatenate slice of lists |
| `join(lsts)` | Alias for `append` (Mathematica spelling) |
| `part(lst, i)` | 1-based index; negative counts from end |
| `range_(n, stop, step)` | Generate integer list; single-arg form `[1..n]` |
| `map_(f, lst)` | Apply `f` to each element (unevaluated) |
| `apply_(f, lst)` | Replace list head with `f` |
| `select(lst, pred)` | Filter elements by predicate |
| `sort_(lst)` | Stable sort by debug repr |
| `flatten(lst, depth)` | Flatten nested lists; `-1` = unlimited |

## Usage

```rust
use cas_list_operations::{make_list, length, first, rest, reverse, range_, append};
use symbolic_ir::int;

let lst = make_list(vec![int(1), int(2), int(3)]);
assert_eq!(length(&lst).unwrap(), int(3));
assert_eq!(first(&lst).unwrap(), int(1));
assert_eq!(reverse(&lst).unwrap(), make_list(vec![int(3), int(2), int(1)]));

// MACSYMA-style range: range_(5) → [1, 2, 3, 4, 5]
let five = range_(5, None, 1).unwrap();
assert_eq!(five, make_list(vec![int(1), int(2), int(3), int(4), int(5)]));
```

## Error handling

All fallible operations return `ListResult<IRNode>` (= `Result<IRNode, ListOperationError>`).

```rust
use cas_list_operations::{first, make_list};
assert!(first(&make_list(vec![])).is_err());
```

## Stack position

```
symbolic-ir  ←  cas-list-operations
```
