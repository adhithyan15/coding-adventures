# Smart-home ONVIF origin policy fixtures v1

This directory is the language-neutral behavior contract for the deterministic
part of ONVIF credential-destination review. Implementations must validate
`cases.json` against `schema.json` before consuming individual cases.

The contract separates review from transport:

- `discovery` cases correlate WS-Addressing `RelatesTo`, the UDP responder, and
  every advertised XAddr before a record may remain a `candidate`.
- `soap_origin` cases approve one query-free canonical SOAP origin and one pinned
  socket address. HTTPS is required unless a case explicitly grants loopback-only HTTP.
- `derived_origin` cases cover device-supplied media, snapshot, and stream URLs.
  They must remain on a reviewed origin, must not downgrade transport, and must
  not contain userinfo or fragments.
- `http_status` cases reject redirects rather than forwarding a UsernameToken.
- `size_policy` cases define redacted limits for URLs, credentials, and
  generated requests.

`approved_resolved_addresses` represents the DNS result captured during host
review, while `observed_resolved_addresses` represents the result checked at a
derived-resource boundary. A later result that differs is `dns_rebinding`; the
transport must use the pinned address while preserving the canonical hostname
for HTTP Host and TLS SNI. Fixtures use private, loopback, multicast, and
non-routable documentation addresses as policy inputs; tests must never contact
those addresses.

Every discovery case supplies an `xaddrs` array because implementations must
review every advertised address. Size cases use the exact observed byte count,
including accepted-at-limit and rejected-one-byte-over boundary cases.
The schema applies URI and IP format checking plus nonempty string constraints;
the CI consumer also parses every expected pinned socket address semantically.

Error codes are stable, redacted categories. Implementations must not include
device-controlled URLs, SOAP text, credentials, or resolver diagnostics in
errors or audit records.
