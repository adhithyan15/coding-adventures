# Changelog

## 0.1.0 — Unreleased

- **New crate**: D18V `net.fetch`, the host-mediated HTTPS operation that a
  Chief vault lease is redeemed into. Before this crate, a `vault.request_lease`
  receipt had nowhere to go: no production tool accepted a `VaultRef`
  (#13980, P1.4c).
- `parse_request` validates the model's arguments:
  - `GET` and `POST` only.
  - A request-header allowlist; CR, LF and NUL are refused in values.
  - Bounded URL, header and body sizes.
  - Duplicate keys refused at every level.
  - A credential is a lease reference, with nowhere for a plaintext secret to
    go.
- `NetAllowlist` authorizes against the calling host's signed `net:dns` and
  `net:connect` capabilities through `operation-primitives`' exact-match
  allowlist:
  - It refuses IP-literal hosts.
  - It treats a malformed signed target as an error, not a skip.
  - It yields `None` for a host with no network capability, which is then not
    offered the tool.
- `is_public` plus single resolution: every resolved address must be public,
  and the connection goes to the checked address. This closes DNS rebinding
  onto the daemon's own loopback API.
- `TlsTransport` connects through `tls-platform`'s `connect_addr`, so SNI and
  certificate verification use the host name while the socket goes to the
  checked address. No redirects are followed.
- Responses:
  - Bounded head and body, decoded by length, chunked or EOF, with a
    `truncated` flag.
  - A response-header allowlist, so `set-cookie` never reaches the model.
  - UTF-8 bodies only.
- Credentials:
  - The secret is injected into exactly one allowlisted header.
  - It must be 8–4096 bytes of printable ASCII.
  - The lease is redeemed only after authorization and the address check.
  - Every echo of the header value or the secret is scrubbed from the response.
  - Request and response buffers are zeroizing.
- `tool_definition()` publishes the D18D definition, whose header schemas
  enumerate the allowlists. It passes D18S S-I7's peer-naming check.
