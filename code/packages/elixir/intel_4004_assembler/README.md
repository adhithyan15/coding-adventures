# Intel 4004 Assembler (Elixir)

An Intel 4004 assembler for educational emulator and compiler backends.

The package parses assembly text, resolves labels in a first pass, then emits Intel 4004 machine-code bytes in a second pass.

## Example

```elixir
alias CodingAdventures.Intel4004Assembler

{:ok, binary} =
  Intel4004Assembler.assemble("""
  ORG 0x000
  _start:
    LDM 5
    XCH R2
    HLT
  """)

binary == <<0xD5, 0xB2, 0x01>>
```
