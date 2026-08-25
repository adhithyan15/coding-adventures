import * as fs from "node:fs";
import * as path from "node:path";
import type { Package } from "./discovery.js";
import {
  UNICODE_VERSION,
  fullUppercase,
  nfc,
  nfkcCasefold,
} from "./tracked-artifact-unicode17.js";

export const TRACKED_ARTIFACT_UNICODE_VERSION = UNICODE_VERSION;

const TRACKED_ARTIFACT_COMPONENT_IDENTITY = "node_modules";
const TRACKED_ARTIFACT_REDACTED_PATH = "repository";

const windowsReservedBasenames = new Set([
  "CON",
  "PRN",
  "AUX",
  "NUL",
  "CONIN$",
  "CONOUT$",
  "CLOCK$",
  "COM1",
  "COM2",
  "COM3",
  "COM4",
  "COM5",
  "COM6",
  "COM7",
  "COM8",
  "COM9",
  "LPT1",
  "LPT2",
  "LPT3",
  "LPT4",
  "LPT5",
  "LPT6",
  "LPT7",
  "LPT8",
  "LPT9",
  "COM¹",
  "COM²",
  "COM³",
  "LPT¹",
  "LPT²",
  "LPT³",
]);

export type TrackedArtifactEntryKind = "regular" | "symlink" | "reparse";

export interface TrackedArtifactEntry {
  readonly ordinal: number;
  readonly path: string;
  readonly entry_kind: TrackedArtifactEntryKind;
}

export interface TrackedArtifactDiagnosticDetails {
  readonly ordinal: number;
  readonly entry_kind: TrackedArtifactEntryKind;
  readonly problem?: string;
}

export interface TrackedArtifactDiagnostic {
  readonly code:
    | "TRACKED_ARTIFACT_FORBIDDEN"
    | "TRACKED_ARTIFACT_PATH_INVALID";
  readonly severity: "error";
  readonly path: string;
  readonly details: TrackedArtifactDiagnosticDetails;
}

/**
 * Validate an already bounded, inert snapshot of tracked repository paths.
 *
 * Snapshot construction deliberately lives outside this function. The pure
 * adapter never enumerates a checkout, follows a link, invokes Git, opens a
 * path, reads the environment, launches a process, or accesses the network.
 */
export function validateTrackedArtifactSnapshot(
  entries: ReadonlyArray<TrackedArtifactEntry>,
  unicodeVersion = TRACKED_ARTIFACT_UNICODE_VERSION,
): TrackedArtifactDiagnostic[] {
  if (unicodeVersion !== TRACKED_ARTIFACT_UNICODE_VERSION) {
    throw new Error(
      `tracked artifact Unicode version must be ${TRACKED_ARTIFACT_UNICODE_VERSION}`,
    );
  }

  const diagnostics: TrackedArtifactDiagnostic[] = [];
  for (const entry of entries) {
    const { normalizedPath, problem } = normalizeTrackedArtifactPath(entry.path);
    const details: TrackedArtifactDiagnosticDetails =
      problem === undefined
        ? { ordinal: entry.ordinal, entry_kind: entry.entry_kind }
        : {
            ordinal: entry.ordinal,
            entry_kind: entry.entry_kind,
            problem,
          };

    if (problem !== undefined) {
      diagnostics.push({
        code: "TRACKED_ARTIFACT_PATH_INVALID",
        severity: "error",
        path: TRACKED_ARTIFACT_REDACTED_PATH,
        details,
      });
      continue;
    }

    if (
      normalizedPath !== undefined &&
      normalizedPath
        .split("/")
        .some(
          (component) =>
            nfkcCasefold(component) === TRACKED_ARTIFACT_COMPONENT_IDENTITY,
        )
    ) {
      diagnostics.push({
        code: "TRACKED_ARTIFACT_FORBIDDEN",
        severity: "error",
        path: normalizedPath,
        details,
      });
    }
  }

  return diagnostics.sort(compareTrackedArtifactDiagnostics);
}

interface NormalizedTrackedArtifactPath {
  readonly normalizedPath?: string;
  readonly problem?: string;
}

function normalizeTrackedArtifactPath(
  rawPath: string,
): NormalizedTrackedArtifactPath {
  // Separator replacement is intentionally lexical. Host path libraries can
  // collapse exactly the empty, dot, and traversal components we must reject.
  const normalizedPath = rawPath.replaceAll("\\", "/");
  if (normalizedPath.length === 0) {
    return { problem: "EMPTY" };
  }
  if (unicodeScalarCount(normalizedPath) > 512) {
    return { problem: "TOO_LONG" };
  }
  if (nfc(normalizedPath) !== normalizedPath) {
    return { problem: "NON_NFC" };
  }
  if (normalizedPath.startsWith("/")) {
    return { problem: "ABSOLUTE" };
  }
  if (/^[A-Za-z]:/.test(normalizedPath)) {
    return { problem: "DRIVE_QUALIFIED" };
  }

  const segments = normalizedPath.split("/");
  if (segments.some((segment) => segment.length === 0)) {
    return { problem: "EMPTY_SEGMENT" };
  }
  if (
    [...normalizedPath].some((character) => {
      const scalar = character.codePointAt(0)!;
      return scalar < 0x20 || '<>:"|?*'.includes(character);
    })
  ) {
    return { problem: "UNSAFE_CHARACTER" };
  }

  for (const segment of segments) {
    if (segment === "." || segment === "..") {
      return { problem: "DOT_SEGMENT" };
    }
    if (segment.endsWith(" ") || segment.endsWith(".")) {
      return { problem: "TRAILING_DOT_OR_SPACE" };
    }
    const basename = fullUppercase(segment.split(".", 1)[0]);
    if (windowsReservedBasenames.has(basename)) {
      return { problem: "RESERVED_BASENAME" };
    }
  }

  return { normalizedPath };
}

function unicodeScalarCount(value: string): number {
  let count = 0;
  for (const _character of value) {
    count += 1;
  }
  return count;
}

function compareTrackedArtifactDiagnostics(
  left: TrackedArtifactDiagnostic,
  right: TrackedArtifactDiagnostic,
): number {
  const codeComparison = compareStrings(left.code, right.code);
  if (codeComparison !== 0) {
    return codeComparison;
  }
  const pathComparison = compareUnicodeScalars(left.path, right.path);
  if (pathComparison !== 0) {
    return pathComparison;
  }
  return compareStrings(canonicalDetails(left.details), canonicalDetails(right.details));
}

function compareUnicodeScalars(left: string, right: string): number {
  const leftScalars = [...left].map((character) => character.codePointAt(0)!);
  const rightScalars = [...right].map((character) => character.codePointAt(0)!);
  const commonLength = Math.min(leftScalars.length, rightScalars.length);
  for (let index = 0; index < commonLength; index += 1) {
    if (leftScalars[index] !== rightScalars[index]) {
      return leftScalars[index] - rightScalars[index];
    }
  }
  return leftScalars.length - rightScalars.length;
}

function canonicalDetails(details: TrackedArtifactDiagnosticDetails): string {
  const canonical: Record<string, number | string> = {
    entry_kind: details.entry_kind,
    ordinal: details.ordinal,
  };
  if (details.problem !== undefined) {
    canonical.problem = details.problem;
  }
  return JSON.stringify(canonical);
}

function compareStrings(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

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
