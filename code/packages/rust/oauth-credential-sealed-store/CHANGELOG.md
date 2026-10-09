# Changelog

## Unreleased

- Documented that callers must pass an anchored `SealedStore`
  (`SealedStore::with_anchor`, VLT01 F11) for a backend that survives a
  restart. Without one, a restored snapshot brings revoked credentials back
  (#13980 P1.20c). There is no code change: the adapter has no production
  constructor yet.
- Added a zero-external-dependency encrypted `CredentialStore` adapter with
  bounded versioned encoding, zeroizing secret buffers, backend-revision-bound
  CAS, opaque provider/account coordinates, and closed error mapping.
