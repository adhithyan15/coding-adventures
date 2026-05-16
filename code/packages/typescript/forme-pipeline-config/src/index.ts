/**
 * @coding-adventures/forme-pipeline-config
 *
 * The user-authored description of *what to build* — `PipelineConfig`
 * types, validator, and TS-form loader.
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
 *     `forme.config.ts` and returns its default export, ready to be
 *     fed to `validateConfig`.
 *
 * See FM03 §2 for the design.  The TOML-form loader (FM03 §2.3) lives
 * in a sibling package (not yet built); both forms produce the same
 * `PipelineConfig` shape declared here.
 */

export type {
  PipelineConfig,
  PipelineSettings,
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
