# chief-of-staff-daemon-config

`chief-of-staff-daemon-config` parses the configuration schema promised by the
D18 Chief specification. It consumes the repository TOML parser's fallible AST,
rejects duplicate and unknown data, and produces typed settings only after every
security invariant has been checked.

The closed schema includes an explicit loopback TCP port, durable orchestrator
state root, operator-credential file, package root, and shell-free host runtime
executable. Secure bootstrap and graceful-stop deadlines are non-zero and capped
at five minutes, so the production composition layer does not infer process
policy or wait without a bound.

The package performs no filesystem or environment access. `~` paths are resolved
only when the caller supplies an explicit absolute home directory, keeping daemon
composition deterministic and testable.

An optional `[data_plane]` table declares exact production authorities without
putting secret bytes in TOML. Directional channel-key entries bind canonical
UUID-v7 pipeline and channel identities plus an exact agent identity to raw
32-byte owner-only files. Ollama entries bind one unique launch model selector to
one explicit endpoint and a non-zero timeout capped at five minutes. The parser
also accepts bounded `smart_home_tool_grants` records with a stable grant ID,
exact Chief host principal, exact `smart_home.*` tool ID, operator identity,
positive Unix-millisecond issuance time, optional later expiry, and optional
`pending`, `active`, or `revoked` lifecycle state. Grant IDs are unique and every
inline record is closed. Existing `[data_plane]` configurations may omit this
additive field. The parser validates and retains only typed declarations; a
separate provisioning adapter performs all file, provider, and durable D23 grant
construction.

## Validation

```sh
cargo test -p chief-of-staff-daemon-config -- --nocapture
cargo clippy -p chief-of-staff-daemon-config --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-daemon-config --no-deps
```
