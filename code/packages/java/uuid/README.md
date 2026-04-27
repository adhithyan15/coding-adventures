# uuid — Java

UUID generation and parsing for all five versions defined in RFC 4122 and
RFC 9562 (v1, v3, v4, v5, v7). No external dependencies — uses only
`java.security.{SecureRandom,MessageDigest}` from the JDK.

## Usage

```java
import com.codingadventures.uuid.UUID;

// v4: random (most common)
UUID u = UUID.v4();
System.out.println(u);             // e.g. "550e8400-e29b-41d4-..."
System.out.println(u.version());   // 4
System.out.println(u.variant());   // "rfc4122"

// v7: time-ordered random — ideal for DB primary keys
UUID u7 = UUID.v7();
// first 48 bits encode millisecond timestamp → sortable by creation time

// v5: deterministic name-based (SHA-1)
UUID dns = UUID.v5(UUID.NAMESPACE_DNS, "python.org");
// → "886313e1-3b8a-5372-9b90-0c9aee199e5d" (RFC test vector)

// v3: deterministic name-based (MD5, legacy)
UUID v3  = UUID.v3(UUID.NAMESPACE_DNS, "python.org");
// → "6fa459ea-ee8a-3ca4-894e-db77e160355e" (RFC test vector)

// v1: time-based
UUID v1 = UUID.v1();

// Parsing — accepts all standard formats
UUID a = UUID.fromString("550e8400-e29b-41d4-a716-446655440000");
UUID b = UUID.fromString("{550e8400-e29b-41d4-a716-446655440000}");
UUID c = UUID.fromString("urn:uuid:550e8400-e29b-41d4-a716-446655440000");
UUID d = UUID.fromString("550e8400e29b41d4a716446655440000");

System.out.println(UUID.NIL); // "00000000-0000-0000-0000-000000000000"
System.out.println(UUID.MAX); // "ffffffff-ffff-ffff-ffff-ffffffffffff"
```

## UUID Versions

| Version | Algorithm | Use case |
|---------|-----------|----------|
| v1 | Time-based (60-bit Gregorian timestamp + random node) | Time-ordered, legacy |
| v3 | Name-based MD5 | Deterministic; legacy compat only |
| v4 | Random (122 bits, CSPRNG) | General-purpose unique IDs |
| v5 | Name-based SHA-1 | Deterministic; preferred over v3 |
| v7 | Time-ordered random (48-bit ms timestamp) | Database primary keys |

## Bit Layout

```
UUID = 128 bits stored as msb (64 bits) + lsb (64 bits):

msb: [time-low 32][time-mid 16][ver 4][time-hi 12]
lsb: [var 2][clock-seq 14][node 48]
```

## Running Tests

```bash
gradle test
```

47 tests covering all 5 UUID versions, all parse formats, properties, RFC test
vectors, uniqueness, ordering, and edge cases.

## Part of the Coding Adventures series

Java counterpart to the Python, Rust, Go, TypeScript, and Kotlin implementations.
