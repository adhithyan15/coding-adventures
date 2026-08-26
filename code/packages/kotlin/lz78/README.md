# LZ78 (Kotlin)

A byte-oriented implementation of the LZ78 dictionary compressor specified by
[`CMP01-lz78.md`](../../../specs/CMP01-lz78.md).

`Lz78.encode` and `decode` expose the teaching token stream. `compress` accepts
an optional maximum dictionary size, while `decode` and `decompress` enforce an
output ceiling (256 MiB by default) before allocating from an untrusted header. They
use the exact big-endian repository wire format and reject invalid
dictionary references, reserved bytes, declared lengths, truncation, and trailing data.

```kotlin
val wire = Lz78.compress("ABABAB".encodeToByteArray())
val original = Lz78.decompress(wire)
```

Run `gradle test jacocoTestReport jacocoTestCoverageVerification` to test it.
