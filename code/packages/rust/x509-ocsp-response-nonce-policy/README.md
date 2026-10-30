# x509-ocsp-response-nonce-policy

`x509-ocsp-response-nonce-policy` is the bounded response-echo policy for the
RFC 8954 OCSP nonce extension. A caller supplies an already decoded
`ResponseData`, the same DER decoder that owns the semantic extension budget,
and the exact 32-octet nonce sent in its corresponding request.

The validator requires exactly one non-critical outer response extension,
requires its OID to be `id-pkix-ocsp-nonce`, decodes the embedded `Nonce` OCTET
STRING without resetting the enclosing depth, enforces RFC 8954's 1–32-octet
wire limit, and compares the entire echoed value with the repository-owned
full-scan equality primitive. Modern client policy is encoded in the API by
accepting only a 32-octet request nonce.

RFC 8954 permits a responder to omit the nonce. This package implements only
the explicit required-echo branch and deliberately rejects that omission.
Deployments choosing a nonce-less freshness strategy need a separate policy;
they must not treat this validator's failure as an authenticated revocation
verdict.

```rust,no_run
use der_asn1::Asn1Decoder;
use x509_ocsp_response_nonce_policy::validate_required_x509_ocsp_response_nonce;
# use x509_ocsp_response_data::X509OcspResponseData;
# fn validate(
#     decoder: &mut Asn1Decoder,
#     response: &X509OcspResponseData<'_>,
# ) -> Result<(), Box<dyn std::error::Error>> {
let requested_nonce = [0x5a; 32];
let proof =
    validate_required_x509_ocsp_response_nonce(decoder, response, &requested_nonce)?;
assert!(std::ptr::eq(proof.response_data(), response));
# Ok(())
# }
```

## Security boundary

The result is non-copyable evidence retaining the exact decoded response and
caller-owned request nonce. Failures contain only a stable category and local
embedded-document offset; they retain no nonce, extension, response,
certificate, provider, or network payload.

This package does not generate or store the request nonce, construct or send an
OCSP request, authenticate the response signature, authorize a responder, bind
status to a certificate or path, decide freshness or revocation, fetch or cache
data, read a clock, establish trust, perform TLS or HTTP, authorize OAuth
transport, open a socket, or access a credential. A nonce echo alone is not an
OCSP revocation verdict or concrete HTTPS readiness.

Primary source: [RFC 8954 section 2.1](https://www.rfc-editor.org/rfc/rfc8954.html#section-2.1).

## Development

```bash
bash BUILD
cargo clippy -p x509-ocsp-response-nonce-policy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-ocsp-response-nonce-policy --no-deps
```
