# sha256 (Kotlin)

SHA-256 cryptographic hash function (FIPS 180-4) implemented from scratch in
pure Kotlin — no `java.security.MessageDigest`.

Kotlin port of the `sha256` package that already exists in Rust, Java, Dart, and
other languages in the monorepo; produces byte-identical digests.

## API

`com.codingadventures.sha256.Sha256`:

| Member | Purpose |
|---|---|
| `fun sha256(data: ByteArray): ByteArray` | 32-byte digest. |
| `fun sha256Hex(data: ByteArray): String` | 64-char lowercase hex digest. |
| `Sha256.Hasher` | Streaming: `update`, non-destructive `digest` / `hexDigest`, `copy`. |

## Usage

```kotlin
import com.codingadventures.sha256.Sha256

val data = "abc".toByteArray()
println(Sha256.sha256Hex(data))
// ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad

val h = Sha256.Hasher()
h.update("ab".toByteArray())
h.update("c".toByteArray())
println(h.hexDigest()) // same digest, computed incrementally
```

## Implementation note

SHA-256 is defined over unsigned 32-bit words. Kotlin's `Int` is a 32-bit
two's-complement value whose `+` and bitwise operators (`and`, `or`, `xor`,
`inv`) match unsigned 32-bit semantics, so no masking is needed — SHR uses
`ushr` (logical shift) and rotations use `Int.rotateRight`. Constants above
`0x7FFFFFFF` are Long literals, truncated to their exact 32-bit pattern with
`.toInt()`.

## Security note

SHA-256 remains cryptographically secure, but a bare hash is **not** a password
scheme — use a purpose-built KDF (scrypt, argon2, PBKDF2) for passwords.

## Running the tests

```
gradle test
```
