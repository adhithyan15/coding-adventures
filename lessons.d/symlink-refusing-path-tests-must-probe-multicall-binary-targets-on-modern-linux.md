---
category: Testing & coverage
---

# Symlink-refusing path tests must probe multicall binary targets on modern Linux

Security-hardened predicates that validate root-owned executable binaries often deliberately use `std::fs::symlink_metadata` rather than `metadata` or `Path::exists` to ensure symbolic links pointing into user-writable directories are refused (e.g. `vault-pm-cli-host`'s `is_trusted_program`).

Unit tests asserting the positive case on live hosts often construct a list of candidate standard utilities (such as `/bin/cat`, `/usr/bin/cat`, `/bin/ls`, `/usr/bin/ls`, `/usr/bin/env`) and filter for candidates where `symlink_metadata().is_file()` is true.

However, on modern Linux distributions (such as Ubuntu 25+ or CI runners transitioning to `uutils-coreutils` or BusyBox multicall layouts), classic individual utilities like `cat`, `ls`, and `env` are implemented as symlinks pointing to `/usr/bin/coreutils` (or `uutils`). Because `symlink_metadata` does not follow the leaf symlink, `is_file()` returns `false` for every symlink in the candidate list. If the candidate list contains only those symlinks, the filtered set becomes empty and the test panics with `"no standard utility resolved to a regular file on this host"`.

When writing tests that probe standard system binary directories for root-owned regular files under a symlink-refusing predicate:
1. Include multicall binaries and standalone shells in the candidate set: `/usr/bin/coreutils`, `/usr/bin/dash`, `/usr/bin/bash`, `/usr/bin/grep`, `/usr/bin/sed`, `/usr/bin/tar`, `/usr/bin/gzip`, along with their traditional `/bin` aliases.
2. Ensure the test continues to require that any candidate evaluated as a regular file passes the strict trust predicate (`uid == 0`, non-group/world-writable regular file).
