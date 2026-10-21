# @coding-adventures/forme-pipeline-config

The user-authored description of *what to build* — `PipelineConfig` types,
validator, and TypeScript/JavaScript and declarative TOML config loaders. First
package of the Forme orchestrator stack (FM03 §2).

## What's exported

| Group       | Exports                                                                                         |
| ----------- | ----------------------------------------------------------------------------------------------- |
| Types       | `PipelineConfig`, `PipelineSettings`, `PluginRuntimeConfig`, `StageInstanceSpec`, `StageRef`, `EdgeSpec`, `OutputSpec` |
| Predicates  | `isStageRef(value)`                                                                             |
| Validation  | `validateConfig(config)` → `ResolvedPipelineConfig` (or throws `ConfigError`)                    |
| Errors      | `ConfigError`, `ConfigErrorEntry`, `ConfigErrorCode`, `CONFIG_ERROR_CODES`                       |
| Loading     | `loadTsConfig(path, options?)`, `parseTomlConfig(source)`, `MAX_TOML_CONFIG_BYTES`                 |

## Validation rules (FM03 §2.4)

`validateConfig` enforces every spec rule in one pass and collects every violation before throwing:

1. **`STAGE_REF_UNRESOLVED`** — `StageRef` requires the FM02 plugin host. The product CLI supplies that host for installed plugins.
2. **`INVALID_STAGE_VALUE`** — stage missing required identification fields.
3. **`API_VERSION_MISMATCH`** — stage targets a different `KERNEL_API_VERSION`.
4. **`DUPLICATE_INSTANCE_ID`** — multiple instances resolve to the same id (auto-derived from `stage.name`, or explicit).
5. **`CAPABILITY_NOT_DECLARED`** — per-instance grants must be a subset of the stage's declared capabilities (FM01 §5.5).
6. **`CONFIG_REQUIRED`** — stage with non-null `configSchema` was given no config.
7. **`UNKNOWN_INSTANCE_ID`** — `EdgeSpec` or `OutputSpec` references a non-existent instance.
8. **`MULTIPLE_INPUT_WIRES`** — more than one wire targets the same default or named input port.
9. **`UNKNOWN_INPUT_PORT`** — `to.port` does not name a declared `stage.inputPorts` entry.
10. **`MISSING_INPUT_PORT_WIRE`** — a required named input has no explicit producer.
11. **`OUTPUT_PORT_UNSUPPORTED`** — `from.port` is set; stages currently expose one output.
12. **`MULTIPLE_OUTPUTS_UNNAMED`** — pipeline has 2+ terminal stages but doesn't name them in `outputs`.
13. **`MALFORMED`** — top-level fields or settings have wrong types.

`settings.pluginRuntimes` optionally maps `deno`, `bun`, or `python` to an
absolute `{ executable, root }` pair. Both paths are validated before the
native sandbox can resolve a referenced plugin stage.

The validator collects ALL violations rather than stopping at the first. `ConfigError.errors` carries the full list; `ConfigError.message` is a multi-line summary.

## Declarative TOML

`parseTomlConfig(source)` lowers FM03's closed TOML vocabulary to the same
`PipelineConfig` value consumed by `validateConfig`. Package stages become
`StageRef`s, so they follow the normal installed-plugin resolution and sandbox
path. Kebab-case settings, plugin runtimes, stages, wires, outputs, named ports,
capabilities, and open per-stage config tables are supported.

The parser rejects duplicate/conflicting paths, unknown schema keys, unsafe key
segments, non-finite or unsafe numeric values, and malformed field types. It
uses null-prototype intermediate tables and explicit source, token, assignment,
nesting, collection, and string limits. TOML config is data; unlike a
TypeScript/JavaScript config, parsing it executes no project code.

## Quick reference

```typescript
import { readFile } from "node:fs/promises";
import {
  loadTsConfig,
  parseTomlConfig,
  validateConfig,
  ConfigError,
} from "@coding-adventures/forme-pipeline-config";

try {
  const config = process.argv[2]?.endsWith(".toml")
    ? parseTomlConfig(await readFile(process.argv[2], "utf8"))
    : await loadTsConfig("./forme.config.ts");
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
