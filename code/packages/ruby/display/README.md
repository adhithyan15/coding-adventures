# Display --- VGA Text-Mode Framebuffer

A Ruby implementation of a VGA text-mode framebuffer display driver, simulating the classic 80x25 text mode that every x86 PC boots into.

## What It Does

The display driver provides a memory-mapped text framebuffer where each cell is 2 bytes: one for the ASCII character and one for the color attribute. Writing to the framebuffer changes what appears on screen.

This is the final visible output layer of the entire computing stack.

## How It Fits in the Stack

```
User Program
  -> sys_write(1, "Hello World\n", 12)
    -> OS Kernel (S04)
      -> display.put_char(byte) for each character
        -> Display Driver (S05) <-- this package
          -> write to framebuffer memory
```

## Usage

```ruby
require "coding_adventures_display"

config = CodingAdventures::Display::DisplayConfig.new
memory = Array.new(config.columns * config.rows * 2, 0)
driver = CodingAdventures::Display::DisplayDriver.new(config, memory)

driver.puts_str("Hello World\n")
driver.puts_str("This is VGA text mode!")

snap = driver.snapshot
puts snap.lines[0]           # "Hello World"
puts snap.contains?("Hello") # true

# Write colored text at a specific position
attr = CodingAdventures::Display.make_attribute(
  CodingAdventures::Display::COLOR_WHITE,
  CodingAdventures::Display::COLOR_BLUE
)
driver.put_char_at(5, 10, 0x58, attr)
```

## API Summary

- `DisplayDriver.new(config, memory)` --- create a driver backed by a memory array
- `put_char(ch)` --- write a character at the cursor, advance cursor
- `put_char_at(row, col, ch, attr)` --- write at a specific position with custom color
- `puts_str(s)` --- write a string character by character
- `clear` --- reset all cells to space + default attribute
- `scroll` --- shift all rows up by one
- `set_cursor(row, col)` / `get_cursor` --- cursor management
- `get_cell(row, col)` --- read a cell's character and attribute
- `snapshot` --- get a read-friendly view of the display

## Running Tests

```bash
bundle install
bundle exec rake test
```
