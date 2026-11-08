/**
 * @coding-adventures/pbt
 *
 * Standalone TypeScript Level 2 property-based testing engine with generator
 * algebra, integrated shrinking, deterministic replay, closed failure vocabulary,
 * and zero runtime dependencies (VER05).
 */

export {
  SplitMix64,
  fnv1a64,
  subSeed,
  Value,
  Verdict,
  FID_COUNTEREXAMPLE,
  FID_EXHAUSTED,
  FID_TIMEOUT,
  FID_PANIC,
  ALL_PBT_FAILURE_IDS,
  PbtFailureId,
  isPbtFailureId,
  isRegisteredFailureId,
  ExhaustedError,
  genBool,
  genU8,
  genU64,
  genI128,
  genPrimitive,
  boundedU64,
  genBoundedInt,
  genBounded,
  genBytes,
  shrinkInt,
  shrinkList,
  shrinkBytes,
  simulateReplay,
  Gen,
  PbtOptions,
  checkProperty,
  assertProperty,
} from "./pbt.js";
