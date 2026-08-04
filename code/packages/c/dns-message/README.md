# dns-message (C)

The **DNS wire-format layer** in pure ISO C17. A faithful port of the Rust
[`dns-message`](../../rust/dns-message) crate: it turns structured DNS questions
and answers into bytes and back. It does **not** open sockets, retry, cache, or
choose a nameserver — which keeps it usable over UDP, TCP, a simulated stack, or
fixtures.

## The wire format (RFC 1035)

A message is a 12-byte header (id, a packed flag word, and four section counts)
followed by the question, answer, authority, and additional sections. Names are
sequences of length-prefixed labels ending in a zero byte; a length byte whose
top two bits are `11` is a **compression pointer** to an earlier offset. The
decoder follows pointers with a visited-flag array and a 128-hop cap, so a
malicious message can't loop it forever (`DNS_ERR_POINTER_LOOP` /
`DNS_ERR_POINTER_OUT_OF_BOUNDS`).

Record data is decoded for the common types (`A`, `AAAA`, `CNAME`, `PTR`,
`SRV`) and preserved verbatim (`DNS_RDATA_RAW`) for everything else.

## API

```c
#include "dns_message.h"

/* Build + serialize a query */
DnsName name;
dns_name_from_ascii("info.cern.ch", &name);
DnsMessage q;
dns_build_query(0x1234, name, dns_record_type_known(DNS_TYPE_A), &q); /* moves name */
uint8_t *bytes; size_t n;
dns_serialize_message(&q, &bytes, &n);   /* caller frees bytes */
free(bytes);
dns_message_free(&q);

/* Parse a response */
DnsMessage p;
if (dns_parse_message(input, input_len, &p).kind == DNS_OK) {
    uint8_t v4[8][4];
    size_t got = dns_message_ipv4_answers(&p, v4, 8);
    dns_message_free(&p);
}
```

- **`DnsName`** — `dns_name_from_ascii`, `_free`, `_clone`, `_is_root`,
  `_to_string`, `_equal`.
- **Codec** — `dns_build_query`, `dns_parse_message`, `dns_serialize_message`.
- **Accessors** — `dns_message_is_success`, `_first_answer_of_type`,
  `_ipv4_answers`, `_ipv6_answers`.
- **Errors** — every function returns a `DnsError { kind, message, detail }`;
  `DNS_ERR_UNSUPPORTED` carries a static `message`, the parametric kinds
  (`LABEL_TOO_LONG`, `POINTER_OUT_OF_BOUNDS`) carry a `detail`. See
  `dns_error_kind_str`.

Structured values own their heap (label strings, record arrays, raw rdata);
`dns_message_free` releases a whole message. The parser is exercised against
malformed input and is clean under AddressSanitizer + UndefinedBehaviorSanitizer.

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.
