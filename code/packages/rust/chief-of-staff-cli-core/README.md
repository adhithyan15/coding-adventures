# chief-of-staff-cli-core

`chief-of-staff-cli-core` is the transport-free operator command layer for the
D18 Chief daemon. It parses a small declarative `cli-builder` command tree and
dispatches host lifecycle operations through an injected, already-authenticated
daemon client.

The current commands are `install-daemon`, `agents`, `doctor`, `register`,
`start`, `stop`, `reconcile`, and `deregister`. `install-daemon` is a typed local
action; the remaining commands require an already-authenticated daemon client.
Credentials and socket endpoints are deliberately absent from argv so the
executable adapter can resolve them through local trusted configuration.

## Validation

```sh
cargo test -p chief-of-staff-cli-core -- --nocapture
cargo clippy -p chief-of-staff-cli-core --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-cli-core --no-deps
```
