# lzss (Kotlin) — CMP02

LZSS lossless compression for Kotlin.  Idiomatic Kotlin port of the Java
CMP02 implementation using a sealed `Token` interface, `data class` for
`Literal` and `Match`, `when` expressions, and `ByteArrayOutputStream`.

## What it does

`LZSS` object exposes:

| Function | Description |
|----------|-------------|
| `compress(ByteArray?)` | Encode bytes into CMP02 wire format |
| `decompress(ByteArray?)` | Decode CMP02 wire format |
| `encode(ByteArray, ...)` | Encode to `List<Token>` |
| `decode(List<Token>, Int)` | Decode token stream |

## Quick start

```kotlin
val original   = "hello hello hello".toByteArray()
val compressed = LZSS.compress(original)
val recovered  = LZSS.decompress(compressed)
check(original.contentEquals(recovered))
```

## Running tests

```
gradle test
```

37 tests covering round-trip, token stream, wire format, edge cases,
overlapping matches, effectiveness, and determinism.
