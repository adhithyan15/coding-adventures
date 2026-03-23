# immutable-list

A persistent (immutable) vector using a 32-way trie with structural sharing, inspired by Clojure's `PersistentVector` (designed by Rich Hickey, based on Phil Bagwell's Hash Array Mapped Tries).

## What is it?

An immutable list is a data structure where every "modification" (push, set, pop) returns a **new** list, leaving the original unchanged. Under the hood, the new and old lists share most of their internal structure via `Arc` reference counting. Only the nodes along the modified path are copied.

This gives you:

- **O(1) amortized push/pop** via a tail buffer that avoids tree traversal ~97% of the time
- **O(log32 n) get/set** which is effectively O(1) for practical sizes (at most 6-7 levels for billions of elements)
- **O(1) clone** via Arc reference counting (no data is copied)
- **Thread safety** with zero synchronization overhead for readers

## How it works

The list is a 32-way branching trie where each internal node has up to 32 children and each leaf holds up to 32 string elements. Index lookup uses **bit partitioning**: the index is split into 5-bit chunks, each selecting a child at one level of the trie.

The **tail buffer** is the key optimization. The last 32 elements live outside the trie in a flat Vec. Most pushes just append to this buffer. Only when it fills (every 32nd push) does it get promoted into the trie as a new leaf node.

## Usage

```rust
use immutable_list::ImmutableList;

// Create and build up a list
let empty = ImmutableList::new();
let one = empty.push("hello".to_string());
let two = one.push("world".to_string());

// Original lists are unchanged
assert_eq!(empty.len(), 0);
assert_eq!(one.len(), 1);
assert_eq!(two.len(), 2);

// Index access
assert_eq!(two.get(0), Some("hello"));
assert_eq!(two.get(1), Some("world"));

// Set returns a new list (structural sharing)
let modified = two.set(0, "hi".to_string());
assert_eq!(modified.get(0), Some("hi"));
assert_eq!(two.get(0), Some("hello")); // original unchanged

// Pop returns (new_list, removed_element)
let (popped, val) = two.pop();
assert_eq!(val, "world");
assert_eq!(popped.len(), 1);

// Build from a slice
let items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
let list = ImmutableList::from_slice(&items);
assert_eq!(list.to_vec(), items);

// Iterate
for elem in list.iter() {
    println!("{}", elem);
}
```

## Layer position

ImmutableList is a foundation package with no dependencies on other packages in this project. It is the Rust core implementation that will be exposed to Python, Ruby, TypeScript, and WASM via FFI bridges.

## Complexity

| Operation  | Time              | Space              |
|------------|-------------------|--------------------|
| `new`      | O(1)              | O(1)               |
| `push`     | O(1) amortized    | O(1) amortized     |
| `get`      | O(log32 n) ~ O(1) | O(1)               |
| `set`      | O(log32 n)        | O(log32 n)         |
| `pop`      | O(1) amortized    | O(1) amortized     |
| `clone`    | O(1)              | O(1)               |
| `iter`     | O(n)              | O(1)               |
| `to_vec`   | O(n)              | O(n)               |
