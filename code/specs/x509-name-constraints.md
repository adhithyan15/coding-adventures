# DNS NameConstraints Structure and Evaluation

## Status

Specification for the repository-owned structural decoders for optional
reviewed-root snapshot `name_constraints` contents and complete certificate
NameConstraints extension values. The snapshot bytes are SEQUENCE contents:
no X.509 extension wrapper and no outer universal SEQUENCE tag or length. That
decoder remains restricted to one non-empty `permittedSubtrees [0]` field. The
complete-DER decoder accepts non-empty permitted and excluded dNSName fields,
independently or together in schema order. A separate bounded evaluator applies
both sets to caller-supplied, already-decoded DNS subjectAltName values. None of
these operations constructs a path, binds evidence to a certificate or anchor,
grants trust, verifies a reference hostname, checks revocation, performs TLS or
HTTP, authorizes OAuth transport, or opens a network connection.

## Standards Boundary

RFC 5280 defines:

```text
NameConstraints ::= SEQUENCE {
     permittedSubtrees       [0]     GeneralSubtrees OPTIONAL,
     excludedSubtrees        [1]     GeneralSubtrees OPTIONAL }

GeneralSubtrees ::= SEQUENCE SIZE (1..MAX) OF GeneralSubtree

GeneralSubtree ::= SEQUENCE {
     base                    GeneralName,
     minimum         [0]     BaseDistance DEFAULT 0,
     maximum         [1]     BaseDistance OPTIONAL }
```

The module uses implicit tagging. Each supported subtree field starts with an
exact context-specific constructed tag `[0]` or `[1]`; that tag replaces the
universal SEQUENCE tag for `GeneralSubtrees`. Every child is an exact universal
constructed `GeneralSubtree` SEQUENCE. Its sole supported field is an exact
primitive context-specific dNSName `[2]` whose contents are non-empty IA5String
bytes. A complete extension value carries the outer universal SEQUENCE; the
reviewed snapshot contents form carries only the exact `[0]` field.

This decoder rejects all other shapes, including:

- empty input, unknown top-level fields, or trailing bytes;
- an empty permitted- or excluded-subtree collection;
- duplicate fields or excluded-before-permitted field order;
- a non-SEQUENCE GeneralSubtree;
- a base using any GeneralName choice other than dNSName `[2]`;
- an empty or non-ASCII dNSName;
- encoded `minimum` or `maximum` distance fields; and
- any trailing GeneralSubtree field.

Rejecting distance fields is intentional. RFC 5280 requires conforming CAs to
use the default minimum of zero and omit maximum, and this bounded slice does
not implement distance evaluation. A later extension may add that behavior
only with its own policy and adversarial tests.

The decoder does not interpret DNS constraint syntax. In particular, a
leading dot such as `.tr` is retained byte-for-byte after IA5 validation. The
evaluator owns the supported DNS syntax and subtree semantics described below;
self-issued-certificate handling remains future path-policy work.

## DNS Subtree Evaluation

The evaluator first validates every decoded permitted subtree and then every
decoded excluded subtree, even when an earlier valid subtree could decide the
result. A subtree is one ASCII domain suffix with
labels of 1 through 63 bytes, a base name of at most 253 bytes, ASCII letters,
digits, and hyphens only, and no leading or trailing hyphen. One leading dot is
permitted only as the subtree marker. Wildcards, empty labels, consecutive
dots, and a trailing root dot are rejected in constraints.

Presented DNS names use the same ASCII label and length rules. They may
contain one wildcard only when `*` is the complete left-most label and at least
one non-wildcard label follows. A trailing root dot is rejected because these
are stored certificate values rather than application presentation strings.
The evaluator performs no IDNA conversion; callers must supply already-encoded
ASCII A-labels.

Matching is case-insensitive and label-by-label, as required by RFC 5280. A
constraint without a leading dot matches the exact name and names formed by
adding one or more labels to its left. A leading-dot constraint matches only
names with at least one label to the left: `.tr` matches `example.tr` and
`*.example.tr`, but not `tr`, `eviltr`, or the invalid literal `.tr`.

Every presented DNS name is rejected if it matches an excluded subtree.
Exclusion takes precedence when both sets match. When a permitted set is
present, every remaining name must match at least one permitted subtree; when
it is absent, exclusion is the only subtree restriction. The empty presented
collection succeeds because DNS constraints do not constrain absent name
forms. One invalid, excluded, or unmatched presented name fails the whole
evaluation; the evaluator never accepts merely because another SAN matches.

## Limits and Representation

```rust
pub const MAX_DNS_SUBTREES: usize = 256;
pub const MAX_PRESENTED_DNS_NAMES: usize = 256;

pub struct NameConstraints<'a> {
    // fixed-capacity borrowed DNS subtree strings in wire order
}

impl<'a> NameConstraints<'a> {
    pub fn permitted_dns_subtrees(&self) -> &[&'a str];
    pub fn excluded_dns_subtrees(&self) -> &[&'a str];
    pub fn permitted_len(&self) -> usize;
    pub fn excluded_len(&self) -> usize;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}

pub fn decode_name_constraints_contents<'a>(
    decoder: &mut der_asn1::Asn1Decoder,
    encoded_contents: &'a [u8],
) -> Result<NameConstraints<'a>, NameConstraintsError>;

pub fn decode_name_constraints<'a>(
    decoder: &mut der_asn1::Asn1Decoder,
    encoded: &'a [u8],
) -> Result<NameConstraints<'a>, NameConstraintsError>;

pub fn evaluate_dns_names(
    constraints: &NameConstraints<'_>,
    presented_dns_names: &[&str],
) -> Result<ConstrainedDnsNames, DnsConstraintError>;
```

The result stores borrowed strings in two independently capped fixed-capacity
arrays. No count or wire length controls an allocation. More than
`MAX_DNS_SUBTREES` entries in either class fail before another value is
retained.

All traversal uses the caller's `Asn1Decoder`. The top-level subtree fields,
every GeneralSubtree SEQUENCE, and every dNSName consume the same total-element
budget. Opening a subtree field and each GeneralSubtree use the same
constructed-depth limit. Canonical identifier and length checks remain owned
by `der-tlv`, while typed tag and IA5 checks remain owned by `der-asn1`.

The presented-name collection is capped at `MAX_PRESENTED_DNS_NAMES`. Combined
with each decoder `MAX_DNS_SUBTREES` cap and the DNS textual length bounds,
evaluation has a fixed worst-case comparison budget and performs no allocation.
Successful evidence retains only the evaluated name count, not hostile names.

## Exact Direct-Link Composition

The direct reviewed-server anchor-policy composition consumes the existing
non-copyable server-link evidence. That evidence already binds one exact target
certificate to target role, identity, validity, purpose, and algorithm policy,
one authenticated RSA SHA-256 child-to-root link, and one exactly selected
reviewed Mozilla snapshot entry. Callers cannot separately provide constraint
bytes, a target certificate, or presented DNS strings to the composition.

If the selected snapshot entry has no out-of-band name constraints, the
composition succeeds without DNS-constraint evidence. If constraint bytes are
present, the composition:

1. decodes those exact selected-entry bytes with one caller-bounded decoder;
2. locates and decodes Subject Alternative Name from the exact target
   certificate retained by the server-policy evidence with a separate
   caller-bounded decoder; and
3. evaluates every decoded target dNSName against the decoded permitted DNS
   subtrees through the shared combined evaluator. Snapshot input never
   supplies excluded subtrees.

Malformed constraints, a missing or malformed target Subject Alternative Name,
invalid DNS syntax, or one DNS name outside every permitted subtree fails the
composition. Other GeneralName forms do not become DNS names. A target with no
dNSName values therefore satisfies DNS-only constraints because that name form
is absent. The non-copyable success result retains the complete consumed link
and an optional payload-free `ConstrainedDnsNames` result, preventing later code
from substituting another anchor, certificate, or SAN collection.

The two DER decoders have separate explicit `Asn1Limits`; resetting one
structure's budget cannot increase the bounded work inside the other. Together
with the 256-name and 256-subtree caps, this gives a fixed upper bound without
heap allocation in the NameConstraints decoder or evaluator.

## Errors and Offsets

Errors expose only a stable category and numeric offset. They retain no input
bytes or decoded names. Offsets are relative to the first octet of the supplied
NameConstraints SEQUENCE contents.

Required categories distinguish:

- typed DER structure failures;
- unsupported, duplicate, or reversed top-level fields;
- empty permitted or excluded subtrees;
- too many subtrees;
- invalid GeneralSubtree tags;
- unsupported GeneralName bases;
- empty DNS subtrees;
- invalid IA5 DNS subtrees; and
- unsupported or trailing GeneralSubtree fields.

Failed reads preserve the shared decoder and cursor transactionality promised
by `der-asn1`. Diagnostics must not include hostile constraint text.

Evaluation errors distinguish too many presented names, invalid permitted or
excluded subtree syntax, invalid presented-name syntax, an excluded presented
name, and a presented name outside all permitted subtrees. They expose only the
stable category and the index in the relevant collection. Debug and display
output never retain or print a DNS name or constraint.

## Adversarial Matrix

Decoder tests cover the exact reviewed bytes `a0 07 30 05 82 03 2e 74 72`,
permitted-only, excluded-only, and combined full-DER values, schema order,
duplicates, multiple data-driven DNS subtrees, independent capacity
boundaries, empty and malformed containers,
universal/application/private/primitive/wrong-number
top-level tags, wrong GeneralSubtree tags, every unsupported GeneralName tag,
empty and non-ASCII DNS values, minimum/maximum/trailing fields, shared depth
and element exhaustion, payload-local offsets, and redacted diagnostics.

Evaluator tests cover exact and subdomain matches, exclusion precedence, the
reviewed `.tr` leading-dot behavior, case folding, label boundaries, multiple
constraints and names, empty collections, complete-leftmost-label wildcards,
every syntax category, the 256-name boundary, unmatched-name failure after
another match, complete constraint prevalidation, deterministic indices, and
redacted diagnostics.

Composition tests cover absent constraints, the exact reviewed `.tr` bytes,
multiple target DNS names, DNS-absent SANs, malformed selected constraint bytes,
missing and malformed target SAN, constraint violations, caller decoder limits,
optional success evidence, and payload-free diagnostics.

## Deliberate Non-Goals

This slice does not decode the surrounding X.509 Extension structure, parse
non-DNS constraints, perform IDNA conversion, evaluate directory names or IP
constraints, construct a generalized multi-hop path, apply
self-issued-certificate rules, check revocation, perform TLS or HTTP, authorize
OAuth transport, or open a network connection. Successful standalone decoding
and evaluation retain their narrow meanings. Successful direct-link composition
adds only that the exact selected anchor's supported DNS constraints were
applied to every dNSName in the exact already-policy-validated direct child; it
does not establish complete certificate-path or transport readiness.
