# x509-ocsp-rsa-signature-routing

This crate routes already validated OCSP RSA algorithm/key-policy evidence to
one fixed repository-owned PKCS #1 v1.5 SHA-256 or SHA-512 signature verifier.
The caller owns the mapping from selected policy-profile index to verifier.
Missing and duplicate mappings fail closed, and success retains both the
consumed policy evidence and exact signature-verification evidence.

This is a narrow cryptographic composition boundary. It does not authorize an
OCSP responder, match `ResponderID`, bind `CertID` or status to a certificate
or path, decide freshness or revocation, establish trust, perform TLS or HTTP,
authorize OAuth transport, or grant network access.
