# Changelog

## Unreleased

### Fixed

- `required_capabilities.json` rewritten to the canonical schema-v1 shape (version, package, capabilities objects, justification) so the hardened capability analyzer accepts it.

- Added compact RFC 8259 serialization with explicit depth limits, stable
  duplicate-member ordering, control-character escaping, and closed rejection
  of non-finite numbers.
- Added a zero-dependency RFC 8259 recursive-descent parser with explicit depth
  limits, exact number grammar, UTF-16 surrogate handling, duplicate-key
  preservation, and closed input-free errors.
