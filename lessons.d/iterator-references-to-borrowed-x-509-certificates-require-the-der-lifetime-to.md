---
category: Rust
---

# Iterator references to borrowed X.509 certificates require the DER lifetime to outlive the reference lifetime

An ordered intermediate-policy helper accepted iterator items shaped as
`&'certificate X509Certificate<'encoded>` but omitted the relationship between
those lifetimes. Rust correctly rejected the signature because the reference
could otherwise outlive the DER borrowed by the decoded certificate. Add the
explicit bound `'encoded: 'certificate` whenever a generic iterator borrows
decoded certificates this way; keeping the two lifetimes separate still
allows a short traversal borrow over longer-lived encoded input.
