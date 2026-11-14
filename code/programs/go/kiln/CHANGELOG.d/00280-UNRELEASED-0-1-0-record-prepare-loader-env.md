## [0.1.0] - UNRELEASED

### Fixed

- M3: the pinned git and the build tool no longer inherit the dynamic-loader
  environment. `source.NewGitAt(dir, bin, env)` runs git with an ALLOWLIST
  environment (`HOME`, the validated `PATH`, the validated `XDG_CONFIG_HOME`, plus
  fixed `GIT_CONFIG_NOSYSTEM`, `GIT_CONFIG_SYSTEM` and `GIT_CONFIG_GLOBAL` set to the
  null device, `GIT_OPTIONAL_LOCKS`, `GIT_TERMINAL_PROMPT`, `GIT_NO_REPLACE_OBJECTS`,
  `GIT_NO_LAZY_FETCH`, `LC_ALL=C`) and starts it in `/`. `record.LoaderEnv` refuses
  a non-empty `LD_PRELOAD` or `LD_AUDIT`, validates `LD_LIBRARY_PATH` like `PATH`,
  keeps `GCONV_PATH` and `LOCPATH` only if absolute and clear of the checkout,
  `tmpdir` and record directory, and drops every other `LD_*`; the build tool gets
  only the validated loader entries, which are also kept out of the state roots and
  recorded in `pre/tools.json` (`loader_env`). `strace` starts in `/`; the build
  tool starts in the record directory.
- `SafePath` resolves an entry that does not exist yet through its deepest existing
  ancestor (`/opt/x/bin` with `/opt/x` a link into the checkout).
- The `SEALED` failure path restores the directories to 0700 before removing
  `SEALED`, and reports both errors if the remove fails.
- `searchable` judges the effective uid, gid and groups against the mode bits (and
  confirms a "no" with `access(2)`); unknown counts as searchable.

### Specification

- KILN04 section 4.8 P11: the allowlist and loader rules, `tools.json` `loader_env`,
  and a 2b-2 item: `record step` refuses unless its inherited `PATH` and loader
  variables equal `tools.json`'s.
