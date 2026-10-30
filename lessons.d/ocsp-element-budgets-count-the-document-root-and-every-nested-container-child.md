---
category: Cryptography & security review
---

# OCSP element budgets count the document root and every nested container child

An OCSP response-nonce test counted leaf fields but omitted the decoded
`SingleResponse` and `CertID` SEQUENCE elements themselves. The complete
fixture consumes 20 elements before semantic nonce decoding: the document root,
responder wrapper and key, producedAt, responses container, SingleResponse,
CertID, algorithm and its fields, both issuer hashes, serial, status, thisUpdate,
response-extension wrapper and collection, generic Extension, OID, and outer
OCTET STRING. The embedded Nonce OCTET STRING is element 21. Write the count as
a schema walk before choosing the one-below falsification limit.
