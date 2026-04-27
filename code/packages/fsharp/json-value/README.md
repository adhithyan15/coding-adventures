# json-value

Typed JSON value representation for F#. This package turns JSON text or native
.NET values into a small, explicit tree of JSON node types.

## Layer 1

This package is part of Layer 1 of the coding-adventures computing stack.

## What It Includes

- `JsonValue.Parse` and `JsonValue.ParseNative` for reading JSON text
- `JsonValue.FromNative` and `JsonValue.ToNative` for bridging .NET values
- Explicit node types for objects, arrays, strings, numbers, booleans, and null
- Literate comments around number fidelity and native conversion rules

## Example

```fsharp
open CodingAdventures.JsonValue.FSharp

let value = JsonValue.Parse("""{"name":"Alice","age":30}""")
let native = value.ToNative()
```

## Development

```bash
bash BUILD
```
