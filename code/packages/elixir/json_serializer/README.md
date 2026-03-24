# json_serializer

Serialize JSON values or native Elixir types to JSON text.

## What It Does

This package converts typed JSON values (from `json_value`) or native Elixir
types into JSON text. Two modes are supported:

1. **Compact** — minimal whitespace, smallest output size
2. **Pretty** — human-readable with configurable indentation

## Where It Fits

```
JSON values / native types → json_serializer → JSON text
                                ↑ THIS PACKAGE
```

## Usage

```elixir
alias CodingAdventures.JsonSerializer

# Serialize typed JSON values to compact text
{:ok, text} = JsonSerializer.serialize({:object, [{"name", {:string, "Alice"}}]})
# => {:ok, ~s({"name":"Alice"})}

# Pretty-print with default options (2-space indent)
{:ok, text} = JsonSerializer.serialize_pretty({:object, [{"name", {:string, "Alice"}}]})
# => {:ok, "{\n  \"name\": \"Alice\"\n}"}

# Serialize native Elixir types directly
{:ok, text} = JsonSerializer.stringify(%{"name" => "Alice", "age" => 30})
# => {:ok, ~s({"name":"Alice","age":30})}

# Pretty-print native types with custom options
{:ok, text} = JsonSerializer.stringify_pretty(
  %{"b" => 2, "a" => 1},
  sort_keys: true, indent_size: 4
)
```

## Configuration Options

| Option             | Default | Description                          |
|--------------------|---------|--------------------------------------|
| `indent_size`      | 2       | Spaces (or tabs) per indent level    |
| `indent_char`      | `" "`   | Character for indentation            |
| `sort_keys`        | false   | Sort object keys alphabetically      |
| `trailing_newline` | false   | Add `\n` at end of output            |

## String Escaping (RFC 8259)

All required JSON escape sequences are handled: `\"`, `\\`, `\b`, `\f`,
`\n`, `\r`, `\t`, and `\uXXXX` for control characters U+0000-U+001F.

## Dependencies

- `json_value` (direct)
- `json_parser`, `json_lexer`, `lexer`, `parser`, `grammar_tools`, `directed_graph` (transitive)
