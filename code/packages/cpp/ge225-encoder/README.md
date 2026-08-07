# ge225-encoder (C++)

A **pure GE-225 instruction encoder**, header-only, ISO C++17. A faithful port of
the Rust [`ge225-encoder`](../../rust/ge225-encoder) crate, in namespace
`ca::ge225_encoder` — the encoding tables for the GE-225 (1959), the mainframe at
Dartmouth where **Dartmouth BASIC** was designed in 1964. A companion to the
[`ibm704-encoder`](../ibm704-encoder), the machine Lisp was born on.

## Word packing

Each 20-bit word is emitted as 3 big-endian bytes (byte 0's top nibble zero):
`byte 0` = 4-bit opcode nibble, `byte 1/2` = 16-bit immediate/address (for
STA/LD/ADD/SUB the low nibble of byte 2 is the register index).

## API

```cpp
#include "ge225_encoder.hpp"
namespace ge = ca::ge225_encoder;

std::array<std::uint8_t, 3> w = ge::encode_lda(5);   // {0x01,0x00,0x05}
auto [op, payload] = ge::decode_word(ge::encode_br(0xABCD));  // 0x06, 0xABCD
```

- `encode_lda`, `encode_sta`, `encode_ld`, `encode_add`, `encode_sub`,
  `encode_br`, `encode_bnz`, `encode_bz`, `encode_bmi`, `encode_jsr` — each
  returns a `std::array<std::uint8_t, 3>`.
- `decode_word` — returns `std::pair<std::uint8_t, std::uint16_t>`.
- `constexpr` `k*OpcodeNibble`s, `kHaltWord` / `kRtsWord`, and the `kLda*` /
  `kGpRegisterCount` capacity constants.

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`.
