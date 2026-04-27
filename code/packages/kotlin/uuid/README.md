# uuid — Kotlin

UUID generation and parsing for all five versions defined in RFC 4122 and
RFC 9562 (v1, v3, v4, v5, v7). Implemented as a Kotlin `data class` for
idiomatic value semantics. No external dependencies.

## Usage

```kotlin
import com.codingadventures.uuid.UUID

// v4: random (most common)
val u = UUID.v4()
println(u)           // e.g. "550e8400-e29b-41d4-..."
println(u.version)   // 4
println(u.variant)   // "rfc4122"

// v7: time-ordered random — ideal for DB primary keys
val u7 = UUID.v7()

// v5: deterministic name-based (SHA-1)
val dns = UUID.v5(UUID.NAMESPACE_DNS, "python.org")
// → "886313e1-3b8a-5372-9b90-0c9aee199e5d" (RFC test vector)

// v3: deterministic name-based (MD5, legacy)
val v3 = UUID.v3(UUID.NAMESPACE_DNS, "python.org")
// → "6fa459ea-ee8a-3ca4-894e-db77e160355e" (RFC test vector)

// Parsing — all standard formats
val a = UUID.fromString("550e8400-e29b-41d4-a716-446655440000")
val b = UUID.fromString("{550e8400-e29b-41d4-a716-446655440000}")
val c = UUID.fromString("urn:uuid:550e8400-e29b-41d4-a716-446655440000")

// data class features
val (msb, lsb) = UUID.v4()  // destructuring
val copy = a.copy()
```

## Running Tests

```bash
gradle test
```

49 tests covering all 5 UUID versions, RFC test vectors, all parse formats,
ordering, uniqueness, and Kotlin `data class` behaviour.

## Part of the Coding Adventures series

Kotlin counterpart to the Python, Rust, Go, TypeScript, and Java implementations.
