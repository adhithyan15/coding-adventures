# json_value

Convert JSON parser ASTs into typed Elixir representations and back.

## What It Does

This package bridges the gap between the generic `ASTNode` tree produced by
`json_parser` and meaningful typed data. It provides:

1. **Tagged tuples** for all six JSON types (object, array, string, number,
   boolean, null)
2. **Conversion to/from native Elixir types** (maps, lists, strings, numbers,
   booleans, nil)
3. **End-to-end parsing** from JSON text to typed values or native types

## Where It Fits

```
JSON text → json_lexer → json_parser → json_value → native Elixir types
                                          ↑ THIS PACKAGE
```

## Usage

```elixir
# Parse JSON text into typed values
{:ok, value} = CodingAdventures.JsonValue.parse(~s({"name": "Alice", "age": 30}))
# => {:ok, {:object, [{"name", {:string, "Alice"}}, {"age", {:number, 30}}]}}

# Convert to native Elixir types
native = CodingAdventures.JsonValue.to_native(value)
# => %{"name" => "Alice", "age" => 30}

# Or parse directly to native types
{:ok, native} = CodingAdventures.JsonValue.parse_native(~s({"name": "Alice"}))
# => {:ok, %{"name" => "Alice"}}

# Convert native types to JSON values
{:ok, json_val} = CodingAdventures.JsonValue.from_native(%{"key" => "value"})
# => {:ok, {:object, [{"key", {:string, "value"}}]}}
```

## JSON Value Types

```
JSON Type    Elixir Representation       Native Elixir Type
---------    ----------------------       ------------------
Object       {:object, [{key, val}]}      map (%{})
Array        {:array, [json_value]}       list ([])
String       {:string, binary}            binary/string
Number       {:number, integer | float}   integer or float
Boolean      {:boolean, boolean}          true/false
Null         :null                        nil
```

## Dependencies

- `json_parser` (direct)
- `json_lexer`, `lexer`, `parser`, `grammar_tools`, `directed_graph` (transitive)
