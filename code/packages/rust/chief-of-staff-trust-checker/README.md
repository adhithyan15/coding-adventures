# chief-of-staff-trust-checker

`chief-of-staff-trust-checker` is the transport-independent privilege gate for
D18 pipeline and Vault authority changes. It validates a bounded exact resource
set, computes the maximum privilege tier, and delegates every non-Tier-0
decision to an injected trusted approval provider.

The core preserves the D18 tier policy:

- Tier 0 proceeds without contacting the provider.
- Tier 1 requests a notification for five seconds and approves only an explicit
  consent or a provider-reported timeout.
- Tier 2 requires biometric assurance within thirty seconds.
- Tier 3 requires hardware-key assurance within sixty seconds.

Denied Tier 1 requests and every Tier 2/3 denial or timeout fail closed. An
approval weaker than the required assurance also fails closed. The provider
owns notification, biometric, hardware-key, clock, and platform interaction;
this crate performs no filesystem, network, environment, terminal, or device
access.

## Validation

```sh
cargo test -p chief-of-staff-trust-checker -- --nocapture
cargo clippy -p chief-of-staff-trust-checker --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-trust-checker --no-deps
```
