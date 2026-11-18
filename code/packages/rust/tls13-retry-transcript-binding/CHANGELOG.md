# Changelog

## 0.1.0

- Bind the exact retained ClientHello1, HelloRetryRequest, ClientHello2, and
  final ServerHello bodies to the repository-owned TLS 1.3 transcript state.
- Select the transcript hash only through the retained policy-authorized cipher
  suite and apply the existing one-shot synthetic `message_hash` rewrite.
