# Bitset

## Overview

A bitset is a compact data structure that stores a sequence of bits — each one
either 0 or 1 — packed into machine-word-sized integers. Instead of using an
entire byte (or worse, a pointer-sized boolean) to represent a single true/false
value, a bitset packs 64 of them into a single `u64`.

### Why Does This Matter?

**Space.** A Python `list` of 10,000 booleans consumes roughly 80,000 bytes
(each `bool` is a heap-allocated object with a pointer in the list). A bitset
storing the same 10,000 bits uses ~1,250 bytes — a **64x improvement**.

**Speed.** When you AND two boolean arrays together, you loop over 10,000
elements. When you AND two bitsets, you loop over ~157 words. The CPU performs
a single 64-bit `AND` instruction on each word, operating on 64 bits at once.
Bulk operations run in O(n/64) time — effectively 64x faster than element-wise
boolean arrays.

**Ubiquity.** Bitsets appear everywhere in systems programming:

| Application              | What the bitset represents                         |
|--------------------------|----------------------------------------------------|
| Bloom filters            | Hash bucket presence — "maybe in set" vs "not"     |
| Register allocation      | Which CPU registers are live at each program point  |
| Graph algorithms         | Visited-node sets for BFS/DFS                      |
| Database engines         | Bitmap indexes — which rows match a column value    |
| File systems             | Free-block bitmaps — which disk blocks are in use   |
| Network routing          | Subnet masks in CIDR notation (e.g., /24)          |
| Garbage collectors       | Mark bits — which heap objects are reachable        |

## Layer Position

The bitset is a standalone foundation package with no dependencies. It sits
beneath higher-level data structures that build on top of bitwise operations.

```
                     Future Consumers
    ┌──────────────┬──────────────┬──────────────────┐
    │ Bloom Filter  │ Bitmap Index │ Adjacency Matrix │
    │ (hashing +    │ (database    │ (graph edges as  │
    │  k bitsets)   │  column ops) │  row bitsets)    │
    └──────┬───────┴──────┬───────┴────────┬─────────┘
           │              │                │
           ▼              ▼                ▼
    ┌────────────────────────────────────────────────┐
    │                    Bitset                       │
    │                                                 │
    │  Foundation layer. No dependencies.              │
    │  Pure bit manipulation + ArrayList-style growth. │
    └─────────────────────────────────────────────────┘
```

**Depends on:** Nothing (standalone foundation).
**Used by:** Future `bloom-filter`, `bitmap-index`, `adjacency-matrix`, and
any package needing compact boolean storage.

## Package Matrix

The bitset ships as 10 packages across two tiers:

### Tier 1: Core + Pure Implementations

| #  | Package             | Language   | Description                               |
|----|---------------------|------------|-------------------------------------------|
| 1  | `bitset`            | Rust       | Core implementation, canonical reference   |
| 2  | `bitset`            | Python     | Pure Python using `list[int]` (64-bit)     |
| 3  | `bitset`            | Ruby       | Pure Ruby using `Array` of `Integer`       |
| 4  | `bitset`            | TypeScript | Pure TS using `Uint32Array` (32-bit words) |
| 5  | `bitset`            | Go         | Pure Go using `[]uint64`                   |
| 6  | `bitset`            | Elixir     | Pure Elixir using list of integers         |

### Tier 2: Native Extensions (Rust Core via FFI)

| #  | Package                  | Language       | Bridge used      |
|----|--------------------------|----------------|------------------|
| 7  | `bitset-python-native`   | Python (C ext) | `python-bridge`  |
| 8  | `bitset-ruby-native`     | Ruby (C ext)   | `ruby-bridge`    |
| 9  | `bitset-node-native`     | Node.js        | `node-bridge`    |
| 10 | `bitset-wasm`            | WASM           | `wasm-bridge`    |

The pure implementations exist for educational value — you can read the Python
version to understand the algorithm without knowing Rust. The native extensions
exist for performance — they call the Rust core through the FFI bridges defined
in the DS01 spec.

## Concepts

### Bit Ordering: LSB-First

Every bitset in this project uses **LSB-first** (Least Significant Bit first)
ordering. Bit 0 is the least significant bit of the first word. Bit 63 is the
most significant bit of the first word. Bit 64 is the least significant bit of
the second word. And so on.

```
                        Word 0                              Word 1
    ┌─────────────────────────────────────────┐ ┌─────────────────────────
    │ bit 63  bit 62  ...  bit 2  bit 1  bit 0│ │ bit 127 ... bit 65  bit 64
    └─────────────────────────────────────────┘ └─────────────────────────

    MSB ◄──────────────────────────────── LSB    MSB ◄──────────────── LSB

    In memory (little-endian view of each word):

    Word 0 = 0b...0000_0000_0000_0101
                                  ▲ ▲
                                  │ └── bit 0 = 1
                                  └──── bit 2 = 1

    If we have set bits 0 and 2, word 0 = 5 (binary 101).
```

Why LSB-first? Because the math is clean:

- **Word index** of bit `i` = `i / 64` (integer division)
- **Bit offset** within that word = `i % 64` (remainder)
- **Bitmask** to isolate bit `i` = `1 << (i % 64)`

These three formulas are the heart of every bitset operation.

```
    Example: Where does bit 137 live?

    word_index = 137 / 64 = 2       (third word, zero-indexed)
    bit_offset = 137 % 64 = 9       (10th bit within word 2)
    mask       = 1 << 9 = 0x200     (binary: 0...0010_0000_0000)

    To set bit 137:   words[2] |= mask
    To clear bit 137: words[2] &= ~mask
    To test bit 137:  (words[2] & mask) != 0
```

### Internal Representation

Each language uses its natural word-sized integer type:

| Language   | Word type     | Bits per word | Notes                              |
|------------|---------------|---------------|------------------------------------|
| Rust       | `u64`         | 64            | Native unsigned 64-bit             |
| Python     | `int`         | 64            | Arbitrary precision, masked to 64  |
| Ruby       | `Integer`     | 64            | Bignum handles overflow seamlessly |
| TypeScript | `Uint32Array` | 32            | No 64-bit integers in JS           |
| Go         | `uint64`      | 64            | Native unsigned 64-bit             |
| Elixir     | integer       | 64            | Arbitrary precision, masked to 64  |

**TypeScript exception:** JavaScript has no 64-bit integer type. `Number` is a
64-bit float that can only represent integers exactly up to 2^53. `BigInt`
exists but is slow and cannot be used in typed arrays. So the TypeScript
implementation uses `Uint32Array` with 32 bits per word. Every formula that
uses `64` in other languages uses `32` in TypeScript: `i / 32`, `i % 32`,
`1 << (i % 32)`.

### ArrayList-Style Growth Model

A bitset does not have a fixed size. Like Java's `ArrayList` or Python's `list`,
it grows dynamically when you address a bit beyond its current capacity. This
is the key design choice that separates a production bitset from a textbook one.

There are two size concepts:

```
    ┌──────────────────────────────────────────────────────────────────┐
    │                          capacity (256 bits = 4 words)           │
    │                                                                  │
    │  ┌──────────────────────────────────────────┐                    │
    │  │              len (200 bits)                │ ··· unused ····  │
    │  │  (highest addressable bit index + 1)       │ (always zero)   │
    │  └──────────────────────────────────────────┘                    │
    └──────────────────────────────────────────────────────────────────┘
        Word 0          Word 1          Word 2          Word 3
    ┌──────────────┬──────────────┬──────────────┬──────────────┐
    │ bits 0-63    │ bits 64-127  │ bits 128-191 │ bits 192-255 │
    │ (full word)  │ (full word)  │ (full word)  │ (partial)    │
    └──────────────┴──────────────┴──────────────┴──────────────┘
                                       ▲                ▲
                                       │                │
                                  last bit of      these trailing bits
                                  len (bit 199)    are always zero
```

- **`len`** (logical size): the highest addressable bit index + 1. A bitset
  created with `new(200)` has `len = 200`, meaning bits 0 through 199 are
  addressable. `len` is the value returned by the `size()` / `len()` method.

- **`capacity`**: the actual number of bits allocated in memory. Always a
  multiple of the word size (64 bits, or 32 for TypeScript). Always >= `len`.
  A bitset with `len = 200` has `capacity = 256` (4 words x 64 bits/word).

**Growth rule:** When `set(i)` or `toggle(i)` is called with `i >= capacity`,
the bitset doubles its capacity repeatedly until `i < capacity`. This is the
same amortized O(1) growth strategy used by `ArrayList`, Python's `list`, and
Rust's `Vec`.

```
    Growth example: bitset starts with new(100)

    Initial state:
      len = 100, capacity = 128 (2 words)

    set(50):   50 < 128, no growth needed
      len = 100, capacity = 128

    set(200):  200 >= 128, need to grow
      double: 128 → 256.  200 < 256, stop.
      allocate 2 new zero words (now 4 words total)
      len = 201, capacity = 256

    set(500):  500 >= 256, need to grow
      double: 256 → 512.  500 < 512, stop.
      allocate 4 new zero words (now 8 words total)
      len = 501, capacity = 512

    set(1000): 1000 >= 512, need to grow
      double: 512 → 1024.  1000 < 1024, stop.
      allocate 8 new zero words (now 16 words total)
      len = 1001, capacity = 1024
```

**Auto-grow semantics:** `set(i)` and `toggle(i)` auto-grow the bitset if `i`
is beyond the current length. After `set(i)`, `len` becomes `max(len, i + 1)`.
`test(i)` and `clear(i)` do NOT auto-grow — testing or clearing a bit beyond
`len` simply returns `false` or does nothing, respectively.

**Clean-trailing-bits invariant:** Bits beyond `len` in the last word must
always be zero. This invariant is critical for correctness of `popcount`,
`any`, `all`, `none`, equality comparison, and `to_integer`. Every operation
that modifies the last word must zero out trailing bits:

```
    Example: len = 200, capacity = 256

    Word 3 holds bits 192-255, but only bits 192-199 are "real".
    Bits 200-255 must always be zero.

    After any operation that touches word 3:
      used_bits_in_last_word = 200 % 64 = 8
      mask = (1 << 8) - 1 = 0xFF
      words[3] &= mask   // zero out bits 8-63 of word 3

    Word 3:  0b00000000...00000000_XXXXXXXX
                                   ^^^^^^^^
                                   bits 192-199 (may be 0 or 1)
             ^^^^^^^^^^^^^^^^^^^^^^^^
             bits 200-255 (always zero)
```

### Bitwise Operations on Different Sizes

When two bitsets of different lengths are combined with AND, OR, XOR, or
AND-NOT, the result has length `max(a.len, b.len)`. The shorter bitset is
conceptually zero-extended to match the longer one.

```
    a:      [1, 0, 1, 1, 0, 1, 0, 0]     len = 8
    b:      [0, 1, 1, 0]                   len = 4 (zero-extended to 8)
    b':     [0, 1, 1, 0, 0, 0, 0, 0]      (after zero extension)

    a OR b: [1, 1, 1, 1, 0, 1, 0, 0]      len = 8

    The implementation doesn't actually extend b. It simply stops reading
    from b's words once they run out, treating missing words as zero.
```

### Truth Tables for Bitwise Operations

These are the fundamental boolean operations, applied word-by-word across
entire 64-bit words at once:

```
    AND — both bits must be 1         OR — at least one bit must be 1
    ┌───┬───┬───────┐                 ┌───┬───┬───────┐
    │ A │ B │ A & B │                 │ A │ B │ A | B │
    ├───┼───┼───────┤                 ├───┼───┼───────┤
    │ 0 │ 0 │   0   │                 │ 0 │ 0 │   0   │
    │ 0 │ 1 │   0   │                 │ 0 │ 1 │   1   │
    │ 1 │ 0 │   0   │                 │ 1 │ 0 │   1   │
    │ 1 │ 1 │   1   │                 │ 1 │ 1 │   1   │
    └───┴───┴───────┘                 └───┴───┴───────┘

    XOR — bits must differ             NOT — flip every bit
    ┌───┬───┬───────┐                 ┌───┬───────┐
    │ A │ B │ A ^ B │                 │ A │  ~A   │
    ├───┼───┼───────┤                 ├───┼───────┤
    │ 0 │ 0 │   0   │                 │ 0 │   1   │
    │ 0 │ 1 │   1   │                 │ 1 │   0   │
    │ 1 │ 0 │   1   │                 └───┴───────┘
    │ 1 │ 1 │   0   │
    └───┴───┴───────┘

    AND-NOT (a & ~b) — bits in A but not in B
    ┌───┬───┬──────────┐
    │ A │ B │ A & ~B   │
    ├───┼───┼──────────┤
    │ 0 │ 0 │    0     │
    │ 0 │ 1 │    0     │
    │ 1 │ 0 │    1     │
    │ 1 │ 1 │    0     │
    └───┴───┴──────────┘
    Useful for "set difference": elements in A that are not in B.
```

## Public API

### Constructor Signatures

**`new(size)`** — Create a bitset with all bits initially zero.

```
    Rust:       Bitset::new(size: usize) -> Bitset
    Python:     Bitset(size: int) -> Bitset
    Ruby:       Bitset.new(size) -> Bitset
    TypeScript: new Bitset(size: number) -> Bitset
    Go:         NewBitset(size int) -> *Bitset
    Elixir:     Bitset.new(size) -> %Bitset{}
```

The initial `len` is `size`. The initial `capacity` is rounded up to the next
multiple of the word size (64, or 32 for TypeScript). All bits start as zero.
`new(0)` is valid and creates an empty bitset with `len = 0`, `capacity = 0`.

**`from_integer(value)`** — Create a bitset from a non-negative integer. Bit 0
of the bitset is the least significant bit of the integer.

```
    Rust:       Bitset::from_integer(value: u128) -> Bitset
    Python:     Bitset.from_integer(value: int) -> Bitset
    Ruby:       Bitset.from_integer(value) -> Bitset
    TypeScript: Bitset.fromInteger(value: number) -> Bitset
    Go:         BitsetFromInteger(value uint64) -> *Bitset
    Elixir:     Bitset.from_integer(value) -> %Bitset{}
```

The `len` of the result is the position of the highest set bit + 1. If
`value == 0`, then `len = 0`. Example: `from_integer(5)` creates a bitset
with `len = 3` (binary `101`, highest bit at position 2).

**`from_binary_str(s)`** — Create a bitset from a string of `'0'` and `'1'`
characters. The leftmost character is the highest bit (conventional binary
notation, not LSB-first).

```
    Rust:       Bitset::from_binary_str(s: &str) -> Result<Bitset, BitsetError>
    Python:     Bitset.from_binary_str(s: str) -> Bitset
    Ruby:       Bitset.from_binary_str(s) -> Bitset
    TypeScript: Bitset.fromBinaryStr(s: string) -> Bitset
    Go:         BitsetFromBinaryStr(s string) -> (*Bitset, error)
    Elixir:     Bitset.from_binary_str(s) -> {:ok, %Bitset{}} | {:error, reason}
```

Example: `from_binary_str("1010")` sets bits 3 and 1, producing the same
bitset as `from_integer(10)`.

### Single-Bit Operations

**`set(i)`** — Set bit `i` to 1. Auto-grows if `i >= len`.

```
    Rust:       fn set(&mut self, i: usize)
    Python:     def set(self, i: int) -> None
    Ruby:       def set(i) -> self
    TypeScript: set(i: number): void
    Go:         func (b *Bitset) Set(i int)
    Elixir:     Bitset.set(bitset, i) -> %Bitset{}
```

**`clear(i)`** — Set bit `i` to 0. No-op if `i >= len` (does not grow).

```
    Rust:       fn clear(&mut self, i: usize)
    Python:     def clear(self, i: int) -> None
    Ruby:       def clear(i) -> self
    TypeScript: clear(i: number): void
    Go:         func (b *Bitset) Clear(i int)
    Elixir:     Bitset.clear(bitset, i) -> %Bitset{}
```

**`test(i)`** — Return whether bit `i` is 1. Returns `false` if `i >= len`
(does not grow).

```
    Rust:       fn test(&self, i: usize) -> bool
    Python:     def test(self, i: int) -> bool
    Ruby:       def test(i) -> true | false
    TypeScript: test(i: number): boolean
    Go:         func (b *Bitset) Test(i int) bool
    Elixir:     Bitset.test(bitset, i) -> boolean
```

**`toggle(i)`** — Flip bit `i` (0 becomes 1, 1 becomes 0). Auto-grows if
`i >= len`.

```
    Rust:       fn toggle(&mut self, i: usize)
    Python:     def toggle(self, i: int) -> None
    Ruby:       def toggle(i) -> self
    TypeScript: toggle(i: number): void
    Go:         func (b *Bitset) Toggle(i int)
    Elixir:     Bitset.toggle(bitset, i) -> %Bitset{}
```

### Bulk Bitwise Operations

All bulk operations return a **new** bitset. They do not modify either operand.
The result has `len = max(a.len, b.len)`.

```
    Rust:       fn and(&self, other: &Bitset) -> Bitset
                fn or(&self, other: &Bitset) -> Bitset
                fn xor(&self, other: &Bitset) -> Bitset
                fn not(&self) -> Bitset
                fn and_not(&self, other: &Bitset) -> Bitset

    Python:     def bitwise_and(self, other: Bitset) -> Bitset      # also: &
                def bitwise_or(self, other: Bitset) -> Bitset       # also: |
                def bitwise_xor(self, other: Bitset) -> Bitset      # also: ^
                def bitwise_not(self) -> Bitset                     # also: ~
                def and_not(self, other: Bitset) -> Bitset

    Ruby:       def bitwise_and(other) -> Bitset                    # also: &
                def bitwise_or(other) -> Bitset                     # also: |
                def bitwise_xor(other) -> Bitset                    # also: ^
                def bitwise_not -> Bitset                           # also: ~
                def and_not(other) -> Bitset

    TypeScript: and(other: Bitset): Bitset
                or(other: Bitset): Bitset
                xor(other: Bitset): Bitset
                not(): Bitset
                andNot(other: Bitset): Bitset

    Go:         func (b *Bitset) And(other *Bitset) *Bitset
                func (b *Bitset) Or(other *Bitset) *Bitset
                func (b *Bitset) Xor(other *Bitset) *Bitset
                func (b *Bitset) Not() *Bitset
                func (b *Bitset) AndNot(other *Bitset) *Bitset

    Elixir:     Bitset.bitwise_and(a, b) -> %Bitset{}
                Bitset.bitwise_or(a, b) -> %Bitset{}
                Bitset.bitwise_xor(a, b) -> %Bitset{}
                Bitset.flip_all(bitset) -> %Bitset{}
                Bitset.and_not(a, b) -> %Bitset{}
```

**Language-specific operator overloading:**

- **Python**: Implements `__and__`, `__or__`, `__xor__`, `__invert__` so you
  can write `a & b`, `a | b`, `a ^ b`, `~a`.
- **Ruby**: Implements `&`, `|`, `^`, `~` operators.
- **Elixir**: Does NOT use `and`, `or`, `not` as method names because these are
  reserved words in Elixir. Uses `bitwise_and`, `bitwise_or`, `flip_all`
  instead. (Elixir's `not` is also reserved — `flip_all` conveys the meaning
  without collision.)

**NOT semantics:** `not()` flips all bits within `len`. Bits beyond `len` remain
zero (clean-trailing-bits invariant). The result has the same `len` as the
input.

### Counting and Query Operations

```
    Rust:       fn popcount(&self) -> usize
                fn len(&self) -> usize
                fn capacity(&self) -> usize
                fn any(&self) -> bool
                fn all(&self) -> bool
                fn none(&self) -> bool

    Python:     def popcount(self) -> int
                def __len__(self) -> int          # len(bitset)
                def capacity(self) -> int
                def any(self) -> bool
                def all(self) -> bool
                def none(self) -> bool

    Ruby:       def popcount -> Integer
                def size -> Integer
                def capacity -> Integer
                def any? -> true | false
                def all? -> true | false
                def none? -> true | false

    TypeScript: popcount(): number
                get size(): number
                get capacity(): number
                any(): boolean
                all(): boolean
                none(): boolean

    Go:         func (b *Bitset) Popcount() int
                func (b *Bitset) Len() int
                func (b *Bitset) Capacity() int
                func (b *Bitset) Any() bool
                func (b *Bitset) All() bool
                func (b *Bitset) None() bool

    Elixir:     Bitset.popcount(bitset) -> integer
                Bitset.size(bitset) -> integer
                Bitset.capacity(bitset) -> integer
                Bitset.any?(bitset) -> boolean
                Bitset.all?(bitset) -> boolean
                Bitset.none?(bitset) -> boolean
```

- **`popcount`**: Count the number of 1-bits. Named after the CPU instruction
  (`POPCNT` on x86) that counts set bits in a word. The implementation applies
  the hardware popcount (or a software fallback) to each word and sums.
- **`len` / `size`**: Returns the logical size (highest addressable bit + 1).
- **`capacity`**: Returns the allocated size in bits (always a multiple of 64).
- **`any`**: Returns `true` if at least one bit is set. Equivalent to
  `popcount() > 0` but short-circuits on the first non-zero word.
- **`all`**: Returns `true` if ALL bits in `0..len` are set. For an empty
  bitset (`len = 0`), `all()` returns `true` — this is **vacuous truth**, the
  same convention used by Python's `all([])`, Rust's `Iterator::all`, and
  mathematical logic ("for all x in {}, P(x)" is true).
- **`none`**: Returns `true` if no bits are set. Equivalent to `!any()`.

### Iteration

**`iter_set_bits`** — Yield the indices of all set bits in ascending order.

```
    Rust:       fn iter_set_bits(&self) -> impl Iterator<Item = usize>
    Python:     def iter_set_bits(self) -> Iterator[int]
    Ruby:       def each_set_bit(&block) -> self   # yields Integer indices
    TypeScript: *iterSetBits(): Generator<number>
    Go:         func (b *Bitset) IterSetBits() []int    // returns slice
    Elixir:     Bitset.set_bits(bitset) -> [integer]    // returns list
```

The implementation must efficiently skip zero words. If a word is zero, all 64
bits in it are zero and we can jump to the next word immediately. Within a
non-zero word, the standard technique is to use trailing-zero-count to find the
lowest set bit, yield its index, then clear it:

```
    Efficient iteration through set bits of a word:

    word = 0b10100100   (bits 2, 5, 7 are set)

    Iteration 1:
      trailing_zeros(word) = 2      → yield base_index + 2
      word &= word - 1              → 0b10100000  (clear lowest set bit)

    Iteration 2:
      trailing_zeros(word) = 5      → yield base_index + 5
      word &= word - 1              → 0b10000000  (clear lowest set bit)

    Iteration 3:
      trailing_zeros(word) = 7      → yield base_index + 7
      word &= word - 1              → 0b00000000  (clear lowest set bit)

    word == 0, stop.
```

### Conversion Operations

**`to_integer`** — Convert the bitset to a non-negative integer.

```
    Rust:       fn to_integer(&self) -> u128
    Python:     def to_integer(self) -> int
    Ruby:       def to_integer -> Integer
    TypeScript: toInteger(): number
    Go:         func (b *Bitset) ToInteger() (uint64, error)
    Elixir:     Bitset.to_integer(bitset) -> integer
```

For Go, this returns an error if the bitset requires more than 64 bits. For
TypeScript, this returns `NaN` or throws if the value exceeds `Number.MAX_SAFE_INTEGER`
(2^53 - 1). Rust limits to `u128`. Python, Ruby, and Elixir have arbitrary
precision integers, so no overflow is possible.

**`to_binary_str`** — Convert to a string of `'0'` and `'1'` characters with
the highest bit on the left (conventional binary notation).

```
    Rust:       fn to_binary_str(&self) -> String
    Python:     def to_binary_str(self) -> str
    Ruby:       def to_binary_str -> String
    TypeScript: toBinaryStr(): string
    Go:         func (b *Bitset) ToBinaryStr() string
    Elixir:     Bitset.to_binary_str(bitset) -> String.t()
```

Example: a bitset with bits 0 and 2 set (`len = 3`) produces `"101"`.

**`repr` / `to_s` / Display** — Human-readable debug representation.

```
    Rust:       impl fmt::Display for Bitset     // e.g. "Bitset(101)"
    Python:     def __repr__(self) -> str         // e.g. "Bitset('101')"
    Ruby:       def to_s -> String                // e.g. "Bitset(101)"
    TypeScript: toString(): string                // e.g. "Bitset(101)"
    Go:         func (b *Bitset) String() string  // e.g. "Bitset(101)"
    Elixir:     defimpl String.Chars              // e.g. "Bitset(101)"
```

### Equality

Two bitsets are equal if and only if they have the same `len` and the same
bits set. Capacity is irrelevant to equality — a bitset with `capacity = 128`
can equal one with `capacity = 256` if their `len` and set bits match.

```
    Rust:       impl PartialEq for Bitset
    Python:     def __eq__(self, other) -> bool
    Ruby:       def ==(other) -> true | false
    TypeScript: equals(other: Bitset): boolean
    Go:         func (b *Bitset) Equal(other *Bitset) bool
    Elixir:     Bitset.equal?(a, b) -> boolean
```

## Data Flow

### Single-Bit Set Operation

Here is the complete step-by-step data flow when `set(137)` is called on a
bitset with `len = 100, capacity = 128` (2 words):

```
    Step 1: Check if growth is needed
    ─────────────────────────────────
    i = 137, capacity = 128
    137 >= 128 → growth needed!

    Step 2: ArrayList-style growth
    ──────────────────────────────
    new_capacity = 128
    while new_capacity <= 137:
        new_capacity *= 2       // 128 → 256
    Allocate 2 new zero words (words[2] and words[3])
    capacity = 256

    Step 3: Update len
    ──────────────────
    len = max(100, 137 + 1) = 138

    Step 4: Compute word index and mask
    ────────────────────────────────────
    word_index = 137 / 64 = 2
    bit_offset = 137 % 64 = 9
    mask = 1u64 << 9 = 0x0000_0000_0000_0200

    Step 5: Set the bit
    ───────────────────
    words[2] |= mask

    Before: words = [w0, w1, 0x0000000000000000, 0x0000000000000000]
    After:  words = [w0, w1, 0x0000000000000200, 0x0000000000000000]

    Step 6: Clean trailing bits (not needed here — bit 137 < 192,
            so we didn't modify the last word's trailing region)

    Final state: len = 138, capacity = 256, bit 137 is now 1
```

### ArrayList-Style Growth in Detail

```
    new(0)           len=0   cap=0    words=[]
      │
      ▼
    set(3)           len=4   cap=64   words=[0x08]
      │               Growth: 0 → 64 (minimum allocation)
      ▼
    set(50)          len=51  cap=64   words=[0x0004000000000008]
      │               No growth needed (50 < 64)
      ▼
    set(100)         len=101 cap=128  words=[..., ...]
      │               Growth: 64 → 128
      ▼
    set(200)         len=201 cap=256  words=[..., ..., ..., ...]
      │               Growth: 128 → 256
      ▼
    set(500)         len=501 cap=512  words=[8 words]
      │               Growth: 256 → 512
      ▼
    set(1023)        len=1024 cap=1024 words=[16 words]
                      Growth: 512 → 1024

    Amortized cost: Each bit set that triggers growth copies O(n) words,
    but since we double each time, the total cost of n set operations is
    O(n) — the same amortized O(1) guarantee as ArrayList/Vec/list.
```

### Bitwise AND Between Two Bitsets

```
    a: len=200, words = [a0, a1, a2, a3]   (4 words, capacity=256)
    b: len=100, words = [b0, b1]            (2 words, capacity=128)

    result = a.and(b)

    Step 1: Determine result size
    ─────────────────────────────
    result.len = max(200, 100) = 200
    result has 4 words (capacity = 256)

    Step 2: AND word by word
    ────────────────────────
    result.words[0] = a0 & b0      // both operands have word 0
    result.words[1] = a1 & b1      // both operands have word 1
    result.words[2] = a2 & 0       // b has no word 2 → treat as 0
    result.words[3] = a3 & 0       // b has no word 3 → treat as 0

    Since anything AND 0 = 0, words beyond the shorter operand are zero.
    This matches the intuition: AND finds bits set in BOTH operands,
    and the shorter one has no bits in the extended region.

    Step 3: Clean trailing bits in result
    ──────────────────────────────────────
    result.len = 200, so bits 200-255 in word 3 must be zero.
    (They already are from the AND, but we enforce the invariant anyway.)
```

### Native Extension Data Flow (Python Example)

```
    Python user code                Python/Rust boundary              Rust core
    ──────────────────              ──────────────────────            ──────────

    from bitset_native import Bitset
    a = Bitset(1000)        ───►    PyInit_bitset_native()
                                    py_bitset_new(size=1000)   ───►  Bitset::new(1000)
                                    wrap in PyCapsule            ◄──  returns Bitset
                            ◄───    return Python object

    a.set(42)               ───►    py_bitset_set(self, i=42)  ───►  bitset.set(42)
                            ◄───    return None                 ◄──  (mutates in place)

    b = a & other           ───►    py_bitset_and(a, b)        ───►  a.and(&b)
                                    wrap result in PyCapsule    ◄──  returns new Bitset
                            ◄───    return Python object

    The Rust `Bitset` struct lives inside a PyCapsule on the Python heap.
    The python-bridge handles all reference counting and type checking.
    The Rust core has zero knowledge of Python objects.
```

## Error Handling

### `from_binary_str` with Invalid Characters

If the input string contains any character other than `'0'` or `'1'`, the
constructor must raise an error:

```
    Rust:       Err(BitsetError::InvalidCharacter { char, position })
    Python:     raise ValueError("invalid character 'x' at position 3")
    Ruby:       raise ArgumentError, "invalid character 'x' at position 3"
    TypeScript: throw new Error("invalid character 'x' at position 3")
    Go:         return nil, fmt.Errorf("invalid character 'x' at position 3")
    Elixir:     {:error, "invalid character 'x' at position 3"}
```

An empty string `""` is valid and produces an empty bitset with `len = 0`.

### `to_integer` Overflow

Languages with fixed-size integers must handle overflow:

| Language   | Max representable  | Overflow behavior                           |
|------------|--------------------|---------------------------------------------|
| Rust       | 128 bits (`u128`)  | Returns `Err(BitsetError::Overflow)`        |
| Go         | 64 bits (`uint64`) | Returns `0, ErrOverflow`                    |
| TypeScript | 53 bits (`number`) | Throws `Error("exceeds MAX_SAFE_INTEGER")`  |
| Python     | Unlimited          | No overflow possible                        |
| Ruby       | Unlimited          | No overflow possible                        |
| Elixir     | Unlimited          | No overflow possible                        |

### Negative Indices

Negative bit indices are not supported. All languages must raise an error
(or panic, in Rust) if a negative index is passed to `set`, `clear`, `test`,
or `toggle`. In languages with unsigned integers (Rust, Go), the type system
prevents this. In Python, Ruby, TypeScript, and Elixir, a runtime check is
needed.

## Test Strategy

### Constructor Tests

- `new(0)` — empty bitset, `len = 0`, `capacity = 0`
- `new(1)` — single bit, `len = 1`, `capacity = 64`
- `new(64)` — exactly one word, `len = 64`, `capacity = 64`
- `new(65)` — crosses word boundary, `len = 65`, `capacity = 128`
- `new(1000)` — large bitset
- `from_integer(0)` — empty
- `from_integer(1)` — single bit at position 0
- `from_integer(0xFF)` — 8 bits
- `from_integer(2**64 - 1)` — full 64-bit word (where applicable)
- `from_binary_str("")` — empty
- `from_binary_str("0")` — single zero bit
- `from_binary_str("1")` — single one bit
- `from_binary_str("10110")` — mixed bits
- `from_binary_str("abc")` — error case

### Single-Bit Operation Tests

- Set and test each bit in a word (bits 0 through 63)
- Set bit at word boundary (bit 63, bit 64)
- Clear a set bit, verify test returns false
- Clear an unset bit (no-op)
- Toggle: 0 → 1 → 0
- Test beyond len returns false without growing
- Clear beyond len is no-op without growing
- Set beyond len triggers growth and updates len

### Auto-Growth Tests

- `new(0)` then `set(0)` — grows from 0 to 64
- `new(64)` then `set(64)` — grows from 64 to 128
- `new(64)` then `set(200)` — grows from 64 to 256 (doubles twice)
- Verify capacity is always a multiple of word size
- Verify capacity doubles (not just adds one word)
- Verify len updates to `i + 1` after `set(i)`
- Verify clean-trailing-bits invariant after growth

### Bulk Operation Tests

- AND, OR, XOR of same-size bitsets
- AND, OR, XOR of different-size bitsets
- NOT preserves len, flips all bits
- AND-NOT as set difference
- Operations on empty bitsets
- Operations where one operand is all-zeros
- Operations where one operand is all-ones
- Verify result is a new object (not a mutation of either input)

### Counting Tests

- `popcount` of empty bitset = 0
- `popcount` of all-ones bitset = len
- `popcount` after various set/clear operations
- `any` / `none` on empty = false / true
- `any` / `none` on non-empty
- `all` on empty = true (vacuous truth)
- `all` on all-ones = true
- `all` on partial = false

### Iteration Tests

- `iter_set_bits` on empty bitset yields nothing
- Single set bit yields one index
- Multiple set bits in one word
- Set bits spanning multiple words
- Set bits at word boundaries (63, 64, 127, 128)
- Verify ascending order
- Verify no duplicates

### Conversion Round-Trip Tests

- `from_integer(n).to_integer() == n` for various n
- `from_binary_str(s).to_binary_str() == s` for various s
- Round-trip through set operations:
  `new(100)` → set bits → `to_integer()` → `from_integer()` → verify same bits

### Edge Cases

- Size 0 bitset: all operations should work (no panics)
- Word-boundary sizes: 64, 128, 192 (exact multiple of word size)
- Word-boundary + 1: 65, 129, 193 (one bit into next word)
- Word-boundary - 1: 63, 127, 191 (one bit before next word)
- Very large bitset: 10,000+ bits
- Equality: same bits different capacity, different len same bits (not equal),
  both empty

### Property-Based Tests

Use a property-testing framework (proptest in Rust, Hypothesis in Python, etc.)
to verify algebraic laws:

| Property              | Law                                            |
|-----------------------|------------------------------------------------|
| Commutativity         | `a & b == b & a`, `a \| b == b \| a`           |
| Associativity         | `(a & b) & c == a & (b & c)`                   |
| Idempotence           | `a & a == a`, `a \| a == a`                    |
| Identity              | `a & all_ones == a`, `a \| empty == a`         |
| Complement            | `a & ~a == empty`, `a \| ~a == all_ones`       |
| Double negation       | `~~a == a`                                     |
| De Morgan's (AND)     | `~(a & b) == ~a \| ~b`                         |
| De Morgan's (OR)      | `~(a \| b) == ~a & ~b`                         |
| Absorption            | `a & (a \| b) == a`, `a \| (a & b) == a`      |
| Distributivity        | `a & (b \| c) == (a & b) \| (a & c)`          |
| Set difference        | `a.and_not(b) == a & ~b`                       |
| Popcount of AND       | `popcount(a & b) <= min(popcount(a), popcount(b))` |
| Popcount of OR        | `popcount(a \| b) >= max(popcount(a), popcount(b))` |

## Implementation Sequence

The implementation follows the standard repo workflow: Rust core first, then
pure implementations in parallel, then native extensions.

```
    Phase 1: Rust core
    ──────────────────
    code/packages/rust/bitset/
    ├── src/lib.rs          ← Full implementation + inline docs
    ├── Cargo.toml
    ├── BUILD
    ├── README.md
    ├── CHANGELOG.md
    └── tests/
        └── bitset_test.rs  ← Including proptest property tests

    Phase 2: Pure implementations (can be done in parallel)
    ───────────────────────────────────────────────────────
    code/packages/python/bitset/
    code/packages/ruby/bitset/
    code/packages/typescript/bitset/
    code/packages/go/bitset/
    code/packages/elixir/bitset/

    Each follows the same structure with language-appropriate
    tooling (pyproject.toml, gemspec, package.json, go.mod, mix.exs).

    Phase 3: Native extensions
    ──────────────────────────
    code/packages/rust/bitset-python-native/
    code/packages/rust/bitset-ruby-native/
    code/packages/rust/bitset-node-native/
    code/packages/rust/bitset-wasm/

    These use the FFI bridges from DS01 to wrap the Rust core.
```

## Future Extensions

The bitset is a building block. Here is what it enables:

1. **Bloom filter** — A probabilistic set membership data structure. Uses `k`
   independent hash functions, each mapping to a bit position. Insert: set all
   `k` bits. Query: test all `k` bits. False positives are possible, false
   negatives are not. The bitset provides the underlying storage.

2. **Compressed bitsets** — For sparse bitsets (millions of bits, few set),
   run-length encoding (RLE) compresses long runs of zeros. Formats like
   Roaring Bitmaps partition the bit space into chunks and choose the best
   representation (array, bitset, or RLE) per chunk.

3. **Adjacency matrix** — Represent a graph as a matrix of bitsets. Row `i` is
   a bitset where bit `j` is set if there is an edge from node `i` to node `j`.
   Reachability queries become bitwise OR chains. Common-neighbor queries become
   bitwise AND.

4. **SIMD acceleration** — Modern CPUs have 256-bit (AVX2) and 512-bit (AVX-512)
   SIMD registers. A SIMD-accelerated bitset processes 4 or 8 words per
   instruction instead of 1. This is a natural optimization for the Rust core
   once the scalar implementation is correct and well-tested.

5. **Bitmap indexes** — Database engines use bitsets to index column values.
   For a column with values {red, green, blue}, three bitsets track which rows
   have each value. Query `WHERE color = 'red' AND size > 10` becomes a
   bitwise AND of two bitsets — orders of magnitude faster than scanning rows.
