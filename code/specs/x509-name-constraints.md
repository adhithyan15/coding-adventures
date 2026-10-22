# Reviewed-Anchor DNS NameConstraints Structure

## Status

Specification for the first repository-owned structural decoder for the
optional `name_constraints` bytes carried by a reviewed Web PKI trust anchor.
The reviewed snapshot preserves the upstream `webpki-roots` bytes exactly. In
that data model the bytes are the DER contents of the RFC 5280
`NameConstraints` SEQUENCE, not an X.509 extension wrapper and not the outer
universal SEQUENCE tag and length.

This first slice supports only the shape present in the pinned reviewed root
set: one `permittedSubtrees [0]` field containing one or more DNS
`GeneralSubtree` values. It is a parser, not a subtree evaluator, path
validator, trust decision, hostname verifier, revocation policy, TLS stack,
HTTP client, OAuth transport, or network authority.

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

The module uses implicit tagging. Therefore the supported input starts with an
exact context-specific constructed tag `[0]`; that tag replaces the universal
SEQUENCE tag for `GeneralSubtrees`. Every child is an exact universal
constructed `GeneralSubtree` SEQUENCE. Its sole supported field is an exact
primitive context-specific dNSName `[2]` whose contents are non-empty IA5String
bytes.

This decoder rejects all other shapes, including:

- empty input or trailing top-level fields;
- `excludedSubtrees [1]`, or both permitted and excluded fields;
- an empty permitted-subtree collection;
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
leading dot such as `.tr` is retained byte-for-byte after IA5 validation. DNS
label validity, subtree matching, permitted/excluded intersection, and
self-issued-certificate handling belong to later policy layers.

## Limits and Representation

```rust
pub const MAX_DNS_SUBTREES: usize = 256;

pub struct NameConstraints<'a> {
    // fixed-capacity borrowed DNS subtree strings in wire order
}

impl<'a> NameConstraints<'a> {
    pub fn permitted_dns_subtrees(&self) -> &[&'a str];
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}

pub fn decode_name_constraints_contents<'a>(
    decoder: &mut der_asn1::Asn1Decoder,
    encoded_contents: &'a [u8],
) -> Result<NameConstraints<'a>, NameConstraintsError>;
```

The result stores borrowed strings in one fixed-capacity array. No count or
wire length controls an allocation. More than `MAX_DNS_SUBTREES` entries fail
before an additional value is retained.

All traversal uses the caller's `Asn1Decoder`. The top-level `[0]` element,
every GeneralSubtree SEQUENCE, and every dNSName consume the same total-element
budget. Opening `[0]` and each GeneralSubtree use the same constructed-depth
limit. Canonical identifier and length checks remain owned by `der-tlv`, while
the typed tag and IA5 checks remain owned by `der-asn1`.

## Errors and Offsets

Errors expose only a stable category and numeric offset. They retain no input
bytes or decoded names. Offsets are relative to the first octet of the supplied
NameConstraints SEQUENCE contents.

Required categories distinguish:

- typed DER structure failures;
- unsupported top-level fields;
- empty permitted subtrees;
- too many subtrees;
- invalid GeneralSubtree tags;
- unsupported GeneralName bases;
- empty DNS subtrees;
- invalid IA5 DNS subtrees; and
- unsupported or trailing GeneralSubtree fields.

Failed reads preserve the shared decoder and cursor transactionality promised
by `der-asn1`. Diagnostics must not include hostile constraint text.

## Adversarial Matrix

Package tests cover the exact reviewed bytes `a0 07 30 05 82 03 2e 74 72`,
multiple data-driven DNS subtrees, the fixed capacity boundary, empty and
malformed containers, universal/application/private/primitive/wrong-number
top-level tags, wrong GeneralSubtree tags, every unsupported GeneralName tag,
empty and non-ASCII DNS values, minimum/maximum/trailing fields, shared depth
and element exhaustion, payload-local offsets, and redacted diagnostics.

## Deliberate Non-Goals

This slice does not decode an `id-ce-nameConstraints` extension, parse excluded
subtrees or non-DNS names, validate DNS syntax, evaluate a certificate name,
bind the result to selected-anchor evidence, construct or validate a path,
grant trust, check time or revocation, verify signatures, perform TLS or HTTP,
authorize OAuth transport, or open a network connection. Successful decoding
proves only that the supplied bytes match this narrow canonical structure.
