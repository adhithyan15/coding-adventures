# x509-rsa-signature-routing

This crate routes exact child-certificate algorithm-profile evidence through a
caller-owned profile-index table to one fixed repository-owned RSA PKCS #1
v1.5 SHA-256, SHA-384, or SHA-512 signature verifier. The issuer key is derived
only from separately consumed exact issuer-certificate RSA key-policy evidence.
Missing and duplicate routes fail before RSA work, and success retains both
non-copyable policy inputs with the exact fixed-verifier result.

This is a narrow public-data cryptographic composition boundary. It does not
decide issuer Name linkage or role, recognize extensions, construct or
validate a path, establish reviewed trust, check target identity, time,
purpose, constraints, or revocation, perform TLS or HTTP, access credentials,
sign data, authorize OAuth transport, or grant network access.

The focused routing tests reuse the public ISRG Root X1 and D-TRUST BR Root CA
2 2023 fixtures already recorded by their owning repository packages and add
Microsoft RSA Root Certificate Authority 2017 from Microsoft's official PKI
publication. Their SHA-256 fingerprints are respectively
`96bcec06264976f37460779acf28c5a7cfe8a3c0aae11a8ffcee05c0bddf08c6`,
`0552e6f83fdf65e8fa9670e666df28a4e21340b510cbe52566f97c4fb94b2bd1`,
and `c741f70f4b2a8d88bf2e71c14122ef53ef10eba0cfa5e64cfa20f418853073e0`.
