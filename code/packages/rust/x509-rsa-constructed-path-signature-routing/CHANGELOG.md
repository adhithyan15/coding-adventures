# Changelog

## 0.24.0 - 2026-11-13

### Added

- Added subordinate-intermediate NameConstraints evaluation that accepts only
  the Name-policy-backed ordered target policy result.
- Added retained Name-policy, reviewed-root, anchor-role, ordered target and
  subordinate constraint, and payload-free evaluation-count coverage over a
  checked-in two-intermediate public-certificate chain.

### Security

- The strong and generic entry points derive the intermediate certificates,
  anchor-to-target order, and self-issued classifications only through their
  consumed evidence, then use the same bounded repository-owned subordinate
  constraint evaluator.
- The strong result retains the exact Name-value policy through supported
  subordinate-CA SAN constraints. It does not apply reviewed-anchor constraints
  to subordinate CAs, constrain subject distinguished names, establish complete
  Name or path validity, check revocation, perform TLS or HTTP, authorize OAuth
  transport, or grant network authority.

## 0.23.0 - 2026-11-13

### Added

- Added ordered intermediate NameConstraints evaluation that accepts only the
  Name-policy-backed reviewed-anchor policy result.
- Added retained Name-policy, reviewed-root, anchor-role, ordered
  intermediate, target-signature, server-policy, anchor-policy, and
  payload-free intermediate-constraint coverage over a checked-in
  two-intermediate public-certificate chain.

### Security

- The strong and generic results derive the target, validated SAN index,
  intermediate certificates, and anchor-to-target order only through their
  consumed evidence, then use the same bounded repository-owned ordered
  constraint evaluator.
- The strong result retains the exact Name-value policy through supported
  intermediate target constraints. It does not constrain subordinate CA
  names, support every GeneralName, establish complete Name or path validity,
  check revocation, perform TLS or HTTP, authorize OAuth transport, or grant
  network authority.

## 0.22.0 - 2026-11-12

### Added

- Added reviewed-anchor DNS-policy evaluation that accepts only the Name-
  policy-backed arbitrary-depth target-signature closure.
- Added retained Name-policy, reviewed-root, anchor-role, intermediate,
  target-signature, server-policy, and anchor-policy coverage over a checked-in
  two-intermediate public-certificate chain.

### Security

- The strong and generic results derive the exact target, validated SAN index,
  and reviewed-anchor constraint bytes only from their consumed closure, then
  use the same bounded repository-owned supported permitted-DNS evaluator.
- The strong result retains the exact Name-value policy through reviewed-anchor
  policy. It does not establish complete Name or path validity, apply excluded
  or non-DNS anchor constraints or every intermediate constraint, check
  revocation, perform TLS or HTTP, authorize OAuth transport, or grant network
  authority.

## 0.21.0 - 2026-11-12

### Added

- Added a target-signature closure that accepts only the Name-policy-backed
  arbitrary-depth intermediate accumulator.
- Added retained Name-policy, reviewed-root, anchor-role, intermediate,
  depth-zero target-signature, and complete server-policy coverage over a
  checked-in two-intermediate public-certificate chain.

### Security

- The strong and generic target-signature closures share one private
  implementation of the exact-construction, depth-zero, construction-target,
  target-nearest issuer, and exact server-policy checks.
- The strong result retains the exact Name-value policy and all reviewed path
  evidence through target authentication. It does not establish complete Name
  or path validity, apply every constraint or revocation policy, perform TLS
  or HTTP, authorize OAuth transport, or grant network authority.

## 0.20.0 - 2026-11-12

### Added

- Added a bounded arbitrary-depth intermediate path-length accumulator that
  accepts only the Name-policy-backed reviewed-construction initializer.
- Added retained-policy success and detached-construction rejection coverage
  over a checked-in two-intermediate public-certificate chain.

### Security

- The strong and generic accumulators now share one private implementation of
  the construction-derived count, caller ceiling, exact-construction,
  immediate-depth, issuer-continuity, authenticated-child, complete-policy,
  and path-length checks.
- The strong result retains the exact Name-value policy, reviewed anchor,
  anchor role policy, ordered signature and policy slices, and final state. It
  does not close the target signature, define complete RFC 5280 Name equality,
  validate the whole path, apply constraints or revocation, perform TLS or
  HTTP, authorize OAuth transport, or grant network authority.

## 0.19.0 - 2026-11-12

### Added

- Added a second ordered intermediate path-length transition that consumes the
  stronger Name-policy-backed first-step result.
- Added retained-policy success and detached-construction rejection coverage
  over a checked-in two-intermediate public-certificate chain.

### Security

- The new transition shares the generic second step's exact-construction,
  immediate-depth, issuer-continuity, authenticated-child, complete-policy,
  and path-length checks while retaining the exact Name-value policy, reviewed
  anchor, anchor role policy, both routed signatures, and both intermediate
  policies through the result.
- This proves exactly two routed public-certificate transitions. It does not
  define complete RFC 5280 Name equality, generalize arbitrary traversal,
  validate the whole path, apply name constraints or revocation, perform TLS
  or HTTP, authorize OAuth transport, or grant network authority.

## 0.18.0 - 2026-11-12

### Added

- Added a first ordered intermediate path-length transition that accepts only
  the Name-policy-backed reviewed-construction initializer.
- Added retained-policy success and detached-construction rejection coverage
  over a fixed public-certificate chain.

### Security

- The transition shares the existing exact-construction, authenticated-child,
  complete intermediate-policy, and construction-derived capacity checks while
  retaining the exact Name-value policy, reviewed anchor, and anchor role
  policy through the result.
- This proves only the first routed public-certificate transition. It does not
  define complete RFC 5280 Name equality, process later intermediates, validate
  the whole path, apply name constraints or revocation, perform TLS or HTTP,
  authorize OAuth transport, or grant network authority.

## 0.17.0 - 2026-11-11

### Added

- Added reviewed-anchor permitted-DNS evaluation over every retained
  non-self-issued intermediate certificate's optional Subject Alternative
  Name.
- Added constrained success, self-issued skip, absent-SAN, unconstrained
  short-circuit, violation, and retained real-chain coverage.

### Security

- Constraint bytes, certificates, order, complete policy, and self-issued
  classifications come only from the consumed proof. Existing path and ASN.1
  ceilings bound all public-certificate evaluation.
- This does not constrain subordinate distinguished names, add excluded or
  non-DNS reviewed-anchor constraints, support every GeneralName, establish
  complete path/name validation, check revocation, perform TLS or HTTP,
  authorize OAuth transport, or grant network authority.

## 0.16.0 - 2026-11-11

### Added

- Added ordered evaluation of every retained intermediate's critical supported
  DNS/IP NameConstraints against each target-nearer non-self-issued
  intermediate certificate's optional Subject Alternative Name.
- Added synthetic broad-to-narrow, self-issued-skip, absent-SAN, and
  subordinate-name violation coverage plus retained real-chain coverage.

### Security

- Certificates, traversal order, complete policy, and self-issued
  classifications come only from the consumed proof. Existing path and ASN.1
  ceilings bound the repeated public-certificate evaluation.
- This does not constrain subordinate distinguished names, apply reviewed
  anchor constraints to subordinate CAs, support every GeneralName, establish
  complete path/name validation, check revocation, perform TLS or HTTP,
  authorize OAuth transport, or grant network authority.

## 0.15.0 - 2026-11-11

### Added

- Added ordered evaluation of every retained intermediate policy's supported
  critical DNS/IP NameConstraints against the exact target identities in an
  arbitrary-depth routed closure.
- Added fixed two-intermediate public-certificate coverage plus synthetic
  ordered broad-to-narrow DNS/IP success and later-exclusion failure coverage.

### Security

- The exact target, validated SAN index, intermediate certificates, and
  anchor-to-target traversal order are derived only from the consumed proof
  chain. The earlier accumulator ceiling bounds visits, and caller-owned ASN.1
  limits bound every repository-owned decode.
- This does not constrain subordinate CA names, support every GeneralName,
  establish complete path/name validation, check revocation, perform TLS or
  HTTP, authorize OAuth transport, or grant network authority.

## 0.14.0 - 2026-11-11

### Added

- Added reviewed-anchor DNS-policy evaluation over the arbitrary-depth routed
  target-signature closure.
- Added fixed public-certificate coverage proving that an unconstrained
  reviewed snapshot entry retains the complete signature closure without
  caller-supplied target, SAN-index, anchor, or constraint inputs.

### Security

- The exact target, already-validated SAN index, and reviewed anchor constraint
  bytes are derived only through the consumed closure before the existing
  bounded permitted-DNS evaluator runs.
- This applies only the reviewed snapshot's supported permitted-DNS subset. It
  does not apply every intermediate NameConstraints value, excluded or non-DNS
  anchor constraints, complete path/name policy, revocation, TLS, HTTP, OAuth,
  or network authority.

## 0.13.0 - 2026-11-11

### Added

- Added an exact-object closure that consumes accumulated reviewed intermediate
  evidence, the construction's routed depth-zero target signature, and complete
  server policy for that exact target.
- Added fixed public-certificate coverage for the retained closure, nonzero
  target depth, detached construction, and byte-equal independently decoded
  target policy.

### Security

- Closure requires construction identity, the exact depth-zero target child,
  issuer continuity with the accumulated target-nearest intermediate, and
  exact-object target-policy identity before retaining all evidence together.
- This supplies repository-owned routed RSA evidence for each selected
  child-to-issuer adjacency only. It does not validate the anchor's
  self-signature, generalized path or name constraints, hostname, revocation,
  TLS, HTTP, OAuth, or network authority.

## 0.12.0 - 2026-11-11

### Added

- Added an allocation-free ordered accumulator for every intermediate on one
  already constructed path, bounded by a caller-owned nonzero intermediate
  ceiling.
- Added fixed public-certificate chain coverage for a one-intermediate empty
  follow-on slice, multi-intermediate evidence retention, exact evidence
  counts, ordered depths, detached construction, and child-policy binding.

### Security

- The accumulator derives the intermediate count from the exact construction,
  requires exact signature and policy counts, and applies each transition only
  after construction identity, depth, issuer continuity, and authenticated
  child-policy identity checks pass.
- This accumulates only intermediate path-length state. It stops before the
  target signature and does not establish complete validation, check names or
  revocation, perform TLS or HTTP, authorize OAuth, or grant network authority.

## 0.11.0 - 2026-11-11

### Added

- Added the immediately target-nearer ordered path-length transition after the
  root-authenticated first intermediate.
- Added fixed public-certificate chain coverage for retained two-step evidence,
  exact depth order, detached-construction rejection, and child-policy binding.

### Security

- The second transition requires the same exact construction, the immediately
  preceding issuer depth, the already processed intermediate as issuer, and
  complete policy for the exact authenticated candidate child before advancing
  retained state.
- This proves exactly two public-certificate transitions. It does not iterate
  over arbitrary paths, establish complete validation, check revocation,
  perform TLS or HTTP, authorize OAuth, or grant network authority.

## 0.10.0 - 2026-11-11

### Added

- Added the first trust-anchor-to-target path-length transition from exact
  reviewed construction-derived initialization and one routed authenticated
  intermediate.
- Added coverage for retained initial and advanced evidence, target-nearer
  anchor-nearest selection on a longer construction, rejection of proof from a
  distinct construction object, and rejection of policy for another child.

### Security

- The transition requires the initializer and terminal proof to retain the
  identical constructed path, authenticates the anchor-nearest intermediate as
  the reviewed root's exact child, and binds complete intermediate policy to
  that child before applying path length.
- This is only one ordered public-certificate transition. It does not
  accumulate remaining intermediates, establish complete path validation,
  check names or revocation, perform TLS or HTTP, authorize OAuth, or grant
  network authority.

## 0.9.0 - 2026-11-03

### Added

- The exact intermediate NameConstraints follow-up now evaluates every retained
  target DNS-ID and IP-ID together against supported critical permitted and
  excluded DNS and IP subtrees.
- Success retains payload-free combined DNS/IP counts while preserving the
  existing DNS-only compatibility evidence after the combined check passes.

### Security

- Mixed constraints can no longer silently drop the IP form after the exact
  intermediate, target, and target SAN index have been derived from the routed
  construction.
- This remains fixed-shape public certificate policy; it does not apply other
  GeneralName forms or non-DNS anchor constraints, verify a hostname, check
  revocation, perform TLS or HTTP, authorize OAuth, or grant network authority.

## 0.8.0 - 2026-11-03

### Changed

- The exact target -> intermediate -> reviewed-root binder now accepts the
  complete construction-bound intermediate policy directly and derives the
  RFC 5280 initial capacity of one intermediate from the retained two-link
  shape.
- The lower-level caller-owned path-length transition remains available for
  future generalized traversal, but detached state can no longer create the
  fixed reviewed two-link proof.

### Security

- Callers cannot weaken or spuriously exhaust the fixed route by supplying an
  unrelated initial path-length state.
- This is fixed-shape public certificate policy only; it does not generalize
  path validation, check revocation, perform TLS or HTTP, authorize OAuth, or
  grant network authority.

## 0.7.0 - 2026-11-03

### Added

- Added bounded critical intermediate DNS NameConstraints policy for the exact
  intermediate and target retained by a routed two-link server path.
- Added coverage for absent constraints, permitted and excluded DNS subtrees,
  criticality, and redacted target violations.

### Security

- The composition derives the intermediate, target, and validated target SAN
  index through one retained proof and keeps all prior anchor-policy evidence.
- Only the repository's supported permitted-and-excluded-DNS subset is applied;
  this does not cover IP or other name forms, generalized path validation,
  hostname verification, revocation, TLS, HTTP, OAuth, or network use.

## 0.6.0 - 2026-11-03

### Added

- Added bounded selected-anchor DNS policy for the exact target and reviewed
  terminal retained by a routed two-link server path.
- Added coverage for unconstrained reviewed roots, permitted DNS subtrees,
  target violations, and missing Subject Alternative Name.

### Security

- The composition derives both target and reviewed constraint bytes through
  one retained proof and keeps diagnostics free of certificate names.
- Only the reviewed snapshot's supported permitted DNS subset is applied; this
  does not cover intermediate, excluded, or non-DNS constraints, revocation,
  TLS, HTTP, OAuth, or network use.

## 0.5.0 - 2026-11-03

### Added

- Added exact-object binding between complete target-server certificate policy
  and one routed target -> intermediate -> reviewed-root construction.
- Added coverage for retained policy/path evidence and rejection of policy for
  a byte-equal independently decoded target.

### Security

- The binding prevents complete identity, validity, purpose, end-entity role,
  critical-extension, or algorithm policy from being attached to another
  construction's target.
- The result does not apply anchor or intermediate constraints, check
  revocation, or authorize TLS, HTTP, OAuth, or network use.

## 0.4.0 - 2026-11-03

### Added

- Added an exact-object composition for one routed
  target -> intermediate -> reviewed-root construction.
- Added coverage for complete two-link evidence retention and rejection of
  proofs projected from distinct constructed-path objects.

### Security

- The composition requires exactly two issuer depths from the same retained
  construction and preserves both routed signatures, complete intermediate
  policy, advanced path-length state, and reviewed-root selection.
- The fixed shape does not validate longer paths, derive anchor constraints,
  or authorize TLS, HTTP, OAuth, or network use.

## 0.3.0 - 2026-11-03

### Added

- Added an exact-object terminal handoff from one data-driven routed
  constructed-path signature proof to reviewed Mozilla root selection.
- Added coverage for successful evidence retention, non-terminal adjacency
  rejection, and byte-equal detached reviewed-root evidence.

### Security

- The handoff requires the authenticated adjacency to end at the construction's
  final selected issuer and retains its certificate-wide issuing-CA policy.
- Reviewed snapshot membership does not grant complete path validity, apply
  trust-anchor constraints, or authorize TLS, HTTP, OAuth, or network use.

## 0.2.0 - 2026-11-03

### Added

- Added exact-object binding between one routed constructed-path signature
  proof and complete policy for its non-terminal intermediate issuer.
- Added a path-length handoff that derives its transition only from the
  retained complete intermediate policy and retains the ordered proof with the
  advanced state.
- Added coverage for terminal issuers, byte-equal detached policy, successful
  evidence retention, and exhausted non-self-issued capacity.

### Security

- Terminal roots and cross-certificate policy fail closed with payload-free
  categories.
- The composition grants no complete traversal, trust, revocation, TLS, HTTP,
  OAuth transport, or network authority.

## 0.1.0 - 2026-10-30

### Added

- Added exact-object binding between one constructed-path adjacency, child
  algorithm policy, issuer role policy, and issuer RSA algorithm/key policy.
- Added data-driven routing to the repository-owned RSA PKCS #1 v1.5 SHA-256,
  SHA-384, and SHA-512 certificate-signature verifiers.
- Added non-copyable success evidence and payload-free failure categories.

### Security

- Cross-certificate evidence and non-CA issuers fail before RSA work.
- The composition grants no complete path, trust, revocation, TLS, HTTP,
  OAuth transport, or network authority.
