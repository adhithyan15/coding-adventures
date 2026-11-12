# X.509 Certificate Issuer-Candidate Selection

**Status:** Bounded unique-candidate selection prerequisite.

## Purpose

Select exactly one candidate certificate whose subject Name a caller-owned
policy equates with one exact child certificate's issuer Name. Callers can use
either a generic callback or the validated, data-driven repository-owned
`X509NameValuePolicy`. This is one bounded input to later certification-path
construction. It is not issuer authentication or path validation.

## Contract

The caller supplies:

- one already decoded child certificate;
- a caller-ordered slice of already decoded candidate certificates;
- a nonzero maximum candidate count; and
- either a distinguished-name equivalence callback or a validated
  `X509NameValuePolicy`.

Pools over the explicit ceiling fail before the callback runs. Otherwise the
selector scans the complete pool in order and invokes the callback exactly once
per candidate with:

1. the child's issuer Name; then
2. that candidate's subject Name.

No match fails as `NoMatch`. More than one match fails as `AmbiguousMatch`,
including when the first match appeared early in the pool. Exactly one match
returns non-copyable evidence binding the exact child certificate, selected
candidate certificate, and caller-order index.

The policy-backed entry point calls `compare_names` for each candidate in
caller order. Its first Name comparison error aborts the scan and is returned
as a nested, payload-free error category. When every comparison succeeds, the
complete pool is still scanned to detect ambiguity. Success returns a distinct
non-copyable evidence type that also retains the exact validated Name policy.
The generic callback API cannot construct or convert into that evidence, so a
later policy-requiring composition can reject generic callback provenance by
type.

## Security boundary

The implementation allocates nothing from certificate input, retains only
borrowed certificate references on success, and exposes payload-free error
categories. Candidate-count rejection occurs before caller policy can observe
any Name.

The callback owns all of its Name normalization and equality semantics. The
repository-owned policy alternative removes executable value callbacks from
this boundary, but its current rules do not provide RFC 4518 normalization,
case or whitespace folding, schema-specific matching, or TeletexString
mapping. Selection therefore does not establish complete RFC 5280
distinguished-name equality, validate the candidate's CA role or extensions,
verify the child's signature, process path length or name constraints,
recognize a trust anchor, check certificate time or revocation, construct a
complete path, perform TLS or HTTP, or open a network connection.

## Required tests

- exact child-issuer to candidate-subject callback direction;
- exactly one callback invocation per candidate in an accepted pool;
- complete-pool scanning after an early match;
- unique selection with exact child, candidate, and index binding;
- fail-closed zero-match and multiple-match outcomes;
- nonzero and exceeded candidate-count rejection before callback invocation;
- policy-backed exact-DER and cross-encoding DirectoryString selection;
- exact policy retention in success evidence distinct from generic callback
  evidence;
- policy-backed limit precedence and complete-pool ambiguity detection;
- fail-fast propagation of unconfigured, malformed, and unsupported value
  categories without retaining payloads;
- redacted errors and opaque success debug output.
