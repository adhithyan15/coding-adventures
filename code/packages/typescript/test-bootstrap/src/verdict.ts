import { Value, Verdict, REGISTERED_FAILURE_IDS } from "./types.js";
import { valuesEqual, encodeValue } from "./deep_equal.js";

export { Verdict };

/**
 * Checks if a failure ID belongs to the closed TF01 vocabulary (§5.3).
 */
export function isRegisteredFailureId(id: string): boolean {
  return REGISTERED_FAILURE_IDS.has(id);
}

/**
 * Constructs a Pass verdict.
 */
export function pass(): Verdict {
  return { status: "pass" };
}

/**
 * Constructs a Fail verdict with a failure ID and detail message.
 */
export function fail(failureId: string, message: string): Verdict {
  return { status: "fail", failureId, message };
}

/**
 * Evaluates deep equality between two Values (TF01 §3.1).
 */
export function deepEqual(actual: Value, expected: Value): Verdict {
  if (valuesEqual(actual, expected)) {
    return pass();
  }
  return fail(
    "not-deep-equal",
    `expected ${encodeValue(expected)}, got ${encodeValue(actual)}`
  );
}

/**
 * Evaluates whether an expected error was raised (TF01 §3.2).
 */
export function errorIs(raised: string | null, expectedId: string): Verdict {
  if (raised === null) {
    return fail("no-error", `expected error ${JSON.stringify(expectedId)}, none raised`);
  }
  if (raised === expectedId) {
    return pass();
  }
  return fail("wrong-error", `expected error ${JSON.stringify(expectedId)}, got ${JSON.stringify(raised)}`);
}
