# LZ78 (Java)

A byte-oriented implementation of the LZ78 dictionary compressor specified by
[`CMP01-lz78.md`](../../../specs/CMP01-lz78.md).

`Lz78.encode` and `decode` expose the teaching token stream. `compress` accepts
an optional maximum dictionary size, while `decode` and `decompress` accept an
output ceiling (256 MiB by default) before allocating from an untrusted header. They
use the repository's exact big-endian format: an eight-byte length/count
header followed by four-byte tokens. Decoding validates dictionary references,
reserved bytes, declared lengths, truncation, and trailing data.

```java
byte[] wire = Lz78.compress("ABABAB".getBytes(StandardCharsets.UTF_8));
byte[] original = Lz78.decompress(wire);
```

Run `gradle test jacocoTestReport jacocoTestCoverageVerification` to test it.
