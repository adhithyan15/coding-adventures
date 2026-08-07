# display (C++)

A VGA text-mode framebuffer simulation, **header-only** in pure ISO C++17
(namespace `ca::display`). A faithful port of the Rust
[`display`](../../rust/display) crate.

## What it does

Simulates the classic 80x25 VGA text-mode framebuffer — a grid of cells, each
2 bytes (character + colour attribute). The `DisplayDriver` tracks a cursor,
interprets `\n`/`\r`/`\t`/backspace, wraps at the right edge, and scrolls when
output runs past the bottom.

## API

- `make_attribute(fg, bg)` and the `COLOR_*` palette constants.
- `DisplayConfig::default_config()` (80x25) / `::compact()` (40x10).
- `DisplayDriver(config, memory)` — clears the screen (like Rust `new`);
  `DisplayDriver::wrap(config, memory)` — preserves existing content.
- `put_char`, `put_char_at`, `puts`, `clear`, `scroll`, `set_cursor`,
  `get_cursor`, `get_cell`.
- `snapshot()` → a `DisplaySnapshot` with `lines`, `cursor`, `rows`, `columns`
  and `to_string_padded()` / `contains()` / `line_at()`.

## Design notes

- **Caller-owned framebuffer.** The driver views a `std::vector<std::uint8_t>&`
  (mirroring the Rust borrowed `&mut [u8]`) — supply at least
  `columns * rows * BYTES_PER_CELL` bytes. Snapshot is a value type
  (`std::vector<std::string>`).
- **Defensive bounds checks.** Every framebuffer access is checked against the
  viewed length, so an undersized buffer degrades to a no-op (Rust would panic).
- **Header-only.** `#include "display.hpp"` and go.

## Usage

```cpp
#include "display.hpp"
using namespace ca::display;

auto cfg = DisplayConfig::default_config();       // 80x25
std::vector<std::uint8_t> mem(cfg.columns * cfg.rows * BYTES_PER_CELL);
DisplayDriver d(cfg, mem);
d.puts("Hello World");
auto snap = d.snapshot();                          // snap.lines[0] == "Hello World"
```

## Building

```sh
sh BUILD           # POSIX: g++ and/or clang++ via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
