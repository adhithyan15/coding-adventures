# json-serializer

Compact and pretty JSON serialization for C#. This package turns `JsonValue`
trees or native .NET values into valid JSON text.

## Layer 1

This package is part of Layer 1 of the coding-adventures computing stack.

## What It Includes

- `JsonSerializer.Serialize` for compact JSON output
- `JsonSerializer.SerializePretty` for indented JSON output
- `JsonSerializer.Stringify` helpers that accept native .NET values directly
- Optional key sorting and trailing newlines for file-friendly output

## Example

```csharp
using CodingAdventures.JsonSerializer;

var text = JsonSerializer.StringifyPretty(new { Name = "Alice", Age = 30 });
Console.WriteLine(text);
```

## Development

```bash
bash BUILD
```
