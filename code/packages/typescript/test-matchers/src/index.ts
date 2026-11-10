export {
  AssertionError,
  Expectation,
  AsyncExpectation,
  MatcherResult,
  expect,
} from "./matchers.js";

export {
  canonicalF64,
  valuesEqual,
  encodeValue,
  numberToF64Bits,
  f64BitsToNumber,
  toValue,
  deepEqual,
} from "./deep_equal.js";

export type { Value } from "./types.js";
