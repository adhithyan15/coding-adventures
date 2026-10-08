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
  onto the daemon's own loopback API. IPv6 is an allowlist of global unicast
  `2000::/3`, minus Teredo, 6to4, ORCHID, benchmarking and documentation.
  NAT64 and every IPv4-embedding form are refused.
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
  - Every echo is masked in place in the raw zeroizing response before
    anything parses or copies it. Each byte becomes `*`, so framing survives
    and no unmasked copy exists. The masked forms are as sent, JSON-escaped and
    percent-encoded, plus a trailing prefix when the stream was cut.
  - `CredentialSource::redeem` receives the destination `host:port` (V-S7), so
    the vault can refuse to send a secret to a host it was not provisioned for.
  - Request and response buffers are zeroizing.
- Chunked decoding checks every server-supplied size against the bytes
  present and the remaining budget, using checked arithmetic. A size near
  `usize::MAX` can no longer wrap and panic.
- `tool_definition()` publishes the D18D definition, whose header schemas
  enumerate the allowlists. It passes D18S S-I7's peer-naming check.

### Security review, round 1 (fixed before the first push)

- **HIGH**: a CR, LF or space in the URL went verbatim onto the request line,
  defeating the header allowlist. Fixed at the root in `operation-primitives`'
  preflight (RFC 3986 characters only), so every caller is covered.
- **MEDIUM**: an echo straddling the truncation point escaped a scrub that
  ran after truncation. Masking now runs first, on the whole raw buffer.
- **MEDIUM**: a lease was not bound to a destination. V-S7 adds that binding.
- **MEDIUM**: chunk-size overflow could panic the daemon.
- **LOW-MEDIUM**: IPv6 ranges that embed or translate IPv4 were treated as
  public.
- **LOW** and **INFO**: response-side copies of the secret, and encoded
  echoes, were missed. Both are covered by in-place masking.
