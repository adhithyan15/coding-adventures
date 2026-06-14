/**
 * `loadTsConfig` — load a `forme.config.ts` (or `.js`) module by path
 * and extract its default-exported `PipelineConfig`.
 *
 * The contract is intentionally tiny: hand a path, get back the
 * config object.  No validation, no DAG construction — those happen
 * in `validateConfig` and the orchestrator respectively.
 *
 * === Why dynamic import? ===
 *
 * The TS form lets stages be referenced *by value* (imported and passed
 * directly), so the config has to be evaluated as JavaScript.  Static
 * `import` won't do — the path is data, not a literal.  Dynamic
 * `import()` is the standard Node/browser primitive for this and works
 * with `tsx`, `ts-node`, ESM Node, and bundlers.
 *
 * === Path resolution ===
 *
 * We require an absolute path or `file://` URL.  Relative paths get
 * resolved against the current working directory — but that's a
 * footgun (CWD varies with how the orchestrator was invoked) so we
 * normalise to a `file://` URL up front and surface bad paths as
 * thrown errors with a clear message.
 *
 * === Why not parse the source ===
 *
 * Parsing TypeScript without executing it would let us inspect the
 * config without running side effects, but it would also force us to
 * statically resolve all `import` statements — duplicating what the
 * Node/TS toolchain already does.  Executing the module via dynamic
 * import is the conventional approach for tools like Vite, Astro, and
 * Vitest's own config loading.  The cost is that a config file with
 * top-level side effects will run them; the spec already constrains
 * configs to be effect-free.
 */

import { pathToFileURL } from "node:url";
import { resolve as resolvePath, isAbsolute } from "node:path";
import type { PipelineConfig } from "./types.js";

export interface LoadTsConfigOptions {
  /**
   * Working directory to resolve relative paths against.  Default:
   * `process.cwd()`.  Tests pin this so they don't depend on the
   * caller's CWD.
   */
  readonly cwd?: string;
  /**
   * Override the dynamic-import implementation.  Test hook — production
   * uses the platform's `import()`.
   */
  readonly importModule?: (specifier: string) => Promise<unknown>;
}

/**
 * Load a `forme.config.{ts,js,mjs}` file and return its default-exported
 * `PipelineConfig` value.  Throws a clear error if the path doesn't
 * resolve, the module fails to import, or the default export is missing.
 *
 * The returned config is *not* validated — callers should pass it to
 * `validateConfig` next.
 */
export async function loadTsConfig(
  path: string,
  options: LoadTsConfigOptions = {},
): Promise<PipelineConfig> {
  if (typeof path !== "string" || path.length === 0) {
    throw new Error("loadTsConfig: path must be a non-empty string");
  }

  const cwd = options.cwd ?? process.cwd();
  const absolute = isAbsolute(path) ? path : resolvePath(cwd, path);
  // Convert to file:// URL so the dynamic import works on Windows where
  // bare paths can be ambiguous (drive-letter prefix vs. URL scheme).
  const specifier = pathToFileURL(absolute).href;

  const importer = options.importModule ?? defaultImport;
  let mod: unknown;
  try {
    mod = await importer(specifier);
  } catch (err) {
    throw new Error(
      `loadTsConfig: failed to import ${specifier}: ${stringify(err)}`,
    );
  }

  if (typeof mod !== "object" || mod === null) {
    throw new Error(
      `loadTsConfig: ${specifier} did not produce an object (got ${typeof mod})`,
    );
  }
  const config = (mod as { default?: unknown }).default;
  if (typeof config !== "object" || config === null) {
    throw new Error(
      `loadTsConfig: ${specifier} has no default export of a PipelineConfig object`,
    );
  }
  return config as PipelineConfig;
}

function defaultImport(specifier: string): Promise<unknown> {
  // Wrapped so test hooks can replace just the import call.
  return import(specifier);
}

function stringify(err: unknown): string {
  if (err instanceof Error) return err.message;
  try { return String(err); } catch { return "<unstringifiable>"; }
}
