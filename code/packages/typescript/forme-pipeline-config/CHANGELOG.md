# Changelog — @coding-adventures/forme-pipeline-config

## 0.1.0 — 2026-05-15

Initial release. First package of the FM03 orchestrator stack — the
user-authored description of *what to build* (PipelineConfig types,
validator, TS-form loader).

### Added

- `PipelineConfig` interface with `name`, `settings`, `stages`,
  optional `wires`, optional `outputs`.
- `PipelineSettings` — storageRoot, cacheDir, reproducibleBuild,
  maxConcurrency, logLevel, bestEffort, deadlineMs.
- `StageInstanceSpec` with optional `id`, `config`, `capabilities`.
- `StageRef` (`{ kind: "stage-ref", packageName, export? }`) for
  the future FM02 plugin-host flow.
- `isStageRef(value)` predicate.
- `EdgeSpec` and `OutputSpec` for explicit wiring + named outputs.
- `validateConfig(config)` — collects every FM03 §2.4 violation in
  one pass, throws a single `ConfigError` summarising all of them.
  Returns a `ResolvedPipelineConfig` carrying the original config
  plus per-spec resolved instance IDs.
- `ConfigError` with structured `errors[]` array; `name = "ConfigError"`,
  multi-line summary in `message`. Entries are frozen.
- `CONFIG_ERROR_CODES` (frozen) and `ConfigErrorCode` type alias —
  `DUPLICATE_INSTANCE_ID`, `API_VERSION_MISMATCH`,
  `STAGE_REF_UNRESOLVED`, `CAPABILITY_NOT_DECLARED`, `CONFIG_REQUIRED`,
  `MULTIPLE_OUTPUTS_UNNAMED`, `UNKNOWN_INSTANCE_ID`,
  `INVALID_STAGE_VALUE`, `MALFORMED`.
- `loadTsConfig(path, options?)` — dynamically imports a
  `forme.config.ts` and returns its default export. Resolves
  relative paths against the supplied (or current) working
  directory; converts to a `file://` URL for cross-platform
  compatibility. Wraps import errors with the file:// URL in the
  message. Test hook (`importModule` option) for unit tests.

### Spec divergences from FM03 §2

- **JSON-Schema validation** of stage configs is deferred to the
  orchestrator. The validator only enforces the *presence* rule
  (`configSchema !== null` ⇒ `config !== undefined`); structural
  schema validation needs a JSON-Schema validator that doesn't yet
  live in the monorepo.
- **TOML form loader** (FM03 §2.3) is not implemented in v0. The
  TS form is the canonical surface; a TOML loader will compile to
  the same `PipelineConfig` shape in a sibling package.

### Notes

- Multiple-terminal detection uses a coarse heuristic: any stage
  whose `produces.name` is in `{DeployArtifact, RequestHandler,
  Feed, SearchIndex}` counts as terminal. The orchestrator's full
  DAG construction (FM03 §3.3) does the precise determination later;
  this catches the common "I forgot to name my outputs" mistake at
  config time.
