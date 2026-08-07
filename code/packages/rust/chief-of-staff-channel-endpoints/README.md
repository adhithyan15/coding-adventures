# chief-of-staff-channel-endpoints

`chief-of-staff-channel-endpoints` turns the D18 channel crypto and durable log
primitives into an authorized one-way channel API.

Every durable channel definition binds exactly one originator identity and
Ed25519 public key to a non-empty, duplicate-free receiver set whose X25519
public keys are also persisted. The originator cannot appear in that receiver
set. Definitions are created atomically, survive backend restart, and can move
one way from active to destroyed.

`DurableOriginator` checks that definition before publishing or distributing a
sealed epoch key. `DurableReceiver` checks membership before reading, verifies
and decrypts every message, and only acknowledges message IDs that it actually
delivered in the current session. Callers inject message IDs and timestamps
through `MessageMetadataSource`, keeping clock and random access outside this
trust-boundary package.

## Validation

```sh
cargo test -p chief-of-staff-channel-endpoints -- --nocapture
cargo clippy -p chief-of-staff-channel-endpoints --all-targets -- -D warnings
```
