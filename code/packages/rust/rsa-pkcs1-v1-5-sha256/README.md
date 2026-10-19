# rsa-pkcs1-v1-5-sha256

Allocation-free validation of the exact RFC 8017 EMSA-PKCS1-v1_5 encoded
message for SHA-256.

The caller supplies the complete fixed-width output from a separate RSA public
operation and the expected 32-byte digest. The crate accepts only `00 01`, at
least eight `ff` padding bytes, the zero separator, the canonical DER SHA-256
`DigestInfo` with present NULL parameters, and the complete matching digest.

```rust
use rsa_pkcs1_v1_5_sha256::validate_sha256_encoded_message;

let digest = [0x42; 32];
let mut encoded = vec![0x00, 0x01];
encoded.extend_from_slice(&[0xff; 8]);
encoded.push(0x00);
encoded.extend_from_slice(&[
    0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01,
    0x65, 0x03, 0x04, 0x02, 0x01, 0x05, 0x00, 0x04, 0x20,
]);
encoded.extend_from_slice(&digest);

validate_sha256_encoded_message(&encoded, &digest).unwrap();
```

## Boundary

This package does not perform RSA arithmetic, parse keys or certificates, hash
messages, choose algorithms, construct paths, select trust roots, check
revocation, run TLS, or open sockets. Arbitrary caller-supplied encoded bytes
do not become an authenticated signature merely because their formatting is
valid.

## Development

```bash
# Run tests
bash BUILD
```
