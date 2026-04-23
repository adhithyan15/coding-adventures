# NET06 — DNS Message Codec

## Overview

DNS (Domain Name System) is the internet's phone book. Humans remember names
like `info.cern.ch`; computers route packets to numeric IP addresses like
`188.184.21.108` or `2001:1458:d00:3c::100:47`. A DNS message is the wire-level
question or answer that bridges those two worlds.

This package specifies a **transport-agnostic DNS message codec**:

- build DNS queries in memory
- serialize them to the RFC 1035 wire format
- parse DNS responses from raw bytes
- expose structured questions, headers, and resource records

It deliberately does **not** open sockets or talk to a nameserver directly.
That separation matters. DNS runs mostly over UDP, sometimes over TCP, and may
later run over TLS or HTTPS. The message format should not care which transport
carried it.

**Analogy:** DNS is like sending an index card to an information desk:

```
Question card:
  "What is the address for info.cern.ch?"

Answer card:
  "info.cern.ch is 188.184.21.108"
  "This answer is valid for 300 seconds"
```

This package defines the shape of the card, not the mail truck that delivers
it.

## Where It Fits

```
User enters "http://info.cern.ch/"
     │
     ▼
url-parser (NET00)
     │  host = "info.cern.ch"
     ▼
future dns-client (NET07 or later)
     │
     ├── dns-message (NET06) ← THIS PACKAGE
     │     build query bytes / parse answer bytes
     │
     └── future udp-client
           send query to recursive resolver on port 53
     ▼
tcp-client (NET01)
     │  connect to resolved IP address
     ▼
http1.0-client (NET05)
```

For now, `tcp-client` still uses the operating system resolver. NET06 is the
first step toward replacing that with a real cross-language DNS stack.

**Depends on:** nothing (std only)
**Depended on by:** future DNS resolver/client packages, browser networking
stack, any program that needs to encode or decode DNS messages

---

## Concepts

### 1. DNS Is a Question-and-Answer Protocol

At its core, a DNS exchange is tiny:

```
Client asks:
  name  = "info.cern.ch"
  type  = A
  class = IN

Server answers:
  name  = "info.cern.ch"
  type  = A
  class = IN
  ttl   = 300
  data  = 188.184.21.108
```

The first version of this package focuses on exactly that shape:

- **standard query** messages (`opcode = QUERY`)
- **Internet class** (`IN`)
- record types needed by a browser-first stack:
  - `A`     — IPv4 address
  - `AAAA`  — IPv6 address
  - `CNAME` — alias to another name

Unknown record types are still preserved as raw bytes so the parser stays
forward-compatible.

### 2. Every DNS Message Starts with a 12-Byte Header

The DNS header is fixed-size and always comes first:

```
0                   1                   2                   3
0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-------------------------------+-------------------------------+
|              ID               |QR| Opcode |AA|TC|RD|RA| Z |RC|
+-------------------------------+-------------------------------+
|           QDCOUNT             |           ANCOUNT             |
+-------------------------------+-------------------------------+
|           NSCOUNT             |           ARCOUNT             |
+-------------------------------+-------------------------------+
```

Meaning:

- `ID` — request/response correlation token
- `QR` — 0 for query, 1 for response
- `Opcode` — the operation; v1 supports only standard query (`0`)
- `AA` — authoritative answer
- `TC` — truncated
- `RD` — recursion desired
- `RA` — recursion available
- `RCODE` — result code (`NOERROR`, `NXDOMAIN`, etc.)
- `QDCOUNT` — number of questions
- `ANCOUNT` — number of answer records
- `NSCOUNT` — number of authority records
- `ARCOUNT` — number of additional records

This package parses and serializes all header fields, even when the first
version only acts on a subset of them.

### 3. Domain Names Are Encoded as Length-Prefixed Labels

DNS does not send names as `"info.cern.ch"` text with dots. It sends labels:

```
"info.cern.ch"

becomes

04 'i' 'n' 'f' 'o'
04 'c' 'e' 'r' 'n'
02 'c' 'h'
00
```

Rules:

- each label is at most 63 bytes
- the full encoded name is at most 255 bytes
- the name ends with a zero-length root label
- the root domain itself is just `00`

The public API should expose names in a structured way, not as raw on-the-wire
bytes. The wire encoding is an implementation detail.

### 4. DNS Messages Have Four Sections

After the header, the message contains up to four sections:

1. **Question** — what is being asked
2. **Answer** — the direct answer
3. **Authority** — which server is authoritative
4. **Additional** — extra helpful records, such as glue

For the first resolver slice, the **Question** and **Answer** sections matter
most. But the parser should still decode all four sections so we do not need to
redesign the API later.

### 5. Responses Commonly Use Name Compression

DNS reuses repeated names through pointers. Instead of writing the same suffix
again, a message can say "jump back to byte 12 and continue reading there."

Example:

```
Question name at byte 12:
  04 info 04 cern 02 ch 00

Answer name:
  C0 0C

Meaning:
  top two bits = 11 → compression pointer
  remaining 14 bits = 0x000C = byte offset 12
```

Compression is one of the trickiest parts of DNS parsing, so the first version
must handle it correctly:

- parser accepts legal compression pointers in names
- serializer for **queries** does not use compression in v1
- serializer for general messages may add compression later
- pointer loops and out-of-bounds jumps are parse errors

This keeps query generation simple without giving up real-world compatibility on
responses.

### 6. DNS Has Protocol Semantics Above Raw Parsing

A DNS message parser should not stop at "bytes parsed successfully." Some
message states are semantically meaningful:

- `RCODE = NXDOMAIN` means "the name does not exist"
- `TC = 1` means "this UDP response was truncated"
- `ANCOUNT = 0` with `NOERROR` often means "no records of that type"
- a response can contain a `CNAME` chain before the final `A` or `AAAA`

The codec does not perform network retries or TCP fallback, but it must expose
these protocol facts clearly so a future client can make the right decision.

### 7. The First Version Is Deliberately Narrow

DNS as deployed today is large:

- `MX`, `TXT`, `NS`, `SOA`, `PTR`, `SRV`, `HTTPS`, `SVCB`, `CAA`, ...
- EDNS(0)
- DNSSEC
- zone transfers
- dynamic updates
- IDNA / punycode concerns
- TCP fallback
- caching and TTL expiration

NET06 is not trying to solve all of DNS. It defines the smallest reusable
message layer that still feels like real DNS, not a toy.

---

## Public API

The examples below use Rust syntax because Rust is the primary implementation
language for the browser networking stack. Other language ports should expose
the same conceptual model.

### Names

```rust
/// A DNS domain name expressed as human-meaningful labels.
///
/// Examples:
/// - ["info", "cern", "ch"]
/// - ["localhost"]
/// - []  // the root domain
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DnsName {
    pub labels: Vec<String>,
}

impl DnsName {
    /// Parse a dotted ASCII name such as "info.cern.ch" or ".".
    pub fn from_ascii(input: &str) -> Result<Self, DnsError>;

    /// Return true when this is the root name (`.`).
    pub fn is_root(&self) -> bool;
}
```

The first version only requires ASCII labels. Internationalized names can be
added later through punycode at a higher layer. Rust implementations should
also implement `Display`, which gives callers the standard `.to_string()` helper
without requiring a custom inherent method.

### Header Fields

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsOpcode {
    Query,
    Unknown(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsResponseCode {
    NoError,
    FormatError,
    ServerFailure,
    NameError,       // NXDOMAIN
    NotImplemented,
    Refused,
    Unknown(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsFlags {
    pub is_response: bool,
    pub opcode: DnsOpcode,
    pub authoritative_answer: bool,
    pub truncated: bool,
    pub recursion_desired: bool,
    pub recursion_available: bool,
    pub response_code: DnsResponseCode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsHeader {
    pub id: u16,
    pub flags: DnsFlags,
    pub question_count: u16,
    pub answer_count: u16,
    pub authority_count: u16,
    pub additional_count: u16,
}
```

### Questions

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsRecordType {
    A,
    NS,
    CNAME,
    SOA,
    PTR,
    MX,
    TXT,
    AAAA,
    Unknown(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsClass {
    IN,
    Unknown(u16),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsQuestion {
    pub name: DnsName,
    pub qtype: DnsRecordType,
    pub qclass: DnsClass,
}
```

Even though v1 primarily targets `A`, `AAAA`, and `CNAME`, the type enum should
reserve the common numeric codes now so future expansion is additive instead of
breaking.

### Resource Records

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsRecordData {
    A([u8; 4]),
    AAAA([u8; 16]),
    CNAME(DnsName),

    /// Any record type this version does not interpret yet.
    Raw(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsResourceRecord {
    pub name: DnsName,
    pub rrtype: DnsRecordType,
    pub class: DnsClass,
    pub ttl: u32,
    pub data: DnsRecordData,
}
```

### Whole Messages

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsMessage {
    pub header: DnsHeader,
    pub questions: Vec<DnsQuestion>,
    pub answers: Vec<DnsResourceRecord>,
    pub authorities: Vec<DnsResourceRecord>,
    pub additionals: Vec<DnsResourceRecord>,
}
```

### Core Functions

```rust
/// Parse raw DNS message bytes into a structured message.
pub fn parse_dns_message(input: &[u8]) -> Result<DnsMessage, DnsError>;

/// Serialize a structured message into DNS wire-format bytes.
///
/// V1 guarantees query serialization. General response serialization is
/// supported when the message only contains record kinds this version knows how
/// to encode.
pub fn serialize_dns_message(message: &DnsMessage) -> Result<Vec<u8>, DnsError>;

/// Build a standard single-question query suitable for transport over UDP.
///
/// Defaults:
/// - opcode = Query
/// - QR = query
/// - RD = true
/// - QDCOUNT = 1
/// - no answers / authorities / additionals
pub fn build_query(id: u16, name: DnsName, qtype: DnsRecordType) -> DnsMessage;
```

### Convenience Helpers

```rust
impl DnsMessage {
    /// True when this is a response with `RCODE = NOERROR`.
    pub fn is_success(&self) -> bool;

    /// Return the first answer of the requested type, if present.
    pub fn first_answer_of_type(&self, qtype: DnsRecordType)
        -> Option<&DnsResourceRecord>;

    /// Return all IPv4 addresses in the answer section.
    pub fn ipv4_answers(&self) -> Vec<[u8; 4]>;

    /// Return all IPv6 addresses in the answer section.
    pub fn ipv6_answers(&self) -> Vec<[u8; 16]>;
}
```

These helpers are intentionally small. Caching policy, CNAME chasing policy,
negative caching, and transport retry behavior belong in a future DNS client
layer, not the codec.

### Error Types

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsError {
    /// Fewer than 12 bytes — not even a complete header.
    TruncatedHeader,

    /// The message ended unexpectedly while parsing a name, question, or record.
    UnexpectedEof,

    /// A label length exceeded 63 bytes.
    LabelTooLong { length: usize },

    /// The full encoded name exceeded 255 bytes.
    NameTooLong,

    /// A compression pointer referenced an invalid offset.
    PointerOutOfBounds { offset: usize },

    /// Compression pointers formed a cycle.
    PointerLoop,

    /// A label could not be represented by this version's ASCII name model.
    NonAsciiLabel,

    /// The message declared counts that do not match the available bytes.
    InvalidSectionCount,

    /// The message uses an unsupported structure for this version.
    Unsupported(&'static str),
}
```

The parser should prefer specific structural errors over vague "invalid input"
messages. DNS debugging is much easier when the failure says *why*.

---

## Encoding and Parsing Rules

### 1. Query Builder Rules

`build_query()` in v1 should:

- create exactly one question
- set `QR = 0`
- set `Opcode = Query`
- set `RD = true`
- set all counts consistently
- encode names without compression
- use class `IN`

This produces the classic recursive resolver query shape used by almost every
stub resolver.

### 2. Name Parsing Rules

The name parser must:

- decode ordinary label sequences
- decode compression pointers
- allow names that mix inline labels and one pointer suffix
- reject pointers to offsets beyond the message length
- reject pointer cycles
- preserve the distinction between the root name and non-root names

### 3. Record Data Rules

For this first version:

- `A` record data length must be exactly 4 bytes
- `AAAA` record data length must be exactly 16 bytes
- `CNAME` record data is itself a DNS name and may use compression
- unknown record types are kept as `Raw(Vec<u8>)`

This lets the parser be strict where structure is known and permissive where
future record types may appear.

### 4. Counts Are Trusted Only After Verification

The header's `QDCOUNT`, `ANCOUNT`, `NSCOUNT`, and `ARCOUNT` tell the parser how
many entries should appear in each section. But a malformed or malicious packet
can lie.

The parser must:

- use the counts as the intended section sizes
- verify enough bytes remain for each parsed element
- fail with `UnexpectedEof` or `InvalidSectionCount` rather than reading past
  the end

### 5. Truncation Is Exposed, Not Hidden

If `TC = 1`, the parser still returns the message successfully if the message is
otherwise structurally valid. Truncation is **protocol information**, not
necessarily a parse failure. A future DNS client may decide to retry over TCP.

---

## Testing Strategy

### 1. Query Serialization

1. **A query:** `build_query(0x1234, "info.cern.ch", A)` serializes to the
   expected header and question bytes.
2. **AAAA query:** same shape with `QTYPE = 28`.
3. **Root query:** `.` encodes as a single zero byte.
4. **Single-label query:** `localhost` encodes correctly.

### 2. Header Parsing

5. **Response flags:** parse `QR`, `AA`, `TC`, `RD`, `RA`, and `RCODE`.
6. **NXDOMAIN response:** `RCODE = NameError`.
7. **Unknown opcode / rcode:** preserved as `Unknown(...)`.

### 3. Name Parsing

8. **Ordinary name:** parse a plain uncompressed label sequence.
9. **Compressed answer name:** parse `C0 0C` pointer to the question name.
10. **Mixed name:** inline label followed by pointer suffix.
11. **Pointer loop:** detect and error.
12. **Pointer out of bounds:** detect and error.
13. **Label too long:** reject labels > 63 bytes.
14. **Name too long:** reject names > 255 bytes.

### 4. Resource Records

15. **A record:** parse IPv4 address bytes.
16. **AAAA record:** parse IPv6 address bytes.
17. **CNAME record:** parse compressed canonical-name target.
18. **Unknown record type:** preserved as raw data.
19. **Wrong A length:** reject non-4-byte payload.
20. **Wrong AAAA length:** reject non-16-byte payload.

### 5. Whole-Message Parsing

21. **Simple response:** one question, one A answer.
22. **CNAME chain response:** one CNAME answer plus one A answer.
23. **Authority and additional sections:** parse all four sections correctly.
24. **Truncated but well-formed response:** parse successfully with
    `header.flags.truncated = true`.
25. **Unexpected EOF:** fail when the bytes end mid-record.

### 6. Round-Trip Tests

26. **Query round-trip:** build query → serialize → parse → recover equivalent
    structured query.
27. **Known response round-trip:** parse structured response, serialize, parse
    again for record kinds this version can encode.

### 7. Real-World Fixtures

28. **Captured recursive resolver response:** `A` answer for a public domain.
29. **Captured response with compression:** ensure realistic pointer layouts are
    accepted.
30. **Negative response fixture:** `NXDOMAIN` or empty `NOERROR` answer set.

All fixtures should be byte-for-byte local test data. The package tests do not
need live network access.

---

## Scope

### In Scope

- RFC 1035 DNS message header parsing and serialization
- DNS name encoding and decoding
- compression-pointer parsing
- single-question query construction
- full-section message parsing (question, answer, authority, additional)
- `A`, `AAAA`, and `CNAME` typed record decoding
- raw preservation for unknown record types
- protocol flags and response codes

### Out of Scope

- opening sockets
- UDP or TCP transport
- resolver retry policy
- TCP fallback on truncation
- hosts-file integration
- search domains
- caching and TTL expiration
- IDNA / Unicode hostname handling
- EDNS(0)
- DNSSEC
- zone transfers
- dynamic update
- mDNS

### Future Follow-On Specs

1. **DNS client / resolver**
   Build on NET06 plus a future UDP client to issue recursive queries to a real
   nameserver.

2. **Caching resolver**
   Add TTL-aware cache entries, negative caching, and in-flight query collapse.

3. **Transport expansion**
   Add TCP framing for truncated responses and later optional DoT / DoH layers.

4. **Record expansion**
   Add typed support for `NS`, `SOA`, `MX`, `TXT`, `SRV`, `PTR`, and newer web
   records.

5. **Browser integration**
   Teach `tcp-client` or a higher orchestration layer to use the DNS client
   instead of the OS resolver when desired.

---

## Implementation Languages

- **Rust** (primary, for Venture and the networking stack)
- **Go**
- **Python**
- **TypeScript**
- **Ruby**
- **Elixir**
- **Perl**
- **Lua**
- **Swift**

The message model should stay conceptually identical across languages even if
individual ports adapt to local idioms.
