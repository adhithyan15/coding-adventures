# intel8008-encoder (C++)

A **pure Intel 8008 instruction encoder**, header-only, ISO C++17. A faithful
port of the Rust [`intel8008-encoder`](../../rust/intel8008-encoder) crate, in
namespace `ca::intel8008_encoder` — the encoding tables for the Intel 8008
(1972), the second-generation 8-bit Intel microprocessor. A companion to the
ported [`intel4004-encoder`](../intel4004-encoder).

## ISA subset

`HLT` (`0x76`) and `RET` (`0x07`) are single bytes; `MVI A, n` is 2 bytes
(`0x3E nn`); `JMP addr` (`0x7C`) and `CAL addr` (`0x7E`) are 3 bytes carrying a
14-bit address, encoded low byte first then the high 6 bits.

## API

```cpp
#include "intel8008_encoder.hpp"
namespace i8 = ca::intel8008_encoder;

std::array<std::uint8_t, 2> mvi = i8::encode_mvi_a(42);   // {0x3E, 0x2A}
std::array<std::uint8_t, 3> jmp = i8::encode_jmp(0x0100); // {0x7C, 0x00, 0x01}
```

- `encode_mvi_a(n)` → `std::array<std::uint8_t, 2>`; `encode_jmp(addr)` /
  `encode_cal(addr)` → `std::array<std::uint8_t, 3>` (all `constexpr`).
- `constexpr` `kHlt` / `kRet` / `kMviA` / `kJmp` / `kCal`, and
  `kGpRegisterCount` / `kMviMax`.

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`.
