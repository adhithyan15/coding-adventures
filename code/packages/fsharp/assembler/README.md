# CodingAdventures.Assembler.FSharp

ARM assembly parser and 32-bit instruction encoder for .NET.

The package supports a compact ARM subset: `MOV`, `ADD`, `SUB`, `AND`, `ORR`,
`EOR`, `RSB`, `CMP`, `LDR`, `STR`, `NOP`, labels, and line comments.

```fsharp
open CodingAdventures.Assembler.FSharp

let assembler = Assembler()
let words =
    assembler.Assemble(
        "MOV R0, #42\n" +
        "ADD R2, R0, R1\n" +
        "STR R2, [R3]")
```
