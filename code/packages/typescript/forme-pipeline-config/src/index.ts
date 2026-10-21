/**
 * @coding-adventures/forme-pipeline-config
 *
 * The user-authored description of *what to build* — `PipelineConfig`
 * types, validator, and TypeScript/TOML loaders.
 *
 * Three responsibilities:
 *
 *   - **Types** — `PipelineConfig`, `PipelineSettings`,
 *     `StageInstanceSpec`, `StageRef`, `EdgeSpec`, `OutputSpec`.
 *
 *   - **Validation** — `validateConfig(config)` checks every FM03 §2.4
 *     rule (unique IDs, capability subset, apiVersion match, …),
 *     collects every violation, and throws a single `ConfigError`
 *     summarising all of them.
 *
 *   - **Loading** — `loadTsConfig(path)` dynamically imports a TS-form
 *     config. `parseTomlConfig(source)` lowers bounded declarative TOML to
 *     exactly the same in-memory shape.
 *
 * See FM03 §2 for the design. Both forms produce the `PipelineConfig`
 * shape declared here and pass through the same validator/orchestrator path.
 */

export type {
  PipelineConfig,
  PipelineSettings,
  PluginRuntimeConfig,
  StageInstanceSpec,
  StageRef,
  EdgeSpec,
  OutputSpec,
} from "./types.js";
export { isStageRef } from "./types.js";

export {
  CONFIG_ERROR_CODES,
  ConfigError,
} from "./errors.js";
export type { ConfigErrorEntry, ConfigErrorCode } from "./errors.js";

export { validateConfig } from "./validate.js";
export type { ResolvedPipelineConfig } from "./validate.js";

export { validateAgainstSchema } from "./json-schema.js";
export type { SchemaViolation, SchemaValidationResult } from "./json-schema.js";

export { loadTsConfig } from "./load.js";
export type { LoadTsConfigOptions } from "./load.js";

export { MAX_TOML_CONFIG_BYTES, parseTomlConfig } from "./parse-toml.js";
