# cli-builder

Declarative CLI parsing for F#. This initial port exposes an F#-friendly surface
over the shared .NET CLI-builder engine so F# consumers can load specs, parse
argv, and generate help without waiting for a full second implementation to
settle.

## Layer 3

This package is part of Layer 3 of the coding-adventures computing stack.

## What It Includes

- `SpecLoader` and `Parser` wrappers that accept F# sequences naturally
- Access to the shared CLI-builder models, results, and error types
- F# entry points for `TokenClassifier`, `PositionalResolver`, `FlagValidator`, and `HelpGenerator`
- A behavior-aligned path for F# while the native translation catches up

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
