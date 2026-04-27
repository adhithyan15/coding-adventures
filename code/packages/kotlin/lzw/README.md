# lzw (Kotlin) — CMP03

LZW lossless compression for Kotlin.  Idiomatic Kotlin port of the Java
CMP03 implementation using `object` singleton, `List<Byte>` dictionary keys,
`when` expressions, Kotlin's `shl`/`shr`/`ushr`/`and`/`or` operators, and
`ByteArrayOutputStream`.

## What it does

`LZW` object exposes:

| Function | Description |
|----------|-------------|
| `compress(ByteArray?)` | Encode bytes into CMP03 wire format |
| `decompress(ByteArray?)` | Decode CMP03 wire format |

## Quick start

```kotlin
val original   = "hello hello hello".toByteArray()
val compressed = LZW.compress(original)
val recovered  = LZW.decompress(compressed)
check(original.contentEquals(recovered))
```

## Running tests

```
gradle test
```

39 tests covering round-trip, code stream structure, wire format, edge cases,
tricky token, bit I/O, effectiveness, and determinism.
