# lz77 (Kotlin) — CMP00

LZ77 lossless compression for Kotlin.  Idiomatic Kotlin port of the Java
CMP00 implementation using `data class Token`, Kotlin pairs, `buildList{}`,
and `ByteArrayOutputStream`.

## What it does

`LZ77` object exposes:

| Function | Description |
|----------|-------------|
| `compress(ByteArray?)` | Encode bytes into CMP00 wire format |
| `decompress(ByteArray?)` | Decode CMP00 wire format |
| `encode(ByteArray, ...)` | Encode to `List<Token>` |
| `decode(List<Token>, ...)` | Decode token stream |

## Quick start

```kotlin
val original   = "hello hello hello".toByteArray()
val compressed = LZ77.compress(original)
val recovered  = LZ77.decompress(compressed)
check(original.contentEquals(recovered))
```

## Running tests

```
gradle test
```

38 tests covering round-trip, token stream, wire format, edge cases,
overlapping matches, effectiveness, initial-buffer seed, and determinism.
