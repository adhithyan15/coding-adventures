## [0.1.0] - UNRELEASED

### Fixed

- M4: the build tool starts `git` with whatever environment it inherited, so a
  `GIT_CONFIG_GLOBAL` file in the checkout naming a program as `core.fsmonitor`
  would have run it unsandboxed. `prepare` now gives the build tool an ALLOWLIST
  environment (`buildToolEnv`): `HOME`, the validated `PATH`, the validated
  `LD_LIBRARY_PATH` if one was set, `source.PinnedEnv()` (git's system and global
  configuration the null device, no locks or prompts, no replace refs, no lazy
  fetch, `LC_ALL=C`) and `source.ConfigEnv()` (`core.fsmonitor=false`,
  `core.hooksPath` the null device, `protocol.file.allow=never` as
  `GIT_CONFIG_COUNT` entries, mirroring Kiln's own `-c` hardening, now one shared
  list). The build tool reads no other variable for its three exports.
  `pre/tools.json` records the exact environment as `build_tool_env` (replacing
  `loader_env`).
- `GCONV_PATH` and `LOCPATH` (colon lists, previously checked as one path) and every
  other `LD_*` are no longer passed on at all; `LD_PRELOAD` and `LD_AUDIT` still
  refuse `prepare`.
- `SafePath` follows a dangling symlink by hand (relative or absolute, with a
  remainder beneath it), at most 40 hops; a loop or too many hops refuses the entry.
- L1: `resolveDeepest` walks the path one component at a time and applies `..` to
  the physically resolved path, never lexically across a link (`a/b ->
  <checkout>/d` with `<checkout>/d/l -> ../x/bin` dangling: the entry `a/b/l` is
  `<checkout>/x/bin` and is refused). The `buildToolEnv` comment no longer claims
  the exports run toolchain probes. P11's 2b-2 item says `record step` compares
  the inherited environment as a multiset of `NAME=value` entries without the
  three `KILN_*` variables and refuses duplicate names.
- `source.PinnedEnv` and `source.ConfigEnv` are exported so both paths use one
  definition.

### Specification

- KILN04 4.8 P11: the allowlists and the reason for each entry, the build tool's
  environment record, and the 2b-2 obligation: the workflow launches the build tool
  with exactly `build_tool_env`, and `record step` refuses unless its inherited
  environment, apart from the three `KILN_*` variables, equals it.

### Tests

- With `GIT_EXEC_PATH`, `GIT_CONFIG_GLOBAL`, `GIT_CONFIG_COUNT`, `GIT_DIR`, `LD_*`,
  `GCONV_PATH`, `GLIBC_TUNABLES`, `TZDIR`, `GITHUB_*` and a secret all set, the
  fake build tool sees exactly the allowlist. The real build-tool round trip runs
  with that minimal environment and passes.
