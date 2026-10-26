---
category: Rust
---

# Bounded inline collections multiply every byte added to an element

Retaining one complete `Asn1Element` in `X509Extension` looked cheap in
isolation, but `X509Extensions` stores 64 elements inline and certificates are
themselves embedded in other inline response structures. The extra framing
metadata multiplied into enough stack use to overflow a normal macOS test
thread. When a composed value needs only one security property from a parsed
element, retain a compact opaque token for that property and add a footprint
regression alongside the downstream stack-sensitive test. Account for padding:
a four-byte token plus an existing Boolean can fit the old alignment slack,
while an eight-byte token still grows every array element.
