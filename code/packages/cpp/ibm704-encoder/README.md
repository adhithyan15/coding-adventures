# ibm704-encoder (C++)

A **pure IBM 704 instruction encoder**, header-only, ISO C++17. A faithful port
of the Rust [`ibm704-encoder`](../../rust/ibm704-encoder) crate, in namespace
`ca::ibm704_encoder` — an encoder for the IBM 704 (1954), the mainframe on which
John McCarthy and his students first ran **Lisp** in 1959.

## Why the 704?

`CAR`/`CDR`, Lisp's universal accessors, were IBM 704 instruction-word field
names — **C**ontents of the **A**ddress / **D**ecrement part of **R**egister.
The 704's 36-bit word split into prefix / decrement / tag / address fields; a
cons cell fit one per word, and `(CAR x)` took the address half, `(CDR x)` the
decrement.

## Word format (idealised, v0.1.0)

| word bits | field | notes |
|-----------|-------|-------|
| 35..27 (9)  | opcode  | `HTR = 0o420`, `CLA = 0o500` |
| 26..15 (12) | zero    | unused |
| 14..0 (15)  | address | ≤ 32 K words |

Each 36-bit word packs into 5 bytes, low byte first, top byte's high nibble zero.

## API

```cpp
#include "ibm704_encoder.hpp"
namespace ib = ca::ibm704_encoder;

std::uint64_t cla_42 = ib::encode_cla(42);          // 0xA'0000'002A
std::array<std::uint8_t, 5> bytes = ib::pack_word(cla_42);
// bytes == {0x2A,0x00,0x00,0x00,0x0A}
// ib::kHtrHaltBytes == ib::pack_word(ib::encode_htr(0)) == {0,0,0,0x80,0x08}
```

- `encode_instruction(opcode, address)` + `encode_htr` / `encode_cla` → a 36-bit
  word (out-of-range address bits masked).
- `pack_word(word)` → `std::array<std::uint8_t, 5>` little-endian wire form.
- `constexpr` constants `kHtr`, `kCla`, `kWordBits`, `kWordMask`,
  `kBytesPerWord`, `kAddrBits`, `kAddrMask`, `kOpcodeShift`, `kHtrHaltBytes`.

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`.
