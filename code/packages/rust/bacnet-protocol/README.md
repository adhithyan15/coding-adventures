# bacnet-protocol

`bacnet-protocol` implements the small BACnet/IP wire boundary needed for
bounded local device discovery and Device-object inspection. It encodes
unconfirmed Who-Is requests, strictly decodes I-Am replies, and owns one fixed
confirmed ReadProperty surface for standardized Device identity and status
properties.

ReadProperty responses must correlate the invoke id, service, Device instance,
property id, application value type, and complete datagram. Character strings
are limited to 128 printable ANSI X3.4 bytes. The package intentionally exposes
no generic object or property selection and implements no property writes,
device control, foreign-device registration, BBMD management, or BACnet/SC.
Its framing follows the BACnet Committee's public BACnet/IP Annex J material,
Who-Is/I-Am service description, and Device-object overview:

- <https://bacnet.org/wp-content/uploads/sites/4/2022/08/Add-1995-135a.pdf>
- <https://bacnet.org/wp-content/uploads/sites/4/2022/08/Add-135-2016bz.pdf>
- <https://bacnet.org/wp-content/uploads/sites/4/2022/06/The-Language-of-BACnet-1.pdf>

```rust
use bacnet_protocol::{decode_i_am, encode_who_is, WhoIsRequest};

let probe = encode_who_is(WhoIsRequest::All).unwrap();
assert_eq!(probe, [0x81, 0x0b, 0, 8, 1, 0, 0x10, 0x08]);
# let _ = decode_i_am;
```
