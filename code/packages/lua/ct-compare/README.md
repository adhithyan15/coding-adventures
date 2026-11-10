# coding-adventures-ct-compare (Lua)

Constant-time byte comparison and selection: the Lua lane of
[CT01](../../../specs/CT01-constant-time-compare.md).

A secret comparison that stops at the first differing byte tells an attacker
how long the matching prefix was. This package compares every byte regardless,
folds the differences into one accumulator and inspects it once at the end.

## API

```lua
local ct = require("coding_adventures.ct_compare")

ct.ct_eq(left, right)                 -- boolean
ct.ct_eq_fixed(left, right)           -- boolean (same as ct_eq in Lua)
ct.ct_select_bytes(left, right, bit)  -- string; left when bit == 1, right when 0
ct.ct_eq_u64(left, right)             -- boolean

ct.ct_eq_bit(left, right)             -- 1 or 0
ct.ct_eq_fixed_bit(left, right)       -- 1 or 0
ct.ct_eq_u64_bit(left, right)         -- 1 or 0
```

* Byte inputs are Lua strings or plain arrays of integers 0..255. Anything else
  (floats, out-of-range values, tables with metatables, other types) raises an
  error rather than being coerced.
* A length mismatch makes `ct_eq` return `false` (or `ct_eq_bit` return `0`)
  without reading either input: lengths are public.
* `ct_select_bytes` raises an error whose message begins with
  `CT_SELECT_LENGTH_MISMATCH` for unequal lengths, before reading either input.
* The selection bit is an **integer** that must be exactly `0` or `1`. Lua has
  no branch-free boolean-to-number conversion (`b and 1 or 0` is a branch), so
  this lane follows Go, Ruby and Elixir. Build the bit with the `_bit`
  variants and never with a conditional:

  ```lua
  local picked = ct.ct_select_bytes(a, b, ct.ct_eq_bit(x, y))
  ```
* Unsigned 64-bit values are Lua integers holding the same 64 bits, so
  `0xFFFFFFFFFFFFFFFF` is `-1`. Floats are rejected.

## What this does and does not guarantee

The Lua source has no content-dependent early return, branch, table index or
loop bound; work depends only on the public lengths. Selection uses the mask
form `right ~ ((left ~ right) & mask)`; `eq_bit` and `eq_u64_bit` derive the
bit arithmetically from the accumulator.

It does **not** guarantee wall-clock constant time, and nothing in Lua could:

* Lua is interpreted. Every operation is a VM instruction dispatched through a
  `switch`, and the interpreter's own branches, caches and allocator are
  outside the program's control.
* The garbage collector can run at any allocation (the loops allocate small
  strings and table slots), adding timing noise and leaving copies of secrets
  in memory that this package cannot zero.
* Strings are immutable and may be interned or shared, so a returned string is
  never "wiped" and its freshness cannot be observed.
* Lua offers no way to inspect or pin generated machine code, and LuaJIT, if
  used, may reintroduce branches on values it knows are 0 or 1. This lane was
  written and tested for PUC-Rio Lua 5.4 (it needs native 64-bit integers and
  bitwise operators).

Treat it as protection against *algorithmic* timing leaks, not against a
local attacker with cache or microarchitectural access.

## Tests

The suite replays all 21 cases of the shared CT01 corpus
(`code/specs/fixtures/ct-compare-v1/cases.json`) and adds every single-byte
mismatch position, all 256 byte values through selection (including all
256x256 byte pairs under both choices), length mismatches, all 64 single-bit
flips of a u64 from several bases, and input validation.

```bash
cd tests && busted . --verbose --pattern=test_
```

Line coverage of the library is 100% of executable lines (measured with a
`debug.sethook` line counter; LuaCov was not installable offline).

## Where it fits

A leaf utility with no dependencies and no capabilities (see
`required_capabilities.json`). MAC verifiers, token checkers and password-hash
verifiers in this repository should compare secrets through it.
