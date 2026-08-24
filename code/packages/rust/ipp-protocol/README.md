# ipp-protocol

Strict bounded IPP/1.1 framing for one fixed read-only printer-status surface.
The crate encodes `Get-Printer-Attributes` for a fixed allowlist and decodes a
fully correlated successful response containing printer identity, state,
reasons, job acceptance, queue count, and uptime.

It does not expose print submission, job queries, queue mutation, arbitrary
attribute selection, authentication, HTTP, TLS, discovery, or session
ownership.

Official references:

- <https://www.rfc-editor.org/rfc/rfc8010.html>
- <https://www.rfc-editor.org/rfc/rfc8011.html>
