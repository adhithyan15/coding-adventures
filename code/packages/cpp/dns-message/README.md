# dns-message (C++)

The **DNS wire-format layer**, header-only, pure ISO C++17. A faithful port of
the Rust [`dns-message`](../../rust/dns-message) crate, in namespace
`ca::dns_message`. It turns structured DNS questions and answers into bytes and
back; it does **not** open sockets, retry, cache, or choose a nameserver.

## The wire format (RFC 1035)

A 12-byte header (id, packed flag word, four section counts) then the question /
answer / authority / additional sections. Names are length-prefixed labels
ending in a zero byte; a length byte with top bits `11` is a **compression
pointer**. The decoder follows pointers with a visited-set and a 128-hop cap so
a malicious message can't loop it forever.

## API

```cpp
#include "dns_message.hpp"
namespace dm = ca::dns_message;

// Build + serialize
dm::Message q = dm::build_query(0x1234, dm::DnsName::from_ascii("info.cern.ch"),
                                dm::RecordType{dm::RecordType::A, 0});
std::vector<std::uint8_t> bytes = dm::serialize_message(q);

// Parse (throws dm::Error on malformed input)
dm::Message p = dm::parse_message(bytes);
for (auto &addr : p.ipv4_answers()) { /* std::array<uint8_t,4> */ }
```

- **`DnsName`** — `from_ascii` (throws), `is_root`, `to_string`, value equality.
- **Codec** — `build_query`, `parse_message` (throws `Error`),
  `serialize_message` (throws on a structurally impossible message).
- **`Message`** — `is_success`, `first_answer_of_type`, `ipv4_answers`,
  `ipv6_answers`; sections are `std::vector`s.
- **`Error`** — a `std::runtime_error` subclass carrying an `ErrorKind` and a
  `detail()` (length/offset for the parametric kinds).

Ownership is automatic (`std::vector` / `std::string`); the header-only decoder
is exercised against malformed input and is clean under ASan + UBSan.

The record-data payload is a tagged struct rather than a `std::variant` because
`CNAME` and `PTR` both carry a `DnsName`, which a variant can't distinguish.

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`.
