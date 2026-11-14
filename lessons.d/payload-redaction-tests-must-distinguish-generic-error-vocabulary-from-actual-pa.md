---
category: Testing & coverage
---

# Payload-redaction tests must distinguish generic error vocabulary from actual payload values

A stable error category can accurately contain generic words such as
`signature`, `certificate`, or `anchor` without leaking the certificate or
signature it rejected. Asserting that those domain words are absent makes a
correct payload-free message fail and does not test the real boundary. Check
for concrete fixture names, DNS names, algorithm identifiers, key sizes,
exponents, fingerprints, or encoded bytes instead, while separately asserting
the complete expected stable message.
