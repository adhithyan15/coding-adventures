# immutable-list-native

A Python native extension wrapping the Rust `immutable-list` crate via
`python-bridge` (zero third-party dependencies).

## What It Does

Provides an `ImmutableList` class -- a persistent (immutable) list data
structure. Every "mutation" (push, set, pop) returns a **new** list, leaving
the original unchanged. Under the hood, the new and old lists share most of
their internal memory via Rust's `Arc` reference counting.

The Rust implementation uses a 32-way trie with a tail buffer (the same
design as Clojure's PersistentVector). This gives effectively O(1) access,
push, and pop for practical list sizes.

## How It Fits in the Stack

```
Python code
    |
    v
immutable-list-native (this package)   <-- Python C extension (.pyd/.so)
    |
    v
python-bridge (Rust)                   <-- Zero-dep CPython C API wrapper
    |
    v
immutable-list (Rust)                  <-- Core persistent vector logic
```

- `immutable-list`: Pure Rust library implementing the 32-way trie
- `python-bridge`: Our zero-dependency Rust wrapper around CPython's C API
- This package: The glue that exposes `ImmutableList` to Python

## Usage

```python
from immutable_list_native import ImmutableList

# Create an empty list
empty = ImmutableList()

# Push elements (returns new list each time)
a = empty.push("hello")
b = a.push("world")
print(a.to_list())   # ['hello']       -- a is unchanged
print(b.to_list())   # ['hello', 'world']

# Build from a Python list
lst = ImmutableList.from_list(["a", "b", "c"])
print(len(lst))      # 3
print(lst[0])        # 'a'
print(lst[-1])       # 'c'

# Set returns a new list
modified = lst.set(1, "B")
print(lst.to_list())       # ['a', 'b', 'c']  -- original unchanged
print(modified.to_list())  # ['a', 'B', 'c']

# Pop returns (new_list, removed_value)
new_lst, val = lst.pop()
print(val)                 # 'c'
print(new_lst.to_list())   # ['a', 'b']

# Iteration
for item in lst:
    print(item)
```

## API

| Method | Description |
|--------|-------------|
| `ImmutableList()` | Create an empty list |
| `ImmutableList.from_list(items)` | Create from a Python list of strings |
| `push(value)` | Append value, return new list |
| `get(index)` | Get element (supports negative indices), None if OOB |
| `set(index, value)` | Replace element, return new list |
| `pop()` | Remove last element, return (new_list, value) |
| `len()` / `__len__` | Number of elements |
| `is_empty()` | True if the list has no elements |
| `to_list()` | Convert to a plain Python list |
| `__getitem__` | Bracket notation with negative index support |
| `__iter__` | Iteration protocol |
| `__eq__` | Element-wise equality |
| `__repr__` | Human-readable representation |

## Building

```bash
# Create venv and install test deps
uv venv --quiet --clear
uv pip install pytest --quiet

# Build the Rust extension
cargo build --release

# Copy the shared library into the Python package
# (platform-specific -- see BUILD file)

# Run tests
PYTHONPATH=src python -m pytest tests/ -v
```
