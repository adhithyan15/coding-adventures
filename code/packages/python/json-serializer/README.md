# json-serializer

Serialize `JsonValue` trees or native Python types to compact or pretty-printed JSON text.

## Where It Fits

```
JSON text  -->  tokens  -->  AST  -->  JsonValue  -->  JSON text
            json-lexer   json-parser   json-value   **json-serializer**
```

This is the **output** stage of the JSON pipeline. It takes structured data and produces valid JSON text per RFC 8259.

## Installation

```bash
pip install -e ../json-value  # dependency
pip install -e ".[dev]"       # this package + dev tools
```

## Usage

### Compact JSON (for APIs, storage, wire)

```python
from json_serializer import stringify

stringify({"name": "Alice", "age": 30})
# '{"name":"Alice","age":30}'

stringify([1, "two", True, None])
# '[1,"two",true,null]'
```

### Pretty JSON (for humans, config files)

```python
from json_serializer import stringify_pretty, SerializerConfig

stringify_pretty({"name": "Alice", "age": 30})
# '{\n  "name": "Alice",\n  "age": 30\n}'

config = SerializerConfig(indent_size=4, sort_keys=True)
stringify_pretty({"b": 2, "a": 1}, config)
# '{\n    "a": 1,\n    "b": 2\n}'
```

### Working with JsonValue directly

```python
from json_value import JsonObject, JsonString, JsonNumber
from json_serializer import serialize, serialize_pretty

obj = JsonObject({"key": JsonString("value")})
serialize(obj)        # '{"key":"value"}'
serialize_pretty(obj) # '{\n  "key": "value"\n}'
```

## Configuration

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `indent_size` | int | 2 | Characters per indentation level |
| `indent_char` | str | `' '` | Space or tab |
| `sort_keys` | bool | False | Alphabetize object keys |
| `trailing_newline` | bool | False | Append `\n` at end of output |

## Dependencies

- `json-value` (sibling package)

## Testing

```bash
python -m pytest tests/ -v --cov=json_serializer --cov-report=term-missing
```
