# json-value

A Swift enum representing any JSON value (null, bool, number, string, array, object).

## Overview

`JsonValue` is the foundational type for the JSON pipeline in the coding-adventures stack. It models JSON's six value types as a Swift enum, giving the compiler the ability to enforce exhaustive pattern matching.

```
json-value  ←— you are here
    ↓
json-lexer  (tokenizes JSON text → [Token])
    ↓
json-parser (tokens → JsonValue)
    ↓
json-serializer (JsonValue → String)
```

## Why an enum?

JSON has exactly six types. A Swift `enum` maps directly to this structure — the compiler prevents you from forgetting a case. Compare to the typical dynamic-language approach where a JSON "value" is any object and type errors are discovered at runtime.

## Usage

```swift
import JsonValue

// Construct values directly
let age   = JsonValue.number(30)
let name  = JsonValue.string("Alice")
let alive = JsonValue.bool(true)
let none  = JsonValue.null
let tags  = JsonValue.array([.string("swift"), .string("json")])
let person = JsonValue.object([
    (key: "name",  value: .string("Alice")),
    (key: "age",   value: .number(30)),
    (key: "alive", value: .bool(true)),
])

// Access values
print(person["name"]?.stringValue ?? "unknown")  // "Alice"
print(tags[0]?.stringValue ?? "")                // "swift"

// Pattern match
switch age {
case .number(let n): print("age is \(n)")
default: break
}

// Equality
assert(JsonValue.null == JsonValue.null)
assert(JsonValue.bool(true) != JsonValue.bool(false))
```

## Object ordering

The `.object` case uses `[(key: String, value: JsonValue)]` (an ordered array of pairs) rather than `[String: JsonValue]`. This preserves the insertion order of keys, which is important for deterministic serialization and matches the behavior of ECMAScript's `JSON.parse`.

## Integration

This package is a dependency of `json-lexer`, `json-parser`, and `json-serializer`. Use those packages to parse JSON text or serialize values back to strings.

## Part of the coding-adventures stack

This package is part of an educational monorepo exploring language implementations, data structures, and algorithms. All code uses Knuth-style literate programming with detailed inline explanations.
