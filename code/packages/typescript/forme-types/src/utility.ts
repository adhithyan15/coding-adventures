/**
 * Utility types — the universal JSON value type and read-only record helper
 * that the rest of the kernel builds on.
 *
 * These are type aliases only.  Nothing here has runtime presence — the
 * compiler erases them.  We export them from a dedicated module so every
 * package in the Forme kernel can import them without dragging in the
 * entire kind taxonomy.
 *
 * === JsonValue ===
 *
 * JsonValue is the canonical "any value safe to serialise as JSON" type.
 * It is intentionally `readonly` end-to-end: once a JsonValue is produced
 * it is treated as immutable.  Stages that need mutable scratch data work
 * with their own local types and only convert to JsonValue at the boundary.
 *
 * Shape:
 *
 *   JsonValue = null
 *             | boolean
 *             | number
 *             | string
 *             | readonly JsonValue[]
 *             | { readonly [key: string]: JsonValue }
 *
 * Note that `undefined` is *not* a valid JsonValue — JSON has no
 * representation for it, and treating it as null surprises callers.
 * If a field is optional, the schema says so and the field is absent
 * from the object rather than present-but-undefined.
 *
 * === ReadonlyRecord ===
 *
 * `ReadonlyRecord<K, V>` is a thin alias for `{ readonly [k in K]: V }`.
 * The standard library's `Record<K, V>` produces a *mutable* mapped type;
 * Forme's contracts are immutable, so we use this alias everywhere.
 */

/**
 * A value that can be losslessly serialised to JSON and round-tripped
 * back.  Used for stage configuration, frontmatter, telemetry payloads,
 * and the canonical-JSON revision-hashing input.
 */
export type JsonValue =
  | null
  | boolean
  | number
  | string
  | readonly JsonValue[]
  | { readonly [key: string]: JsonValue };

/**
 * Read-only key-value mapping.  Equivalent to `Readonly<Record<K, V>>`
 * but spelled out so consumers can see the intent without reading the
 * standard library's mapped-type definition.
 */
export type ReadonlyRecord<K extends string, V> = {
  readonly [key in K]: V;
};
