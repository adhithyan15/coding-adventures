# Changelog

## 0.1.0

- Add strict bounded `LIST VAR` request and response framing.
- Correlate every list record to the requested UPS name.
- Decode only the protocol's quoted-string escapes and reject duplicates.
