# coding-adventures-bitset-native

Rust-backed bitset for Python -- a native extension built with our
zero-dependency `python-bridge` (no PyO3, no pyo3-ffi, no bindgen).

## What is this?

A drop-in replacement for `coding-adventures-bitset` that runs all bit
manipulation in Rust. The Python interface is identical -- you can swap
the import and everything works the same, just faster.

## How it works

1. The Rust `bitset` crate provides all algorithms (set, clear, test,
   toggle, AND, OR, XOR, NOT, AND-NOT, popcount, iteration, etc.)
2. `python-bridge` provides zero-dependency Rust wrappers around Python's
   C API (no headers needed at build time)
3. This crate (`bitset-native`) is a `cdylib` that implements the
   `PyInit_bitset_native` entry point, creating a Python module with a
   `Bitset` class backed by the Rust implementation

## Building

```bash
# From this directory:
cargo build --release

# Copy the shared library:
# Linux:  cp target/release/libbitset_native.so src/bitset_native/bitset_native.so
# macOS:  cp target/release/libbitset_native.dylib src/bitset_native/bitset_native.so
# Windows: copy target\release\bitset_native.dll src\bitset_native\bitset_native.pyd
```

Or use the BUILD file which handles everything:

```bash
bash BUILD
```

## Usage

```python
from bitset_native import Bitset, BitsetError

# Create a bitset
bs = Bitset(100)
bs.set(42)
bs.set(7)
assert bs.test(42)
assert bs.popcount() == 2

# From integer
bs = Bitset.from_integer(0b10110)
assert list(bs.iter_set_bits()) == [1, 2, 4]

# Bulk operations
a = Bitset.from_integer(0b1100)
b = Bitset.from_integer(0b1010)
c = a & b  # bitwise AND
assert c.to_integer() == 0b1000

# Iteration
for bit_index in bs:
    print(f"Bit {bit_index} is set")
```

## API

- **Constructor**: `Bitset(size=0)`
- **Class methods**: `from_integer(value)`, `from_binary_str(s)`
- **Single-bit**: `set(i)`, `clear(i)`, `test(i)`, `toggle(i)`
- **Bulk ops**: `bitwise_and(other)`, `bitwise_or(other)`, `bitwise_xor(other)`, `bitwise_not()`, `and_not(other)`
- **Queries**: `popcount()`, `capacity()`, `any()`, `all()`, `none()`
- **Iteration**: `iter_set_bits()`, `__iter__`
- **Conversion**: `to_integer()`, `to_binary_str()`
- **Protocols**: `__len__`, `__contains__`, `__repr__`, `__eq__`, `__hash__`
- **Operators**: `&`, `|`, `^`, `~`

## Dependencies

- **Rust crates**: `bitset` (path dep), `python-bridge` (path dep)
- **Python packages**: none (zero runtime deps)
- **Build-time**: Rust toolchain, Python 3.12+
