# cli-builder

Declarative CLI parsing for F# with a native implementation for spec loading,
flag validation, help generation, and subcommand routing.

## Layer 3

This package is part of Layer 3 of the coding-adventures computing stack.

## What It Includes

- Native F# models for CLI specs, flags, arguments, commands, and parser results
- `SpecLoader` support for file-backed and in-memory JSON specs
- `TokenClassifier`, `PositionalResolver`, `FlagValidator`, and `HelpGenerator`
- `Parser` support for GNU, POSIX, subcommand-first, and traditional parsing modes

## Example

```fsharp
open CodingAdventures.CliBuilder.FSharp

let parser = Parser("./my-tool.json", [ "my-tool"; "--verbose"; "input.txt" ])
let result = parser.Parse()
```

## Development

```bash
bash BUILD
```
