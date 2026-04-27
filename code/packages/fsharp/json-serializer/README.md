# json-serializer

Compact and pretty JSON serialization for F#. This package turns `JsonValue`
trees or native .NET values into valid JSON text.

## Layer 1

This package is part of Layer 1 of the coding-adventures computing stack.

## What It Includes

- `JsonSerializer.Serialize` for compact JSON output
- `JsonSerializer.SerializePretty` for indented JSON output
- `JsonSerializer.Stringify` helpers that accept native .NET values directly
- Optional key sorting and trailing newlines for file-friendly output

## Example

```fsharp
open CodingAdventures.JsonSerializer.FSharp

let text = JsonSerializer.StringifyPretty(box {| Name = "Alice"; Age = 30 |})
printfn "%s" text
```

## Development

```bash
bash BUILD
```
