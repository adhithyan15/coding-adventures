import * as fs from "node:fs";
import * as path from "node:path";
import type { Package } from "./discovery.js";

const CI_MANAGED_TOOLCHAIN_LANGUAGES = new Set([
  "python",
  "ruby",
  "typescript",
  "rust",
  "elixir",
  "lua",
  "perl",
]);

export function validateCIFullBuildToolchains(
  root: string,
  packages: ReadonlyArray<Pick<Package, "language">>,
): string | null {
  const ciPath = path.join(root, ".github", "workflows", "ci.yml");
  if (!fs.existsSync(ciPath)) {
    return null;
  }

  const workflow = fs.readFileSync(ciPath, "utf-8");
  if (!workflow.includes("Full build on main merge")) {
    return null;
  }

  const compactWorkflow = workflow.replace(/\s+/g, "");
  const missingOutputBinding: string[] = [];
  const missingMainForce: string[] = [];

  for (const lang of languagesNeedingCIToolchains(packages)) {
    const outputBinding =
      "needs_" + lang + ":${{steps.toolchains.outputs.needs_" + lang + "}}";
    if (!compactWorkflow.includes(outputBinding)) {
      missingOutputBinding.push(lang);
    }

    if (!compactWorkflow.includes(`needs_${lang}=true`)) {
      missingMainForce.push(lang);
    }
  }

  if (missingOutputBinding.length === 0 && missingMainForce.length === 0) {
    return null;
  }

  const parts: string[] = [];
  if (missingOutputBinding.length > 0) {
    parts.push(
      "detect outputs for forced main full builds are not normalized through " +
        `steps.toolchains for: ${missingOutputBinding.join(", ")}`,
    );
  }
  if (missingMainForce.length > 0) {
    parts.push(
      "forced main full-build path does not explicitly enable toolchains for: " +
        missingMainForce.join(", "),
    );
  }

  return `${ciPath.split(path.sep).join("/")}: ${parts.join("; ")}`;
}

function languagesNeedingCIToolchains(
  packages: ReadonlyArray<Pick<Package, "language">>,
): string[] {
  return [
    ...new Set(
      packages
        .map((pkg) => pkg.language)
        .filter((lang) => CI_MANAGED_TOOLCHAIN_LANGUAGES.has(lang)),
    ),
  ].sort();
}
