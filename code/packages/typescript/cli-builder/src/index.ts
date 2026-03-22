/**
 * @coding-adventures/cli-builder
 *
 * A declarative CLI argument parsing library driven by directed graphs
 * and state machines. Write a JSON spec file; get a fully parsed,
 * validated, and typed result back.
 *
 * === Quick Start ===
 *
 * ```typescript
 * import { Parser } from "@coding-adventures/cli-builder";
 *
 * // Using a spec file:
 * const parser = new Parser("./my-tool-spec.json", process.argv);
 * const result = parser.parse();
 *
 * if ("text" in result) {
 *   // HelpResult
 *   process.stdout.write(result.text + "\n");
 *   process.exit(0);
 * } else if ("version" in result && !("flags" in result)) {
 *   // VersionResult
 *   process.stdout.write(result.version + "\n");
 *   process.exit(0);
 * } else {
 *   // ParseResult
 *   console.log(result.flags, result.arguments);
 * }
 * ```
 *
 * === Architecture ===
 *
 * The library is built from five components:
 *
 * 1. **SpecLoader** — reads and validates the JSON spec file, building
 *    the internal `CliSpec` type. Uses `DirectedGraph.hasCycle()` to
 *    detect circular `requires` dependencies.
 *
 * 2. **TokenClassifier** — classifies each argv token into a typed event
 *    (LONG_FLAG, SHORT_FLAG, STACKED_FLAGS, POSITIONAL, etc.) using
 *    longest-match-first disambiguation.
 *
 * 3. **Parser** — drives the three-phase algorithm: routing (directed
 *    graph traversal), scanning (modal state machine), and validation.
 *
 * 4. **PositionalResolver** — assigns positional tokens to named argument
 *    slots using the "last-wins" partitioning algorithm.
 *
 * 5. **FlagValidator** — checks flag constraints: conflicts_with, requires
 *    (transitive via G_flag), required flags, and exclusive groups.
 *
 * @module cli-builder
 */

export { Parser } from "./parser.js";
export { SpecLoader } from "./spec-loader.js";
export { TokenClassifier } from "./token-classifier.js";
export { PositionalResolver, coerceValue } from "./positional-resolver.js";
export { FlagValidator } from "./flag-validator.js";
export { HelpGenerator } from "./help-generator.js";
export { CliBuilderError, SpecError, ParseErrors } from "./errors.js";
export type { ParseError } from "./errors.js";
export type {
  CliSpec,
  CommandDef,
  FlagDef,
  ArgDef,
  ExclusiveGroup,
  ParseResult,
  HelpResult,
  VersionResult,
  ParserResult,
  ValueType,
  ParsingMode,
} from "./types.js";
export type {
  TokenEvent,
  EndOfFlagsToken,
  LongFlagToken,
  LongFlagWithValueToken,
  SingleDashLongToken,
  ShortFlagToken,
  ShortFlagWithValueToken,
  StackedFlagsToken,
  PositionalToken,
  UnknownFlagToken,
} from "./token-classifier.js";
