# display (C)

A VGA text-mode framebuffer simulation, in **pure ISO C17**. A faithful port of
the Rust [`display`](../../rust/display) crate.

## What it does

Simulates the classic 80x25 VGA text-mode framebuffer — a grid of cells, each
2 bytes (byte 0 = character, byte 1 = colour attribute). The `DisplayDriver`
tracks a cursor, interprets a few control characters (`\n`, `\r`, `\t`,
backspace), wraps at the right edge, and scrolls when output runs past the
bottom.

## API

- `display_make_attribute(fg, bg)` — pack a foreground/background colour into an
  attribute byte; the `DisplayColor` enum gives the VGA palette.
- `display_init` / `display_wrap` — start a driver over caller-owned memory
  (init clears; wrap preserves existing content).
- `display_put_char`, `display_put_char_at`, `display_puts` — writing.
- `display_clear`, `display_scroll`, `display_set_cursor`, `display_get_cursor`,
  `display_get_cell` — screen and cursor management.
- `display_snapshot` (+ `_free`, `_contains`, `_line_at`, `_to_padded`) — a
  frozen text view of the display.

## Design notes

- **Caller-owned framebuffer.** You supply a buffer of at least
  `columns * rows * DISPLAY_BYTES_PER_CELL` bytes (mirroring the Rust borrowed
  `&mut [u8]`); the driver only views it and never frees it. Only `snapshot`
  allocates — release it with `display_snapshot_free`.
- **Defensive bounds checks.** Every framebuffer access is checked against the
  borrowed length, so an undersized buffer degrades to a no-op instead of a
  buffer overflow (Rust would panic here).
- **Faithful divergences.** Rust `Vec<String>` snapshot lines → a malloc'd
  `char **` of trimmed, NUL-terminated lines. Snapshot's `to_padded` size is
  computed with `size_t`-overflow guards.

## Usage

```c
#include "display.h"

DisplayConfig cfg = display_config_default();          /* 80x25 */
uint8_t *mem = malloc(cfg.columns * cfg.rows * DISPLAY_BYTES_PER_CELL);
DisplayDriver d;
display_init(&d, cfg, mem, cfg.columns * cfg.rows * DISPLAY_BYTES_PER_CELL);
display_puts(&d, "Hello World");
/* display_get_cell(&d, 0, 0).character == 'H' */
free(mem);
```

## Building

```sh
sh BUILD           # POSIX: GCC and/or Clang via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
