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
  "java",
  "kotlin",
  "haskell",
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

export function validateBuildContracts(
  root: string,
  packages: ReadonlyArray<Pick<Package, "language" | "path">>,
): string | null {
  const errors: string[] = [];

  const ciError = validateCIFullBuildToolchains(root, packages);
  if (ciError !== null) {
    errors.push(ciError);
  }

  errors.push(...validateLuaIsolatedBuildFiles(packages));
  errors.push(...validatePerlBuildFiles(packages));

  if (errors.length === 0) {
    return null;
  }

  return errors.join("\n  - ");
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

function validateLuaIsolatedBuildFiles(
  packages: ReadonlyArray<Pick<Package, "language" | "path">>,
): string[] {
  const errors: string[] = [];

  for (const pkg of packages) {
    if (pkg.language !== "lua") {
      continue;
    }

    const selfRock =
      "coding-adventures-" + path.basename(pkg.path).replaceAll("_", "-");
    const buildLines = new Map<string, string[]>();

    for (const buildPath of luaBuildFiles(pkg.path)) {
      const lines = readBuildLines(buildPath);
      buildLines.set(path.basename(buildPath), lines);
      if (lines.length === 0) {
        continue;
      }

      const foreignRemove = firstForeignLuaRemove(lines, selfRock);
      if (foreignRemove !== null) {
        errors.push(
          `${slashPath(buildPath)}: Lua BUILD removes unrelated rock ${foreignRemove}; isolated package builds should only remove the package they are rebuilding`,
        );
      }

      const stateMachineIndex = firstLineContaining(lines, [
        "../state_machine",
        "..\\state_machine",
      ]);
      const directedGraphIndex = firstLineContaining(lines, [
        "../directed_graph",
        "..\\directed_graph",
      ]);
      if (
        stateMachineIndex !== null &&
        directedGraphIndex !== null &&
        stateMachineIndex < directedGraphIndex
      ) {
        errors.push(
          `${slashPath(buildPath)}: Lua BUILD installs state_machine before directed_graph; isolated LuaRocks builds require directed_graph first`,
        );
      }

      if (
        (hasGuardedLocalLuaInstall(lines) ||
          (path.basename(buildPath) === "BUILD_windows" &&
            hasLocalLuaSiblingInstall(lines))) &&
        !selfInstallDisablesDeps(lines, selfRock)
      ) {
        errors.push(
          `${slashPath(buildPath)}: Lua BUILD bootstraps sibling rocks but the final self-install does not pass --deps-mode=none or --no-manifest`,
        );
      }
    }

    const missingWindowsDeps = missingLuaSiblingInstalls(
      buildLines.get("BUILD") ?? [],
      buildLines.get("BUILD_windows") ?? [],
    );
    if (missingWindowsDeps.length > 0) {
      errors.push(
        `${slashPath(path.join(pkg.path, "BUILD_windows"))}: Lua BUILD_windows is missing sibling installs present in BUILD: ${missingWindowsDeps.join(", ")}`,
      );
    }
  }

  return errors;
}

function validatePerlBuildFiles(
  packages: ReadonlyArray<Pick<Package, "language" | "path">>,
): string[] {
  const errors: string[] = [];

  for (const pkg of packages) {
    if (pkg.language !== "perl") {
      continue;
    }

    for (const buildPath of luaBuildFiles(pkg.path)) {
      for (const line of readBuildLines(buildPath)) {
        if (
          line.includes("cpanm") &&
          line.includes("Test2::V0") &&
          !line.includes("--notest")
        ) {
          errors.push(
            `${slashPath(buildPath)}: Perl BUILD bootstraps Test2::V0 without --notest; isolated Windows installs can fail while installing the test framework itself`,
          );
          break;
        }
      }
    }
  }

  return errors;
}

function luaBuildFiles(pkgPath: string): string[] {
  return fs
    .readdirSync(pkgPath, { withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.startsWith("BUILD"))
    .map((entry) => path.join(pkgPath, entry.name))
    .sort();
}

function readBuildLines(buildPath: string): string[] {
  return fs
    .readFileSync(buildPath, "utf-8")
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0 && !line.startsWith("#"));
}

function firstForeignLuaRemove(
  lines: ReadonlyArray<string>,
  selfRock: string,
): string | null {
  for (const line of lines) {
    const match = /\bluarocks remove --force ([^ \t]+)/.exec(line);
    if (match !== null && match[1] !== selfRock) {
      return match[1];
    }
  }
  return null;
}

function firstLineContaining(
  lines: ReadonlyArray<string>,
  needles: ReadonlyArray<string>,
): number | null {
  for (const [index, line] of lines.entries()) {
    if (needles.some((needle) => line.includes(needle))) {
      return index;
    }
  }
  return null;
}

function hasGuardedLocalLuaInstall(lines: ReadonlyArray<string>): boolean {
  return lines.some(
    (line) =>
      line.includes("luarocks show ") &&
      (line.includes("../") || line.includes("..\\")),
  );
}

function hasLocalLuaSiblingInstall(lines: ReadonlyArray<string>): boolean {
  return luaSiblingInstallDirs(lines).length > 0;
}

function selfInstallDisablesDeps(
  lines: ReadonlyArray<string>,
  selfRock: string,
): boolean {
  return lines.some(
    (line) =>
      line.includes("luarocks make") &&
      line.includes(selfRock) &&
      (line.includes("--deps-mode=none") ||
        line.includes("--deps-mode none") ||
        line.includes("--no-manifest")),
  );
}

function missingLuaSiblingInstalls(
  unixLines: ReadonlyArray<string>,
  windowsLines: ReadonlyArray<string>,
): string[] {
  const windowsDeps = new Set(luaSiblingInstallDirs(windowsLines));
  return luaSiblingInstallDirs(unixLines).filter((dep) => !windowsDeps.has(dep));
}

function luaSiblingInstallDirs(lines: ReadonlyArray<string>): string[] {
  const deps = new Set<string>();

  for (const line of lines) {
    if (!line.includes("luarocks make")) {
      continue;
    }

    const match = /\bcd\s+([.][.][\\/][^ \t\r\n&()]+)/.exec(line);
    if (match === null) {
      continue;
    }

    deps.add(match[1].replaceAll("\\", "/"));
  }

  return [...deps].sort();
}

function slashPath(filepath: string): string {
  return filepath.split(path.sep).join("/");
}
