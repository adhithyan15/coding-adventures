# intel4004-encoder (C++)

A **pure Intel 4004 instruction encoder**, header-only, ISO C++17. A faithful
port of the Rust [`intel4004-encoder`](../../rust/intel4004-encoder) crate, in
namespace `ca::intel4004_encoder` — the encoding tables for the Intel 4004
(1971), the world's first commercial microprocessor. A companion to the ported
[`ibm704-encoder`](../ibm704-encoder) and [`ge225-encoder`](../ge225-encoder).

## ISA subset

`LDM n` (`0xD0|n`), `LD r` (`0xA0|r`), `XCH r` (`0xB0|r`) are single bytes;
`JUN a` is 2 bytes (`0100 aaaa aaaaaaaa`). The 4004 has no formal `HLT`;
`JUN 0x000` at ROM address 0 loops forever (`kHaltLoop = {0x40, 0x00}`).

## API

```cpp
#include "intel4004_encoder.hpp"
namespace i4 = ca::intel4004_encoder;

std::uint8_t a = i4::encode_ldm(5);              // 0xD5
std::array<std::uint8_t, 2> j = i4::encode_jun(0xABC);  // {0x4A, 0xBC}
```

- `encode_ldm` / `encode_ld` / `encode_xch` — single-byte ops (nibble masked).
- `encode_jun(addr)` — returns a `std::array<std::uint8_t, 2>` (12-bit address).
- `constexpr` `kLdmOpcode` / `kLdOpcode` / `kXchOpcode` / `kJunOpcode`,
  `kHaltLoop`, and the `kGpRegisterCount` / `kLdmMax` / `kLdmMinSigned` capacity
  constants.

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`.
