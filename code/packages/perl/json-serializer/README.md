# CodingAdventures::JsonSerializer (Perl)

Schema-aware JSON serializer/deserializer built on top of `CodingAdventures::JsonValue`.

## What it does

`JsonSerializer` extends `JsonValue` with four higher-level operations:

| Function | Purpose |
|---|---|
| `encode($value, \%opts)` | Serialize with indent, sort_keys, allow_nan, max_depth |
| `decode($json_str, \%opts)` | Parse with comment stripping and trailing-comma tolerance |
| `validate($value, \%schema)` | Validate a native Perl value against a JSON Schema subset |
| `schema_encode($value, \%schema)` | Encode with type coercion and property filtering |

## Where it fits in the stack

```
JsonSerializer    ← this module
      ↓
 JsonValue         (native Perl value round-trip)
      ↓
 JsonParser, JsonLexer, Parser, Lexer, GrammarTools, StateMachine, DirectedGraph
```

## Usage

```perl
use CodingAdventures::JsonSerializer;

# Pretty-print with sorted keys (default)
my $json = CodingAdventures::JsonSerializer::encode(
    { b => 2, a => 1 },
    { indent => 2 }
);
# {
#   "a": 1,
#   "b": 2
# }

# Decode JSONC (with comments)
my $v = CodingAdventures::JsonSerializer::decode(
    '{ "name": "Alice" /* the user */ }',
    { allow_comments => 1 }
);
print $v->{name};  # Alice

# Decode with trailing commas (non-strict, the default)
my $arr = CodingAdventures::JsonSerializer::decode('[1, 2, 3,]');
print $arr->[2];   # 3

# Validate against a schema
my $schema = {
    type       => 'object',
    required   => ['name'],
    properties => {
        name => { type => 'string', minLength => 1 },
        age  => { type => 'integer', minimum => 0 },
    },
    additional_properties => 0,
};
my ($ok, $errs) = CodingAdventures::JsonSerializer::validate(
    { name => 'Alice', age => 30 }, $schema
);
print $ok;  # 1

# Schema-guided encoding: coerce number → string, drop extra fields
my $api_schema = {
    type                  => 'object',
    additional_properties => 0,
    properties => {
        price => { type => 'string' },
        qty   => { type => 'number' },
    },
};
my $s = CodingAdventures::JsonSerializer::schema_encode(
    { price => 9.99, qty => 3, internal => 'secret' }, $api_schema
);
# {"price":"9.99","qty":3}   (internal dropped; price coerced to string)
```

## Options

### `encode($value, \%opts)`

| Option | Type | Default | Description |
|---|---|---|---|
| `indent` | int | 0 | Spaces per indent level (0 = compact) |
| `sort_keys` | bool | 1 | Sort object keys alphabetically |
| `allow_nan` | bool | 0 | Emit NaN/Infinity as quoted strings instead of null |
| `max_depth` | int | 100 | Raise error if nesting exceeds this depth |

### `decode($json_str, \%opts)`

| Option | Type | Default | Description |
|---|---|---|---|
| `allow_comments` | bool | 0 | Strip `//` and `/* */` comments before parsing |
| `strict` | bool | 0 | When 0, strip trailing commas; when 1, reject them |

### `validate($value, \%schema)`

Supported schema keywords: `type`, `properties`, `required`, `additional_properties`, `items`, `minItems`, `maxItems`, `minimum`, `maximum`, `minLength`, `maxLength`, `pattern`, `enum`.

Returns `(1, undef)` on success; `(0, \@errors)` on failure (all errors collected).

### `schema_encode($value, \%schema, \%opts)`

Applies coercions (number → string when schema says `type => 'string'`) and filters unknown properties when `additional_properties => 0`, then calls `encode`.

## Running the tests

```sh
cpanm --notest .
prove -l -v t/
```

Or use the BUILD file with the monorepo build tool.
