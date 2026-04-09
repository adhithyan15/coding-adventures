# Bitset

A compact, dynamically-growing boolean array for the coding-adventures Swift stack.

## Overview

The `bitset` package implements a bitset: an array of bits backed by an array of `UInt64` words. It packs 64 booleans into every 8 bytes of memory, offering up to a 64x space saving and a substantial speedup for bulk bitwise operations (AND, OR, XOR, NOT) compared to standard `Bool` arrays.

This is a Layer 0 foundation component, having no dependencies. It dynamically allocates space akin to `ArrayList` as elements beyond its capacity are set or toggled.

## Features

- **Dynamic Growth**: Accessing past the current `len` automatically increases the capacity.
- **Bulk Bitwise Operations**: Operates 64 bits at a time using native hardware instructions.
- **LSB-First Layout**: Fast modulus mathematics. Bit index `i` maps to word `i / 64` and bit `i % 64`.

## Usage

```swift
let b1 = try Bitset(fromBinaryStr: "1101")
let b2 = Bitset(fromInteger: 5) // 0101

let result = b1.and(b2)
print(result.toBinaryStr()) // "0101"
```
