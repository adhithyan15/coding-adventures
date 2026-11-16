# Changelog

## Unreleased

- Added a provider/trace-audited, bounded, wipe-on-drop complete HTTP/1 request
  release that joins exact custody-built client-secret material to one
  validated request line and existing certificate/SNI/Host destination proof.
- Required a fresh provider/trace audit before every borrow of the prepared
  credential-bearing wire bytes.
- Kept TLS, HTTP I/O, sockets, response handling, credential storage, signing,
  and every transport effect outside this pure in-memory boundary.
