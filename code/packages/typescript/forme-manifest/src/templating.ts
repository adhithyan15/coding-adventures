/**
 * templating.ts — resolve `$variable` references in capability
 * `detail` strings (FM02 §3.4).
 *
 * Three variables are recognised:
 *
 *   - `$storageRoot` — absolute path of the pipeline's
 *     `settings.storageRoot`.
 *   - `$cacheDir`    — absolute path of `settings.cacheDir`, or
 *                      `null` if unset (in which case any reference
 *                      throws — the plugin shouldn't have requested
 *                      a cache-dir grant for a pipeline without one).
 *   - `$pluginDir`   — absolute path of the plugin's installation
 *                      directory.
 *
 * `$$` is the literal-dollar escape (per FM02 §3.4).  Any other
 * `$identifier` is rejected with `TEMPLATE_UNKNOWN_VARIABLE` —
 * silent passthrough would defeat the whole point of validating
 * templates at install time.
 *
 * The templating engine is deliberately tiny: no expressions, no
 * conditionals, no `$var.field`, no `${var}` syntax.  It exists so
 * a plugin's manifest can declare `"filesystem:read:$storageRoot"`
 * without knowing the user's project layout.  Nothing more.
 *
 * @module templating
 */

import { ManifestError } from "./errors.js";

/** The runtime environment supplying values for `$variable` references. */
export interface TemplateEnv {
  /** Absolute path to the pipeline's storage root.  Required. */
  readonly storageRoot: string;
  /** Absolute path to the cache dir, or null if not configured. */
  readonly cacheDir: string | null;
  /** Absolute path to the plugin's installation directory. */
  readonly pluginDir: string;
}

/** Names of the recognised variables. */
export const RECOGNISED_VARIABLES = Object.freeze([
  "storageRoot",
  "cacheDir",
  "pluginDir",
] as const);

/**
 * Substitute `$variable` references in a string against the provided
 * environment.  Returns the resolved string.  Throws `ManifestError`
 * on unknown variables, malformed templates, or attempts to
 * dereference a null `$cacheDir`.
 *
 * Examples:
 *   resolveCapabilityTemplate("filesystem:read:$storageRoot",
 *     { storageRoot: "/abs", cacheDir: null, pluginDir: "/p" })
 *   // → "filesystem:read:/abs"
 *
 *   resolveCapabilityTemplate("filesystem:read:$$literal", env)
 *   // → "filesystem:read:$literal"
 */
export function resolveCapabilityTemplate(
  template: string,
  env: TemplateEnv,
): string {
  if (typeof template !== "string") {
    throw new ManifestError({
      code: "TEMPLATE_MALFORMED",
      message: "resolveCapabilityTemplate: template must be a string",
    });
  }

  let out = "";
  let i = 0;
  while (i < template.length) {
    const ch = template[i]!;
    if (ch !== "$") {
      out += ch;
      i++;
      continue;
    }
    // `$$` is the literal-dollar escape.
    if (template[i + 1] === "$") {
      out += "$";
      i += 2;
      continue;
    }
    // `$<identifier>` — scan until a non-identifier char.
    let end = i + 1;
    while (end < template.length && isIdentChar(template[end]!)) end++;
    if (end === i + 1) {
      throw new ManifestError({
        code: "TEMPLATE_MALFORMED",
        message: `bare "$" at position ${i}; use "$$" to emit a literal dollar`,
      });
    }
    const name = template.slice(i + 1, end);
    if (!(RECOGNISED_VARIABLES as readonly string[]).includes(name)) {
      throw new ManifestError({
        code: "TEMPLATE_UNKNOWN_VARIABLE",
        message: `unrecognised template variable "$${name}"; ` +
                 `recognised: $${RECOGNISED_VARIABLES.join(", $")}`,
      });
    }
    out += lookup(name as (typeof RECOGNISED_VARIABLES)[number], env);
    i = end;
  }
  return out;
}

/**
 * Does the string contain any `$variable` reference (excluding the
 * `$$` escape)?  Useful for cheap "does this need resolving?" checks
 * before paying the cost of full templating.
 */
export function hasTemplate(template: string): boolean {
  for (let i = 0; i < template.length; i++) {
    if (template[i] === "$") {
      if (template[i + 1] === "$") {
        i++;
        continue;
      }
      // Bare `$<ident>` — at least one identifier char following.
      if (i + 1 < template.length && isIdentChar(template[i + 1]!)) return true;
    }
  }
  return false;
}

// ─── Helpers ────────────────────────────────────────────────────────

function isIdentChar(ch: string): boolean {
  return (ch >= "a" && ch <= "z") ||
         (ch >= "A" && ch <= "Z") ||
         (ch >= "0" && ch <= "9") ||
         ch === "_";
}

function lookup(
  name: (typeof RECOGNISED_VARIABLES)[number],
  env: TemplateEnv,
): string {
  switch (name) {
    case "storageRoot":
      if (!env.storageRoot) {
        throw new ManifestError({
          code: "TEMPLATE_UNKNOWN_VARIABLE",
          message: `template references "$storageRoot" but env.storageRoot is empty`,
        });
      }
      return env.storageRoot;
    case "cacheDir":
      if (env.cacheDir === null) {
        throw new ManifestError({
          code: "TEMPLATE_UNKNOWN_VARIABLE",
          message: `template references "$cacheDir" but the pipeline has no cacheDir`,
        });
      }
      return env.cacheDir;
    case "pluginDir":
      if (!env.pluginDir) {
        throw new ManifestError({
          code: "TEMPLATE_UNKNOWN_VARIABLE",
          message: `template references "$pluginDir" but env.pluginDir is empty`,
        });
      }
      return env.pluginDir;
  }
}
