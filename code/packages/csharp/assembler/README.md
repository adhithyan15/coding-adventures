# CodingAdventures.Assembler.CSharp

ARM assembly parser and 32-bit instruction encoder for .NET.

The package supports a compact ARM subset: `MOV`, `ADD`, `SUB`, `AND`, `ORR`,
`EOR`, `RSB`, `CMP`, `LDR`, `STR`, `NOP`, labels, and line comments.

```csharp
using CodingAdventures.Assembler;

var assembler = new Assembler();
uint[] words = assembler.Assemble("""
MOV R0, #42
ADD R2, R0, R1
STR R2, [R3]
""");
```
