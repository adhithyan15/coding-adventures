# Display -- VGA Text-Mode Framebuffer

A Rust implementation of a VGA text-mode framebuffer display driver, simulating the classic 80x25 text mode that every x86 PC boots into.

## What It Does

The display driver provides a memory-mapped text framebuffer where each cell is 2 bytes: one for the ASCII character and one for the color attribute. Writing to the framebuffer changes what appears on screen.

This is the final visible output layer of the entire computing stack.

## How It Fits in the Stack

```
User Program
  -> sys_write(1, "Hello World\n", 12)
    -> OS Kernel (S04)
      -> display.put_char(byte) for each character
        -> Display Driver (S05) <-- this crate
          -> write to framebuffer memory
```

## Usage

```rust
use display::{DisplayDriver, DisplayConfig, BYTES_PER_CELL, make_attribute};
use display::{COLOR_WHITE, COLOR_BLUE};

let config = DisplayConfig::default();
let mut memory = vec![0u8; config.columns * config.rows * BYTES_PER_CELL];
let mut driver = DisplayDriver::new(config, &mut memory);

driver.puts("Hello World\n");
driver.puts("This is VGA text mode!");

let snap = driver.snapshot();
assert_eq!(snap.lines[0], "Hello World");
assert!(snap.contains("Hello"));

// Write colored text at a specific position
let attr = make_attribute(COLOR_WHITE, COLOR_BLUE);
driver.put_char_at(5, 10, b'X', attr);
```

## API Summary

- `DisplayDriver::new(config, memory)` -- create a driver backed by a memory slice
- `put_char(ch)` -- write a character at the cursor, advance cursor
- `put_char_at(row, col, ch, attr)` -- write at a specific position with custom color
- `puts(s)` -- write a string character by character
- `clear()` -- reset all cells to space + default attribute
- `scroll()` -- shift all rows up by one
- `set_cursor(row, col)` / `get_cursor()` -- cursor management
- `get_cell(row, col)` -- read a cell's character and attribute
- `snapshot()` -- get a read-friendly view of the display

## Running Tests

```bash
cargo test -p display
```
