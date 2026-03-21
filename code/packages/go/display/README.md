# Display — VGA Text-Mode Framebuffer

A Go implementation of a VGA text-mode framebuffer display driver, simulating the classic 80x25 text mode that every x86 PC boots into.

## What It Does

The display driver provides a memory-mapped text framebuffer where each cell is 2 bytes: one for the ASCII character and one for the color attribute. Writing to the framebuffer changes what appears on screen.

This is the final visible output layer of the entire computing stack. When "Hello World" appears on the display, it means every layer worked: logic gates, ALU, pipeline, cache, bootloader, kernel, and display driver.

## How It Fits in the Stack

```
User Program
  -> sys_write(1, "Hello World\n", 12)
    -> OS Kernel (S04)
      -> display.PutChar(byte) for each character
        -> Display Driver (S05) <-- this package
          -> write to framebuffer memory
```

## Usage

```go
package main

import (
    "fmt"
    "github.com/adhithyan15/coding-adventures/code/packages/go/display"
)

func main() {
    config := display.DefaultDisplayConfig()
    memory := make([]byte, config.Columns*config.Rows*display.BytesPerCell)
    driver := display.NewDisplayDriver(config, memory)

    driver.Puts("Hello World\n")
    driver.Puts("This is VGA text mode!")

    snap := driver.Snapshot()
    fmt.Println(snap.Lines[0])           // "Hello World"
    fmt.Println(snap.Contains("Hello"))  // true

    // Write colored text at a specific position
    attr := display.MakeAttribute(display.ColorWhite, display.ColorBlue)
    driver.PutCharAt(5, 10, 'X', attr)
}
```

## API Summary

- `NewDisplayDriver(config, memory)` — create a driver backed by a memory region
- `PutChar(ch)` — write a character at the cursor, advance cursor
- `PutCharAt(row, col, ch, attr)` — write at a specific position with custom color
- `Puts(s)` — write a string character by character
- `Clear()` — reset all cells to space + default attribute
- `Scroll()` — shift all rows up by one
- `SetCursor(row, col)` / `GetCursor()` — cursor management
- `GetCell(row, col)` — read a cell's character and attribute
- `Snapshot()` — get a read-friendly view of the display

## Running Tests

```bash
go test ./... -v -cover
```
