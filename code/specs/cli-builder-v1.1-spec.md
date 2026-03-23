# CLI Builder v1.1 Specification

Four backwards-compatible additions to the CLI Builder type system and parse result.

---

## 1. Overview

CLI Builder v1.0 has been battle-tested across 56 Unix tool implementations in 6
languages. After six rounds of tool building (Tiers 0–6), four friction points
emerged that can be addressed with purely additive changes. No existing spec files
or parsing behavior change — old specs with `"cli_builder_spec_version": "1.0"`
continue to work identically.

### 1.1 What's New

| # | Feature | Summary |
|---|---------|---------|
| 1 | Enum optional values | Enum flags can act as boolean when no value is given |
| 2 | Flag presence detection | ParseResult reports which flags were explicitly set |
| 3 | int64 default for integers | All languages must use 64-bit signed integers |
| 4 | Count type | New `"type": "count"` for flags like `-vvv` |

### 1.2 Backwards Compatibility

All four features are purely additive:

- **Feature 1** adds an optional `"default_when_present"` field to flag definitions.
  Existing flags without this field behave exactly as before.
- **Feature 2** adds an `"explicit_flags"` field to ParseResult. Existing code that
  doesn't read this field is unaffected.
- **Feature 3** is a clarification, not a change. All six implementations already
  use 64-bit integers internally. This section formalizes the requirement.
- **Feature 4** adds a new value to the `type` enum. Existing specs don't use it.

The spec version remains `"1.0"` — these are additive features within the v1.0
schema, not breaking changes that would warrant a new schema version. However,
implementations should bump their library version to `1.1.0`.

---

## 2. Feature 1: Enum Optional Values (`default_when_present`)

### 2.1 Motivation

Many Unix tools have flags that work both as a boolean toggle and as an enum
selector. The canonical example is `--color`:

```
ls --color          # Enables color with default value "always"
ls --color=auto     # Enables color with explicit value "auto"
ls --color=never    # Disables color
ls                  # No color flag at all → uses default (usually "auto")
```

In v1.0, an enum flag MUST receive a value — `--color` without `=value` is a
parse error. This forces tool authors to use two separate flags or handle the
logic outside the parser.

### 2.2 Spec Changes

Add an optional `"default_when_present"` field to the flag definition schema
(§2.2 of the v1.0 spec):

```json
{
  "id": "color",
  "long": "color",
  "description": "Colorize output",
  "type": "enum",
  "enum_values": ["always", "auto", "never"],
  "default": "auto",
  "default_when_present": "always"
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `default_when_present` | string | no | Value to use when the flag is present but no value is given (e.g., `--color` without `=always`). Only valid when `type` is `"enum"`. Must be one of `enum_values`. |

### 2.3 Parsing Behavior

When the parser encounters a flag with `"default_when_present"` set:

1. **`--color=always`** → value is `"always"` (normal enum parsing, unchanged)
2. **`--color always`** → This is ambiguous. The parser must check whether the next
   token is a valid enum value. If it is, consume it as the value. If it is not
   (or there is no next token), use `default_when_present`.
3. **`--color`** (at end of argv, or next token starts with `-`) → value is
   `"always"` (the `default_when_present` value)
4. **Flag absent** → value is `"auto"` (the `default` value, unchanged)

**Disambiguation rule for `--flag value` form:**

When the parser sees `--color` followed by a non-flag token `T`:
- If `T` is in `enum_values` → consume `T` as the value
- If `T` is NOT in `enum_values` → use `default_when_present` and leave `T`
  unconsumed (it becomes a positional argument or the next flag's value)

This is consistent with how GNU `--color[=WHEN]` works in coreutils.

### 2.4 Validation

At spec load time:
- If `default_when_present` is set, `type` MUST be `"enum"`.
- `default_when_present` MUST be one of `enum_values`.
- If `default_when_present` is set but `enum_values` is empty, that's a spec error.

### 2.5 Short Flags

For short flags with `default_when_present`, the same rules apply:
- `-c always` → check if next token is a valid enum value
- `-c=always` → value is `"always"` (if short flag value syntax is supported)
- `-c` (at end or next is a flag) → use `default_when_present`

In stacked short flags (e.g., `-lc`), a flag with `default_when_present` must be
the LAST flag in the stack (since it might not consume a value). If it appears
in the middle, it uses `default_when_present` and parsing continues with the
remaining stack characters.

---

## 3. Feature 2: Flag Presence Detection (`explicit_flags`)

### 3.1 Motivation

In v1.0, ParseResult's `flags` map contains entries for ALL flags in scope —
both those the user explicitly typed and those filled in with defaults. There is
no way to distinguish between:

```
ls -l          # User explicitly set "long_listing" to true
ls             # "long_listing" defaults to false
```

For boolean flags this is fine (true = present, false = absent). But for flags
with non-null defaults, the distinction matters:

```
sort --separator=,    # User explicitly chose comma
sort                  # Separator defaults to whitespace
```

Both produce `{"separator": ","}` or `{"separator": " "}` — but the tool may
want to know whether the user made an active choice. Real-world use cases:

- `--color` defaulting to `"auto"` but detecting if the user explicitly set it
- Config file merging: CLI flags override config, but only if explicitly set
- Warnings: "you didn't specify --format, defaulting to json"

### 3.2 ParseResult Changes

Add an `explicit_flags` field to the ParseResult output:

```json
{
  "program": "ls",
  "command_path": ["ls"],
  "flags": {
    "long_listing": true,
    "color": "auto",
    "separator": "\t"
  },
  "arguments": {},
  "explicit_flags": ["long_listing"]
}
```

| Field | Type | Description |
|---|---|---|
| `explicit_flags` | array of strings | List of flag IDs that were explicitly provided by the user on the command line. Flags filled in with defaults are NOT included. |

### 3.3 Implementation

During parsing, maintain a set/list of flag IDs that the user explicitly typed.
Every time a flag token is consumed from argv (whether as `--flag`, `--flag=val`,
`-f`, or `-f val`), add its `id` to the explicit set.

Flags that are only present because of `default` values are NOT in this set.

Built-in flags (`--help`, `--version`) trigger special results and never appear
in ParseResult, so they are never in `explicit_flags`.

### 3.4 Language-Specific Types

| Language | Type |
|---|---|
| Go | `ExplicitFlags []string` on `ParseResult` struct |
| Python | `explicit_flags: list[str]` on `ParseResult` dataclass |
| Ruby | Add `:explicit_flags` to the `ParseResult` Struct |
| Rust | `pub explicit_flags: Vec<String>` on `ParseResult` struct |
| TypeScript | `explicitFlags: string[]` on `ParseResult` interface |
| Elixir | `:explicit_flags` key on `ParseResult` defstruct |

---

## 4. Feature 3: int64 Default for Integers

### 4.1 Motivation

All six implementations already parse integers as 64-bit signed values internally:

| Language | Current Implementation |
|---|---|
| Go | `strconv.ParseInt(raw, 10, 64)` → `int64` |
| Python | `int(raw)` → arbitrary precision (always ≥ 64-bit) |
| Ruby | `Integer(str)` → arbitrary precision (always ≥ 64-bit) |
| Rust | `raw.parse::<i64>()` → `i64` |
| TypeScript | `Number(raw)` → IEEE 754 double (53-bit integer precision) |
| Elixir | `Integer.parse(value)` → arbitrary precision (always ≥ 64-bit) |

### 4.2 Formalization

This section formalizes what's already true in practice:

> **Requirement:** When `type` is `"integer"`, implementations MUST parse and
> store the value as a signed 64-bit integer (or the language's closest
> equivalent). The valid range is −2^63 to 2^63 − 1.

**Language-specific notes:**

- **TypeScript**: JavaScript's `Number` type can only represent integers exactly
  up to 2^53 − 1 (`Number.MAX_SAFE_INTEGER`). Values outside this range should
  produce an `invalid_value` error. Implementations MAY use `BigInt` internally
  but MUST convert to `Number` for the ParseResult if the value fits in 53 bits.
- **Python, Ruby, Elixir**: These languages have arbitrary-precision integers.
  Values outside the int64 range (−2^63 to 2^63 − 1) should produce an
  `invalid_value` error to maintain cross-language consistency.

### 4.3 Validation

Add a range check after parsing: if the parsed integer is outside
[−9,223,372,036,854,775,808, 9,223,372,036,854,775,807], emit an
`invalid_value` error with a message like:

```
Integer value '99999999999999999999' is out of range (must fit in 64 bits)
```

### 4.4 No Spec File Changes

This feature requires no changes to the JSON spec format. The `"integer"` type
already exists; this is a behavioral clarification for implementations.

---

## 5. Feature 4: Count Type

### 5.1 Motivation

Many Unix tools use repeated flags to increase verbosity or debug level:

```
curl -v        # verbose
curl -vv       # more verbose
curl -vvv      # maximum verbosity

gcc -O         # optimization level 1
gcc -O -O -O   # same flag repeated 3 times
```

In v1.0, repeating a non-repeatable flag is a `duplicate_flag` error. The
`repeatable` option collects values into an array, but for boolean flags that
produces `[true, true, true]` — not a count.

### 5.2 Spec Changes

Add `"count"` as a new value in the type system (§3 of the v1.0 spec):

| Type | Description | Validation at Parse Time |
|---|---|---|
| `count` | Counts the number of times the flag appears. No value token is consumed. | N/A (like boolean, no value) |

Example spec:

```json
{
  "id": "verbose",
  "short": "v",
  "long": "verbose",
  "description": "Increase verbosity (can be repeated: -v, -vv, -vvv)",
  "type": "count",
  "default": 0
}
```

### 5.3 Parsing Behavior

- Each occurrence of the flag increments the count by 1.
- `-v` → `{"verbose": 1}`
- `-vv` → `{"verbose": 2}` (stacked short flags, each `v` counts as one)
- `-v -v -v` → `{"verbose": 3}`
- `--verbose --verbose` → `{"verbose": 2}`
- Flag absent → `{"verbose": 0}` (or `default` if specified)

### 5.4 Interaction with Other Features

- **`repeatable`**: Ignored for count flags. Count flags are inherently repeatable
  — specifying `"repeatable": true` is redundant but not an error.
- **`conflicts_with`**: Works as expected. If a count flag conflicts with another
  flag, any non-zero count triggers the conflict.
- **`default`**: If specified, must be a non-negative integer. The count starts
  from 0 (not from the default). The default is only used when the flag is absent.
- **`explicit_flags`**: A count flag is in `explicit_flags` if it appears at least
  once (count ≥ 1).

### 5.5 Short Flag Stacking

In stacked short flags like `-vvv`, each character `v` increments the count.
This is the natural behavior since stacking is already defined as "each character
is a separate flag occurrence."

### 5.6 No Value Consumed

Like boolean flags, count flags do NOT consume a value token. `--verbose 5` does
NOT set verbose to 5 — the `5` is a positional argument. To set a specific level,
use an integer flag instead.

### 5.7 Result Type

The parsed value for a count flag is always a non-negative integer:

| Language | Type |
|---|---|
| Go | `int64` (0, 1, 2, ...) |
| Python | `int` |
| Ruby | `Integer` |
| Rust | `serde_json::Value::Number` (i64) |
| TypeScript | `number` |
| Elixir | `integer` |

---

## 6. Implementation Checklist

For each language implementation:

- [ ] Add `"count"` to the valid type list
- [ ] Handle count type in flag scanning (no value consumed, increment counter)
- [ ] Handle count type in short flag stacking (each character increments)
- [ ] Add `"default_when_present"` field support to flag definitions
- [ ] Implement enum-without-value parsing using `default_when_present`
- [ ] Validate `default_when_present` at spec load time
- [ ] Add `explicit_flags` field to ParseResult
- [ ] Track which flags are explicitly set during parsing
- [ ] Add int64 range validation for integer types
- [ ] Update help generator to show `[=VALUE]` for enum flags with `default_when_present`
- [ ] Add tests for all four features
- [ ] Update CHANGELOG.md to v1.1.0
- [ ] Update README.md if needed
- [ ] Bump library version to 1.1.0

### 6.1 Test Cases

**Count type:**
1. Single occurrence: `-v` → count is 1
2. Stacked: `-vvv` → count is 3
3. Repeated long: `--verbose --verbose` → count is 2
4. Mixed: `-vv --verbose` → count is 3
5. Absent: (no flag) → count is 0
6. Default value: count absent with `"default": 0` → 0
7. Count flag in `explicit_flags` when present
8. Count flag NOT in `explicit_flags` when absent

**Enum optional values:**
1. `--color=always` → `"always"` (standard enum)
2. `--color` at end of argv → uses `default_when_present`
3. `--color` followed by a flag → uses `default_when_present`
4. `--color auto` where `auto` is a valid enum value → `"auto"`
5. `--color somefile.txt` where `somefile.txt` is NOT an enum value → uses
   `default_when_present`, `somefile.txt` becomes positional
6. Spec validation: `default_when_present` not in `enum_values` → error
7. Spec validation: `default_when_present` on non-enum flag → error

**Flag presence detection:**
1. Explicit flag appears in `explicit_flags`
2. Default-only flag does NOT appear in `explicit_flags`
3. Boolean flag set to true → in `explicit_flags`
4. Boolean flag left as default false → NOT in `explicit_flags`
5. Multiple explicit flags → all in `explicit_flags`
6. Repeated flag → appears once in `explicit_flags` (not duplicated)

**int64 range:**
1. Normal integer: `"42"` → 42
2. Negative integer: `"-100"` → -100
3. Max int64: `"9223372036854775807"` → accepted
4. Min int64: `"-9223372036854775808"` → accepted
5. Overflow: `"9223372036854775808"` → `invalid_value` error
6. Underflow: `"-9223372036854775809"` → `invalid_value` error

---

## 7. Version History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-03-20 | Initial release |
| 1.1 | 2026-03-22 | Enum optional values, flag presence detection, int64 formalization, count type |
