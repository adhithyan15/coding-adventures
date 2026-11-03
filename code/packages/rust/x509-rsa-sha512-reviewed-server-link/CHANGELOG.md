# Changelog

## 0.6.0 - 2026-11-03

### Added

- Applied the exact retained intermediate's supported critical permitted and
  excluded DNS constraints to the exact target retained by the complete
  selected-anchor SHA-512 proof.
- Added non-copyable result evidence retaining the entire prior proof beside
  optional payload-free constrained-name evidence.

### Security

- Present intermediate NameConstraints must be critical, both decoders use
  separate caller-owned limits, excluded names take precedence, and failures
  expose only stable payload-free categories.
- This fixed two-link policy does not process non-DNS constraints, construct or
  search paths, verify hostnames, check revocation, or grant TLS, HTTP, OAuth,
  or network authority.

## 0.5.0 - 2026-11-02

### Added

- Added selected reviewed-anchor DNS policy for the complete RSA SHA-512
  two-link server proof that retains advanced path-length state.
- Derived the constraint bytes and target SAN only from the consumed proof and
  retained that full proof beside optional constrained-name evidence.

### Security

- No detached target or reviewed snapshot entry can be substituted during the
  selected-anchor decision, and caller-owned limits bound both decoders.
- This supported DNS subset does not apply intermediate or non-DNS
  constraints, verify hostnames, check revocation, construct paths, or grant
  TLS, HTTP, OAuth, or network authority.

## 0.4.0 - 2026-11-02

### Added

- Added a non-copyable exact-target binding for the reviewed RSA SHA-512
  two-link proof that already retains advanced path-length state.
- Retained target policy, complete intermediate policy, both authenticated
  links, advanced path-length state, and exact reviewed-root selection through
  one distinct result.

### Security

- The target authenticated by the path-length proof must be the same decoded
  object retained by server policy; byte-equal independently decoded targets
  fail closed.
- This evidence join does not derive path-length state or grant path search,
  constraint, hostname, revocation, TLS, HTTP, OAuth, or network authority.

## 0.3.0 - 2026-10-30

### Added

- Added a non-copyable binding between exact target-server policy and an
  authenticated RSA SHA-512 target-to-intermediate-to-reviewed-root proof.
- Added an exact decoded-target identity gate and retained access to both
  complete consumed inputs.

### Security

- Independently decoded byte-equal targets cannot contribute mixed policy and
  authenticated-link evidence.
- This evidence join grants no path construction, path-length, constraint,
  hostname, revocation, TLS, HTTP, OAuth, or network authority.

## 0.2.0 - 2026-10-30

### Added

- Added bounded evaluation of the exact selected reviewed anchor's optional
  DNS NameConstraints against every dNSName in the exact retained target SAN.
- Added separate caller-owned ASN.1 budgets, payload-free failures, and
  non-copyable success retaining the complete reviewed server-link proof.

### Security

- Constraint and SAN inputs are derived only from the consumed proof, so a
  caller cannot substitute another target certificate or reviewed snapshot
  entry after authentication.
- This supported DNS-policy subset grants no multi-hop path, intermediate or
  IP-constraint, hostname, revocation, TLS, HTTP, OAuth, or network authority.

## 0.1.0 - 2026-10-30

### Added

- Added a non-copyable composition that binds exact target-server policy to
  an authenticated RSA SHA-512 issuer link terminating at a reviewed root.
- Added an exact decoded-target identity gate, payload-free mismatch errors,
  and redacted debug output.

### Security

- The binding performs no new parsing or signature verification and grants no
  anchor-constraint, multi-hop path, revocation, TLS, HTTP, OAuth transport,
  or network authority.
