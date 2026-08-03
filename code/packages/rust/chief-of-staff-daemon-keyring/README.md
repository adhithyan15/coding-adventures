# chief-of-staff-daemon-keyring

`chief-of-staff-daemon-keyring` turns the D18 daemon's validated trusted-key
configuration into the `PackageKeyring` used by signed-package verification.
Each configured `.pub` file contains exactly 32 raw Ed25519 public-key bytes.
Keys must use a canonical non-identity point in the prime-order subgroup.
Production keys receive the existing Tier 3 ceiling; developer keys remain
capped at Tier 1.

The adapter resolves `~/` only against an explicit home directory. It rejects
missing paths, final-component symlinks, directories, devices, files of any
other length, and path replacement detected between inspection and the opened
file handle. Error messages expose neither configured paths nor key bytes. It
never reads the process environment or writes key material.

## Validation

```sh
cargo test -p chief-of-staff-daemon-keyring -- --nocapture
cargo clippy -p chief-of-staff-daemon-keyring --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-daemon-keyring --no-deps
```
