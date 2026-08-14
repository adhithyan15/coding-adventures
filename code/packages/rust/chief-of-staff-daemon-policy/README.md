# chief-of-staff-daemon-policy

`chief-of-staff-daemon-policy` supplies the first production policy adapters for
the D18 Chief daemon. A 256-bit bearer credential authenticates a loopback client
with constant-time comparison and zeroized storage. Successful authentication
creates an opaque connection-local operator session; all current lifecycle API
operations require that session. The session exposes only the stable non-secret
`operator:local` requester identity used to construct Trust Checker context;
the bearer credential itself is never retained in that context.

Channel and pipeline topology changes use an exact immutable tier resolver built
from the validated daemon config. Every referenced agent, channel, package hash,
and selected model must be explicitly assigned; missing authority fails closed.
Trust Checker can authorize a fully declared Tier 0 request without interaction.
An optional validated Tier 1 notification command is launched directly through
the environment-cleared, bounded protocol in `chief-of-staff-notification-approval`.
Omitting it keeps every interactive request unavailable; configuring it enables
only Tier 1, while biometric Tier 2 and hardware-key Tier 3 remain fail-closed.
The local bearer authenticates the requester but never acts as privilege approval.

The package generates credential material but performs no terminal or network
access. Outer composition owns protected persistence, helper selection, and
delivery to the CLI.

## Validation

```sh
cargo test -p chief-of-staff-daemon-policy -- --nocapture
cargo clippy -p chief-of-staff-daemon-policy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-daemon-policy --no-deps
```
