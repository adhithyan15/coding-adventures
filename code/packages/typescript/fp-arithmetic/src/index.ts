/**
 * FP Arithmetic -- IEEE 754 floating-point arithmetic from logic gates.
 *
 * This package is the shared foundation for both the CPU stack (FPU) and all
 * three accelerator stacks (GPU/TPU/NPU). Every floating-point operation in
 * the computing stack ultimately passes through this layer.
 *
 * Built entirely from logic gates (AND, OR, XOR, NOT) and ripple-carry adders.
 */

// Formats
export { type FloatFormat, type FloatBits, FP32, FP16, BF16 } from "./formats.js";

// Encoding/decoding
export {
  floatToBits,
  bitsToFloat,
  intToBitsMsb,
  bitsMsbToInt,
  bitLength,
} from "./ieee754.js";

// Special value detection
export { isNaN, isInf, isZero, isDenormalized, allOnes, allZeros } from "./ieee754.js";

// Arithmetic -- addition, subtraction, comparison
export { fpAdd, fpSub, fpNeg, fpAbs, fpCompare } from "./fp-adder.js";

// Arithmetic helpers (exported for testing)
export {
  shiftRight,
  shiftLeft,
  findLeadingOne,
  subtractUnsigned,
  addBitsMsb,
} from "./fp-adder.js";

// Multiplication
export { fpMul } from "./fp-multiplier.js";

// Fused multiply-add and conversion
export { fpFma, fpConvert } from "./fma.js";

// Pipelined operations
export {
  PipelinedFPAdder,
  PipelinedFPMultiplier,
  PipelinedFMA,
  FPUnit,
} from "./pipeline.js";
