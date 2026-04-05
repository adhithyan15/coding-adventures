/**
 * # Stats — Barrel Export
 *
 * This file re-exports every function from the stats package, enabling
 * convenient imports like:
 *
 *     import { mean, variance, chiSquared } from "@coding-adventures/stats";
 *
 * For tree-shaking, each function lives in its own file. Bundlers that
 * support tree-shaking (Webpack, Rollup, esbuild) will only include the
 * functions you actually import.
 */

// Descriptive statistics (scalar)
export { mean } from "./mean.js";
export { median } from "./median.js";
export { mode } from "./mode.js";
export { variance } from "./variance.js";
export { standardDeviation } from "./standard_deviation.js";
export { min } from "./min.js";
export { max } from "./max.js";
export { range } from "./range.js";

// Frequency analysis
export { frequencyCount } from "./frequency_count.js";
export { frequencyDistribution } from "./frequency_distribution.js";
export { chiSquared } from "./chi_squared.js";
export { chiSquaredText } from "./chi_squared_text.js";

// Cryptanalysis helpers
export { indexOfCoincidence } from "./index_of_coincidence.js";
export { entropy } from "./entropy.js";

// Constants
export { ENGLISH_FREQUENCIES } from "./english_frequencies.js";
