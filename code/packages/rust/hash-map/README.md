# hash_map

DT18 hash map implemented in Rust.

This crate mirrors the DT18 spec with two collision strategies:

- `CollisionStrategy::Chaining`
- `CollisionStrategy::OpenAddressing`

Supported hash functions:

- `siphash` / `siphash_2_4`
- `fnv1a` / `fnv1a_32`
- `murmur3` / `murmur3_32`
- `djb2`

## Usage

```rust
use hash_map::{
    from_entries, merge, CollisionStrategy, HashAlgorithm, HashMap,
};

let map = HashMap::<String, i32>::new(16, CollisionStrategy::Chaining)
    .set("hello".to_string(), 42)
    .set("world".to_string(), 7);

assert_eq!(map.get(&"hello".to_string()), Some(&42));
assert!(map.has(&"world".to_string()));

let open = HashMap::<i32, &str>::with_options(8, "open_addressing", "fnv1a")
    .set(1, "one")
    .set(9, "nine");

assert_eq!(open.capacity(), 8);
assert_eq!(open.size(), 2);

let pairs = from_entries([(1, "a"), (2, "b")]);
assert_eq!(pairs.size(), 2);

let merged = merge(
    from_entries([("a", 1), ("b", 2)]),
    from_entries([("b", 99), ("c", 3)]),
);
assert_eq!(merged.get(&"b"), Some(&99));

let _alg = HashAlgorithm::SipHash24;
```
