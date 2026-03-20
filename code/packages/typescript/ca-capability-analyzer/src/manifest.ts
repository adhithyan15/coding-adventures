/**
 * # Capability Manifest — Declaration vs. Reality
 *
 * ## The Core Problem
 *
 * A capability manifest is a JSON file (`required_capabilities.json`) where a
 * project declares what OS capabilities it needs. The manifest says "I need
 * filesystem read access and network connections." The analyzer then checks
 * the actual code to see if reality matches the declaration.
 *
 * There are two kinds of problems we can find:
 *
 * 1. **Undeclared capabilities**: The code uses something not in the manifest.
 *    This is a security concern — the code is doing more than it says it does.
 *
 * 2. **Unused declarations**: The manifest declares a capability the code never
 *    uses. This isn't a security risk, but it's sloppy — like requesting
 *    permissions you don't need.
 *
 * ## Manifest Format
 *
 * The manifest file is a JSON object with this structure:
 *
 * ```json
 * {
 *   "name": "my-package",
 *   "version": "1.0.0",
 *   "capabilities": [
 *     { "category": "fs", "action": "read", "target": "/data/*" },
 *     { "category": "net", "action": "connect", "target": "*" }
 *   ]
 * }
 * ```
 *
 * ## Glob Matching
 *
 * Targets in the manifest support simple glob patterns:
 * - `*` matches anything
 * - `*.txt` matches files ending in .txt
 * - `/data/*` matches anything under /data/
 *
 * We implement a simple glob matcher rather than pulling in a dependency.
 */

import { DetectedCapability } from "./analyzer.js";

// ============================================================================
// Types
// ============================================================================

/**
 * A single capability declaration in the manifest.
 * This represents what the project *says* it needs.
 */
export interface DeclaredCapability {
  /** The broad category: fs, net, proc, env, ffi */
  category: string;
  /** The specific action: read, write, exec, connect, * for any */
  action: string;
  /** The target pattern — may include globs like /data/* */
  target: string;
}

/**
 * The complete manifest file structure.
 */
export interface CapabilityManifest {
  /** Package name */
  name: string;
  /** Package version */
  version: string;
  /** List of declared capabilities */
  capabilities: DeclaredCapability[];
}

/**
 * The result of comparing detected capabilities against the manifest.
 */
export interface ComparisonResult {
  /** Detected capabilities not covered by any manifest entry */
  undeclared: DetectedCapability[];
  /** Manifest entries with no matching detected capability */
  unused: DeclaredCapability[];
  /** Detected capabilities that match a manifest entry */
  matched: DetectedCapability[];
}

// ============================================================================
// Manifest Loading
// ============================================================================

/**
 * ## parseManifest — Parse a manifest from JSON
 *
 * Takes the raw JSON string of a `required_capabilities.json` file and
 * parses it into a structured CapabilityManifest.
 *
 * Performs basic validation:
 * - Must be a valid JSON object
 * - Must have a `capabilities` array
 * - Each capability must have category, action, and target strings
 *
 * @param json - The raw JSON string
 * @returns A parsed CapabilityManifest
 * @throws Error if the JSON is invalid or doesn't match the expected format
 *
 * @example
 * ```ts
 * const manifest = parseManifest(`{
 *   "name": "my-app",
 *   "version": "1.0.0",
 *   "capabilities": [
 *     { "category": "fs", "action": "read", "target": "*" }
 *   ]
 * }`);
 * ```
 */
export function parseManifest(json: string): CapabilityManifest {
  let parsed: unknown;
  try {
    parsed = JSON.parse(json);
  } catch {
    throw new Error("Invalid JSON in capability manifest");
  }

  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    throw new Error("Capability manifest must be a JSON object");
  }

  const obj = parsed as Record<string, unknown>;

  if (!Array.isArray(obj.capabilities)) {
    throw new Error("Capability manifest must have a 'capabilities' array");
  }

  const capabilities: DeclaredCapability[] = [];
  for (let i = 0; i < obj.capabilities.length; i++) {
    const cap = obj.capabilities[i];
    if (typeof cap !== "object" || cap === null) {
      throw new Error(`capabilities[${i}] must be an object`);
    }

    const c = cap as Record<string, unknown>;
    if (typeof c.category !== "string") {
      throw new Error(`capabilities[${i}].category must be a string`);
    }
    if (typeof c.action !== "string") {
      throw new Error(`capabilities[${i}].action must be a string`);
    }
    if (typeof c.target !== "string") {
      throw new Error(`capabilities[${i}].target must be a string`);
    }

    capabilities.push({
      category: c.category,
      action: c.action,
      target: c.target,
    });
  }

  return {
    name: typeof obj.name === "string" ? obj.name : "",
    version: typeof obj.version === "string" ? obj.version : "",
    capabilities,
  };
}

// ============================================================================
// Glob Matching
// ============================================================================

/**
 * ## simpleGlobMatch — Minimal Glob Matcher
 *
 * We implement glob matching ourselves to avoid pulling in `minimatch` or
 * similar dependencies. This keeps the analyzer zero-dependency (besides
 * the TypeScript compiler API).
 *
 * ### Supported Patterns
 *
 * | Pattern    | Meaning                        | Example Match         |
 * |------------|--------------------------------|----------------------|
 * | `*`        | Matches anything               | (anything)           |
 * | `*.txt`    | Matches suffix                 | "readme.txt"         |
 * | `/data/*`  | Matches prefix                 | "/data/file.csv"     |
 * | `exact`    | Exact string match             | "exact" only         |
 *
 * ### How it works
 *
 * We convert the glob pattern to a regular expression:
 * - `*` becomes `.*` (match anything)
 * - `.` and other regex metacharacters are escaped
 * - The pattern is anchored to match the full string (^ and $)
 *
 * @param pattern - The glob pattern (from the manifest)
 * @param value - The actual value (from detection)
 * @returns true if the value matches the pattern
 *
 * @example
 * ```ts
 * simpleGlobMatch("*", "anything")           // true
 * simpleGlobMatch("/data/*", "/data/x.csv")  // true
 * simpleGlobMatch("*.txt", "readme.md")      // false
 * ```
 */
export function simpleGlobMatch(pattern: string, value: string): boolean {
  // Fast path: wildcard matches everything
  if (pattern === "*") return true;

  // Fast path: exact match
  if (!pattern.includes("*")) return pattern === value;

  // Convert glob to regex:
  // 1. Escape regex metacharacters (except *)
  // 2. Replace * with .*
  // 3. Anchor with ^ and $
  const escaped = pattern
    .replace(/[.+?^${}()|[\]\\]/g, "\\$&")  // Escape regex specials
    .replace(/\*/g, ".*");                    // Convert * to .*

  const regex = new RegExp(`^${escaped}$`);
  return regex.test(value);
}

// ============================================================================
// Capability Comparison
// ============================================================================

/**
 * ## capabilityMatchesDeclaration — Single Capability Check
 *
 * Determines whether a single detected capability is covered by a single
 * manifest declaration.
 *
 * ### Matching Rules
 *
 * For a detected capability to match a declaration:
 * 1. **Category must match exactly** — fs != net
 * 2. **Action must match** — either exactly, or the declaration uses "*"
 * 3. **Target must match** — either exactly, via glob, or the declaration uses "*"
 *
 * ### Truth Table
 *
 * | Declared         | Detected            | Match? | Why                        |
 * |------------------|---------------------|--------|----------------------------|
 * | fs:read:*        | fs:read:/etc/passwd | Yes    | * target matches anything  |
 * | fs:*:*           | fs:write:/tmp/x     | Yes    | * action and target        |
 * | fs:read:*.txt    | fs:read:data.txt    | Yes    | Glob match on target       |
 * | fs:read:*        | fs:write:/tmp/x     | No     | Action mismatch            |
 * | net:connect:*    | fs:read:/etc/passwd | No     | Category mismatch          |
 */
export function capabilityMatchesDeclaration(
  detected: DetectedCapability,
  declared: DeclaredCapability,
): boolean {
  // Category must match exactly
  if (detected.category !== declared.category) return false;

  // Action must match (declared * matches any action)
  if (declared.action !== "*" && detected.action !== declared.action) return false;

  // Target must match via glob (declared * matches any target)
  if (!simpleGlobMatch(declared.target, detected.target)) return false;

  return true;
}

/**
 * ## compareCapabilities — The Main Comparison Function
 *
 * Compares all detected capabilities against all manifest declarations.
 * Produces three lists:
 *
 * 1. **matched**: Detected capabilities covered by the manifest (good)
 * 2. **undeclared**: Detected capabilities NOT in the manifest (security concern)
 * 3. **unused**: Manifest declarations with no matching detection (sloppy)
 *
 * ### Algorithm
 *
 * For each detected capability, we scan all manifest declarations looking
 * for a match. If at least one declaration matches, the capability is "covered."
 * If none match, it's "undeclared."
 *
 * Then, for each manifest declaration, we check if any detected capability
 * matched it. If none did, it's "unused."
 *
 * This is O(n*m) where n is detected capabilities and m is declarations.
 * For typical projects (tens to hundreds of capabilities, single-digit
 * declarations), this is perfectly fast.
 *
 * @param detected - All capabilities found by the analyzer
 * @param manifest - The parsed capability manifest
 * @returns A ComparisonResult with matched, undeclared, and unused lists
 *
 * @example
 * ```ts
 * const result = compareCapabilities(
 *   [{ category: "fs", action: "read", target: "/etc/passwd", ... }],
 *   { capabilities: [{ category: "fs", action: "read", target: "*" }] }
 * );
 * // result.matched.length === 1, result.undeclared.length === 0
 * ```
 */
export function compareCapabilities(
  detected: DetectedCapability[],
  manifest: CapabilityManifest,
): ComparisonResult {
  const matched: DetectedCapability[] = [];
  const undeclared: DetectedCapability[] = [];

  /**
   * Track which manifest declarations were matched.
   * We use an array of booleans parallel to manifest.capabilities.
   */
  const declarationUsed = new Array(manifest.capabilities.length).fill(false) as boolean[];

  for (const cap of detected) {
    let isMatched = false;

    for (let i = 0; i < manifest.capabilities.length; i++) {
      if (capabilityMatchesDeclaration(cap, manifest.capabilities[i])) {
        isMatched = true;
        declarationUsed[i] = true;
        // Don't break — a capability could match multiple declarations,
        // and we want to mark all matching declarations as "used"
      }
    }

    if (isMatched) {
      matched.push(cap);
    } else {
      undeclared.push(cap);
    }
  }

  // Find unused declarations
  const unused: DeclaredCapability[] = manifest.capabilities
    .filter((_, i) => !declarationUsed[i]);

  return { matched, undeclared, unused };
}
