# rsa-pss-message-prime

Bounded, allocation-free construction of the canonical RFC 8017 EMSA-PSS
message-prime value.

The caller supplies selected message-hash and salt lengths, an output ceiling,
and an exact output buffer. The encoder validates every length before writing
`M' = 0x00 00 00 00 00 00 00 00 || mHash || salt` and returns borrowed views
of the complete output and both copied fields.

```rust
use rsa_pss_message_prime::{
    encode_rsa_pss_message_prime, RsaPssMessagePrimePolicy,
};

let policy = RsaPssMessagePrimePolicy::new(4, 2, 14).unwrap();
let mut output = [0xff; 14];
let encoded = encode_rsa_pss_message_prime(
    &[1, 2, 3, 4],
    &[0xaa, 0xbb],
    &mut output,
    policy,
)
.unwrap();
assert_eq!(encoded.bytes(), &[0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 0xaa, 0xbb]);
```

## Boundary

This package only constructs public bytes in caller-owned memory. It does not
hash a message or `M'`, compare `H`, run MGF1, validate a PSS data block, parse
or operate on an RSA key, apply RSAVP1, select an algorithm, or establish a
signature verdict. It performs no credential, signing, storage, clock, entropy,
filesystem, network, or other external effect.

## Development

```bash
bash BUILD
```
