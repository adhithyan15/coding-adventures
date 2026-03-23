# UUID Library

## Overview

A **Universally Unique Identifier (UUID)** is a 128-bit label used to
identify information without requiring a central authority. This spec defines
a UUID library for all six supported languages: Python, TypeScript, Ruby, Go,
Rust, and Elixir.

The library covers:

- **UUID v1** — time-based, using the current timestamp and MAC address (or
  random node ID when MAC is unavailable)
- **UUID v3** — name-based using MD5 (deterministic from a namespace + name;
  kept for backwards compatibility with systems that generate v3 UUIDs)
- **UUID v4** — random (the most common variant; just 122 bits of randomness)
- **UUID v5** — name-based using SHA-1 (deterministic from a namespace + name;
  preferred over v3 for new systems)
- **UUID v7** — time-ordered random (newer RFC 9562 standard; millisecond
  timestamp in the high bits for sortability)
- **Parsing** — converting a string like `"550e8400-e29b-41d4-a716-446655440000"`
  into a structured UUID value
- **Validation** — checking whether a string is a well-formed UUID
- **Formatting** — producing the standard 8-4-4-4-12 hyphenated hex string
- **Comparison** — equality, ordering (for v7 sort), and nil/max checks
- **Namespace constants** — the four well-known RFC 4122 namespaces (DNS,
  URL, OID, X.500)

**Why build our own instead of using stdlib?**

1. **Zero external dependencies.** This repo builds everything from first
   principles. Understanding UUIDs is foundational to distributed systems.
2. **Cross-language consistency.** The same API surface across all six
   languages makes it easy to reason about UUID semantics regardless of
   language.
3. **Learning.** UUIDs touch clock handling, bit manipulation, cryptographic
   hashing, and the IEEE 802 MAC address format. Reading the source teaches
   all of these.
4. **Control.** We can support v7 (RFC 9562) which many standard libraries
   don't yet include.

---

## What Is a UUID?

```
UUID structure (128 bits):

  xxxxxxxx-xxxx-Mxxx-Nxxx-xxxxxxxxxxxx
  |        |    |    |    |
  |        |    |    |    +-- node (48 bits)
  |        |    |    +------- clock_seq (14 bits) + variant bits (2 bits)
  |        |    +------------ version (4 bits) + time_mid or rand (12 bits)
  |        +----------------- time_mid (16 bits)
  +-------------------------- time_low (32 bits)

  M = version nibble (1, 3, 4, 5, or 7)
  N = variant nibble (8, 9, a, or b for RFC 4122 UUIDs)

Example UUID (v4, random):
  550e8400-e29b-41d4-a716-446655440000
  ^^^^^^^^ ^^^^ ^    ^^^^  ^^^^^^^^^^^^
  32 hex   16   ver  clk   48 hex (node)
  chars    chars     seq   chars
```

### Version and Variant Fields

```
Bit layout of the 128-bit UUID (big-endian):

  Bits 0-31:    time_low        (version 1) or random (v4) or timestamp (v7)
  Bits 32-47:   time_mid        (version 1) or random (v4) or timestamp (v7)
  Bits 48-51:   version         (4 bits, value 1/3/4/5/7)
  Bits 52-63:   time_hi_and_ver (version 1) or random (v4/v7)
  Bits 64-65:   variant         (must be 0b10 for RFC 4122)
  Bits 66-79:   clock_seq       (14 bits, version 1) or random (v4/v7)
  Bits 80-127:  node            (48 bits)

Version encoding:
  0001 = v1 (time-based)
  0011 = v3 (name-based, MD5)
  0100 = v4 (random)
  0101 = v5 (name-based, SHA-1)
  0111 = v7 (time-ordered random)

Variant encoding:
  10xx = RFC 4122 (standard)      -- what we generate
  0xxx = NCS backward compat      -- legacy
  110x = Microsoft GUID           -- legacy
  1111 = reserved                 -- legacy
```

### Version Details

```
v1 — Time-Based
  60-bit timestamp: 100-nanosecond intervals since 1582-10-15 00:00:00 UTC
  (the Gregorian epoch, 122,192,928,000,000,000 intervals before Unix epoch)
  14-bit clock sequence: randomized at startup, incremented if clock goes backward
  48-bit node: MAC address of the host, or a cryptographically random value

v3 — Name-Based (MD5)
  Input: a namespace UUID + a name string (encoded as UTF-8)
  MD5( namespace_bytes || name_utf8 ) -> 128-bit hash
  The full 16-byte MD5 output is used (it already fits in 128 bits)
  Set version nibble to 3
  Set variant bits to 10
  Result: always the same UUID for the same (namespace, name) pair
  Note: v5 (SHA-1) is preferred for new systems; v3 exists for backwards
        compatibility with systems that already generate v3 UUIDs
  RFC test vector:
    v3(NAMESPACE_DNS, "python.org") = 6fa459ea-ee8a-3ca4-894e-db77e160355e

v4 — Random
  122 bits of cryptographically random data
  Version nibble set to 4
  Variant bits set to 10
  Everything else is random
  Simplest to implement. No time or MAC dependence.

v5 — Name-Based (SHA-1)
  Input: a namespace UUID + a name string (encoded as UTF-8)
  SHA-1( namespace_bytes || name_utf8 ) -> 160-bit hash
  Take the first 128 bits of the hash
  Set version nibble to 5
  Set variant bits to 10
  Result: always the same UUID for the same (namespace, name) pair
  Used for stable IDs from natural keys (e.g., URL -> UUID)

v7 — Time-Ordered Random (RFC 9562)
  Bits 0-47:  Unix millisecond timestamp (48 bits, big-endian)
  Bits 48-51: version = 7
  Bits 52-63: random (12 bits)
  Bits 64-65: variant = 10
  Bits 66-127: random (62 bits)
  Result: lexicographically sortable by creation time
  Preferred for database primary keys (better index locality than v4)
```

---

## Where It Fits

```
Application Code (Actor D19, Chief of Staff D18, File System D15, etc.)
|   UUID.v4()                    --> random ID for any entity
|   UUID.v7()                    --> sortable ID for database rows
|   UUID.v5(namespace, name)     --> stable ID from a natural key
|   UUID.parse("550e8400-...")   --> parse a UUID from string
|   UUID.valid?("...")           --> validate a UUID string
|   uuid.to_s                   --> "550e8400-e29b-41d4-a716-446655440000"
v
uuid (THIS SPEC)
|   depends on --> stdlib CSPRNG (SecureRandom / crypto/rand / OsRng)
|   depends on --> stdlib SHA-1  (hashlib / crypto/sha1 / sha1 crate)
|   no other dependencies
```

**Depends on:** sha1 (for v5), md5 (for v3), and language stdlib CSPRNG

**Used by:**
- Actor (D19) — message envelope IDs
- Chief of Staff (D18) — agent and channel IDs
- File System (D15) — inode / file IDs in virtual filesystem
- Any package needing unique identifiers

---

## Public API

The API is described in Python-style pseudocode. Language-specific mappings
follow in a table.

```python
# =====================================================================
# Data type: UUID
# =====================================================================

class UUID:
    """A 128-bit universally unique identifier.

    Internally stored as bytes (16 bytes / 128 bits) in network byte
    order (big-endian). This is the most portable internal representation
    and matches the wire format.

    All fields below are derived lazily or eagerly from those bytes:
      - version: int  (1, 4, 5, or 7)
      - variant: str  ("rfc4122", "microsoft", "ncs", "reserved")
    """
    bytes: bytes  # 16 bytes, big-endian


# =====================================================================
# Namespace constants (RFC 4122 Section 4.3)
# =====================================================================

NAMESPACE_DNS  = UUID("6ba7b810-9dad-11d1-80b4-00c04fd430c8")
NAMESPACE_URL  = UUID("6ba7b811-9dad-11d1-80b4-00c04fd430c8")
NAMESPACE_OID  = UUID("6ba7b812-9dad-11d1-80b4-00c04fd430c8")
NAMESPACE_X500 = UUID("6ba7b814-9dad-11d1-80b4-00c04fd430c8")

NIL = UUID("00000000-0000-0000-0000-000000000000")  # all zeros
MAX = UUID("ffffffff-ffff-ffff-ffff-ffffffffffff")  # all ones


# =====================================================================
# Generation
# =====================================================================

def v1() -> UUID:
    """Generate a UUID v1 (time-based).

    Uses the current UTC time as a 60-bit count of 100-nanosecond intervals
    since 1582-10-15 (Gregorian epoch).

    Node ID:
      - Try to obtain the host MAC address.
      - If unavailable (e.g., sandboxed environment), generate 48 random bits
        and set the multicast bit (bit 40) to 1 to indicate a random node ID.
        This is compliant with RFC 4122 Section 4.5.

    Clock sequence:
      - Initialized to a random 14-bit value at module load time.
      - Incremented if the clock goes backward (to avoid duplicates).

    Example:
        >>> u = v1()
        >>> u.version
        1
    """


def v4() -> UUID:
    """Generate a UUID v4 (random).

    Uses the OS cryptographically secure random number generator:
      Python: os.urandom(16)
      TypeScript: crypto.getRandomValues()
      Ruby: SecureRandom.random_bytes(16)
      Go: crypto/rand.Read()
      Rust: OsRng (from rand crate) or getrandom crate
      Elixir: :crypto.strong_rand_bytes(16)

    Sets:
      - Bits 48-51 (version nibble) to 0100 (4)
      - Bits 64-65 (variant bits) to 10

    Example:
        >>> u = v4()
        >>> u.version
        4
        >>> u.variant
        'rfc4122'
    """


def v3(namespace: UUID, name: str) -> UUID:
    """Generate a UUID v3 (name-based, MD5).

    Algorithm:
      1. Concatenate namespace.bytes (16 bytes) with name encoded as UTF-8.
      2. Compute MD5 of the concatenation -> 16 bytes.
      3. The 16 bytes are used directly (MD5 output already fits in 128 bits).
      4. Set version nibble (bits 48-51) to 0011 (3).
      5. Set variant bits (bits 64-65) to 10.

    Deterministic: same (namespace, name) always yields the same UUID.

    Note: prefer v5() for new code. v3 exists only for compatibility with
    systems that already generate v3 UUIDs.

    RFC test vector:
        >>> v3(NAMESPACE_DNS, "python.org")
        UUID("6fa459ea-ee8a-3ca4-894e-db77e160355e")
    """


def v5(namespace: UUID, name: str) -> UUID:
    """Generate a UUID v5 (name-based, SHA-1).

    Algorithm:
      1. Concatenate namespace.bytes (16 bytes) with name encoded as UTF-8.
      2. Compute SHA-1 of the concatenation -> 20 bytes.
      3. Take the first 16 bytes.
      4. Set version nibble (bits 48-51) to 0101 (5).
      5. Set variant bits (bits 64-65) to 10.

    Deterministic: same (namespace, name) always yields the same UUID.

    Example:
        >>> v5(NAMESPACE_DNS, "python.org")
        UUID("886313e1-3b8a-5372-9b90-0c9aee199e5d")
    """


def v7() -> UUID:
    """Generate a UUID v7 (time-ordered, random).

    Algorithm (RFC 9562 Section 5.7):
      1. Get current Unix timestamp in milliseconds (48-bit value).
      2. Lay out 128 bits:
           Bits  0-47:  timestamp_ms (big-endian, 48 bits)
           Bits 48-51:  version = 7
           Bits 52-63:  rand_a (12 random bits)
           Bits 64-65:  variant = 10
           Bits 66-127: rand_b (62 random bits)
      3. All random bits from CSPRNG.

    Property: v7 UUIDs generated in the same millisecond are not ordered
    relative to each other, but UUIDs from different milliseconds are
    strictly ordered by time.

    Example:
        >>> u1 = v7()
        >>> u2 = v7()
        >>> u1 < u2  # almost certainly true (unless same ms)
        True
    """


# =====================================================================
# Parsing
# =====================================================================

def parse(text: str) -> UUID:
    """Parse a UUID from its standard string representation.

    Accepts all of the following formats:
      "550e8400-e29b-41d4-a716-446655440000"  (standard, hyphenated)
      "550E8400-E29B-41D4-A716-446655440000"  (uppercase)
      "550e8400e29b41d4a716446655440000"       (compact, no hyphens)
      "{550e8400-e29b-41d4-a716-446655440000}" (braces, Windows GUID style)
      "urn:uuid:550e8400-e29b-41d4-a716-446655440000" (URN form)

    Case-insensitive. Strips leading/trailing whitespace.

    Raises:
        UUIDError: if the text is not a valid UUID representation.

    Example:
        >>> parse("550e8400-e29b-41d4-a716-446655440000").version
        4
    """


# =====================================================================
# Validation
# =====================================================================

def is_valid(text: str) -> bool:
    """Return True if the string is a valid UUID representation.

    Checks format only -- does not validate version or variant semantics.
    Accepts the same formats as parse().

    Example:
        >>> is_valid("not-a-uuid")
        False
        >>> is_valid("550e8400-e29b-41d4-a716-446655440000")
        True
    """


# =====================================================================
# Inspection
# =====================================================================

@property
def version(self) -> int | None:
    """The UUID version (1, 3, 4, 5, or 7), or None if not RFC 4122.

    Extracted from bits 48-51 of the UUID bytes.

    Example:
        >>> v4().version
        4
    """

@property
def variant(self) -> str:
    """The UUID variant string.

    Returns one of:
      "rfc4122"    -- bits 64-65 are 10 (standard)
      "microsoft"  -- bits 64-66 are 110 (legacy Windows GUID)
      "ncs"        -- bit 64 is 0 (legacy NCS compat)
      "reserved"   -- bits 64-67 are 111x (reserved)
    """

@property
def is_nil(self) -> bool:
    """True if all 128 bits are zero."""

@property
def is_max(self) -> bool:
    """True if all 128 bits are one."""

@property
def int_value(self) -> int:
    """The UUID as a 128-bit unsigned integer (big-endian)."""

@property
def hex(self) -> str:
    """The UUID as 32 lowercase hex characters (no hyphens).

    Example: "550e8400e29b41d4a716446655440000"
    """

@property
def urn(self) -> str:
    """The UUID as a URN string.

    Example: "urn:uuid:550e8400-e29b-41d4-a716-446655440000"
    """


# =====================================================================
# Formatting
# =====================================================================

def __str__(self) -> str:
    """Return the standard hyphenated lowercase string representation.

    Format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx  (8-4-4-4-12)
    Always lowercase.

    Example:
        >>> str(v4())
        "550e8400-e29b-41d4-a716-446655440000"
    """


# =====================================================================
# Comparison and hashing
# =====================================================================

def __eq__(self, other: UUID) -> bool:
    """Two UUIDs are equal iff their 128 bytes are identical."""

def __lt__(self, other: UUID) -> bool:
    """Lexicographic ordering on the raw bytes (big-endian integer comparison).

    This makes v7 UUIDs sort chronologically when compared with <.
    """

def __hash__(self) -> int:
    """UUID is hashable -- can be used as a dict key or in a set."""
```

### Language-Specific API Mapping

```
Concept           Python           TypeScript         Ruby
---------         ------           ----------         ----
UUID type         UUID class       UUID class         UUID class (module)
Generate v4       UUID.v4()        UUID.v4()          UUID.v4
Generate v7       UUID.v7()        UUID.v7()          UUID.v7
Generate v5       UUID.v5(ns, n)   UUID.v5(ns, n)     UUID.v5(ns, n)
Generate v1       UUID.v1()        UUID.v1()          UUID.v1
Parse             UUID.parse(s)    UUID.parse(s)      UUID.parse(s)
Validate          UUID.is_valid(s) UUID.isValid(s)    UUID.valid?(s)
To string         str(u) / u.str  u.toString()       u.to_s
Version           u.version        u.version          u.version
Nil UUID          UUID.NIL         UUID.NIL           UUID::NIL
Namespaces        UUID.NAMESPACE_DNS               (same pattern)
Error type        UUIDError        UUIDError          UUID::Error

Concept           Go               Rust               Elixir
---------         --               ----               ------
UUID type         UUID struct      Uuid struct        %UUID{} struct / 16-byte binary
Generate v4       uuid.V4()        Uuid::v4()         UUID.v4()
Generate v7       uuid.V7()        Uuid::v7()         UUID.v7()
Generate v5       uuid.V5(ns, n)   Uuid::v5(ns, n)    UUID.v5(ns, name)
Generate v1       uuid.V1()        Uuid::v1()         UUID.v1()
Parse             uuid.Parse(s)    Uuid::parse(s)     UUID.parse(s)
Validate          uuid.IsValid(s)  Uuid::is_valid(s)  UUID.valid?(s)
To string         u.String()       u.to_string()      UUID.to_string(u)
Version           u.Version()      u.version()        UUID.version(u)
Nil UUID          uuid.Nil         Uuid::nil()        UUID.nil()
Error type        error            UuidError          {:error, reason}
```

---

## Internal Representation

All implementations store a UUID as **16 raw bytes in big-endian (network)
byte order**. This is the most portable representation.

```
Byte index:   0  1  2  3  |  4  5  |  6  7  |  8  9  | 10 11 12 13 14 15
UUID field:   time_low     | t_mid  | t_hi+v | clk_seq|  node (48 bits)
```

Other derived representations (string, integer, version, variant) are
computed from these bytes. Never store a UUID as a string internally.

---

## Algorithms

### Algorithm: v4()

```
v4() -> UUID
============

1. bytes = CSPRNG.random_bytes(16)   -- 16 bytes of OS randomness

2. Set version nibble:
   bytes[6] = (bytes[6] & 0x0F) | 0x40
   -- Clear the top 4 bits of byte 6, set them to 0100 (version 4)

3. Set variant bits:
   bytes[8] = (bytes[8] & 0x3F) | 0x80
   -- Clear the top 2 bits of byte 8, set them to 10 (RFC 4122 variant)

4. return UUID(bytes)

Truth table for version nibble (byte 6):
  Input high nibble  After & 0x0F  After | 0x40  Result nibble
  ----------------   -----------   -----------   -------------
  0x00               0x00          0x40          4 (0100)
  0xAB               0x0B          0x4B          4 (0100)
  0xFF               0x0F          0x4F          4 (0100)
```

### Algorithm: v5(namespace, name)

```
v5(namespace: UUID, name: str) -> UUID
======================================

1. data = namespace.bytes + name.encode("utf-8")
   -- 16 namespace bytes followed by name bytes

2. digest = SHA1(data)
   -- 20 bytes

3. bytes = digest[0:16]
   -- Take the first 16 bytes of the 20-byte SHA-1 output

4. Set version nibble:
   bytes[6] = (bytes[6] & 0x0F) | 0x50
   -- Version 5 = 0101

5. Set variant bits:
   bytes[8] = (bytes[8] & 0x3F) | 0x80
   -- RFC 4122 variant = 10

6. return UUID(bytes)

Verification vector (from RFC 4122 Appendix B):
  namespace = NAMESPACE_DNS = "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
  name      = "python.org"
  expected  = "886313e1-3b8a-5372-9b90-0c9aee199e5d"

  SHA-1("6ba7b810-..." bytes || "python.org") must yield this UUID
  after version/variant bit fixup.
```

### Algorithm: v7()

```
v7() -> UUID
============

1. ms = current_unix_time_milliseconds()
   -- 48-bit value (milliseconds since 1970-01-01 00:00:00 UTC)
   -- Valid until year 10895 (well beyond concern)

2. rand = CSPRNG.random_bytes(10)
   -- 80 bits of randomness for rand_a (12 bits) + rand_b (62 bits)

3. Build 16 bytes:
   bytes[0] = (ms >> 40) & 0xFF
   bytes[1] = (ms >> 32) & 0xFF
   bytes[2] = (ms >> 24) & 0xFF
   bytes[3] = (ms >> 16) & 0xFF
   bytes[4] = (ms >>  8) & 0xFF
   bytes[5] =  ms        & 0xFF
   bytes[6] = (rand[0] & 0x0F) | 0x70   -- version 7 in high nibble
   bytes[7] =  rand[1]
   bytes[8] = (rand[2] & 0x3F) | 0x80   -- variant bits
   bytes[9] =  rand[3]
   bytes[10..15] = rand[4..9]

4. return UUID(bytes)

Diagram:
  Bits 0-47:   [ timestamp_ms (48 bits, big-endian)          ]
  Bits 48-51:  [ 0111 = version 7 ]
  Bits 52-63:  [ rand_a (12 random bits)                     ]
  Bits 64-65:  [ 10 = RFC 4122 variant ]
  Bits 66-127: [ rand_b (62 random bits)                     ]
```

### Algorithm: v1()

```
v1() -> UUID
============

The v1 algorithm requires three inputs:
  T   = current time as 60-bit count of 100ns intervals since 1582-10-15
  CS  = 14-bit clock sequence (module-level state)
  N   = 48-bit node ID (MAC address or random)

Gregorian offset:
  The UUID epoch (1582-10-15) is 12,219,292,800 seconds before Unix epoch.
  In 100ns intervals: 12219292800 * 10_000_000 = 122_192_928_000_000_000

Step 1: Compute timestamp T
  unix_100ns = current_unix_time_nanoseconds() // 100
  T = unix_100ns + 122_192_928_000_000_000

Step 2: Get clock sequence CS
  At startup: CS = CSPRNG.random_int(14_bits)
  If T < last_T: CS = (CS + 1) & 0x3FFF   -- clock went backward

Step 3: Get node ID N
  Try to read a MAC address from the OS network interfaces.
  If unavailable: N = CSPRNG.random_bytes(6), then set bit 40 of N to 1
  (multicast bit signals random node ID per RFC 4122 Section 4.5).

Step 4: Lay out the 128 bits
  time_low  = T & 0xFFFFFFFF           (bits 0-31)
  time_mid  = (T >> 32) & 0xFFFF       (bits 32-47)
  time_hi   = (T >> 48) & 0x0FFF       (bits 52-63, 12 bits)

  bytes[0..3]  = time_low  (big-endian)
  bytes[4..5]  = time_mid  (big-endian)
  bytes[6]     = 0x10 | (time_hi >> 8)   -- version nibble 0001
  bytes[7]     = time_hi & 0xFF
  bytes[8]     = 0x80 | (CS >> 8)        -- variant bits 10, high clock seq
  bytes[9]     = CS & 0xFF
  bytes[10..15] = N (node ID, 6 bytes)

Step 5: return UUID(bytes)
```

### Algorithm: parse(text)

```
parse(text: str) -> UUID
========================

1. strip = text.strip()

2. If strip starts with "urn:uuid:":
   strip = strip[9:]

3. If strip starts with "{" and ends with "}":
   strip = strip[1:-1]

4. Remove all hyphens:
   hex_str = strip.replace("-", "")

5. Validate:
   - len(hex_str) must be 32
   - all characters must be hex digits [0-9a-fA-F]
   If either check fails: raise UUIDError

6. bytes = hex_decode(hex_str)   -- 16 bytes

7. return UUID(bytes)
```

---

## Error Handling

All errors are surfaced as a single error type per language:

```
Language      Error type
--------      ----------
Python        UUIDError(Exception)
TypeScript    UUIDError extends Error
Ruby          UUID::Error < StandardError
Go            error (returned as second value)
Rust          UuidError (enum with variants: InvalidFormat, InvalidLength)
Elixir        {:error, :invalid_format} | {:error, :invalid_length}
```

Functions that can fail:
- `parse()` — raises/returns error on invalid input
- `v5()` — never fails (SHA-1 always succeeds)
- `v1()`, `v4()`, `v7()` — can fail only if the OS CSPRNG is unavailable
  (treat as panic/fatal in that case — no UUID library can function)

---

## Language-Specific Implementation Notes

### Python

```python
# src/uuid_lib/__init__.py   (package name: coding-adventures-uuid)
#
# Uses:
#   os.urandom(16)          -- CSPRNG
#   hashlib.sha1()          -- SHA-1 for v5
#   time.time_ns() // 100   -- 100ns timestamp for v1
#   uuid module is NOT used -- zero stdlib UUID dependency
#
# The UUID class is a @dataclass with bytes: bytes field.
# Implement __str__, __eq__, __hash__, __lt__ for full comparison support.
```

### TypeScript

```typescript
// src/index.ts   (package: @coding-adventures/uuid)
//
// Uses:
//   crypto.getRandomValues()  -- CSPRNG (Web Crypto API, available in Node.js)
//   crypto.subtle.digest("SHA-1", ...)  -- SHA-1 for v5 (async -> sync wrapper)
//   Date.now()                -- millisecond timestamp for v7
//   performance.now()         -- sub-millisecond precision for v1
//
// UUID is a class. All static methods (v4, v5, v7, v1, parse, isValid).
// toString() returns the canonical hyphenated form.
// Implement Symbol.toPrimitive for string coercion.
//
// Note: crypto.subtle.digest is async in the browser but we can wrap it
// synchronously in Node.js using the sync variant or WebCrypto polyfill.
// Alternative: use a pure-JS SHA-1 implementation (just ~30 lines).
```

### Ruby

```ruby
# lib/coding_adventures_uuid.rb
#
# Uses:
#   SecureRandom.random_bytes(16)  -- CSPRNG
#   Digest::SHA1                   -- SHA-1 for v5
#   Process.clock_gettime(Process::CLOCK_REALTIME, :nanosecond)  -- for v1
#
# UUID is a module with a UUID class inside.
# Implement to_s, ==, <=>, hash (Comparable module).
# UUID::Error < StandardError for errors.
# Boolean methods use ? suffix: valid?, nil?, max?
```

### Go

```go
// Package uuid   (module: github.com/adhithyan15/coding-adventures/uuid)
//
// Uses:
//   crypto/rand.Read()    -- CSPRNG
//   crypto/sha1           -- SHA-1 for v5
//   time.Now().UnixNano() -- nanosecond timestamp
//   net.Interfaces()      -- MAC address for v1
//
// UUID is a [16]byte type alias.
// Methods: Version() int, Variant() string, IsNil() bool, IsMax() bool,
//          String() string, Hex() string, URN() string, IntValue() *big.Int
// Functions: V1(), V4(), V5(ns UUID, name string), V7(), Parse(s string),
//            IsValid(s string) bool
// Errors returned as (UUID, error) pairs.
```

### Rust

```rust
// src/lib.rs   (crate: coding-adventures-uuid)
//
// Uses:
//   getrandom crate OR OsRng from rand crate -- CSPRNG
//   sha1 crate (no_std compatible)           -- SHA-1 for v5
//   std::time::SystemTime                     -- timestamps
//
// Uuid is a struct wrapping [u8; 16].
// Implement Display, PartialEq, Eq, PartialOrd, Ord, Hash.
// UuidError enum with InvalidFormat and InvalidLength variants.
// All parse/generation functions return Result<Uuid, UuidError>.
//
// Note: The rand and getrandom crates are the only external dependencies
// permitted. SHA-1 can be implemented in ~50 lines inline to avoid the
// sha1 crate dependency (pure algorithm, no crypto security requirements).
```

### Elixir

```elixir
# lib/uuid.ex   (app: coding_adventures_uuid)
#
# Uses:
#   :crypto.strong_rand_bytes(16)          -- CSPRNG
#   :crypto.hash(:sha, data)               -- SHA-1 for v5
#   System.os_time(:millisecond)           -- millisecond timestamp for v7
#   System.os_time(:nanosecond) div 100    -- 100ns timestamp for v1
#
# UUID is represented as a 16-byte binary (<<byte1, byte2, ...>>).
# Module functions: v1/0, v4/0, v5/2, v7/0, parse/1, valid?/1,
#                   to_string/1, version/1, variant/1,
#                   nil_uuid/0, max_uuid/0
# Errors returned as {:ok, uuid} | {:error, :invalid_format}
# Namespace constants defined as module attributes.
```

---

## Testing Strategy

All tests target **95%+ line coverage**. Each version has unit tests,
and there is a shared set of cross-version and round-trip tests.

### Unit Tests: v4()

1. **Returns a UUID**: `v4()` returns a UUID object (not nil, not error)
2. **Version is 4**: `v4().version == 4`
3. **Variant is rfc4122**: `v4().variant == "rfc4122"`
4. **Unique**: two calls to `v4()` produce different UUIDs
5. **16 bytes**: internal bytes length is exactly 16
6. **Version nibble**: byte[6] high nibble == 0x4
7. **Variant bits**: byte[8] high 2 bits == 0b10
8. **String format**: `str(v4())` matches the regex `^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$`

### Unit Tests: v7()

9.  **Returns a UUID**: `v7()` returns a valid UUID
10. **Version is 7**: `v7().version == 7`
11. **Variant is rfc4122**: `v7().variant == "rfc4122"`
12. **Monotonic within same ms**: rapid calls in the same ms may share timestamp prefix (non-deterministic -- just check uniqueness)
13. **Ordering**: `v7()` followed immediately by another `v7()` satisfies `u1 <= u2` (bytes comparison)
14. **Timestamp embedded**: top 48 bits of a v7 UUID correspond to a timestamp within ±1 second of now
15. **String format**: matches `^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$`

### Unit Tests: v5()

16. **Deterministic**: `v5(NAMESPACE_DNS, "python.org") == v5(NAMESPACE_DNS, "python.org")`
17. **Different names**: `v5(NAMESPACE_DNS, "a") != v5(NAMESPACE_DNS, "b")`
18. **Different namespaces**: `v5(NAMESPACE_DNS, "x") != v5(NAMESPACE_URL, "x")`
19. **Version is 5**: `v5(...).version == 5`
20. **Variant is rfc4122**: `v5(...).variant == "rfc4122"`
21. **RFC test vector (DNS + "python.org")**: `v5(NAMESPACE_DNS, "python.org") == UUID("886313e1-3b8a-5372-9b90-0c9aee199e5d")`
22. **RFC test vector (URL + "http://www.widgets.com/")**: `v5(NAMESPACE_URL, "http://www.widgets.com/") == UUID("21f7f8de-8051-5b89-8680-0195ef798b6a")`
23. **Empty name**: `v5(NAMESPACE_DNS, "")` returns a valid UUID (not error)
24. **Unicode name**: `v5(NAMESPACE_DNS, "日本語")` works (UTF-8 encoded)

### Unit Tests: v1()

25. **Returns a UUID**: `v1()` returns a valid UUID
26. **Version is 1**: `v1().version == 1`
27. **Variant is rfc4122**: `v1().variant == "rfc4122"`
28. **Unique across rapid calls**: 100 calls to `v1()` yield 100 distinct UUIDs
29. **Timestamp is recent**: top 60 bits decode to a time within ±10 seconds of now
30. **String format**: matches `^[0-9a-f]{8}-[0-9a-f]{4}-1[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$`

### Unit Tests: parse()

31. **Canonical form**: `parse("550e8400-e29b-41d4-a716-446655440000")` succeeds
32. **Uppercase**: `parse("550E8400-E29B-41D4-A716-446655440000")` succeeds
33. **Compact form**: `parse("550e8400e29b41d4a716446655440000")` succeeds
34. **Braces form**: `parse("{550e8400-e29b-41d4-a716-446655440000}")` succeeds
35. **URN form**: `parse("urn:uuid:550e8400-e29b-41d4-a716-446655440000")` succeeds
36. **Leading/trailing whitespace**: `parse("  550e8400-...  ")` succeeds
37. **Case-insensitive output**: result of `parse("UPPER-...")` converts back to lowercase
38. **Nil UUID**: `parse("00000000-0000-0000-0000-000000000000").is_nil == True`
39. **Max UUID**: `parse("ffffffff-ffff-ffff-ffff-ffffffffffff").is_max == True`
40. **Invalid: too short**: raises UUIDError
41. **Invalid: too long**: raises UUIDError
42. **Invalid: non-hex chars**: raises UUIDError
43. **Invalid: wrong hyphen positions**: raises UUIDError (if strict mode -- see notes)
44. **Empty string**: raises UUIDError
45. **Round-trip**: `parse(str(v4())) == v4()` ... i.e., `u = v4(); parse(str(u)) == u`

### Unit Tests: is_valid()

46. **Valid canonical**: `is_valid("550e8400-e29b-41d4-a716-446655440000") == True`
47. **Valid compact**: `is_valid("550e8400e29b41d4a716446655440000") == True`
48. **Valid URN**: `is_valid("urn:uuid:...") == True`
49. **Invalid string**: `is_valid("not-a-uuid") == False`
50. **Empty string**: `is_valid("") == False`
51. **Almost valid (35 chars)**: `is_valid("550e8400-e29b-41d4-a716-44665544000")` == False
52. **Non-string input** (where type system allows): `is_valid(None)` == False

### Unit Tests: Inspection

53. **version on v4**: `parse("550e8400-e29b-41d4-a716-446655440000").version == 4`
54. **variant on rfc4122**: byte[8] in [0x80..0xBF] -> "rfc4122"
55. **is_nil**: `NIL.is_nil == True`, `v4().is_nil == False`
56. **is_max**: `MAX.is_max == True`, `v4().is_max == False`
57. **hex**: `str` without hyphens is 32 lowercase hex chars
58. **urn**: begins with "urn:uuid:" followed by canonical form
59. **int_value on NIL**: `NIL.int_value == 0`
60. **int_value on MAX**: `MAX.int_value == 2**128 - 1`

### Unit Tests: Comparison and Hashing

61. **Equality**: `parse("550e8400-...") == parse("550e8400-...")` (same bytes)
62. **Inequality**: `v4() != v4()` (almost certainly)
63. **Ordering nil < v7**: `NIL < v7()` (nil is all zeros)
64. **Ordering v7 sequence**: 10 v7 UUIDs generated in sequence are sorted when sorted by `<`
65. **Hash consistency**: `hash(u) == hash(parse(str(u)))` (equal UUIDs have equal hash)
66. **Set dedup**: two equal UUIDs deduplicate in a set
67. **Dict key**: UUID can be used as a dictionary/map key

### Unit Tests: Namespace Constants

68. **NAMESPACE_DNS value**: equals `UUID("6ba7b810-9dad-11d1-80b4-00c04fd430c8")`
69. **NAMESPACE_URL value**: equals `UUID("6ba7b811-9dad-11d1-80b4-00c04fd430c8")`
70. **NAMESPACE_OID value**: equals `UUID("6ba7b812-9dad-11d1-80b4-00c04fd430c8")`
71. **NAMESPACE_X500 value**: equals `UUID("6ba7b814-9dad-11d1-80b4-00c04fd430c8")`

### Unit Tests: Formatting

72. **str() lowercase**: `str(u)` has no uppercase hex
73. **str() hyphenated**: `str(u)` matches `[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}`
74. **hex property**: 32 chars, no hyphens, lowercase
75. **urn property**: starts with "urn:uuid:" and rest is canonical form

### Round-Trip Tests

76. **v4 round-trip**: `parse(str(v4()))` succeeds and equals original
77. **v5 round-trip**: `parse(str(v5(NAMESPACE_DNS, "test")))` succeeds
78. **v7 round-trip**: `parse(str(v7()))` succeeds and equals original
79. **v1 round-trip**: `parse(str(v1()))` succeeds and equals original
80. **NIL round-trip**: `parse(str(NIL)) == NIL`
81. **MAX round-trip**: `parse(str(MAX)) == MAX`

### Coverage Target

95%+ line coverage in all six languages.

---

## Package Structure

Each language follows the monorepo's standard src layout:

```
code/packages/python/uuid/
  src/coding_adventures_uuid/
    __init__.py        -- UUID class, generate, parse, validate, constants
    _csprng.py         -- OS randomness wrapper
    _sha1.py           -- pure-Python SHA-1 (or hashlib delegate)
    _v1.py             -- v1 generation, clock-sequence state
  tests/
    test_uuid.py
  pyproject.toml
  BUILD
  CHANGELOG.md
  README.md

code/packages/typescript/uuid/
  src/
    index.ts           -- UUID class + all public exports
    sha1.ts            -- SHA-1 for v5 (sync, no external dep)
    v1.ts              -- v1 generation, clock-sequence state
  tests/
    uuid.test.ts
  package.json
  BUILD
  CHANGELOG.md
  README.md

code/packages/ruby/uuid/
  lib/
    coding_adventures_uuid.rb
    coding_adventures_uuid/
      uuid.rb
      error.rb
      namespaces.rb
      v1.rb
  spec/
    uuid_spec.rb
  Gemfile
  coding_adventures_uuid.gemspec
  BUILD
  CHANGELOG.md
  README.md

code/packages/go/uuid/
  uuid.go              -- UUID type, all functions
  v1.go                -- v1 generation, clock-sequence state
  sha1.go              -- SHA-1 helper for v5
  uuid_test.go
  go.mod
  BUILD
  CHANGELOG.md
  README.md

code/packages/rust/uuid/
  src/
    lib.rs             -- UUID struct, generation, parse, compare
    v1.rs              -- v1 generation, clock-sequence state
    sha1.rs            -- inline SHA-1 for v5
    error.rs           -- UuidError enum
  tests/
    uuid_test.rs
  Cargo.toml
  BUILD
  CHANGELOG.md
  README.md

code/packages/elixir/uuid/
  lib/
    coding_adventures_uuid.ex    -- UUID module, all public functions
    coding_adventures_uuid/
      v1.ex                       -- v1 generation
      namespaces.ex               -- namespace constants
  test/
    coding_adventures_uuid_test.exs
  mix.exs
  BUILD
  CHANGELOG.md
  README.md
```

---

## Trade-Offs

| Decision | Pro | Con |
|----------|-----|-----|
| Internal bytes representation | Portable, efficient, canonical | Extra conversion for string I/O |
| v5 not v3 (SHA-1 not MD5) | SHA-1 is stronger; MD5 is cryptographically broken | MD5 is still valid per RFC 4122; some systems generate v3 |
| v7 over v4 for DB keys | Time-ordered; better B-tree index locality | Slightly more complex implementation |
| Inline SHA-1 (Rust) | Zero external crate dependency | ~50 lines of boilerplate |
| Random node ID for v1 | No MAC address leak, simpler sandboxed env | Less stable across processes |
| Accept multiple parse formats | User-friendly | Slightly more complex parser |
| v3 (MD5) included | Compatible with systems generating v3 UUIDs | MD5 is broken; v3 must not be used for security |

---

## Future Extensions

- **v3 (MD5-based)**: add if consumers need compatibility with MD5-based UUIDs
- **UUID database type**: helper to store/retrieve UUID as 16-byte BLOB in SQLite
- **UUID short form**: base58 or base32 encoding for human-friendly display
- **Batch generation**: generate N unique UUIDs atomically (useful for bulk inserts)
- **CLI tool**: `uuid v4`, `uuid v7`, `uuid v5 dns python.org` from the command line
