# Changelog

## 1.2.0 - 2026-08-02

- Added a stable query-forbidden SOAP case so credentials cannot be sent to or
  durably associated with query-bearing service URLs.
- Added nonempty-string, URI, IP-address, and pinned-socket semantic validation;
  fixture confidence now checks the produced discovery record rather than its
  own expected text.

## 1.1.0 - 2026-08-02

- Made operation inputs schema-specific, modeled every discovery XAddr, split
  approved and observed resolver evidence, and added exact size boundaries.
- Added CI schema validation, unique case-ID checks, and consistency checks for
  accepted results and stable error codes.

## 1.0.0 - 2026-08-01

- Defined correlated discovery, pinned SOAP origins, derived media URI policy,
  DNS-rebinding rejection, loopback-only plaintext fixtures, redirect denial,
  and stable size-limit categories.
