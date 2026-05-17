/**
 * @coding-adventures/forme-style-orchestrator
 *
 * One-call orchestration over the FM04 Style IR family.  Wraps
 * **validate → compose theme → dispatch to translator** in a single
 * `compile(doc, target, options)` entry point.
 *
 * ```ts
 * import { compile, isCompileError } from "@coding-adventures/forme-style-orchestrator";
 * import { createThemeRegistry } from "@coding-adventures/forme-style-theme";
 *
 * const themes = createThemeRegistry();
 * themes.register({ name: "dark", tokens: { colors: { text: { kind: "rgb", r: 240, g: 240, b: 240 } } } });
 *
 * const result = compile(doc, "css", {
 *   activeContexts: ["screen"],
 *   theme: "dark",
 *   themeRegistry: themes,
 * });
 *
 * if (isCompileError(result)) {
 *   console.error("validation failed:", result.errors);
 * } else {
 *   console.log(result.output);    // CSS text
 *   console.log(result.warnings);  // any translator warnings
 * }
 * ```
 *
 * Per FM04 §13 (composition concerns) and FM03 (orchestrator
 * concerns).  No new spec ground broken — pure integration glue
 * that exists so users don't have to re-wire the four sibling
 * packages by hand on every call.
 *
 * @module index
 */

export {
  compile, isCompileError, isCompileSuccess, fingerprintDocument,
} from "./orchestrator.js";
export type {
  CompileOptions, CompileResult, CompileTarget,
} from "./orchestrator.js";
