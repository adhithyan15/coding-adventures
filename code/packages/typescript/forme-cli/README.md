# @coding-adventures/forme-cli

The headless product driver for Forme. It loads a TypeScript or JavaScript
project config, validates the typed DAG, runs the orchestrator, formats stable
diagnostics, and returns documented process exit codes.

The command surface is declared in `forme.cli.json` and parsed by
`@coding-adventures/cli-builder`. The shared parser owns subcommand routing,
flag validation, duplicate detection, fuzzy flag suggestions, help, and version
output; the Forme package only dispatches the validated result to product
behavior.

```bash
forme check
forme build
forme run # FM03-compatible alias for build
forme build --reproducible
forme build --report dist/.forme-build-report.json
forme clean
forme watch
forme watch --port 4321 --debounce 100
forme build --config pipelines/site.ts
forme build --help
```

The default config search checks `forme.config.ts`, `.mts`, `.js`, and `.mjs`
in the current directory. Relative stage paths execute from the config's
directory, so the same project behaves consistently whether the CLI is invoked
inside a monorepo, through an npm script, or from an installed external
project.

## Commands

- `build` validates the config and DAG, runs the pipeline once, and prints the
  stable output names and build ID. `--reproducible` overrides the config for
  that invocation without mutating the imported value. `--report` writes a
  deterministic JSON summary of per-stage cache statistics, output manifests,
  and per-file hashes without duplicating artifact bytes. When
  `settings.cacheDir` is configured, unchanged capability-free downstream
  invocations are restored across separate CLI processes from a
  containment-checked filesystem cache beneath the project root.
- `check` performs config-schema, capability, wiring, kind, and output
  validation without invoking a stage or writing output.
- `clean` validates the pipeline, then removes its configured cache directory
  and `outDir` values only from stages that produce `DeployArtifact`. Targets
  must be descendants of the project directory; project-root and outside
  deletion is refused.
- `watch` runs the real pipeline once, serves successful `DeployArtifact`
  outputs from memory on loopback, and coalesces authored project changes into
  rebuilds. Browsers reconnect to the live-reload event stream automatically.
  A failed rebuild reports diagnostics while the server keeps the last good
  output available. Generated output, cache, `node_modules`, and `.git` trees
  are excluded from filesystem notifications. Sources and capability-bearing
  stages remain conservative until FM-B032 adds explicit external-state
  revisions and side-effect replay contracts.

`--config` is a global option and can be written before or after a command.
`build` also accepts the `run` alias. Help and version output are generated
from the same checked-in CLI Builder specification used for parsing, so the
documented command surface cannot drift into a second hand-written parser.

Exit codes are `0` for success, `1` for a completed but unsuccessful pipeline,
`2` for usage/configuration/I/O failures, and `130` for cooperative SIGINT
cancellation. Diagnostics use `forme: CODE: message` on stderr and never rely
on exception stacks for expected failures.

TypeScript and JavaScript configs are executable modules. The CLI treats the
selected project config as trusted code, matching FM03's direct-import host; it
does not claim to sandbox config evaluation. Plugin isolation remains a later
Forme milestone.

## Repository bootstrap helper

The `forme-local-bootstrap` companion is repository plumbing for local
`file:` dependencies. It discovers those dependencies across npm manifest
fields, rejects cycles, sorts siblings by package name, and installs them
leaf-first. Published consumers use normal package-manager installation and do
not need this helper.
