# Smart-home ONVIF origin policy fixtures v1

This directory is the language-neutral behavior contract for the deterministic
part of ONVIF credential-destination review. Implementations must validate
`cases.json` against `schema.json` before consuming individual cases.

The contract separates review from transport:

- `discovery` cases correlate WS-Addressing `RelatesTo`, the UDP responder, and
  every advertised XAddr before a record may remain a `candidate`.
- `soap_origin` cases approve one canonical SOAP origin and one pinned socket
  address. HTTPS is required unless a case explicitly grants loopback-only HTTP.
- `derived_origin` cases cover device-supplied media, snapshot, and stream URLs.
  They must remain on a reviewed origin, must not downgrade transport, and must
  not contain userinfo or fragments.
- `http_status` cases reject redirects rather than forwarding a UsernameToken.
- `size_policy` cases define redacted limits for URLs, credentials, and
  generated requests.

`resolved_addresses` represents the single DNS result captured during host
review. A later result that differs is `dns_rebinding`; the transport must use
the pinned address while preserving the canonical hostname for HTTP Host and
TLS SNI. Fixture IP addresses use the documentation ranges and must never be
contacted by tests.

Error codes are stable, redacted categories. Implementations must not include
device-controlled URLs, SOAP text, credentials, or resolver diagnostics in
errors or audit records.
