---
category: Cryptography & security review
---

# Security boundary tests must drive fail-closed branches through the consuming path

The first `CertificateVerify` test proved success, wrong message type, and a
policy rejection, while the diagnostics test only constructed other error
variants directly. That missed whether malformed framing, a malformed or
missing final offer, and an unoffered scheme actually reject inside the
authenticated ordered callback without advancing the transcript. For a new
security boundary, drive every primary fail-closed branch through the real
consuming path and assert both state preservation and terminal failure; direct
error construction is only a diagnostics test.
