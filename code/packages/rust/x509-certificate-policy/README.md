# x509-certificate-policy

`x509-certificate-policy` supplies small RFC 5280 policy gates above the
repository-owned structural certificate decoder. Its first gate rejects the
first critical extension whose OID is not recognized by a caller-supplied
policy. Its certificate-role gate composes the repository-owned Basic
Constraints, Key Usage, and Extended Key Usage decoders under one explicit
semantic budget.

An intended issuing CA must carry critical `cA = TRUE` Basic Constraints and
Key Usage with `keyCertSign`. An intended end entity may omit those extensions,
but it must not assert either CA status or certificate-signing usage. Extended
Key Usage is decoded when present and retained as borrowed evidence without
authorizing any purpose.

## Security boundary

The recognizer is invoked only for critical extensions, in certificate wire
order. An absent extension sequence and unrecognized non-critical extensions
are accepted. A rejected critical extension fails closed with only its
zero-based sequence index; the error retains no OID or certificate bytes.

Recognition is not semantic validation. The caller must separately decode and
enforce every recognized extension according to the certificate's role. This
crate's role result proves only the extension relationships above; callers must
still run the unknown-critical-extension gate and validate every other
recognized extension. The role gate does not select algorithm-dependent TLS
key usages, authorize an Extended Key Usage purpose, or grant a certificate
role or trust. It does not validate names, paths, signatures, algorithms,
keys, validity against a clock, revocation, hostnames, trust roots, TLS, HTTP,
or OAuth transport.

## Example

```rust,no_run
use der_asn1::Asn1Limits;
use x509_certificate::decode_x509_certificate;
use x509_certificate_policy::{
    validate_certificate_role_extensions, validate_critical_extensions, X509CertificateRole,
};

# fn check(encoded: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
let certificate = decode_x509_certificate(encoded, Asn1Limits::default())?;
// This deliberately closed example recognizes only the three extensions its
// next gate decodes. A broader profile must validate every additional OID it
// recognizes before assigning trust.
validate_critical_extensions(&certificate, |oid| {
    oid.equals(&[2, 5, 29, 19])
        || oid.equals(&[2, 5, 29, 15])
        || oid.equals(&[2, 5, 29, 37])
})?;
let evidence = validate_certificate_role_extensions(
    &certificate,
    X509CertificateRole::EndEntity,
    Asn1Limits::default(),
)?;
assert_eq!(evidence.role(), X509CertificateRole::EndEntity);
# Ok(())
# }
```

## Verification

```bash
bash BUILD
cargo clippy -p x509-certificate-policy --all-targets -- -D warnings
cargo doc -p x509-certificate-policy --no-deps
```
