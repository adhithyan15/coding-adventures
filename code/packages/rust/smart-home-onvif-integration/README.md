# smart-home-onvif-integration

This package provides the first production camera integration for D23:

- ONVIF WS-Discovery probes over bounded UDP.
- namespace-aware, `RelatesTo`-correlated ProbeMatch parsing and candidate D23
  discovery records bound to their UDP responder.
- WS-Security UsernameToken password-digest SOAP requests.
- a host-reviewed HTTPS-by-default origin policy that rejects userinfo,
  fragments, SOAP query strings, plaintext downgrade, public/multicast/unspecified destinations,
  DNS rebinding, cross-origin media URLs, and redirects before credentials are
  constructed or sent.
- address-pinned HTTP/1.1 and certificate-verifying HTTPS transport that keeps
  the reviewed hostname for Host and TLS SNI, including normalized bracketed
  IPv6 literals.
- device information, media profile, snapshot URI, and RTSP stream URI reads.
- normalized camera devices/entities with no media URI or credential material in
  runtime state.
- process-local snapshot and stream registration through the narrow
  `CameraMediaEndpointRegistry` surface; the host-owned camera-media service
  never returns a device endpoint URI to the lease holder and retains the
  reviewed canonical host plus pinned socket until trusted executor dispatch.
- stable redacted TLS/error categories, a client-level response ceiling even
  for injected transports, and fail-closed duplicate endpoint-reference
  conflict handling.

The `smart-home-onvif-integration` binary can run `discover` or inspect one
device service. Inspection reads `ONVIF_USERNAME` and `ONVIF_PASSWORD` from the
environment and emits only a sanitized profile summary. Production inspection
requires a private/link-local/loopback HTTPS origin that resolves to one stable
address. Plaintext HTTP and RTSP are available only through an explicit
loopback fixture policy in library tests. Vault-mediated credential delivery
and capability approval remain separate roadmap owners.

The versioned language-neutral hostile-origin contract lives at
`../../../specs/fixtures/smart-home-onvif-origin-v1`. It covers discovery
correlation, source/XAddr mismatch, secure transport, address review, pinned DNS
results plus observed rebinding evidence, derived media origins, exact size
boundaries, query-free credential destinations, and redirect denial. CI validates
the schema, operation-specific fields, URI/IP semantics, unique IDs, and
accepted/code consistency before Rust consumes it. Production URL, credential,
and request boundaries use the same size-policy function consumed by the fixture.
Errors expose stable redacted policy codes rather than device-controlled URLs,
resolver diagnostics, TLS details, or SOAP fault text.
