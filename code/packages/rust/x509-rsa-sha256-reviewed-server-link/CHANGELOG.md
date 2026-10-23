# Changelog

## 0.7.0 - 2026-10-23

### Added

- Applied supported critical excluded dNSName constraints from the exact
  retained intermediate, with exclusion taking precedence over permission and
  broader target wildcards rejected when any expansion overlaps an exclusion.
- Migrated direct, two-link anchor, and intermediate consumers to generalized
  DNS-constraint evidence so decoded exclusions cannot be silently ignored.
- Added exact-consumer coverage for combined permitted and excluded subtrees;
  reviewed-root snapshot constraints remain permitted-only reviewed input.

## 0.6.0 - 2026-10-23

### Added

- Applied an exact retained intermediate's critical NameConstraints extension
  to every dNSName in the exact target retained by the reviewed two-link path.
- Added distinct non-copyable evidence retaining the complete root anchor-policy
  proof plus optional payload-free intermediate DNS count evidence.
- Added fail-closed criticality, full-DER framing, unsupported-form, caller-limit,
  violation, redaction, and real unconstrained-intermediate coverage.

## 0.5.0 - 2026-10-23

### Added

- Applied the selected reviewed root's supported permitted DNS constraints to
  the exact target retained by a complete two-link server-path proof.
- Added distinct non-copyable success evidence retaining the entire
  target-to-intermediate-to-reviewed-root path plus optional payload-free DNS
  count evidence.
- Added public-API runtime coverage over the static public Let's Encrypt chain,
  proving the unconstrained ISRG Root X1 branch retains the exact target and
  reviewed root without weakening upstream evidence construction.

## 0.4.0 - 2026-10-23

### Added

- Bound complete target-server certificate policy to the exact authenticated
  target child retained by a reviewed two-link RSA SHA-256 path.
- Added a distinct non-copyable success type that retains the server policy,
  both authenticated links, complete intermediate policy, advanced path-length
  state, and reviewed-root selection.
- Added public-API runtime coverage over a static public Let's Encrypt
  target-to-R13-to-ISRG-Root-X1 chain, including retained target,
  intermediate, root, and path-length evidence plus byte-equal distinct-target
  rejection, without introducing a forging constructor for upstream evidence.

## 0.3.0 - 2026-10-22

### Added

- Applied exact selected-anchor permitted DNS constraints to every dNSName in
  the exact target certificate retained by direct reviewed-server-link evidence.
- Added separate caller-owned decoder limits, optional payload-free DNS success
  evidence, and stable redacted parsing and constraint failure categories.
- Preserved unconstrained-anchor support while keeping excluded/non-DNS
  constraints, multi-hop paths, revocation, and transport outside this boundary.

## 0.2.0 - 2026-10-22

### Added

- Added a fail-closed anchor-policy composition that consumes the complete
  reviewed server link and accepts only selected snapshot entries without
  out-of-band name constraints.
- Added distinct non-copyable success evidence plus a payload-free
  `UnsupportedAnchorNameConstraints` failure category.
- Documented that constrained anchors remain unavailable until a
  repository-owned parser and subtree evaluator can apply their constraints.

## 0.1.0 - 2026-10-21

### Added

- Added a non-copyable composition that binds exact target-server policy to
  one reviewed-root RSA SHA-256 issuer link for the same decoded child object.
- Added payload-free mismatch errors, redacted debug output, and focused tests
  for same-object acceptance and byte-equal distinct-object rejection.
- Documented that anchor constraints, path construction, revocation, TLS,
  HTTP, OAuth transport, and network authority remain outside this package.
