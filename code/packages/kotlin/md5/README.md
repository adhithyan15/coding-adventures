# md5 (Kotlin)

MD5 message-digest algorithm (RFC 1321) implemented from scratch in pure Kotlin
— no `java.security.MessageDigest`.

Kotlin port of the `md5` package that already exists in Rust, Java, Dart, and
other languages in the monorepo; produces byte-identical digests.

> **Security:** MD5 is cryptographically **broken** — practical collisions
> exist. Never use it for signatures or passwords. Checksum use only.

## API

`com.codingadventures.md5.Md5`:

| Member | Purpose |
|---|---|
| `fun sumMd5(data: ByteArray): ByteArray` | 16-byte digest. |
| `fun hexString(data: ByteArray): String` | 32-char lowercase hex digest. |
| `Md5.Digest` | Streaming: `update`, non-destructive `digest` / `hexDigest`, `copy`. |

## Usage

```kotlin
import com.codingadventures.md5.Md5

println(Md5.hexString("abc".toByteArray())) // 900150983cd24fb0d6963f7d28e17f72

val h = Md5.Digest()
h.update("ab".toByteArray())
h.update("c".toByteArray())
println(h.hexDigest()) // same digest, computed incrementally
```

## Implementation note

MD5 is **little-endian** throughout (block parsing, length field, digest output),
the opposite of SHA-1/SHA-256. Kotlin's `Int` is native 32-bit two's-complement,
so unsigned 32-bit arithmetic needs no masking; `Int.rotateLeft` performs the
rotations and block bytes are masked with `and 0xff` before shifting. Constants
above `0x7FFFFFFF` are Long literals, truncated via `.toInt()`.

## Running the tests

```
gradle test
```
