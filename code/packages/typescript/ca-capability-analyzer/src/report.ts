/**
 * Formatting and reporting for capability analysis results.
 */

import type { AnalysisResult } from "./types.js";

export function formatReportText(result: AnalysisResult, verbose: boolean = false): string {
  const lines: string[] = [];

  lines.push("================================================================================");
  lines.push(` Capability & CWE Analysis: ${result.packageName ?? result.packageDir}`);
  lines.push("================================================================================");
  lines.push(`Directory:       ${result.packageDir}`);
  lines.push(`Inspected files: ${result.inspectedFiles.length}`);
  lines.push(
    `Declared caps:   ${
      result.declaredCapabilities.length > 0 ? result.declaredCapabilities.join(", ") : "(none — pure computation)"
    }`
  );
  lines.push("");

  if (result.detectedCapabilities.length === 0) {
    lines.push("Detected capabilities: none (pure computation)");
  } else {
    lines.push(`Detected capabilities (${result.detectedCapabilities.length}):`);
    for (const cap of result.detectedCapabilities) {
      lines.push(`  - [${cap.capability}] ${cap.file}:${cap.line}:${cap.col} -> ${cap.evidence}`);
    }
  }
  lines.push("");

  if (result.detectedWeaknesses.length > 0) {
    lines.push(`Detected VER15 Weaknesses (${result.detectedWeaknesses.length}):`);
    for (const w of result.detectedWeaknesses) {
      lines.push(`  - [${w.cwe}] ${w.name} at ${w.file}:${w.line}:${w.col}`);
      lines.push(`      Evidence: ${w.evidence}`);
      lines.push(`      Detail:   ${w.description}`);
    }
    lines.push("");
  }

  if (result.exemptWeaknesses.length > 0) {
    lines.push(`Exempt Weaknesses (${result.exemptWeaknesses.length}):`);
    for (const w of result.exemptWeaknesses) {
      lines.push(`  - [${w.cwe}] ${w.name} (exempt in required_capabilities.json)`);
    }
    lines.push("");
  }

  if (result.violations.length > 0) {
    lines.push("Violations:");
    for (const v of result.violations) {
      lines.push(`  * ${v}`);
    }
    lines.push("");
  }

  lines.push("--------------------------------------------------------------------------------");
  lines.push(`Verdict: ${result.ok ? "PASS" : "FAIL"}`);
  lines.push("================================================================================");

  return lines.join("\n");
}

export function formatReportJson(result: AnalysisResult): string {
  return JSON.stringify(result, null, 2);
}
