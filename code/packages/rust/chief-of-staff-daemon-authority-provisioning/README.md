# chief-of-staff-daemon-authority-provisioning

This package turns the daemon's strictly typed `[data_plane]` configuration
into the immutable model-provider authority consumed by
`chief-of-staff-host-data-plane`.

Each Ollama declaration registers its model tag as the exact launch selector.
Endpoints must be explicit `http://host:port` authorities, and timeouts are
non-zero and capped at five minutes. Provisioning makes no reachability
probe, so daemon startup never turns a temporarily stopped model server into a
configuration mutation or a secret-bearing error.

## Channel keys are not here

Until D18S P2.6d-2c this package also read every `[data_plane]
channel_keys` file into the daemon. It no longer reads any. Each agent's
channel keys belong to that agent's broker: the broker launcher
(`chief-of-staff-broker-launcher`) opens one agent's key files, through
`chief-of-staff-daemon-secret-file`, only when it launches that agent's
broker. The daemon refuses to start with `channel_keys` and no
`[hosts.broker]`.

## Validation

```sh
cargo test -p chief-of-staff-daemon-authority-provisioning -- --nocapture
cargo clippy -p chief-of-staff-daemon-authority-provisioning --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-daemon-authority-provisioning --no-deps
```
