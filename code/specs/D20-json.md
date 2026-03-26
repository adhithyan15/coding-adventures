# D20 — JSON Ecosystem

## Overview

The JSON pipeline in this monorepo has four layers. The first two (lexer and
parser) already exist. This spec defines the final two: **json-value** and
**json-serializer**.

```
JSON Pipeline (complete):

  '{"name": "Alice", "age": 30}'     <-- JSON text (string)
           |
           v
  json-lexer (tokenize)               <-- ALREADY EXISTS
           |
           v
  [LBRACE, STRING("name"), COLON,    <-- Token stream
   STRING("Alice"), COMMA,
   STRING("age"), COLON,
   NUMBER("30"), RBRACE, EOF]
           |
           v
  json-parser (parse)                  <-- ALREADY EXISTS
           |
           v
  ASTNode(rule="value", children=[    <-- Generic AST
    ASTNode(rule="object", ...)
  ])
           |
           v
  json-value (THIS SPEC, Package 1)    <-- AST --> JsonValue types
           |
           v
  JsonValue.Object({                   <-- Typed JSON representation
    "name": JsonValue.String("Alice"),
    "age": JsonValue.Number(30)
  })
           |
           +---> to_native() --> {"name": "Alice", "age": 30}  <-- native dict/Hash/map
           |
           v
  json-serializer (THIS SPEC, Package 2)  <-- JsonValue/native --> JSON text
           |
           v
  '{"name":"Alice","age":30}'          <-- Compact JSON text
  OR
  '{                                   <-- Pretty-printed JSON text
    "name": "Alice",
    "age": 30
  }'
```

**Why two packages instead of one?**

The user wants the Unix philosophy: each package does one thing well.

1. **json-value** answers: "What does this JSON *mean*?" It provides a typed
   intermediate representation (JsonValue) that preserves all JSON semantics.
   Users who want OOP access use JsonValue directly. Users who want native
   types (dict, Hash, map) call `to_native()`.

2. **json-serializer** answers: "How do I write JSON text?" It takes JsonValue
   or native types and produces compact or pretty-printed JSON text with
   configurable formatting.

Separating them means a user can parse JSON into typed values without pulling
in serialization code, or serialize native types without caring about the AST.

**Why build our own instead of using stdlib?**

1. **Zero external dependencies.** The coding-adventures monorepo is dependency-free.
2. **We already have the hard parts.** The lexer and parser exist. json-value
   is just a tree walk. json-serializer is just recursive string building.
3. **We become our own consumers.** The Actor package (D19) needs JSON for
   envelope serialization. Using our own validates the full pipeline.
4. **Learning opportunity.** The gap between "I can parse JSON" and "I can use
   parsed JSON in my program" is where most engineers never look.

---

## Where It Fits

```
Application Code (Actor D19, Chief of Staff D18, etc.)
|   json_serializer.stringify({"key": "val"})  --> '{"key":"val"}'
|   json_value.parse("...") --> JsonValue       --> to_native() --> dict
v
json-serializer  <-- Package 2 (THIS SPEC)
|   serialize(JsonValue) --> text
|   serialize_pretty(JsonValue, config) --> text
|
json-value       <-- Package 1 (THIS SPEC)
|   from_ast(ASTNode) --> JsonValue
|   to_native(JsonValue) --> dict/Hash/map
|   from_native(native) --> JsonValue
v
json-parser      <-- ALREADY EXISTS
|   parse_json(text) --> ASTNode tree
v
json-lexer       <-- ALREADY EXISTS
|   tokenize_json(text) --> Token stream
v
Grammar Engine (lexer + parser base packages)
|   GrammarLexer, GrammarParser
v
Grammar Files (code/grammars/)
    json.tokens, json.grammar
```

**Depends on:** json-parser (which depends on json-lexer -> lexer -> grammar-tools)

**Used by:**
- Actor (D19) -- envelope serialization/deserialization
- Chief of Staff (D18) -- agent manifests, channel metadata
- Any package that needs to read or write JSON

---

## Package 1: json-value

### Purpose

Convert a json-parser AST into a **typed** JSON representation. This is the
bridge between the generic `ASTNode` tree and meaningful data.

Two representations are provided:

1. **JsonValue** (OOP/typed) -- a discriminated union / enum / sealed class
   that preserves the JSON type information. Users who want pattern matching,
   type-safe access, or custom traversal use this.

2. **Native types** (dynamic) -- the language's built-in dict/Hash/map, list,
   string, number, boolean, null types. Users who just want to read JSON data
   use this.

### JsonValue Type Hierarchy

JSON has exactly 6 value types. JsonValue mirrors them:

```
JsonValue
  |-- JsonObject(pairs: OrderedMap<String, JsonValue>)
  |-- JsonArray(elements: List<JsonValue>)
  |-- JsonString(value: String)
  |-- JsonNumber(value: Number)    # int or float, language-dependent
  |-- JsonBool(value: Boolean)
  |-- JsonNull()
```

**Why OrderedMap for objects?** RFC 8259 says JSON objects are "unordered
collections of name/value pairs" but practically, insertion order matters
for human readability, round-trip fidelity, and deterministic output. We
preserve insertion order.

### Language-Specific JsonValue Representations

```
Language      JsonValue Implementation
----------   ------------------------------------------------
Python        @dataclass subclasses of a JsonValue base class
              JsonObject, JsonArray, JsonString, JsonNumber,
              JsonBool, JsonNull
Go            JsonValue interface with concrete structs
              JsonObject, JsonArray, JsonString, JsonNumber,
              JsonBool, JsonNull implementing JsonValue
Rust          enum JsonValue { Object, Array, Str, Number,
              Bool, Null } -- the standard Rust approach
TypeScript    Discriminated union with 'type' field
              { type: 'object', value: Map<string, JsonValue> }
Ruby          Classes under JsonValue module
              JsonValue::Object, JsonValue::Array, etc.
Elixir        Tagged tuples or structs
              {:object, ordered_map}, {:array, list}, etc.
```

### JsonValue <-> Native Type Mapping

```
JsonValue         Python    Go               Ruby       TypeScript   Rust                         Elixir
---------         ------    --               ----       ----------   ----                         ------
JsonObject        dict      map[string]any   Hash       object {}    HashMap<String, JsonValue>   map
JsonArray         list      []any            Array      any[]        Vec<JsonValue>               list
JsonString        str       string           String     string       String                       binary
JsonNumber(int)   int       float64 or int   Integer    number       i64 or f64                   integer
JsonNumber(float) float     float64          Float      number       f64                          float
JsonBool          bool      bool             TrueClass  boolean      bool                         boolean
JsonNull          None      nil              nil        null         Option::None / unit          nil
```

**Note on numbers:** JSON does not distinguish integer and float. However:
- `42` has no decimal point or exponent --> store as integer type
- `3.14` has a decimal point --> store as float type
- `1e10` has an exponent --> store as float type

This matches the behavior of Python's `json.loads`, Ruby's `JSON.parse`, etc.

### Public API

```python
# === Construction ===

class JsonValue:
    """Base class for all JSON value types."""
    pass

class JsonObject(JsonValue):
    """JSON object -- ordered collection of key-value pairs."""
    pairs: dict[str, JsonValue]  # insertion-ordered (Python 3.7+)

class JsonArray(JsonValue):
    """JSON array -- ordered sequence of values."""
    elements: list[JsonValue]

class JsonString(JsonValue):
    """JSON string value."""
    value: str

class JsonNumber(JsonValue):
    """JSON number -- integer or floating-point."""
    value: int | float

class JsonBool(JsonValue):
    """JSON boolean."""
    value: bool

class JsonNull(JsonValue):
    """JSON null."""
    pass


# === Conversion: AST --> JsonValue ===

def from_ast(ast: ASTNode) -> JsonValue:
    """Convert a json-parser AST node to a JsonValue.

    This is a recursive tree walk that dispatches on rule_name:
    - "value"  -> unwrap and recurse into the meaningful child
    - "object" -> collect pairs into JsonObject
    - "pair"   -> extract key (STRING token) and value (recursive)
    - "array"  -> collect elements into JsonArray
    - Token(STRING)  -> JsonString (value already unescaped by lexer)
    - Token(NUMBER)  -> JsonNumber (int if no decimal/exponent, float otherwise)
    - Token(TRUE)    -> JsonBool(True)
    - Token(FALSE)   -> JsonBool(False)
    - Token(NULL)    -> JsonNull()
    """


# === Conversion: JsonValue --> native types ===

def to_native(value: JsonValue) -> dict | list | str | int | float | bool | None:
    """Convert a JsonValue to native language types.

    JsonObject  --> dict (Python) / map (Go) / Hash (Ruby)
    JsonArray   --> list / slice / Array
    JsonString  --> str / string / String
    JsonNumber  --> int or float
    JsonBool    --> bool / boolean
    JsonNull    --> None / nil / null

    The conversion is recursive -- nested JsonValues are also converted.
    """


# === Conversion: native types --> JsonValue ===

def from_native(value: dict | list | str | int | float | bool | None) -> JsonValue:
    """Convert native language types to a JsonValue.

    dict   --> JsonObject (keys must be strings)
    list   --> JsonArray
    str    --> JsonString
    int    --> JsonNumber
    float  --> JsonNumber
    bool   --> JsonBool
    None   --> JsonNull

    Raises JsonValueError if the value contains non-JSON-compatible types
    (functions, custom objects, etc.).
    """


# === Convenience: text --> JsonValue (via lexer + parser + from_ast) ===

def parse(text: str) -> JsonValue:
    """Parse JSON text into a JsonValue.

    Internally calls: json_parser.parse_json(text) --> AST --> from_ast(ast)

    Raises JsonValueError if the text is not valid JSON.
    """


# === Convenience: text --> native types ===

def parse_native(text: str) -> dict | list | str | int | float | bool | None:
    """Parse JSON text directly into native language types.

    Equivalent to: to_native(parse(text))

    This is the most common use case -- "give me a dict from this JSON string."
    """
```

### Algorithm: from_ast(node)

```
from_ast(node)
==============

This is the core tree walk. It converts the generic ASTNode tree
produced by json-parser into a typed JsonValue tree.

1. If node is a Token (leaf):
   match node.type:
     "STRING"  -> return JsonString(node.value)
                  (node.value is already unescaped by the lexer --
                   "\n" is a real newline, not backslash-n)
     "NUMBER"  -> if value contains '.' or 'e' or 'E':
                    return JsonNumber(parse_float(node.value))
                  else:
                    return JsonNumber(parse_int(node.value))
     "TRUE"    -> return JsonBool(true)
     "FALSE"   -> return JsonBool(false)
     "NULL"    -> return JsonNull()
     _         -> skip (structural tokens: LBRACE, RBRACE, etc.)

2. If node is an ASTNode:
   match node.rule_name:

     "value" ->
       The value rule wraps exactly one meaningful child.
       Find the first child that is either:
         a) An ASTNode (rule_name is "object" or "array")
         b) A Token with type STRING, NUMBER, TRUE, FALSE, or NULL
       Return from_ast(that child).

     "object" ->
       Create an empty OrderedMap.
       For each child that is an ASTNode with rule_name="pair":
         key_str, val = walk_pair(pair_node)
         map[key_str] = val
       Return JsonObject(map).

     "pair" ->
       Find the STRING token -> key
       Find the ASTNode with rule_name="value" -> recursive from_ast -> val
       Return (key, val)

     "array" ->
       Create an empty list.
       For each child that is an ASTNode with rule_name="value":
         element = from_ast(value_node)
         list.append(element)
       Return JsonArray(list).

       NOTE: Handle edge case where array elements might be
       direct Token children rather than wrapped in ASTNode("value").
```

---

## Package 2: json-serializer

### Purpose

Convert JsonValue or native types into JSON text. Supports two modes:

1. **Compact** -- minimal whitespace, smallest output size
2. **Pretty** -- human-readable with configurable indentation

### Configuration

```python
@dataclass
class SerializerConfig:
    """Configuration for JSON pretty-printing.

    Attributes:
        indent_size: Number of spaces (or tabs) per indentation level.
                     Default: 2
        indent_char: Character to use for indentation. Must be ' ' or '\t'.
                     Default: ' ' (space)
        sort_keys:   Whether to sort object keys alphabetically.
                     Default: False (preserve insertion order)
        trailing_newline: Whether to add a newline at the end of output.
                          Default: False
    """
    indent_size: int = 2
    indent_char: str = " "
    sort_keys: bool = False
    trailing_newline: bool = False
```

### Public API

```python
# === Core API: JsonValue --> text ===

def serialize(value: JsonValue) -> str:
    """Serialize a JsonValue to compact JSON text.

    No unnecessary whitespace. Suitable for wire transmission.

    Example:
        >>> serialize(JsonObject({"name": JsonString("Alice"), "age": JsonNumber(30)}))
        '{"name":"Alice","age":30}'
    """


def serialize_pretty(value: JsonValue, config: SerializerConfig = None) -> str:
    """Serialize a JsonValue to pretty-printed JSON text.

    Uses the provided config, or defaults (2-space indent, no key sorting).

    Example:
        >>> serialize_pretty(JsonObject({"name": JsonString("Alice")}))
        '{\\n  "name": "Alice"\\n}'
    """


# === Convenience API: native types --> text ===

def stringify(value: dict | list | str | int | float | bool | None) -> str:
    """Convert native types to compact JSON text.

    Equivalent to: serialize(from_native(value))

    Example:
        >>> stringify({"name": "Alice", "age": 30})
        '{"name":"Alice","age":30}'
    """


def stringify_pretty(
    value: dict | list | str | int | float | bool | None,
    config: SerializerConfig = None,
) -> str:
    """Convert native types to pretty-printed JSON text.

    Equivalent to: serialize_pretty(from_native(value))
    """
```

### Algorithm: serialize(value)

```
serialize(value: JsonValue) -> str
==================================

Recursive dispatch on JsonValue variant:

1. JsonNull:
   return "null"

2. JsonBool(b):
   return "true" if b else "false"

3. JsonNumber(n):
   if n is integer:
     return str(n)     # e.g., 42 --> "42"
   if n is float:
     if n is Infinity or NaN:
       raise JsonSerializerError("Cannot serialize Infinity/NaN")
     return float_to_string(n)
     # e.g., 3.14 --> "3.14"
     # Note: avoid trailing zeros where possible, but "1.0" is fine

4. JsonString(s):
   return '"' + escape_json_string(s) + '"'

5. JsonArray(elements):
   if elements is empty:
     return "[]"
   parts = [serialize(elem) for elem in elements]
   return "[" + ",".join(parts) + "]"

6. JsonObject(pairs):
   if pairs is empty:
     return "{}"
   parts = [serialize_string(key) + ":" + serialize(val) for key, val in pairs]
   return "{" + ",".join(parts) + "}"


escape_json_string(s) -> str
============================

Per RFC 8259, these characters MUST be escaped:

Character        Escape     Reason
---------        ------     ------
" (quote)        \"         Delimiter
\ (backslash)    \\         Escape char
Backspace        \b         Control char (U+0008)
Form feed        \f         Control char (U+000C)
Newline          \n         Control char (U+000A)
Carriage return  \r         Control char (U+000D)
Tab              \t         Control char (U+0009)
U+0000-U+001F    \uXXXX    All control characters not covered above

Forward slash (/) is NOT escaped -- RFC 8259 allows but does not require it.
```

### Algorithm: serialize_pretty(value, config, depth)

```
serialize_pretty(value: JsonValue, config: SerializerConfig, depth: int = 0) -> str
===================================================================================

Like serialize() but with indentation and newlines.

indent = config.indent_char * config.indent_size
current_indent = indent * depth
next_indent = indent * (depth + 1)

1. JsonNull, JsonBool, JsonNumber, JsonString:
   Same as serialize() -- primitives have no internal structure to indent.

2. JsonArray(elements):
   if elements is empty:
     return "[]"
   lines = []
   for each element in elements:
     lines.append(next_indent + serialize_pretty(element, config, depth + 1))
   return "[\n" + ",\n".join(lines) + "\n" + current_indent + "]"

3. JsonObject(pairs):
   if pairs is empty:
     return "{}"
   keys = sort(pairs.keys()) if config.sort_keys else pairs.keys()
   lines = []
   for each key in keys:
     val_str = serialize_pretty(pairs[key], config, depth + 1)
     lines.append(next_indent + serialize_string(key) + ": " + val_str)
   return "{\n" + ",\n".join(lines) + "\n" + current_indent + "}"

4. If config.trailing_newline and depth == 0:
   append "\n" to result
```

---

## Dependencies

```
json-serializer
|   depends on --> json-value
|                   |   depends on --> json-parser
|                   |                   |   depends on --> json-lexer
|                   |                   |                   |   depends on --> lexer
|                   |                   |                   |   depends on --> grammar-tools
|                   |                   |   depends on --> parser
|                   |                   |                   |   depends on --> grammar-tools
|
|   used by --> Actor (D19)
|   used by --> Chief of Staff (D18)
|   used by --> any package needing JSON I/O
```

---

## Testing Strategy

### json-value Tests

#### Unit Tests -- from_ast()

1. **AST object to JsonObject**: parse `{}` --> AST --> from_ast --> JsonObject with empty pairs
2. **AST array to JsonArray**: parse `[]` --> AST --> from_ast --> JsonArray with empty elements
3. **AST string to JsonString**: parse `"hello"` --> AST --> from_ast --> JsonString("hello")
4. **AST integer to JsonNumber**: parse `42` --> AST --> from_ast --> JsonNumber(42) (integer)
5. **AST negative integer**: parse `-17` --> AST --> from_ast --> JsonNumber(-17)
6. **AST float to JsonNumber**: parse `3.14` --> AST --> from_ast --> JsonNumber(3.14) (float)
7. **AST exponent to JsonNumber**: parse `1e10` --> AST --> from_ast --> JsonNumber (float)
8. **AST true to JsonBool**: parse `true` --> AST --> from_ast --> JsonBool(true)
9. **AST false to JsonBool**: parse `false` --> AST --> from_ast --> JsonBool(false)
10. **AST null to JsonNull**: parse `null` --> AST --> from_ast --> JsonNull
11. **AST simple object**: parse `{"a": 1}` --> JsonObject with one pair
12. **AST multi-key object**: parse `{"a": 1, "b": 2}` --> JsonObject with two pairs
13. **AST simple array**: parse `[1, 2, 3]` --> JsonArray with three JsonNumber elements
14. **AST mixed array**: parse `[1, "two", true, null]` --> JsonArray with mixed types
15. **AST nested object**: parse `{"a": {"b": 1}}` --> JsonObject containing JsonObject
16. **AST nested array**: parse `[[1, 2], [3, 4]]` --> JsonArray containing JsonArrays
17. **AST complex nested**: parse `{"users": [{"name": "Alice"}]}` --> deep nesting
18. **AST string with escapes**: parse `"hello\nworld"` --> JsonString with actual newline
19. **AST empty string**: parse `""` --> JsonString("")
20. **AST zero**: parse `0` --> JsonNumber(0)

#### Unit Tests -- to_native()

21. **JsonObject to dict/Hash/map**: JsonObject({"a": JsonNumber(1)}) --> {"a": 1}
22. **JsonArray to list**: JsonArray([JsonNumber(1), JsonNumber(2)]) --> [1, 2]
23. **JsonString to str**: JsonString("hello") --> "hello"
24. **JsonNumber int to int**: JsonNumber(42) --> 42
25. **JsonNumber float to float**: JsonNumber(3.14) --> 3.14
26. **JsonBool to bool**: JsonBool(true) --> true
27. **JsonNull to None/nil**: JsonNull --> None/nil/null
28. **Nested to_native**: deeply nested JsonValue --> deeply nested native types

#### Unit Tests -- from_native()

29. **dict to JsonObject**: {"a": 1} --> JsonObject({"a": JsonNumber(1)})
30. **list to JsonArray**: [1, 2] --> JsonArray([JsonNumber(1), JsonNumber(2)])
31. **str to JsonString**: "hello" --> JsonString("hello")
32. **int to JsonNumber**: 42 --> JsonNumber(42)
33. **float to JsonNumber**: 3.14 --> JsonNumber(3.14)
34. **bool to JsonBool**: true --> JsonBool(true)
35. **None/nil to JsonNull**: None --> JsonNull
36. **Nested from_native**: deeply nested native --> deeply nested JsonValue
37. **Non-string key error**: {1: "val"} --> raise JsonValueError
38. **Non-JSON type error**: function/class --> raise JsonValueError

#### Unit Tests -- parse() and parse_native()

39. **parse returns JsonValue**: parse('{"a": 1}') is JsonObject
40. **parse_native returns native**: parse_native('{"a": 1}') == {"a": 1}
41. **parse invalid JSON**: parse('not json') --> raise JsonValueError
42. **parse_native invalid JSON**: parse_native('{') --> raise JsonValueError

#### Round-trip Tests

43. **Round-trip via JsonValue**: value --> from_native --> to_native --> value (match)
44. **Round-trip nested**: complex nested structure survives from_native --> to_native

### json-serializer Tests

#### Unit Tests -- serialize() (compact)

45. **Serialize JsonNull**: serialize(JsonNull) --> "null"
46. **Serialize JsonBool true**: serialize(JsonBool(true)) --> "true"
47. **Serialize JsonBool false**: serialize(JsonBool(false)) --> "false"
48. **Serialize JsonNumber int**: serialize(JsonNumber(42)) --> "42"
49. **Serialize JsonNumber negative**: serialize(JsonNumber(-5)) --> "-5"
50. **Serialize JsonNumber float**: serialize(JsonNumber(3.14)) --> "3.14"
51. **Serialize JsonString simple**: serialize(JsonString("hello")) --> '"hello"'
52. **Serialize JsonString escapes newline**: serialize(JsonString("a\nb")) --> '"a\\nb"'
53. **Serialize JsonString escapes quote**: serialize(JsonString('say "hi"')) --> '"say \\"hi\\""'
54. **Serialize JsonString escapes backslash**: serialize(JsonString("a\\b")) --> '"a\\\\b"'
55. **Serialize JsonString escapes tab**: serialize(JsonString("\t")) --> '"\\t"'
56. **Serialize JsonString control chars**: serialize(JsonString("\x00")) --> '"\\u0000"'
57. **Serialize empty object**: serialize(JsonObject({})) --> "{}"
58. **Serialize simple object**: serialize(JsonObject({"a": JsonNumber(1)})) --> '{"a":1}'
59. **Serialize empty array**: serialize(JsonArray([])) --> "[]"
60. **Serialize simple array**: serialize(JsonArray([JsonNumber(1)])) --> "[1]"
61. **Serialize nested**: object with arrays and objects inside
62. **Serialize Infinity error**: serialize(JsonNumber(Infinity)) --> raise error
63. **Serialize NaN error**: serialize(JsonNumber(NaN)) --> raise error

#### Unit Tests -- serialize_pretty()

64. **Pretty empty object**: serialize_pretty(JsonObject({})) --> "{}"
65. **Pretty simple object**: serialize_pretty(JsonObject({"a": JsonNumber(1)})) -->
    '{\n  "a": 1\n}'
66. **Pretty nested object**: verify indentation increases at each level
67. **Pretty array**: serialize_pretty(JsonArray([JsonNumber(1), JsonNumber(2)])) -->
    '[\n  1,\n  2\n]'
68. **Custom indent size**: config with indent_size=4 uses 4 spaces
69. **Tab indent**: config with indent_char='\t' uses tabs
70. **Sort keys**: config with sort_keys=True sorts keys alphabetically
71. **Trailing newline**: config with trailing_newline=True adds '\n' at end

#### Unit Tests -- stringify() and stringify_pretty()

72. **stringify dict**: stringify({"a": 1}) --> '{"a":1}'
73. **stringify list**: stringify([1, 2]) --> '[1,2]'
74. **stringify string**: stringify("hello") --> '"hello"'
75. **stringify int**: stringify(42) --> '42'
76. **stringify bool**: stringify(True) --> 'true'
77. **stringify None**: stringify(None) --> 'null'
78. **stringify_pretty**: stringify_pretty({"a": 1}) --> '{\n  "a": 1\n}'

#### Full Round-trip Tests (parse + serialize)

79. **parse then serialize**: parse_native('{"a":1}') --> stringify --> '{"a":1}'
80. **Complex round-trip**: nested structure survives parse_native --> stringify
81. **Escapes round-trip**: string with all escape chars survives round-trip
82. **Number round-trip**: integers and floats survive round-trip
83. **Empty containers round-trip**: {} and [] survive round-trip

### Coverage Target

Target 95%+ line coverage. Every JSON type, every escape sequence, every
error path.

---

## Trade-Offs

| Decision | Pro | Con |
|----------|-----|-----|
| Two packages vs one monolith | Composable, each does one thing | More packages to maintain |
| JsonValue intermediate type | Type-safe, pattern-matchable | Extra allocation vs direct-to-native |
| Preserve insertion order | Round-trip fidelity | Slightly more memory than unordered map |
| No streaming/SAX mode | Simpler implementation | Entire document must fit in memory |
| Integer/float distinction | Matches user expectations (42 != 42.0) | Extra logic in number handling |
| Config struct for pretty | Extensible without breaking API | More verbose than positional args |

---

## Future Extensions

- **Streaming parser**: SAX-style event-based parsing for huge JSON files
- **JSON Pointer (RFC 6901)**: path-based access like `/users/0/name`
- **JSON Patch (RFC 6902)**: diff and patch operations on JsonValue trees
- **JSON Schema validation**: validate JsonValue against a JSON Schema
- **Custom serialization hooks**: user-defined serializers for custom types
