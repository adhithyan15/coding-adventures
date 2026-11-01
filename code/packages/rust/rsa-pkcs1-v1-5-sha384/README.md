# rsa-pkcs1-v1-5-sha384

Allocation-free validation of the exact RFC 8017 EMSA-PKCS1-v1_5 encoded
message for SHA-384.

The caller supplies the complete modulus-width output from a separate RSA
public operation and the expected 48-byte digest. The crate accepts only
`00 01`, at least eight `ff` padding bytes, the zero separator, the canonical
DER SHA-384 `DigestInfo` with present NULL parameters, and the complete
matching digest.

```rust
use rsa_pkcs1_v1_5_sha384::validate_sha384_encoded_message;

let digest = [0x42; 48];
let mut encoded = vec![0x00, 0x01];
encoded.extend_from_slice(&[0xff; 8]);
encoded.push(0x00);
encoded.extend_from_slice(&[
    0x30, 0x41, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01,
    0x65, 0x03, 0x04, 0x02, 0x02, 0x05, 0x00, 0x04, 0x30,
]);
encoded.extend_from_slice(&digest);

validate_sha384_encoded_message(&encoded, &digest).unwrap();
```

## Security boundary

This package performs no RSA arithmetic, hashing, key or certificate parsing,
algorithm routing, signature verdict, path validation, trust-root selection,
revocation, TLS, HTTP, OAuth authorization, credential access, signing, I/O,
or external effect. Arbitrary caller-supplied bytes do not become an
authenticated signature merely because their formatting is valid. A later
composition must bind this gate to the exact RSA public-operation result and
repository-owned SHA-384 digest for the exact signed bytes.

## Development

```bash
bash BUILD
cargo clippy -p rsa-pkcs1-v1-5-sha384 --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p rsa-pkcs1-v1-5-sha384 --no-deps
```
