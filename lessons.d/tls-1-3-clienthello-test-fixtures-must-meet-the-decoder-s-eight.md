---
category: Testing & coverage
---

# TLS 1.3 ClientHello test fixtures must meet the decoder's eight-byte extension-vector minimum

A composition-package fixture reused the six-byte minimum `supported_versions`
extension shape accepted by the ServerHello decoder. Every test stopped in the
ClientHello decoder with `ExtensionVectorTooShort`, before reaching the layer
under test, because `tls13-client-hello-message` deliberately requires at least
eight extension-payload bytes. Build ClientHello fixtures from that package's
known-valid shape (for example, a four-byte `supported_versions` payload) and
confirm both prerequisite decoders accept each fixture before attributing a
composition test failure to the composition layer.
