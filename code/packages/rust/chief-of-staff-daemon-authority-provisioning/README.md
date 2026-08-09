# chief-of-staff-daemon-authority-provisioning

This package turns the daemon's optional, strictly typed `[data_plane]`
configuration into the two immutable authorities consumed by
`chief-of-staff-host-data-plane`.

Each channel-key declaration is scoped to one canonical UUID-v7 pipeline, one
exact agent identity, one canonical UUID-v7 channel, and one direction. Raw
32-byte files are resolved only against the explicit home directory and read
through `chief-of-staff-daemon-secret-file`, so links, non-regular objects,
foreign ownership, broad permissions, short reads, and long reads all fail
closed. Temporary and retained secret buffers are zeroizing.

Each Ollama declaration registers its model tag as the exact launch selector.
Endpoints must be explicit `http://host:port` authorities and timeouts are
non-zero and capped at five minutes. Provisioning performs no reachability
probe, so daemon startup never turns a temporarily stopped model server into a
configuration mutation or a secret-bearing error.

The package deliberately does not inject the resulting authorities into the
daemon composition root. That remains a small follow-up once this loading
boundary is independently reviewed and merged.

## Validation

```sh
cargo test -p chief-of-staff-daemon-authority-provisioning -- --nocapture
cargo clippy -p chief-of-staff-daemon-authority-provisioning --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-daemon-authority-provisioning --no-deps
```
