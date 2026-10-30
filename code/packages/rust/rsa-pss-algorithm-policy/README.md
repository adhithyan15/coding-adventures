# rsa-pss-algorithm-policy

Data-driven policy validation for already-decoded RFC 8017 RSA-PSS
verification parameters.

The caller supplies an explicit allowlist of message-hash, MGF1-hash, and salt
length profiles plus a salt-work ceiling. Validation requires trailer field 1,
applies the ceiling, then requires exact profile membership. SHA-256, SHA-384,
and SHA-512 are represented without silently coupling the message and MGF1
hash choices; the allowlist owns that decision.

```rust
use rsa_pss_algorithm_policy::{
    validate_rsa_pss_algorithm_parameters, RsaPssAlgorithmParameters,
    RsaPssAlgorithmPolicy, RsaPssAlgorithmProfile, RsaPssDigestAlgorithm,
};

let profile = RsaPssAlgorithmProfile::new(
    RsaPssDigestAlgorithm::Sha256,
    RsaPssDigestAlgorithm::Sha256,
    32,
);
let profiles = [profile];
let policy = RsaPssAlgorithmPolicy::new(&profiles, 64).unwrap();
let parameters = RsaPssAlgorithmParameters::new(
    RsaPssDigestAlgorithm::Sha256,
    RsaPssDigestAlgorithm::Sha256,
    32,
    1,
);
let validated = validate_rsa_pss_algorithm_parameters(parameters, policy).unwrap();
assert_eq!(validated.profile(), profile);
```

## Boundary

This package validates already-decoded public parameters only. It does not
parse X.509 or DER AlgorithmIdentifier values, select an implementation, hash,
run MGF1, validate PSS encoding, parse or operate on an RSA key, apply RSAVP1,
or establish a signature verdict. It performs no credential, signing, storage,
clock, entropy, filesystem, network, or other external effect.

## Development

```bash
bash BUILD
```
