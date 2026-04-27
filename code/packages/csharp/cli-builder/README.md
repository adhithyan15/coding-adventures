# cli-builder

Declarative CLI parsing for C#. Write a JSON spec describing commands, flags,
and positional arguments, and this package handles loading, validation,
parsing, and help generation.

## Layer 3

This package is part of Layer 3 of the coding-adventures computing stack.

## What It Includes

- `SpecLoader` for validating and normalizing CLI specs
- `Parser` for routing subcommands and parsing argv
- `TokenClassifier`, `PositionalResolver`, `FlagValidator`, and `HelpGenerator`
- Structured `ParseErrors` and `SpecError` types instead of ad-hoc exceptions

## Example

```csharp
using CodingAdventures.CliBuilder;

var parser = new Parser("./my-tool.json", ["my-tool", "--verbose", "input.txt"]);
var result = parser.Parse();
```

## Development

```bash
bash BUILD
```
