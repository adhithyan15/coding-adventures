# x509-ocsp-rsa-signature-routing

This crate routes already validated OCSP RSA algorithm/key-policy evidence to
one fixed repository-owned PKCS #1 v1.5 SHA-256, SHA-384, or SHA-512 signature
verifier. The caller owns the mapping from selected policy-profile index to
verifier. Missing and duplicate mappings fail closed, and success retains both
the consumed policy evidence and exact signature-verification evidence.

This is a narrow public-data cryptographic composition boundary. It performs
no credential access, signing, storage, clock acquisition, audit event, trace
emission, I/O, or other external effect. It does not authorize an OCSP
responder, match `ResponderID`, bind `CertID` or status to a certificate or
path, decide freshness or revocation, establish trust, perform TLS or HTTP,
authorize OAuth transport, or grant network access. Concrete HTTPS and live-
provider readiness remain unavailable.

The focused SHA-384 routing test reuses the synthetic public OCSP response,
RSA public key, and signature interoperability vector recorded by
`x509-ocsp-rsa-pkcs1-v1-5-sha384-signature`. The temporary private key used to
generate that public vector was deleted by the owning package's recorded
fixture-generation process and is not a dependency of this package or its
tests.
