# sha1 (Java)

SHA-1 cryptographic hash function (FIPS 180-4) implemented from scratch in pure
Java — no `java.security.MessageDigest`.

Java port of the `sha1` package that already exists in Rust, Dart, and other
languages in the monorepo; produces byte-identical digests.

> **Security:** SHA-1 is **broken** for collision resistance (SHAttered, 2017).
> Never use it for signatures or certificates. Legacy/checksum use only.

## API

`com.codingadventures.sha1.Sha1`:

| Member | Purpose |
|---|---|
| `static byte[] sum1(byte[] data)` | 20-byte digest. |
| `static String hexString(byte[] data)` | 40-char lowercase hex digest. |
| `Sha1.Digest` | Streaming: `update`, non-destructive `digest` / `hexDigest`, `copy`. |

## Usage

```java
import com.codingadventures.sha1.Sha1;
import java.nio.charset.StandardCharsets;

byte[] data = "abc".getBytes(StandardCharsets.UTF_8);
System.out.println(Sha1.hexString(data));
// a9993e364706816aba3e25717850c26c9cd0d89d
```

## Implementation note

SHA-1 is **big-endian** like SHA-256 (opposite of MD5); five state words, 80
rounds. Java's native 32-bit `int` needs no masking; uses `Integer.rotateLeft`
and `& 0xff` byte masking before shifts.

## Running the tests

```
gradle test
```
