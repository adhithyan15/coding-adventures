# chief-of-staff-daemon-config

`chief-of-staff-daemon-config` parses the configuration schema promised by the
D18 Chief specification. It consumes the repository TOML parser's fallible AST,
rejects duplicate and unknown data, and produces typed settings only after every
security invariant has been checked.

The package performs no filesystem or environment access. `~` paths are resolved
only when the caller supplies an explicit absolute home directory, keeping daemon
composition deterministic and testable.

## Validation

```sh
cargo test -p chief-of-staff-daemon-config -- --nocapture
cargo clippy -p chief-of-staff-daemon-config --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-daemon-config --no-deps
```
