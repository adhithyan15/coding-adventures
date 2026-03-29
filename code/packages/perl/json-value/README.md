# CodingAdventures::JsonValue

A JSON value evaluator and serializer for the coding-adventures monorepo. It walks the Abstract Syntax Tree produced by `CodingAdventures::JsonParser` and converts JSON values into native Perl data structures (hashrefs, arrayrefs, scalars). It also serializes native Perl values back to JSON strings.

## What it does

Given the AST for `{"name": "Alice", "age": 30}`, the evaluator produces:

```perl
{
    name => "Alice",
    age  => 30,
}
```

The reverse direction:

```perl
CodingAdventures::JsonValue::to_json({ name => "Alice", age => 30 });
# → '{"age":30,"name":"Alice"}'
```

## How it fits in the stack

```
CodingAdventures::JsonValue  ← this module
             ↓
CodingAdventures::JsonParser  (provides AST)
             ↓
CodingAdventures::JsonLexer, JsonParser::ASTNode
```

## Usage

```perl
use CodingAdventures::JsonValue;

# One-step parse + evaluate
my $t = CodingAdventures::JsonValue::from_string('{"name":"Alice","age":30}');
print $t->{name};   # Alice
print $t->{age};    # 30

# JSON null
my $v = CodingAdventures::JsonValue::from_string('null');
print CodingAdventures::JsonValue::is_null($v);  # 1 (true)

# Serialize to compact JSON
print CodingAdventures::JsonValue::to_json({ x => 1, y => 2 });
# → {"x":1,"y":2}

# Serialize to pretty JSON (2-space indent)
print CodingAdventures::JsonValue::to_json({ x => 1, y => 2 }, 2);
# → {
#     "x": 1,
#     "y": 2
#   }

# Round-trip
my $original = '{"tags":["perl","json"],"ok":1}';
my $v = CodingAdventures::JsonValue::from_string($original);
my $back = CodingAdventures::JsonValue::to_json($v);
# back → '{"ok":1,"tags":["perl","json"]}'  (keys sorted alphabetically)
```

## API

### `from_string($json_str)`

Parse a JSON string and return the evaluated native Perl value. Dies on error.

### `evaluate($ast_node)`

Walk an ASTNode from `CodingAdventures::JsonParser` and return the native Perl value.

### `to_json($value, $indent)`

Serialize a native Perl value to a JSON string. `$indent` (optional, default 0) enables pretty-printing with that many spaces per level. Object keys are sorted alphabetically for deterministic output.

### `is_null($v)`

Returns 1 if `$v` is the JSON null sentinel (`$CodingAdventures::JsonValue::NULL`), empty string otherwise.

### `$NULL`

The JSON null sentinel — a blessed reference of class `CodingAdventures::JsonValue::Null`.

## Type mapping

| JSON type | Perl type                         |
|-----------|-----------------------------------|
| object    | hashref `{}`                      |
| array     | arrayref `[]`                     |
| string    | scalar string                     |
| number    | scalar number (integer or float)  |
| true      | `1`                               |
| false     | `0`                               |
| null      | `$NULL` (blessed Null sentinel)   |

## Null sentinel note

Perl's `undef` is ambiguous — you cannot distinguish "key absent" from "key maps to JSON null" in a hash. The `$NULL` sentinel is a blessed reference that can be stored as a hash/array value and identified with `is_null($v)`.

## Version

0.01
