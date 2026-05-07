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
- auxiliary security header parsing
- security level and key identifier modeling
- beacon payload parsing:
  superframe fields, GTS descriptors, pending addresses, and residual payload
- scan-facing PAN descriptors derived from received beacon frames
- payload extraction
- optional FCS handling

Not yet implemented:

- AES-CCM security processing
- MAC command payload semantics
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

## Security Boundary

This package parses the IEEE 802.15.4 auxiliary security header. It does not
decrypt payloads, authenticate MICs, choose keys, or enforce replay protection.

That split is intentional:

- `ieee802154-core` owns frame structure
- `ieee802154-security` will own AES-CCM, nonce construction, replay windows,
  and key lookup

Secured payload bytes remain payload bytes here. Higher layers can inspect the
security control field, frame counter, key identifier, and expected MIC length
before handing the frame to a security package.

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

## Beacon Scanning

`BeaconPayload::parse` decodes the fixed IEEE 802.15.4 beacon payload fields:
superframe specification, GTS fields, pending short/extended addresses, and the
remaining beacon payload bytes. `PanDescriptor::from_beacon_frame` combines a
received beacon frame with radio metadata such as channel, channel page, and
link quality so higher Zigbee and Thread discovery layers can rank candidates
without learning the MAC byte layout.
