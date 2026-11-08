/**
 * @coding-adventures/fuzz
 *
 * Standalone TypeScript Level 2 coverage-guided fuzz testing engine with
 * deterministic mutator algebra, 65,536-bit bitmap coverage tracking, crash
 * deduplication, and zero runtime dependencies (VER06).
 */

export {
  SplitMix64,
  fnv1a64,
  FID_FUZZ_CRASH,
  FID_FUZZ_TIMEOUT,
  FID_FUZZ_OOM,
  FID_FUZZ_LEAK,
  FID_FUZZ_PANIC,
  FUZZ_FAILURE_IDS,
  isFuzzFailureId,
  bitFlip,
  byteFlip,
  byteArithmetic,
  boundaryInsert,
  blockDelete,
  blockInsert,
  blockDuplicate,
  splice,
  mutateSequence,
  corpusAdmit,
  COVERAGE_MAP_SIZE,
  COVERAGE_BYTES,
  CoverageBitmap,
  crashSignature,
  isDuplicateCrash,
  formatTapFailure,
  calculateEnergy,
  FuzzOutcome,
  FuzzOptions,
  CrashRecord,
  FuzzReport,
  CorpusSeed,
  minimizeCrashingInput,
  fuzzTarget,
} from "./fuzz.js";
