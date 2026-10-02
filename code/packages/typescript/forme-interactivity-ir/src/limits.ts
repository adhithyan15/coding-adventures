import { InteractivityError } from "./error.js";
import { appendPath, rejectProxy } from "./safe.js";
import type { InteractivityLimits } from "./types.js";

export const DEFAULT_INTERACTIVITY_LIMITS: Readonly<InteractivityLimits> = Object.freeze({
  maxCanonicalBytes: 1_048_576,
  maxState: 256,
  maxBindings: 1_024,
  maxHandlers: 1_024,
  maxIslands: 256,
  maxEffectsPerHandler: 64,
  maxPredicateOperands: 64,
  maxExpressionDepth: 32,
  maxExpressionNodes: 8_192,
  maxJsonDepth: 32,
  maxJsonNodes: 16_384,
  maxStringBytes: 4_096,
});

export const HARD_INTERACTIVITY_LIMITS: Readonly<InteractivityLimits> = Object.freeze({
  maxCanonicalBytes: 16_777_216,
  maxState: 4_096,
  maxBindings: 16_384,
  maxHandlers: 16_384,
  maxIslands: 4_096,
  maxEffectsPerHandler: 1_024,
  maxPredicateOperands: 1_024,
  maxExpressionDepth: 128,
  maxExpressionNodes: 131_072,
  maxJsonDepth: 128,
  maxJsonNodes: 262_144,
  maxStringBytes: 65_536,
});

const LIMIT_KEYS = Object.freeze(Object.keys(DEFAULT_INTERACTIVITY_LIMITS) as Array<keyof InteractivityLimits>);

/** Validate limits without invoking caller-controlled getters. */
export function snapshotLimits(input?: Partial<InteractivityLimits>): Readonly<InteractivityLimits> {
  if (input === undefined) return DEFAULT_INTERACTIVITY_LIMITS;
  if (input === null || typeof input !== "object") {
    throw new InteractivityError("INVALID_TYPE", "$.limits", "limits must be a plain object");
  }
  rejectProxy(input, "$.limits");
  if (Array.isArray(input)) {
    throw new InteractivityError("INVALID_TYPE", "$.limits", "limits must be a plain object");
  }
  const proto = Object.getPrototypeOf(input);
  if (proto !== Object.prototype && proto !== null) {
    throw new InteractivityError("INVALID_TYPE", "$.limits", "limits must be a plain object");
  }
  const ownKeys = Reflect.ownKeys(input);
  if (ownKeys.length > LIMIT_KEYS.length) {
    throw new InteractivityError("INVALID_LIMIT", "$.limits", "too many limit properties");
  }
  if (ownKeys.some(key => typeof key === "symbol")) {
    throw new InteractivityError("SYMBOL_KEY", "$.limits", "symbol limit keys are not allowed");
  }
  const descriptors = new Map<string, PropertyDescriptor>();
  for (const key of ownKeys as string[]) {
    if (!LIMIT_KEYS.includes(key as keyof InteractivityLimits)) {
      throw new InteractivityError("UNKNOWN_FIELD", appendPath("$.limits", key), "unknown limit");
    }
    const descriptor = Object.getOwnPropertyDescriptor(input, key);
    if (descriptor === undefined) throw new Error("internal error: own key has no descriptor");
    descriptors.set(key, descriptor);
    if (!("value" in descriptor)) {
      throw new InteractivityError("ACCESSOR", appendPath("$.limits", key), "limit accessors are not allowed");
    }
    if (!descriptor.enumerable) {
      throw new InteractivityError("INVALID_LIMIT", appendPath("$.limits", key), "limit must be enumerable");
    }
  }

  const snapshot = {} as Record<keyof InteractivityLimits, number>;
  for (const key of LIMIT_KEYS) {
    const descriptor = descriptors.get(key);
    const value = descriptor === undefined
      ? DEFAULT_INTERACTIVITY_LIMITS[key]
      : descriptor.value;
    if (typeof value !== "number" || !Number.isSafeInteger(value) || value <= 0 || value > HARD_INTERACTIVITY_LIMITS[key]) {
      throw new InteractivityError(
        "INVALID_LIMIT",
        `$.limits.${key}`,
        `limit must be a positive safe integer no greater than ${HARD_INTERACTIVITY_LIMITS[key]}`,
      );
    }
    snapshot[key] = value;
  }
  return Object.freeze(snapshot);
}
