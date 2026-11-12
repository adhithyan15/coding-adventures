# X.509 Name Value Policy

## Status

Implemented as a zero-external-dependency Rust policy layer above
`x509-name`, `x509-name-comparison`, and `x509-directory-string`.

## Purpose

Bind a bounded caller-supplied set of attribute-type object identifiers to
repository-owned value-comparison rules before structural distinguished-name
comparison. This removes executable comparison callbacks from issuer-link and
path-construction callers while keeping the attribute registry data driven.

Each policy entry selects exactly one of these rules:

- exact canonical DER element equality; or
- exact Unicode-scalar equality across supported RFC 5280 DirectoryString
  encodings.

An empty policy is valid and rejects every non-empty Name. Policy construction
rejects more than 64 entries, malformed object identifiers, and duplicate
identifiers. Comparison preflights every attribute in both Names before any
pair matching begins. An unconfigured attribute type fails closed. For an
attribute configured with the DirectoryString-scalar rule, a malformed value
or TeletexString without a repository-owned T.61 mapping also fails closed with
a payload-free error. The exact-DER rule intentionally treats the complete
canonical value element as opaque and does not assert a DirectoryString schema.

## Public contract

```rust
pub enum X509NameValueComparisonRule {
    ExactDer,
    DirectoryStringScalar,
}

pub struct X509NameValuePolicyEntry<'a> { /* private fields */ }
pub struct X509NameValuePolicy<'a> { /* private fields */ }

impl<'a> X509NameValuePolicyEntry<'a> {
    pub const fn new(
        attribute_type: &'a [u64],
        rule: X509NameValueComparisonRule,
    ) -> Self;
}

impl<'a> X509NameValuePolicy<'a> {
    pub fn new(entries: &'a [X509NameValuePolicyEntry<'a>])
        -> Result<Self, X509NameValuePolicyDefinitionError>;

    pub fn compare_names(
        self,
        left: &X509Name<'_>,
        right: &X509Name<'_>,
    ) -> Result<bool, X509NameValueComparisonError>;
}
```

The policy borrows validated definition data and allocates nothing from
certificate input. At most 128 attributes are preflighted across two bounded
Names. Structural matching remains bounded by `x509-name-comparison` to 4,096
candidate attribute comparisons.

## Tests

Tests cover:

- invalid, duplicate, over-limit, and empty policy definitions;
- data-driven exact-DER and DirectoryString-scalar selection;
- cross-encoding DirectoryString equality and case-sensitive mismatch;
- unconfigured attribute types in either Name;
- malformed and unsupported TeletexString values under the DirectoryString-
  scalar rule in either Name;
- opaque TeletexString and non-DirectoryString equality under exact-DER policy;
- structural mismatch without policy-data leakage; and
- repeated attribute types under the existing fixed Name bounds.

## Deliberate exclusions

- RFC 4518 StringPrep, Unicode normalization, case folding, insignificant-space
  handling, T.61 mapping, or built-in attribute schemas;
- a built-in attribute OID registry or a claim of complete RFC 5280
  distinguished-name equality;
- issuer selection, certificate-link authentication, path construction or
  validation, directoryName constraints, or trust-anchor processing;
- signature verification, algorithms, clocks, revocation, hostname
  verification, trust-root loading, TLS, sockets, HTTP, OAuth transport, or
  any credential or external effect.
