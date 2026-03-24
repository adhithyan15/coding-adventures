# json_serializer

Serializes JsonValue objects and native Ruby types into compact or pretty-printed JSON text.

## Where It Fits

```
JsonValue tree (from json_value)
  |  (serialize / serialize_pretty)
  v
JSON text string

Ruby Hash/Array/etc. (native types)
  |  (stringify / stringify_pretty)
  v
JSON text string
```

## Usage

```ruby
require "coding_adventures_json_serializer"

JS = CodingAdventures::JsonSerializer
JV = CodingAdventures::JsonValue

# Serialize JsonValue to compact JSON
value = JV::Object.new(pairs: { "name" => JV::String.new(value: "Alice") })
JS.serialize(value)
# => '{"name":"Alice"}'

# Serialize JsonValue to pretty JSON
JS.serialize_pretty(value)
# => "{\n  \"name\": \"Alice\"\n}"

# Stringify native Ruby types to compact JSON
JS.stringify({"name" => "Alice", "age" => 30})
# => '{"name":"Alice","age":30}'

# Stringify with pretty-printing and custom config
config = JS::SerializerConfig.new(indent_size: 4, sort_keys: true)
JS.stringify_pretty({"b" => 2, "a" => 1}, config: config)
# => "{\n    \"a\": 1,\n    \"b\": 2\n}"
```

## Configuration

| Option | Default | Description |
|--------|---------|-------------|
| `indent_size` | 2 | Spaces (or tabs) per indent level |
| `indent_char` | `" "` | Character for indentation (`" "` or `"\t"`) |
| `sort_keys` | false | Sort object keys alphabetically |
| `trailing_newline` | false | Add `\n` at end of output |

## Dependencies

- `coding_adventures_json_value` (and its transitive deps)
