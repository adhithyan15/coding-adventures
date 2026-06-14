# @coding-adventures/forme-pipeline-config

The user-authored description of *what to build* — `PipelineConfig` types, validator, and TS-form config loader. First package of the Forme orchestrator stack (FM03 §2).

## What's exported

| Group       | Exports                                                                                         |
| ----------- | ----------------------------------------------------------------------------------------------- |
| Types       | `PipelineConfig`, `PipelineSettings`, `StageInstanceSpec`, `StageRef`, `EdgeSpec`, `OutputSpec` |
| Predicates  | `isStageRef(value)`                                                                             |
| Validation  | `validateConfig(config)` → `ResolvedPipelineConfig` (or throws `ConfigError`)                    |
| Errors      | `ConfigError`, `ConfigErrorEntry`, `ConfigErrorCode`, `CONFIG_ERROR_CODES`                       |
| Loading     | `loadTsConfig(path, options?)` → `Promise<PipelineConfig>`                                       |

## Validation rules (FM03 §2.4)

`validateConfig` enforces every spec rule in one pass and collects every violation before throwing:

1. **`STAGE_REF_UNRESOLVED`** — `StageRef` requires the FM02 plugin host. v0 supports direct-import flows only.
2. **`INVALID_STAGE_VALUE`** — stage missing required identification fields.
3. **`API_VERSION_MISMATCH`** — stage targets a different `KERNEL_API_VERSION`.
4. **`DUPLICATE_INSTANCE_ID`** — multiple instances resolve to the same id (auto-derived from `stage.name`, or explicit).
5. **`CAPABILITY_NOT_DECLARED`** — per-instance grants must be a subset of the stage's declared capabilities (FM01 §5.5).
6. **`CONFIG_REQUIRED`** — stage with non-null `configSchema` was given no config.
7. **`UNKNOWN_INSTANCE_ID`** — `EdgeSpec` or `OutputSpec` references a non-existent instance.
8. **`MULTIPLE_OUTPUTS_UNNAMED`** — pipeline has 2+ terminal stages but doesn't name them in `outputs`.
9. **`MALFORMED`** — top-level fields or settings have wrong types.

The validator collects ALL violations rather than stopping at the first. `ConfigError.errors` carries the full list; `ConfigError.message` is a multi-line summary.

## Quick reference

```typescript
import {
  loadTsConfig,
  validateConfig,
  ConfigError,
} from "@coding-adventures/forme-pipeline-config";

try {
  const config = await loadTsConfig("./forme.config.ts");
  const resolved = validateConfig(config);
  // resolved.config + resolved.resolvedIds — ready for the orchestrator
} catch (e) {
  if (e instanceof ConfigError) {
    for (const { path, code, message } of e.errors) {
      console.error(`  ${path}: ${message} [${code}]`);
    }
  } else {
    throw e;
  }
}
```

## Coverage

```bash
npm install
npx vitest run --coverage
```

Targets 100% line + branch.
