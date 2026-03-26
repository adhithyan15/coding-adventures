# Bitset (Go)

A compact boolean array that packs bits into 64-bit words (`uint64`), providing
O(n/64) bulk bitwise operations and efficient memory usage.

## What is a Bitset?

A bitset stores a sequence of bits (0 or 1) packed into machine-word-sized
integers. Instead of using an entire byte per boolean, a bitset packs 64
booleans into a single `uint64`. This gives an 8x memory improvement over
`[]bool` and enables bulk operations that process 64 bits per CPU instruction.

## Where it fits

The bitset is a standalone foundation package with no dependencies. It sits
beneath higher-level data structures like Bloom filters, bitmap indexes, and
adjacency matrices.

```
                 Future Consumers
    +-------------+--------------+------------------+
    | Bloom Filter | Bitmap Index | Adjacency Matrix |
    +------+------+------+-------+--------+---------+
           |              |                |
           v              v                v
    +--------------------------------------------+
    |                   Bitset                    |
    |  Foundation layer. No dependencies.         |
    +--------------------------------------------+
```

## Installation

```bash
go get github.com/adhithyan15/coding-adventures/code/packages/go/bitset
```

## Usage

### Creating a Bitset

```go
package main

import (
    "fmt"
    "github.com/adhithyan15/coding-adventures/code/packages/go/bitset"
)

func main() {
    // Create a bitset with 100 addressable bits (all zero)
    bs := bitset.NewBitset(100)

    // Create from an integer
    bs2 := bitset.BitsetFromInteger(42)  // binary: 101010

    // Create from a binary string
    bs3, err := bitset.BitsetFromBinaryStr("1010")
    if err != nil {
        panic(err)
    }

    fmt.Println(bs, bs2, bs3)
}
```

### Single-Bit Operations

```go
bs := bitset.NewBitset(100)

bs.Set(0)       // set bit 0 to 1
bs.Set(42)      // set bit 42 to 1
bs.Clear(0)     // set bit 0 to 0
bs.Toggle(10)   // flip bit 10

if bs.Test(42) {
    fmt.Println("bit 42 is set")
}

// Auto-growth: setting beyond length grows the bitset
bs.Set(500)  // grows capacity automatically
```

### Bulk Bitwise Operations

All bulk operations return a new bitset without modifying the operands.

```go
a := bitset.BitsetFromInteger(0b1100)
b := bitset.BitsetFromInteger(0b1010)

intersection := a.And(b)    // 0b1000
union := a.Or(b)            // 0b1110
symDiff := a.Xor(b)         // 0b0110
complement := a.Not()       // flips all bits within length
difference := a.AndNot(b)   // 0b0100
```

### Queries

```go
bs := bitset.NewBitset(100)
bs.Set(10)
bs.Set(50)

bs.Popcount()   // 2 (number of set bits)
bs.Len()        // 100 (logical length)
bs.Capacity()   // 128 (allocated bits, multiple of 64)
bs.Any()        // true (at least one bit set)
bs.All()        // false (not all bits set)
bs.None()       // false (some bits are set)
```

### Iteration

```go
bs := bitset.BitsetFromInteger(0b10100101)  // bits 0, 2, 5, 7
indices := bs.IterSetBits()
// indices == []int{0, 2, 5, 7}
```

### Conversion

```go
bs := bitset.BitsetFromInteger(42)

val, err := bs.ToInteger()  // 42 (error if > 64 bits)
str := bs.ToBinaryStr()     // "101010"
repr := bs.String()         // "Bitset(101010)"
```

## API Reference

| Function / Method | Description |
|---|---|
| `NewBitset(size)` | Create bitset with `size` zero bits |
| `BitsetFromInteger(v)` | Create from uint64 value |
| `BitsetFromBinaryStr(s)` | Create from "01" string (MSB left) |
| `Set(i)` | Set bit i to 1 (auto-grows) |
| `Clear(i)` | Set bit i to 0 (no-op if out of range) |
| `Test(i)` | Check if bit i is set |
| `Toggle(i)` | Flip bit i (auto-grows) |
| `And(other)` | Bitwise AND (intersection) |
| `Or(other)` | Bitwise OR (union) |
| `Xor(other)` | Bitwise XOR (symmetric difference) |
| `Not()` | Bitwise NOT (complement) |
| `AndNot(other)` | AND-NOT (set difference) |
| `Popcount()` | Count set bits |
| `Len()` | Logical length |
| `Capacity()` | Allocated bits |
| `Any()` | Any bit set? |
| `All()` | All bits set? |
| `None()` | No bits set? |
| `IterSetBits()` | Indices of set bits |
| `ToInteger()` | Convert to uint64 |
| `ToBinaryStr()` | Convert to binary string |
| `String()` | Human-readable representation |
| `Equal(other)` | Structural equality |

## Build

```bash
go test ./... -v -cover
```
