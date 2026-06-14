# CodingAdventures.Brainfuck.FSharp

Standalone Brainfuck translator and interpreter for .NET.

The package ignores non-command characters as comments, validates bracket
matching during translation, executes on a classic 30,000-cell byte tape, and
sets input EOF reads to zero.

```fsharp
open CodingAdventures.Brainfuck.FSharp

let result = Brainfuck.execute "+++++++++[>++++++++<-]>."
let output = result.Output // "H"

let program = Brainfuck.translate "++[>+<-]"
let moved = Brainfuck.executeProgram program [||]
```
