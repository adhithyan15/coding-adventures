---
category: Testing & coverage
---

# A one-version supported_versions ClientHello fixture needs another extension to meet the eight-byte payload minimum

An acceptance fixture encoded a ClientHello with only a one-version
`supported_versions` extension. That extension occupies seven payload bytes:
four bytes of extension framing plus three bytes of data. The repository-owned
ClientHello decoder correctly rejected it because TLS 1.3 requires at least
eight bytes in the extension vector. Add another legal ClientHello extension,
such as empty `padding`, when testing a one-version offer; do not weaken the
decoder or inflate the version offer merely to make the fixture parse.
