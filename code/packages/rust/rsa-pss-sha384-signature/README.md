# rsa-pss-sha384-signature

Bounded repository-owned verification for one fixed RSASSA-PSS profile:
SHA-384, MGF1-SHA-384, a 48-byte salt, and trailer field `0xbc`.

The caller supplies a previously validated RSA public key, an exact
modulus-width signature, an already-computed 48-byte message digest, exact
caller-owned DB scratch, and independent RSA and encoded-message ceilings. The
package derives `emBits = modBits - 1`, applies the repository-owned bounded RSA
public operation, and requires the complete fixed-profile EMSA-PSS verdict.

```rust
# use rsa_pss_sha384_signature::{verify_rsa_pss_sha384_signature, RsaPssSha384SignatureLimits};
# use x509_rsa_public_key::ValidatedRsaPublicKey;
# fn verify(key: ValidatedRsaPublicKey<'_>, signature: &[u8], digest: &[u8; 48]) {
let limits = RsaPssSha384SignatureLimits::new(512, 512).unwrap();
let encoded_message_length = (key.modulus_bits() - 1).div_ceil(8);
let mut data_block_scratch = vec![0u8; encoded_message_length - 48 - 1];
let evidence = verify_rsa_pss_sha384_signature(
    key,
    signature,
    digest,
    &mut data_block_scratch,
    limits,
);
# let _ = evidence;
# }
```

## Boundary

Success binds only the exact validated public key, signature bytes, and
already-computed digest passed to this operation. This package does not hash a
message, parse RSA-PSS parameters or certificates, choose an algorithm or
issuer, establish certificate roles or a path, select trust roots, check time,
names, or revocation, run TLS or HTTP, authorize OAuth transport, or perform an
external effect. Signature evidence is not certificate, HTTPS, or live-provider
readiness. All operands are public verification data; no constant-time claim is
made and no private RSA operation is exposed.

## Development

```bash
bash BUILD
```
