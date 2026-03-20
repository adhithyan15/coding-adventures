/**
 * # Capability Analyzer — Public API
 *
 * This is the entry point for the `@coding-adventures/ca-capability-analyzer` package.
 * It re-exports everything consumers need from the analyzer and manifest modules.
 *
 * ## Quick Start
 *
 * ```ts
 * import { analyzeSource, parseManifest, compareCapabilities } from "@coding-adventures/ca-capability-analyzer";
 *
 * // Analyze a source file
 * const result = analyzeSource('import fs from "fs"; fs.readFileSync("/data");', "app.ts");
 * console.log(result.capabilities);
 * // [{ category: "fs", action: "*", target: "*", ... }, { category: "fs", action: "read", target: "/data", ... }]
 *
 * // Compare against a manifest
 * const manifest = parseManifest('{ "capabilities": [{ "category": "fs", "action": "read", "target": "*" }] }');
 * const comparison = compareCapabilities(result.capabilities, manifest);
 * console.log(comparison.undeclared);  // Capabilities used but not declared
 * ```
 */

export {
  analyzeSource,
  analyzeFiles,
  type DetectedCapability,
  type BannedConstruct,
  type AnalysisResult,
} from "./analyzer.js";

export {
  parseManifest,
  compareCapabilities,
  capabilityMatchesDeclaration,
  simpleGlobMatch,
  type DeclaredCapability,
  type CapabilityManifest,
  type ComparisonResult,
} from "./manifest.js";

export { main } from "./cli.js";
