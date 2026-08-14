# chief-of-staff-cli-core

`chief-of-staff-cli-core` is the transport-free operator command layer for the
D18 Chief daemon. It parses a small declarative `cli-builder` command tree and
dispatches host lifecycle operations through an injected, already-authenticated
daemon client.

The current commands are `install-daemon`, `agents`, `doctor`, `register`,
`start`, `stop`, `reconcile`, `deregister`, `wire`, and `unwire`.
`install-daemon` is a typed local action; the remaining commands require an
already-authenticated daemon client. Credentials and socket endpoints are
deliberately absent from argv so the executable adapter can resolve them through
local trusted configuration.

`wire` constructs one complete `HostPipelineBinding` before dispatch. Its
package hash and agent identity use lowercase hexadecimal argv values, while
pipeline and channel identities use canonical lowercase dashed UUID-v7 text.
`--channel NAME:read|write:UUID_V7` is repeatable and canonicalized by the shared
launch contract. Optional Level 1 model settings must provide
`--model`, `--temperature`, and `--max-tokens` together. `unwire` accepts only
the validated host name; the authenticated daemon and Trust Checker remain the
sole mutation and authorization authority.

## Validation

```sh
cargo test -p chief-of-staff-cli-core -- --nocapture
cargo clippy -p chief-of-staff-cli-core --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-cli-core --no-deps
```
