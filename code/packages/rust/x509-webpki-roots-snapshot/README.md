# x509-webpki-roots-snapshot

This package contains one reviewed Mozilla public-Web-PKI trust-anchor snapshot
derived from `webpki-roots` 1.0.9. Only the generated anchor bytes enter the
production package. Neither `webpki-roots`, `rustls-pki-types`, `rustls`, nor a
WebPKI implementation is compiled or linked.

## Reviewed provenance

- Upstream repository: `rustls/webpki-roots`
- Release/tag: `1.0.9` / `v/1.0.9`
- Upstream tag commit: `0a553dbc8b3f18ea05c4f881cffa3f2d005d0d30`
- crates.io archive SHA-256:
  `7dcd9d09a39985f5344844e66b0c530a33843579125f23e21e9f0f220850f22a`
- Root count: `121`
- Canonical root-set SHA-256:
  `0aa729fbb339061e8aad2e49fefd83674cd6b2e45eeb586c0d456cf33b70dc01`

The canonical digest covers the exact ordered sequence of subject DER,
SubjectPublicKeyInfo DER, and optional name-constraints DER, with the format
and length framing defined in `code/specs/x509-webpki-roots-snapshot.md`.
`tools/verify_snapshot.py`, the package tests, and the pinned constants all fail
closed if those bytes, their order, count, format, or provenance drift.
The generator reads only the exact regular
`webpki-roots-1.0.9/src/lib.rs` member directly from the checksum-verified
crate archive; separate source files, links, duplicate members, and unexpected
member paths or types are rejected.

## Deployment policy

This is a static Mozilla public-Web-PKI snapshot. It deliberately excludes OS,
enterprise, user-installed, and local trust roots. Deployed clients must review
for a new upstream release at least every 90 days and within 14 days of a known
Mozilla root removal or trust incident. Updates require a new exact tag/commit,
archive checksum, root count, canonical digest, generated snapshot, tests, and
security review; version ranges and unattended root changes are forbidden.

## Security boundary

The package supplies trust-anchor data only. It does not parse certificates,
construct or validate paths, verify names or signatures, enforce algorithms or
key strength, process constraints, check revocation, read a clock, implement
TLS or HTTP, open sockets, or provide platform trust. A root's presence is not
proof that any certificate chains to it or is currently trustworthy.

The checked-in `LICENSE` is the upstream CDLA-Permissive-2.0 data license.

## Verification

```bash
bash BUILD
cargo clippy -p x509-webpki-roots-snapshot --all-targets -- -D warnings
cargo doc -p x509-webpki-roots-snapshot --no-deps
```
