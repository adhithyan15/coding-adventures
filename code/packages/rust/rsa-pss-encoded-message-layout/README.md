# rsa-pss-encoded-message-layout

Bounded, data-driven validation of the public outer layout of one RFC 8017
EMSA-PSS encoded message.

The caller supplies policy-selected hash and salt lengths plus an encoded-byte
ceiling. The decoder derives `emLen` from the exact public `emBits`, requires
that complete recovered width, proves the selected parameters fit, checks the
`0xbc` trailer and forbidden high bits, then returns borrowed slices for the
exact `maskedDB` and `H` fields.

```rust
use rsa_pss_encoded_message_layout::{
    decode_rsa_pss_encoded_message_layout, RsaPssEncodedMessageLayoutPolicy,
};

let policy = RsaPssEncodedMessageLayoutPolicy::new(32, 32, 256).unwrap();
let mut encoded_message = [0u8; 256];
encoded_message[255] = 0xbc;
let layout =
    decode_rsa_pss_encoded_message_layout(&encoded_message, 2047, policy).unwrap();
assert_eq!(layout.masked_data_block().len(), 223);
assert_eq!(layout.hash().len(), 32);
```

## Boundary

This package handles only public byte layout. It does not unmask or validate
the data block, run MGF1, hash the signed message or `M'`, compare `H`, parse or
operate on an RSA key, apply RSAVP1, select an algorithm, or establish a
signature verdict. It performs no credential, signing, storage, clock, entropy,
filesystem, network, or other external effect.

## Development

```bash
bash BUILD
```
