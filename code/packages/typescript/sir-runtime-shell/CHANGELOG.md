# Changelog

## 0.1.0 — initial release

First release of the SIR shell runtime for TypeScript/JavaScript, per
`code/specs/sir-runtime.md`. Provides the runtime landing point for the SIR
`backtick` builtin — the Ruby→SIR frontend lowers a backtick literal `` `cmd` ``
to `BuiltinCall("backtick", [StrLit cmd])`, which previously had no TypeScript
runtime, so emitted code using a backtick failed.

### Added

- `backtick(command): string` — run `command` via the system shell and return
  its captured stdout as a string, modelling Ruby's `` `cmd` `` expression.
  Implemented with `execSync(command, { encoding: "utf8" })` from the Node
  built-in `node:child_process` (no third-party dependencies). The child's exit
  status is ignored: `execSync` throws on a non-zero exit, so the error is caught
  and its captured `stdout` returned (falling back to `""`), mirroring Ruby
  returning stdout regardless of `$?`. Standard error is not part of the returned
  value (Ruby's backtick value is stdout only).
- `Val` — the universal SIR value type alias at this boundary.
- Full standard layout: `package.json`, `tsconfig.json`, `vitest.config.ts`,
  `BUILD`, `BUILD_windows`, `required_capabilities.json` (one `process`/`spawn`
  capability), README. vitest suite at 100% coverage; `tsc --strict` clean.

### Design note — running via the shell is intentional

Using `execSync` (which routes through the system shell) is load-bearing, not an
oversight: Ruby backtick is *defined* as "run via the system shell" (`/bin/sh -c`
on POSIX, `cmd.exe /c` on Windows), so shell features (pipes, redirections,
globbing, `$VAR`) are part of the builtin's contract. The `command` is
author-supplied — the string literal from the compiled program's own Ruby source
— and this package interpolates no external/untrusted runtime input, so it
introduces no new injection surface beyond what Ruby itself grants. The package
therefore declares a single `process`/`spawn` capability.
