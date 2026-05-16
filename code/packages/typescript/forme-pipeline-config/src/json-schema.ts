/**
 * json-schema.ts — a tiny JSON Schema (draft-07 subset) validator
 * focused on the shape of stage configuration objects.
 *
 * The point: when a stage declares
 *
 *     configSchema: {
 *       type: "object",
 *       required: ["glob"],
 *       properties: { glob: { type: "string" }, root: { type: "string" } },
 *     }
 *
 * and the user's pipeline supplies `config: { gloob: "**\/*.md" }`
 * (typo), the validator catches this at `buildPipeline()` time with
 * a precise message rather than letting the misconfig blow up
 * inside the stage's `run()` later (or worse, accept it as
 * `glob: undefined` and silently misbehave).
 *
 * ═══ Subset supported ════════════════════════════════════════════
 *
 * Keywords we accept (everything else is silently ignored, per
 * draft-07 forward-compatibility):
 *
 *   - `type` — one of: "string", "number", "integer", "boolean",
 *     "array", "object", "null".  May also be an array of those
 *     for union types.
 *   - `enum` — array of allowed values; checked via deep equality.
 *   - `const` — single allowed value; checked via deep equality.
 *   - `required` — array of property names; applies on type=object.
 *   - `properties` — map of property name → sub-schema; applies on
 *     type=object.  Properties not declared but present are allowed
 *     by default (matches draft-07; flip via `additionalProperties:
 *     false`).
 *   - `additionalProperties` — boolean.  When false on an object,
 *     properties not in `properties` are rejected.
 *   - `items` — sub-schema for array elements.
 *   - `minLength` / `maxLength` — string length bounds.
 *   - `minItems` / `maxItems` — array length bounds.
 *   - `minimum` / `maximum` — numeric inclusive bounds.
 *   - `pattern` — regex string that must match (string types only).
 *   - `oneOf` / `anyOf` / `allOf` — composition keywords; each takes
 *     an array of sub-schemas.
 *
 * What we DON'T support (clearly out of scope for stage configs):
 *
 *   - `$ref` / `$id` / `$schema` — no schema resolution; stages
 *     ship their own schemas inline.
 *   - `format` (`"email"`, `"date-time"`, etc.) — would require a
 *     format registry.  Stages can use `pattern` instead.
 *   - `if` / `then` / `else` — branching is rarely needed for
 *     configs; bigger expressivity at the cost of subtle bugs.
 *   - `propertyNames`, `dependencies`, `not` — uncommon; out of
 *     scope.
 *   - `multipleOf`, `exclusiveMinimum/Maximum`, `uniqueItems` —
 *     uncommon for stage configs.
 *
 * Unknown keywords are silently ignored.  This matches draft-07's
 * forward-compat policy: a schema that declares a future keyword
 * still validates the documented half.
 *
 * @module json-schema
 */

import type { JsonValue } from "@coding-adventures/forme-types";
import type { JsonSchema } from "@coding-adventures/forme-stage";

/** Per-finding violation entry. */
export interface SchemaViolation {
  /** JSON-pointer-ish path INTO the validated value. */
  readonly path: string;
  /** Human-readable explanation. */
  readonly message: string;
}

/** Result of running the validator. */
export interface SchemaValidationResult {
  readonly ok: boolean;
  readonly violations: readonly SchemaViolation[];
}

/**
 * Validate a value against a JSON Schema (draft-07 subset).
 *
 * Never throws on malformed schema input — instead, treats malformed
 * keywords as "no constraint" and skips them.  This is intentional:
 * an upstream typo in a stage's schema shouldn't crash the validator;
 * the result just reports fewer violations than it would have.
 *
 * Returns `{ ok: true, violations: [] }` on success.
 *
 * Returns `{ ok: false, violations: [...] }` listing every constraint
 * the value violated.  All violations are surfaced in one pass.
 */
export function validateAgainstSchema(
  value: JsonValue | undefined,
  schema: JsonSchema,
): SchemaValidationResult {
  const violations: SchemaViolation[] = [];
  walk(value, asObject(schema), "", violations);
  return { ok: violations.length === 0, violations };
}

// ─── Recursive walker ───────────────────────────────────────────────

function walk(
  value: JsonValue | undefined,
  schema: Record<string, JsonValue> | null,
  path: string,
  out: SchemaViolation[],
): void {
  if (schema === null) return;

  // Composition keywords short-circuit individual constraint checks.
  if (Array.isArray(schema["allOf"])) {
    for (const sub of schema["allOf"] as readonly JsonValue[]) {
      walk(value, asObject(sub), path, out);
    }
  }
  if (Array.isArray(schema["anyOf"])) {
    const subs = schema["anyOf"] as readonly JsonValue[];
    let anyOk = false;
    for (const sub of subs) {
      const r = validateAgainstSchema(value, sub as JsonSchema);
      if (r.ok) { anyOk = true; break; }
    }
    if (!anyOk && subs.length > 0) {
      out.push({ path: path || "$",
        message: `value matches none of the ${subs.length} schemas in anyOf` });
    }
  }
  if (Array.isArray(schema["oneOf"])) {
    const subs = schema["oneOf"] as readonly JsonValue[];
    let matchCount = 0;
    for (const sub of subs) {
      const r = validateAgainstSchema(value, sub as JsonSchema);
      if (r.ok) matchCount++;
    }
    if (matchCount !== 1 && subs.length > 0) {
      out.push({ path: path || "$",
        message: `value matches ${matchCount} schemas in oneOf (expected exactly 1)` });
    }
  }

  // Type check first — if the actual type doesn't match the declared
  // type, downstream constraint checks are meaningless.
  if (!checkType(value, schema, path, out)) return;

  if (Array.isArray(schema["enum"])) {
    checkEnum(value, schema["enum"] as readonly JsonValue[], path, out);
  }
  if (schema["const"] !== undefined) {
    if (!deepEqual(value as JsonValue, schema["const"] as JsonValue)) {
      out.push({ path: path || "$",
        message: `value does not equal const ${JSON.stringify(schema["const"])}` });
    }
  }

  // Per-type constraints.
  switch (typeof value) {
    case "string":
      checkString(value, schema, path, out);
      break;
    case "number":
      checkNumber(value, schema, path, out);
      break;
  }
  if (Array.isArray(value)) {
    checkArray(value, schema, path, out);
  } else if (value !== null && typeof value === "object") {
    checkObject(value as Record<string, JsonValue>, schema, path, out);
  }
}

// ─── Type ───────────────────────────────────────────────────────────

function checkType(
  value: JsonValue | undefined,
  schema: Record<string, JsonValue>,
  path: string,
  out: SchemaViolation[],
): boolean {
  const t = schema["type"];
  if (t === undefined) return true;
  const allowed = typeof t === "string" ? [t] : Array.isArray(t) ? t.map(String) : [];
  if (allowed.length === 0) return true;
  const actual = jsonTypeOf(value);
  // "integer" requires actual === "number" AND the value is an integer.
  if (allowed.includes("integer") && actual === "number"
      && Number.isInteger(value as number)) {
    return true;
  }
  if (allowed.includes(actual)) return true;
  out.push({ path: path || "$",
    message: `expected type ${allowed.join(" | ")}, got ${actual}` });
  return false;
}

function jsonTypeOf(value: JsonValue | undefined): string {
  if (value === undefined) return "undefined";
  if (value === null) return "null";
  if (Array.isArray(value)) return "array";
  if (typeof value === "object") return "object";
  return typeof value;  // "string" | "number" | "boolean"
}

// ─── Enum ───────────────────────────────────────────────────────────

function checkEnum(
  value: JsonValue | undefined,
  allowed: readonly JsonValue[],
  path: string,
  out: SchemaViolation[],
): void {
  if (value === undefined) return;
  for (const candidate of allowed) {
    if (deepEqual(value, candidate)) return;
  }
  out.push({ path: path || "$",
    message: `value ${JSON.stringify(value)} is not one of the allowed enum members` });
}

// ─── String ─────────────────────────────────────────────────────────

function checkString(
  value: string,
  schema: Record<string, JsonValue>,
  path: string,
  out: SchemaViolation[],
): void {
  const minLen = schema["minLength"];
  if (typeof minLen === "number" && value.length < minLen) {
    out.push({ path: path || "$",
      message: `string length ${value.length} is less than minLength ${minLen}` });
  }
  const maxLen = schema["maxLength"];
  if (typeof maxLen === "number" && value.length > maxLen) {
    out.push({ path: path || "$",
      message: `string length ${value.length} exceeds maxLength ${maxLen}` });
  }
  const pat = schema["pattern"];
  if (typeof pat === "string") {
    let re: RegExp | null = null;
    try { re = new RegExp(pat); } catch { /* malformed; skip */ }
    if (re && !re.test(value)) {
      out.push({ path: path || "$",
        message: `string does not match pattern ${JSON.stringify(pat)}` });
    }
  }
}

// ─── Number ─────────────────────────────────────────────────────────

function checkNumber(
  value: number,
  schema: Record<string, JsonValue>,
  path: string,
  out: SchemaViolation[],
): void {
  const min = schema["minimum"];
  if (typeof min === "number" && value < min) {
    out.push({ path: path || "$",
      message: `value ${value} is less than minimum ${min}` });
  }
  const max = schema["maximum"];
  if (typeof max === "number" && value > max) {
    out.push({ path: path || "$",
      message: `value ${value} exceeds maximum ${max}` });
  }
}

// ─── Array ──────────────────────────────────────────────────────────

function checkArray(
  value: readonly JsonValue[],
  schema: Record<string, JsonValue>,
  path: string,
  out: SchemaViolation[],
): void {
  const minItems = schema["minItems"];
  if (typeof minItems === "number" && value.length < minItems) {
    out.push({ path: path || "$",
      message: `array has ${value.length} items, fewer than minItems ${minItems}` });
  }
  const maxItems = schema["maxItems"];
  if (typeof maxItems === "number" && value.length > maxItems) {
    out.push({ path: path || "$",
      message: `array has ${value.length} items, more than maxItems ${maxItems}` });
  }
  const items = schema["items"];
  if (items !== undefined && items !== null) {
    const itemSchema = asObject(items as JsonSchema);
    if (itemSchema) {
      for (let i = 0; i < value.length; i++) {
        walk(value[i]!, itemSchema, `${path}[${i}]`, out);
      }
    }
  }
}

// ─── Object ─────────────────────────────────────────────────────────

function checkObject(
  value: Record<string, JsonValue>,
  schema: Record<string, JsonValue>,
  path: string,
  out: SchemaViolation[],
): void {
  const required = schema["required"];
  if (Array.isArray(required)) {
    for (const key of required) {
      if (typeof key === "string" && !(key in value)) {
        out.push({ path: path ? `${path}.${key}` : key,
          message: "required property is missing" });
      }
    }
  }

  const propsRaw = schema["properties"];
  const properties = (propsRaw !== null && typeof propsRaw === "object" && !Array.isArray(propsRaw))
    ? propsRaw as Record<string, JsonValue>
    : null;

  // Validate every declared property against its sub-schema.
  if (properties) {
    for (const [key, subSchema] of Object.entries(properties)) {
      if (key in value) {
        const sub = asObject(subSchema as JsonSchema);
        if (sub) walk(value[key]!, sub, path ? `${path}.${key}` : key, out);
      }
    }
  }

  // Catch unknown properties when additionalProperties === false.
  if (schema["additionalProperties"] === false && properties) {
    for (const key of Object.keys(value)) {
      if (!(key in properties)) {
        out.push({ path: path ? `${path}.${key}` : key,
          message: "additional property is not allowed (additionalProperties: false)" });
      }
    }
  }
}

// ─── Helpers ────────────────────────────────────────────────────────

function asObject(s: JsonSchema | undefined): Record<string, JsonValue> | null {
  if (s === null || s === undefined) return null;
  if (typeof s !== "object") return null;
  if (Array.isArray(s)) return null;
  return s as Record<string, JsonValue>;
}

/**
 * Structural equality for JsonValue, with prototype-pollution
 * defensiveness: we use a denylist on object keys so neither side
 * can accidentally read `__proto__` and have prototype-chain
 * inheritance leak into the comparison.
 */
function deepEqual(a: JsonValue, b: JsonValue): boolean {
  if (a === b) return true;
  if (a === null || b === null) return false;
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
      if (!deepEqual(a[i]!, b[i]!)) return false;
    }
    return true;
  }
  if (typeof a === "object" && typeof b === "object") {
    const ao = a as Record<string, JsonValue>;
    const bo = b as Record<string, JsonValue>;
    const akeys = Object.keys(ao);
    const bkeys = Object.keys(bo);
    if (akeys.length !== bkeys.length) return false;
    for (const k of akeys) {
      // Defence: ignore prototype keys outright (matches FM02 §3 defence pattern).
      if (k === "__proto__" || k === "constructor" || k === "prototype") continue;
      if (!Object.prototype.hasOwnProperty.call(bo, k)) return false;
      if (!deepEqual(ao[k]!, bo[k]!)) return false;
    }
    return true;
  }
  return false;
}
