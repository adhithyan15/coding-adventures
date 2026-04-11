# hash_set

DT19 hash set implemented in Rust.

This crate wraps `hash_map::HashMap<T, ()>` and exposes the usual set
operations:

- `add`
- `remove`
- `discard`
- `contains`
- `union`
- `intersection`
- `difference`
- `symmetric_difference`
- `is_subset`
- `is_superset`
- `is_disjoint`
- `equals`

## Usage

```rust
use hash_set::HashSet;

let a = HashSet::from_list([1, 2, 3]);
let b = HashSet::from_list([3, 4, 5]);

assert!(a.contains(&1));
assert_eq!(a.size(), 3);
assert_eq!(a.union(b.clone()).to_list().len(), 5);
assert!(a.is_disjoint(&HashSet::from_list([10, 20])));
```
