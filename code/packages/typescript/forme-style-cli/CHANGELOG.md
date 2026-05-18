# Changelog — @coding-adventures/forme-style-cli

## 0.1.0 — 2026-05-17

Initial release.  Seventh FM04-family package — the first with
non-empty `capabilities` (`["fs"]`).  Per FM03 §5 CLI surface.

### Added

- **npm `bin` entry `forme-style`** wired to `src/bin.ts` (a thin
  shim around the testable `run(argv, io)` function).
- `run(argv, io): Promise<number>` — programmatic entry point
  exported from `src/index.ts`.  Returns the process exit code
  rather than calling `process.exit`, so tests can exercise the
  CLI in-process without subprocess spawning.
- `CliIO` interface — injectable side-effect surface
  (`stdout` / `stderr` write sinks, `readFile` / `writeFile`
  promises, `readStdin` promise).
- Exit-code constants `EXIT_OK` (0), `EXIT_VALIDATOR_FAIL` (1),
  `EXIT_IO_OR_ARG_ERROR` (2), `EXIT_UNKNOWN_TARGET` (3).

### CLI surface

```
forme-style <doc.json> --target <css|latex|terminal>
            [--theme NAME] [--themes themes.json]
            [--active CTX,...] [--used ID,...]
            [--scope STR] [--out FILE]
            [--help|-h]
```

`-` as the positional argument reads the document from stdin.
Omitting `--out` writes to stdout.

### Spec adherence

Implements FM03 §5 CLI surface verbatim.  No spec divergences.

### Behavioural notes

- **Validator rejection ⇒ exit 1** with all errors printed to
  stderr (multi-error reporting — one line per `StyleErrorEntry`).
- **Warnings always print to stderr**, never suppress success.  A
  translator that warn-skipped a property still produces exit 0.
- **Theme name without `--themes` flag ⇒ exit 2** with the
  required-flag message.
- **Malformed individual theme entries are silently skipped** so a
  single bad theme in a larger themes file doesn't take down the
  whole run.  Top-level shape violations (themes not an array, etc.)
  exit 2.
- **`--active` / `--used` lists** split on comma; whitespace
  trimmed; empty entries dropped.
- **Output to stdout for non-terminal targets ends with a newline**
  if the underlying string doesn't already (avoids the shell prompt
  sharing a line with the last bytes).

### Security posture

Three concerns addressed:

- **Untrusted JSON input.**  `JSON.parse` failures (document or
  themes file) are captured with the error message and surface as
  exit 2 — never a stack trace to the user.
- **Theme registry from untrusted file.**  Loader iterates the
  `themes` array and hands each entry to the registry's `register`
  which itself refuses forbidden names (`__proto__` etc.).
  Malformed entries skip silently.
- **Path handling.**  Paths are passed verbatim to the injected
  `io.readFile` / `io.writeFile`.  The production wiring uses
  `fs.promises.readFile/writeFile`, which honour OS permissions.
  No path normalisation / sandboxing performed — caller's
  responsibility per CLI conventions.

### Capabilities

`["fs"]` — first FM04 package with non-empty capabilities.
Justification: reads the input document file (or stdin), reads an
optional themes file, writes output to a file or stdout.  No
network, no shell, no env reads beyond `process.argv`.

### Tests

30 tests in `cli.test.ts`:

- Help: `--help`, `-h`, no-args (3)
- Happy-path dispatch: CSS / LaTeX / terminal (3)
- Stdin / stdout / `--out` file streaming (2)
- Themes: load + apply / missing → warn / no `--themes` rejected /
  bad JSON / non-object top-level / missing `themes` array /
  malformed-entries skipped (7)
- Context / used / scope pass-through (4)
- Every exit-code path: validator fail / missing input file / bad
  JSON / missing --target / unknown --target / unknown flag / too
  many positional / flag without value / flag-followed-by-flag /
  write failure (10)
- Warning propagation (1)

Coverage: **97.12% line / 95.78% branch** — above the FM04 §14.4
≥95% line target.  Uncovered lines are the orchestrator
"unexpected exception" defensive catch (unreachable via the
orchestrator's documented contract) and the trailing-newline
branch for non-terminal output.

### v0 simplifications (documented)

- **No `--watch` mode.**  Lands with the dev server (forme-dev-server).
- **No `--config` global-defaults file.**  Lands when a documented
  config format exists.
