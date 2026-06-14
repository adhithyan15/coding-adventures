# CodingAdventures.Brainfuck.CSharp

Standalone Brainfuck translator and interpreter for .NET.

The package ignores non-command characters as comments, validates bracket
matching during translation, executes on a classic 30,000-cell byte tape, and
sets input EOF reads to zero.

```csharp
using CodingAdventures.Brainfuck;

BrainfuckResult result = Brainfuck.Execute("+++++++++[>++++++++<-]>.");
string output = result.Output; // "H"

BrainfuckInstruction[] program = Brainfuck.Translate("++[>+<-]");
BrainfuckResult moved = Brainfuck.Execute(program);
```
