# x509-ip-name-constraint-policy

Allocation-free RFC 5280 IPv4 and IPv6 NameConstraints policy.

The caller supplies ordered permitted and excluded `iPAddress` constraint
values plus presented Subject Alternative Name IP address values. Constraints
must contain an exact IPv4 address and mask (8 octets) or IPv6 address and mask
(32 octets). Masks must be CIDR-contiguous. Presented values must be exact IPv4
(4 octets) or IPv6 (16 octets) addresses. Exclusions take precedence, and a
non-empty permitted set requires each presented address to match a subnet of
the same address family.

## Security boundary

This package owns only public byte-level subnet policy. It does not decode DER,
NameConstraints, Subject Alternative Name, or certificates; bind evidence to
a certificate or path; process DNS or other GeneralName forms; construct or
validate paths; parse or operate on keys; verify signatures; select trust;
check revocation; perform TLS or HTTP; authorize OAuth; access credentials;
sign; perform I/O; or cause an external effect. Later repository-owned
composition must decode and bind the exact constraint and presented-name bytes.

## Development

```bash
bash BUILD
cargo clippy -p x509-ip-name-constraint-policy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-ip-name-constraint-policy --no-deps
```
