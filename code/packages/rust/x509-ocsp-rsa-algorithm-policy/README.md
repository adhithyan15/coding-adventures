# x509-ocsp-rsa-algorithm-policy

Response-bound, data-driven OCSP RSA algorithm and key policy.

The package selects exactly one caller-supplied profile for one structurally
decoded `BasicOCSPResponse` and one separately validated RSA public key. A
profile contains:

- an exact signature algorithm OID and absent-or-exact-DER parameter rule;
- inclusive RSA modulus and signature bit-length ranges; and
- a canonical strictly increasing list of permitted odd public exponents.

Success is non-copyable evidence retaining the exact response, RSA key, and
selected profile index. The package does not select or invoke a verifier,
authenticate a response, authorize a responder, bind status to a certificate
or path, decide freshness or revocation, establish trust, implement TLS or
HTTP, perform network effects, or access credentials. It therefore does not
make concrete OAuth HTTPS or live-provider use ready.

## Example

```rust
use x509_ocsp_rsa_algorithm_policy::{
    validate_x509_ocsp_rsa_algorithm_policy,
    X509OcspRsaAlgorithmParametersPolicy,
    X509OcspRsaAlgorithmProfile,
};

# fn validate(
#     response: &x509_ocsp_basic_response::X509OcspBasicResponse<'_>,
#     key: x509_rsa_public_key::ValidatedRsaPublicKey<'_>,
# ) -> Result<(), Box<dyn std::error::Error>> {
let profiles = [X509OcspRsaAlgorithmProfile::new(
    &[1, 2, 840, 113_549, 1, 1, 13],
    X509OcspRsaAlgorithmParametersPolicy::ExactDer(&[0x05, 0x00]),
    2_048,
    4_096,
    2_048,
    4_096,
    &[65_537],
)?];
let evidence = validate_x509_ocsp_rsa_algorithm_policy(response, key, &profiles)?;
assert!(std::ptr::eq(evidence.response(), response));
# Ok(())
# }
```

## Development

```bash
cargo test -p x509-ocsp-rsa-algorithm-policy --all-targets
cargo clippy -p x509-ocsp-rsa-algorithm-policy --all-targets -- -D warnings
```
