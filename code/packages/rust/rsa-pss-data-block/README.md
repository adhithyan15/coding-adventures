# rsa-pss-data-block

Bounded, data-driven validation of one public unmasked RFC 8017 EMSA-PSS data
block.

The caller supplies the selected salt length and a data-block byte ceiling.
After checking those bounds, the validator requires the exact canonical
`DB = PS || 0x01 || salt` structure, proves every `PS` octet is zero, and
returns borrowed views of the complete block and selected salt.

```rust
use rsa_pss_data_block::{validate_rsa_pss_data_block, RsaPssDataBlockPolicy};

let policy = RsaPssDataBlockPolicy::new(4, 16).unwrap();
let data_block = [0, 0, 0, 1, 0xaa, 0xbb, 0xcc, 0xdd];
let validated = validate_rsa_pss_data_block(&data_block, policy).unwrap();
assert_eq!(validated.padding_string_length(), 3);
assert_eq!(validated.salt(), &[0xaa, 0xbb, 0xcc, 0xdd]);
```

## Boundary

This package handles only public unmasked byte structure. It does not run
MGF1, clear unused high bits, hash the signed message or `M'`, compare `H`,
parse or operate on an RSA key, apply RSAVP1, select an algorithm, or establish
a signature verdict. It performs no credential, signing, storage, clock,
entropy, filesystem, network, or other external effect.

## Development

```bash
bash BUILD
```
