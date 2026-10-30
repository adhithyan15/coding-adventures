# rsa-pss-data-block-unmask

Bounded, allocation-free RFC 8017 EMSA-PSS data-block unmasking.

The caller supplies complete equal-length `maskedDB` and MGF1-mask values, a
caller-owned exact output buffer, the public unused-high-bit count derived from
`emBits`, and a data-block byte ceiling. Only after every bound passes does the
operation write the complete XOR and clear the mandated high bits.

```rust
use rsa_pss_data_block_unmask::{
    unmask_rsa_pss_data_block, RsaPssDataBlockUnmaskPolicy,
};

let policy = RsaPssDataBlockUnmaskPolicy::new(4).unwrap();
let mut output = [0xff; 4];
let unmasked = unmask_rsa_pss_data_block(
    &[0x80, 0xaa, 0x55, 0xff],
    &[0xff, 0x0f, 0xf0, 0x0f],
    &mut output,
    1,
    policy,
)
.unwrap();
assert_eq!(unmasked.data_block(), &[0x7f, 0xa5, 0xa5, 0xf0]);
```

## Boundary

This package only transforms caller-provided public verification bytes. It
does not run MGF1, validate `PS || 0x01 || salt`, hash a message or `M'`,
compare `H`, parse or operate on an RSA key, apply RSAVP1, select an algorithm,
or establish a signature verdict. It performs no credential, signing, storage,
clock, entropy, filesystem, network, or other external effect. This API makes
no constant-time guarantee; RSA-PSS verification values are public.

## Development

```bash
bash BUILD
```
