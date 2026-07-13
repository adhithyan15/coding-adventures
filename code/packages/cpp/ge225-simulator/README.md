# ge225-simulator (C++)

A **GE-225 CPU simulator**, header-only, pure ISO C++17. A faithful port of the
Rust [`ge225-simulator`](../../rust/ge225-simulator) crate, in namespace
`ca::ge225_simulator`: a fetch-decode-execute emulator for the GE-225 (1959),
the mainframe **Dartmouth BASIC** was designed on. It models the 20-bit machine —
`A`/`Q` registers, index-register groups, a bit-addressed memory, the console
typewriter and card reader, and the full instruction set.

## API

```cpp
#include "ge225_simulator.hpp"
namespace ge = ca::ge225_simulator;

ge::Simulator s(4096);
s.load_words({ge::encode_instruction(000, 0, 10),   // LDA 10
              ge::encode_instruction(001, 0, 11),   // ADD 11
              ge::assemble_fixed("NOP")}, 0);
s.run(4);
std::int32_t a = s.a();
std::vector<ge::Trace> t = s.run(1);   // step traces
```

- **Free functions** — `encode_instruction` / `decode_instruction` /
  `assemble_fixed` / `assemble_shift` (throw `Error` on bad input),
  `pack_words` / `unpack_words`.
- **`Simulator`** — `load_words`, `read_word` / `write_word`, `step` (returns a
  `Trace`) / `run`, the typewriter and card reader, and accessors (`a()`,
  `q()`, `pc()`, `x_word(slot)`, …).
- **`Error`** — a `std::runtime_error` subclass carrying an `ErrorKind`.

## UB-safe arithmetic

Rust's `i32`/`i64` wrapping shifts (e.g. `(v << 24) >> 24` sign extension) would
be signed-overflow UB if ported naively; every left shift and double-word
bit-shuffle here uses **unsigned** types, keeping only arithmetic right shifts
signed. Clean under ASan + UBSan.

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`.
