import * as fs from "node:fs";
import * as path from "node:path";
import type { Package } from "./discovery.js";
import {
  UNICODE_VERSION,
  casefold,
  fullUppercase,
  nfc,
  nfkcCasefold,
} from "./tracked-artifact-unicode17.js";

export const TRACKED_ARTIFACT_UNICODE_VERSION = UNICODE_VERSION;

const TRACKED_ARTIFACT_COMPONENT_IDENTITY = "node_modules";
const TRACKED_ARTIFACT_REDACTED_PATH = "repository";
const ORPHAN_SCAN_ROOT = "code";
const ORPHAN_LEDGER_PATH = "code/BUILD-EXEMPTIONS";
const ORPHAN_BUILD_NAMES = [
  "BUILD",
  "BUILD_windows",
  "BUILD_mac",
  "BUILD_linux",
  "BUILD_mac_and_linux",
] as const;
const ORPHAN_SKIP_COMPONENTS = new Set([
  ".git",
  "target",
  "node_modules",
  "vendor",
  ".venv",
  "_build",
  "deps",
  ".build",
  "dist-newstyle",
  ".cargo",
]);

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

export type OrphanManifestKind = "package" | "virtual_workspace";
export type OrphanBuildFileState = "runnable" | "empty";

export interface OrphanManifest {
  readonly path: string;
  readonly kind: OrphanManifestKind;
}

export interface OrphanBuildFile {
  readonly path: string;
  readonly state: OrphanBuildFileState;
}

export interface OrphanExemption {
  readonly line: number;
  readonly kind: string;
  readonly path: string;
  readonly reason: string;
}

export interface OrphanCrateSnapshot {
  readonly directories: ReadonlyArray<string>;
  readonly manifests: ReadonlyArray<OrphanManifest>;
  readonly build_files: ReadonlyArray<OrphanBuildFile>;
  readonly exemptions: ReadonlyArray<OrphanExemption>;
}

export interface OrphanCrateDiagnosticDetails {
  readonly build_path?: string;
  readonly entry_path?: string;
  readonly kind?: string;
  readonly line?: number;
  readonly manifest_kind?: OrphanManifestKind;
  readonly problem?: string;
}

export interface OrphanCrateDiagnostic {
  readonly code:
    | "ORPHAN_CRATE_EMPTY_BUILD"
    | "ORPHAN_CRATE_UNLISTED"
    | "ORPHAN_EXEMPTION_INVALID"
    | "ORPHAN_EXEMPTION_STALE";
  readonly severity: "error";
  readonly path: string;
  readonly details: OrphanCrateDiagnosticDetails;
}

export interface OrphanCrateValidationResult {
  readonly valid: boolean;
  readonly diagnostic_codes: string[];
  readonly pending_exemption_count: number;
  readonly diagnostics: OrphanCrateDiagnostic[];
}

/**
 * Validate an already bounded, inert Cargo/BUILD/exemption snapshot.
 *
 * Snapshot construction deliberately lives outside this function. The pure
 * adapter never enumerates a checkout, opens a path, invokes Git, launches a
 * process, reads the environment, or accesses the network.
 */
export function validateOrphanCrateSnapshot(
  snapshot: OrphanCrateSnapshot,
): OrphanCrateValidationResult {
  const manifests = snapshot.manifests.filter(
    (manifest) => !isOrphanArtifactPath(manifest.path),
  );
  const directories = new Set(snapshot.directories);
  const manifestByPath = new Map(
    manifests.map((manifest) => [manifest.path, manifest]),
  );
  const runnableCoverage = new Map(
    manifests.map((manifest) => [
      manifest.path,
      findCoveringBuild(snapshot.build_files, manifest.path, "runnable"),
    ]),
  );
  const emptyBuilds = new Map(
    manifests.map((manifest) => [
      manifest.path,
      findCoveringBuild(snapshot.build_files, manifest.path, "empty"),
    ]),
  );

  const diagnostics: OrphanCrateDiagnostic[] = [];
  const seenExemptionPaths = new Set<string>();
  const validExemptions: OrphanExemption[] = [];

  // Reserve every portable identity before applying policy-field precedence.
  // An invalid first spelling therefore cannot hide a later normalized alias.
  for (const exemption of snapshot.exemptions) {
    let identity: string | undefined;
    let pathProblem: string | undefined;
    if (!isPortableOrphanPath(exemption.path)) {
      pathProblem = "PATH_UNSAFE";
    } else {
      identity = casefold(nfc(exemption.path));
      if (!isUnderOrphanScanRoot(exemption.path)) {
        pathProblem = "PATH_OUTSIDE_SCAN";
      } else if (isOrphanArtifactPath(exemption.path)) {
        pathProblem = "PATH_ARTIFACT";
      }
    }

    const duplicate =
      identity !== undefined && seenExemptionPaths.has(identity);
    if (identity !== undefined && !duplicate) {
      seenExemptionPaths.add(identity);
    }

    let problem: string | undefined;
    if (exemption.kind !== "EXCLUDED" && exemption.kind !== "PENDING") {
      problem = "UNKNOWN_KIND";
    } else if (isPythonBlank(exemption.reason)) {
      problem = "REASON_MISSING";
    } else if (duplicate) {
      problem = "DUPLICATE_PATH";
    } else {
      problem = pathProblem;
    }

    if (problem !== undefined) {
      diagnostics.push({
        code: "ORPHAN_EXEMPTION_INVALID",
        severity: "error",
        path: ORPHAN_LEDGER_PATH,
        details: { line: exemption.line, problem },
      });
      continue;
    }
    validExemptions.push(exemption);
  }

  const activeExemptions = new Map<string, OrphanExemption>();
  let pendingExemptionCount = 0;
  for (const exemption of validExemptions) {
    let staleProblem: string | undefined;
    if (!directories.has(exemption.path)) {
      staleProblem = "MISSING_DIRECTORY";
    } else if (!manifestByPath.has(exemption.path)) {
      staleProblem = "NO_MANIFEST";
    } else if (runnableCoverage.get(exemption.path) !== undefined) {
      staleProblem = "COVERED";
    }

    if (staleProblem !== undefined) {
      diagnostics.push({
        code: "ORPHAN_EXEMPTION_STALE",
        severity: "error",
        path: ORPHAN_LEDGER_PATH,
        details: {
          entry_path: exemption.path,
          kind: exemption.kind,
          line: exemption.line,
          problem: staleProblem,
        },
      });
      continue;
    }

    activeExemptions.set(exemption.path, exemption);
    if (exemption.kind === "PENDING") {
      pendingExemptionCount += 1;
    }
  }

  for (const manifest of manifests) {
    const manifestPath = manifest.path;
    if (
      runnableCoverage.get(manifestPath) !== undefined ||
      activeExemptions.has(manifestPath)
    ) {
      continue;
    }

    const emptyBuild = emptyBuilds.get(manifestPath);
    if (emptyBuild === undefined) {
      diagnostics.push({
        code: "ORPHAN_CRATE_UNLISTED",
        severity: "error",
        path: manifestPath,
        details: { manifest_kind: manifest.kind },
      });
    } else {
      diagnostics.push({
        code: "ORPHAN_CRATE_EMPTY_BUILD",
        severity: "error",
        path: manifestPath,
        details: {
          build_path: emptyBuild.path,
          manifest_kind: manifest.kind,
        },
      });
    }
  }

  diagnostics.sort(compareValidationDiagnostics);
  return {
    valid: diagnostics.length === 0,
    diagnostic_codes: [...new Set(diagnostics.map(({ code }) => code))].sort(
      compareStrings,
    ),
    pending_exemption_count: pendingExemptionCount,
    diagnostics,
  };
}

function findCoveringBuild(
  buildFiles: ReadonlyArray<OrphanBuildFile>,
  manifestPath: string,
  state: OrphanBuildFileState,
): OrphanBuildFile | undefined {
  const buildNameRank = new Map<string, number>(
    ORPHAN_BUILD_NAMES.map((name, index) => [name, index]),
  );
  const candidates = buildFiles.filter((buildFile) => {
    if (buildFile.state !== state) {
      return false;
    }
    const separator = buildFile.path.lastIndexOf("/");
    if (separator < 0) {
      return false;
    }
    const parent = buildFile.path.slice(0, separator);
    const name = buildFile.path.slice(separator + 1);
    return (
      isUnderOrphanScanRoot(parent) &&
      (manifestPath === parent || manifestPath.startsWith(`${parent}/`)) &&
      buildNameRank.has(name)
    );
  });

  candidates.sort((left, right) => {
    const leftSeparator = left.path.lastIndexOf("/");
    const rightSeparator = right.path.lastIndexOf("/");
    const leftParent = left.path.slice(0, leftSeparator);
    const rightParent = right.path.slice(0, rightSeparator);
    const depthComparison =
      rightParent.split("/").length - leftParent.split("/").length;
    if (depthComparison !== 0) {
      return depthComparison;
    }
    const rankComparison =
      buildNameRank.get(left.path.slice(leftSeparator + 1))! -
      buildNameRank.get(right.path.slice(rightSeparator + 1))!;
    return rankComparison !== 0
      ? rankComparison
      : compareUnicodeScalars(left.path, right.path);
  });
  return candidates[0];
}

function isPortableOrphanPath(value: string): boolean {
  if (
    value.length === 0 ||
    unicodeScalarCount(value) > 512 ||
    nfc(value) !== value ||
    value.startsWith("/") ||
    value.includes("\\") ||
    value.includes("//") ||
    /^[A-Za-z]:/.test(value)
  ) {
    return false;
  }
  if (
    [...value].some((character) => {
      const scalar = character.codePointAt(0)!;
      return scalar < 0x20 || '<>:"|?*'.includes(character);
    })
  ) {
    return false;
  }
  return value.split("/").every((component) => {
    if (
      component.length === 0 ||
      component === "." ||
      component === ".." ||
      component.endsWith(" ") ||
      component.endsWith(".")
    ) {
      return false;
    }
    return !windowsReservedBasenames.has(
      fullUppercase(component.split(".", 1)[0]),
    );
  });
}

function isUnderOrphanScanRoot(value: string): boolean {
  return value === ORPHAN_SCAN_ROOT || value.startsWith(`${ORPHAN_SCAN_ROOT}/`);
}

function isOrphanArtifactPath(value: string): boolean {
  return value
    .split("/")
    .some((component) => ORPHAN_SKIP_COMPONENTS.has(component));
}

function isPythonBlank(value: string): boolean {
  return /^[\u0009-\u000d\u001c-\u0020\u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]*$/u.test(
    value,
  );
}

function compareValidationDiagnostics(
  left: OrphanCrateDiagnostic,
  right: OrphanCrateDiagnostic,
): number {
  const codeComparison = compareStrings(left.code, right.code);
  if (codeComparison !== 0) {
    return codeComparison;
  }
  const pathComparison = compareUnicodeScalars(left.path, right.path);
  return pathComparison !== 0
    ? pathComparison
    : compareStrings(canonicalDetails(left.details), canonicalDetails(right.details));
}

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

function canonicalDetails(details: object): string {
  return `{${Object.entries(details)
    .filter((entry): entry is [string, number | string] => entry[1] !== undefined)
    .sort(([left], [right]) => compareStrings(left, right))
    .map(
      ([key, value]) =>
        `${pythonAsciiJson(key)}: ${
          typeof value === "number" ? String(value) : pythonAsciiJson(value)
        }`,
    )
    .join(", ")}}`;
}

function pythonAsciiJson(value: string): string {
  let encoded = '"';
  for (const character of value) {
    const scalar = character.codePointAt(0)!;
    if (scalar < 0x7f) {
      encoded += JSON.stringify(character).slice(1, -1);
    } else if (scalar <= 0xffff) {
      encoded += `\\u${scalar.toString(16).padStart(4, "0")}`;
    } else {
      const adjusted = scalar - 0x10000;
      const high = 0xd800 + (adjusted >> 10);
      const low = 0xdc00 + (adjusted & 0x3ff);
      encoded += `\\u${high.toString(16)}\\u${low.toString(16)}`;
    }
  }
  return `${encoded}"`;
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
