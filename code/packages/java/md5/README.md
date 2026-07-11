# md5 (Java)

MD5 message-digest algorithm (RFC 1321) implemented from scratch in pure Java —
no `java.security.MessageDigest`.

Java port of the `md5` package that already exists in Rust, Python, Dart, and
other languages in the monorepo; produces byte-identical digests.

> **Security:** MD5 is cryptographically **broken** — practical collisions
> exist. Never use it for signatures or passwords. Checksum use only.

## API

`com.codingadventures.md5.Md5`:

| Member | Purpose |
|---|---|
| `static byte[] sumMd5(byte[] data)` | 16-byte digest. |
| `static String hexString(byte[] data)` | 32-char lowercase hex digest. |
| `Md5.Digest` | Streaming: `update`, non-destructive `digest` / `hexDigest`, `copy`. |

## Usage

```java
import com.codingadventures.md5.Md5;
import java.nio.charset.StandardCharsets;

byte[] data = "abc".getBytes(StandardCharsets.UTF_8);
System.out.println(Md5.hexString(data)); // 900150983cd24fb0d6963f7d28e17f72

Md5.Digest h = new Md5.Digest();
h.update("ab".getBytes(StandardCharsets.UTF_8));
h.update("c".getBytes(StandardCharsets.UTF_8));
System.out.println(h.hexDigest()); // same digest, computed incrementally
```

## Implementation note

MD5 is **little-endian** throughout (block parsing, length field, digest output),
the opposite of SHA-1/SHA-256. Java's `int` is native 32-bit two's-complement, so
unsigned 32-bit arithmetic needs no masking; `Integer.rotateLeft` performs the
rotations and block bytes are masked with `& 0xff` before shifting.

## Running the tests

```
gradle test
```
