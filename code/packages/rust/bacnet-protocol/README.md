# bacnet-protocol

`bacnet-protocol` implements the small BACnet/IP wire boundary needed for
bounded local device discovery. It encodes unconfirmed Who-Is requests and
strictly decodes I-Am replies carried by original unicast, original broadcast,
or forwarded BACnet Virtual Link Layer frames.

The package intentionally does not implement property writes, device control,
foreign-device registration, BBMD management, or BACnet/SC. Its framing follows
the BACnet Committee's public BACnet/IP Annex J material and Who-Is/I-Am service
description:

- <https://bacnet.org/wp-content/uploads/sites/4/2022/08/Add-1995-135a.pdf>
- <https://bacnet.org/wp-content/uploads/sites/4/2022/08/Add-135-2016bz.pdf>

```rust
use bacnet_protocol::{decode_i_am, encode_who_is, WhoIsRequest};

let probe = encode_who_is(WhoIsRequest::All).unwrap();
assert_eq!(probe, [0x81, 0x0b, 0, 8, 1, 0, 0x10, 0x08]);
# let _ = decode_i_am;
```
