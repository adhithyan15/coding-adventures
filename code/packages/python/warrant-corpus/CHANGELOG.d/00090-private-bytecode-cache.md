## Unreleased

- New `isolation.verify_isolated_cache`: `run`, `check` and `sabotage` refuse to start (named error `pycache-isolation`, exit 2) unless `sys.pycache_prefix` is a private directory under the temporary directory, owned by the current user, mode 0700, named in `WARRANT_PYCACHE_PRIVATE` (the entry script creates it and re-runs itself before importing). The manifest records `pycache_isolated: true` (required by `validate-manifest`); sabotage children inherit the cache. The test suite runs under a private cache fixture. VER04i 8.1.
