# ieee802154-core

From-scratch IEEE 802.15.4 MAC frame primitives.

This package is the first executable brick under the Zigbee and Thread learning
tracks. It intentionally starts with bytes: parse a MAC frame, inspect its frame
control field, decode addressing, and encode it again.

## Scope

Current scope:

- frame control field parsing
- frame type decoding
- address mode decoding
- optional sequence-number suppression
- PAN id compression for common intra-PAN frames
- short and extended address parsing
- payload extraction
- optional FCS handling

Not yet implemented:

- auxiliary security header parsing
- AES-CCM security processing
- MAC command payload semantics
- beacon payload semantics
- CSMA/CA state machines
- PHY/baseband behavior

Those belong in later D24 packages.

## Example

```rust
use ieee802154_core::{MacFrame, Address};

let bytes = [
    0x41, 0x98, // data frame, PAN compression, short dst/src, 2006 version
    0x07,       // sequence number
    0x34, 0x12, // destination PAN
    0x78, 0x56, // destination short address
    0xbc, 0x9a, // source short address
    0x01, 0x02, // payload
];

let frame = MacFrame::parse_without_fcs(&bytes).unwrap();
assert_eq!(frame.sequence_number, Some(7));
assert_eq!(frame.destination, Some(Address::Short(0x5678)));
assert_eq!(frame.source, Some(Address::Short(0x9abc)));
assert_eq!(frame.payload, vec![0x01, 0x02]);
```

## Layer Position

```text
zigbee-nwk / sixlowpan / thread-mle
    |
    v
ieee802154-core
    |
    v
ieee802154-mac / ieee802154-security / ieee802154-radio
```

The package has no hardware dependency. Radio drivers and simulators should
consume these types rather than reimplementing frame parsing.
