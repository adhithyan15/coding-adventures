# Reviewed webpki-roots snapshot

**Status:** Implemented as a data-only prerequisite. It is not a certificate
verifier, path validator, platform trust store, TLS stack, or HTTPS transport.

## Source and review record

The sole external input is the `webpki-roots` 1.0.9 crates.io archive at
upstream tag `v/1.0.9`, commit
`0a553dbc8b3f18ea05c4f881cffa3f2d005d0d30`. Its crates.io SHA-256 is
`7dcd9d09a39985f5344844e66b0c530a33843579125f23e21e9f0f220850f22a`.
The reviewed set contains 121 ordered trust anchors and has canonical root-set
SHA-256 `0aa729fbb339061e8aad2e49fefd83674cd6b2e45eeb586c0d456cf33b70dc01`.

No upstream executable code is a production dependency. In particular,
`webpki-roots`, its `rustls-pki-types` dependency, Rustls, WebPKI libraries,
and certificate generators are not compiled or linked. A standard-library-only
review tool extracts the published static subject, SubjectPublicKeyInfo, and
optional name-constraints DER bytes into the checked-in snapshot. It reads the
one exact regular `webpki-roots-1.0.9/src/lib.rs` member directly from the
checksum-verified crate archive and rejects separate source inputs, links,
duplicates, unexpected member paths or types, invalid sizes, and malformed
UTF-8.

## Canonical format

The binary snapshot begins with the exact ASCII and NUL magic
`webpki-roots-canonical-v1\0`. Each anchor then appears in upstream order as:

1. an eight-byte big-endian subject length and the subject DER bytes;
2. an eight-byte big-endian SubjectPublicKeyInfo length and those DER bytes;
3. one byte: zero for absent name constraints or one for present constraints;
4. when present, an eight-byte big-endian constraint length and its DER bytes.

Lengths must fit the host `usize`; subjects, keys, and present constraints must
be non-empty; the constraint flag must be zero or one; and the input must end
exactly at an anchor boundary. The root-set digest is SHA-256 over the entire
canonical binary snapshot, including its domain-separating magic and framing.

## Update and CI policy

`snapshot.json`, package constants, the binary, and documentation pin the exact
reviewed provenance. The package BUILD runs a standard-library-only verifier;
Rust tests independently enforce the format, root count, and digest. Any root
addition, removal, reorder, or byte change fails CI until all reviewed pins are
updated together.

Deployed clients must check upstream at least every 90 days and within 14 days
of a known Mozilla removal or trust incident. Updates require explicit review;
version ranges and unattended refreshes are forbidden. This static Mozilla set
excludes OS, enterprise, user-installed, and local roots.

## Non-authority

The snapshot only supplies candidate trust-anchor bytes. It performs no full
certificate parsing, constraint enforcement, path construction or validation,
hostname or signature verification, algorithm or key policy, validity or
revocation checking, TLS, HTTP, socket access, or platform-trust integration.
Concrete OAuth HTTPS remains blocked on those repository-owned layers.
