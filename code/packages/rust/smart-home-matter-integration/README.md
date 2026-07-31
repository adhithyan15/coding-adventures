# smart-home-matter-integration

`smart-home-matter-integration` connects commissioned Matter application
endpoints to the normalized D23 smart-home runtime.

The adapter installs an opaque fabric/controller boundary, projects externally
commissioned nodes and endpoint cluster inventories into normalized topology,
maps typed Matter attribute reports into confirmed state, and creates
authorized Matter command invocations for a secure-session host to deliver.

This package does not claim to implement commissioning, CASE/PASE, certificate
validation, fabric storage, Interaction Model encoding, subscriptions, or
network I/O. Those remain an explicit production host boundary. Durable records
contain only a `VaultRef`, never fabric key or certificate bytes.

## Validation

```sh
./smart-home-matter-integration/BUILD
cargo clippy -p smart-home-matter-integration --all-targets -- -D warnings
```
