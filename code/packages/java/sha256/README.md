# sha256 (Java)

SHA-256 cryptographic hash function (FIPS 180-4) implemented from scratch in
pure Java — no `java.security.MessageDigest`.

Java port of the `sha256` package that already exists in Rust, Python, Dart, and
other languages in the coding-adventures monorepo; produces byte-identical
digests.

## API

`com.codingadventures.sha256.Sha256`:

| Member | Purpose |
|---|---|
| `static byte[] sha256(byte[] data)` | 32-byte digest. |
| `static String sha256Hex(byte[] data)` | 64-char lowercase hex digest. |
| `Sha256.Hasher` | Streaming: `update`, non-destructive `digest` / `hexDigest`, `copy`. |

## Usage

```java
import com.codingadventures.sha256.Sha256;
import java.nio.charset.StandardCharsets;

byte[] data = "abc".getBytes(StandardCharsets.UTF_8);
System.out.println(Sha256.sha256Hex(data));
// ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad

Sha256.Hasher h = new Sha256.Hasher();
h.update("ab".getBytes(StandardCharsets.UTF_8));
h.update("c".getBytes(StandardCharsets.UTF_8));
System.out.println(h.hexDigest()); // same digest, computed incrementally
```

## Implementation note

SHA-256 is defined over unsigned 32-bit words. Java's `int` is a 32-bit
two's-complement value whose `+` and bitwise operators wrap and mix exactly as
unsigned 32-bit arithmetic requires, so no masking is needed — SHR uses `>>>`
(logical shift) and rotations use `Integer.rotateRight`.

## Security note

SHA-256 remains cryptographically secure, but a bare hash is **not** a password
scheme — use a purpose-built KDF (scrypt, argon2, PBKDF2) for passwords.

## Running the tests

```
gradle test
```
