## [0.1.1] - 2026-11-24

### Security

- PIF-003a security review: the gate fails closed on an unvetted base (`--allow` or `--signoff` required, exit 3, `--unsafe-no-allow` to override, `base_trust` in the report); a sign-off must carry `base-environment`, `goal` lines and `goals-sha256` and its goal ids are validated and unique; a chain needs a claim file unless `--allow-no-claim`; allow-list lines bind the role (`<role> <digest>`); `withdrawn_logic` is passed to the reader (`--withdrawn-logic`); control files and store entries must be regular files (symlink and FIFO safe).
