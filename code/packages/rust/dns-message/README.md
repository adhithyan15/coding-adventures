# dns-message

`dns-message` is a transport-agnostic DNS wire-format codec. It builds DNS
queries, serializes messages to bytes, parses DNS responses, decodes compressed
names, and exposes structured questions and resource records.

It intentionally does not open sockets or resolve hostnames by itself. A future
DNS client can send these bytes over UDP, TCP, a simulated stack, or a test
fixture without changing the message model.

## What It Provides

- DNS names as structured label lists
- DNS header flags and response codes
- Question and resource-record models
- Query construction for standard recursive `A`, `AAAA`, and other record types
- Wire-format serialization with uncompressed names
- Response parsing with DNS compression-pointer support
- Typed `A`, `AAAA`, and `CNAME` record data
- Raw preservation for unknown record types

## Example

```rust
use dns_message::{
    build_query, parse_dns_message, serialize_dns_message, DnsName, DnsRecordType,
};

fn main() -> Result<(), dns_message::DnsError> {
    let query = build_query(
        0x1234,
        DnsName::from_ascii("info.cern.ch")?,
        DnsRecordType::A,
    );

    let bytes = serialize_dns_message(&query)?;

    // A transport layer can send `bytes` over UDP, TCP, or a test fixture.
    let parsed = parse_dns_message(&bytes)?;
    assert_eq!(parsed.questions[0].name.to_string(), "info.cern.ch");

    Ok(())
}
```

## How It Fits The Stack

`dns-message` is the protocol-core layer for DNS:

```text
future dns-client
  ├── dns-message  // encode and parse DNS wire messages
  └── udp-client   // send and receive opaque datagrams
```

This separation keeps DNS message parsing reusable and keeps UDP free of
application-protocol knowledge.

## Development

```bash
bash BUILD
```
