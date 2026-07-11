# sha1 (Kotlin)

SHA-1 cryptographic hash function (FIPS 180-4) implemented from scratch in pure
Kotlin — no `java.security.MessageDigest`.

Kotlin port of the `sha1` package that already exists in Rust, Java, Dart, and
other languages in the monorepo; produces byte-identical digests.

> **Security:** SHA-1 is **broken** for collision resistance (SHAttered, 2017).
> Never use it for signatures or certificates. Legacy/checksum use only.

## API

`com.codingadventures.sha1.Sha1`:

| Member | Purpose |
|---|---|
| `fun sum1(data: ByteArray): ByteArray` | 20-byte digest. |
| `fun hexString(data: ByteArray): String` | 40-char lowercase hex digest. |
| `Sha1.Digest` | Streaming: `update`, non-destructive `digest` / `hexDigest`, `copy`. |

## Usage

```kotlin
import com.codingadventures.sha1.Sha1

println(Sha1.hexString("abc".toByteArray()))
// a9993e364706816aba3e25717850c26c9cd0d89d
```

## Implementation note

SHA-1 is **big-endian** like SHA-256 (opposite of MD5); five state words, 80
rounds. Kotlin's native 32-bit `Int` needs no masking; uses `Int.rotateLeft`,
`ushr`, and `and 0xff` byte masking. Constants above `0x7FFFFFFF` are Long
literals, truncated via `.toInt()`.

## Running the tests

```
gradle test
```
