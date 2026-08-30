/**
 * toolchain-detection.ts -- Pure extra-CI toolchain declarations
 * ===========================================================================
 *
 * A normal build tool discovers packages from disk and may ask the host which
 * compilers are installed.  Neither operation belongs in this module.  The
 * conformance boundary is deliberately closer to a truth table: the caller
 * supplies package names, languages, and inert BUILD-file text; this module
 * returns the complete set of toolchain booleans implied by that snapshot.
 *
 * Keeping the boundary process-free has two useful consequences.  Tests can
 * replay the same repository-shaped input on every implementation language,
 * and malformed input cannot become a shell command, path lookup, or host
 * probe.  The only parser below recognizes one exact comment grammar:
 *
 *     # needs-toolchain: NAME
 *
 * Everything else remains inert text.
 */

export const MAX_BUILD_BYTES = 65_536;
export const MAX_BUILD_LINES = 4_096;
export const MAX_AGGREGATE_BUILD_BYTES = 1_048_576;

/** The sorted registry is both the result schema and the accepted vocabulary. */
export const CANONICAL_TOOLCHAINS = Object.freeze([
  "cpp",
  "dart",
  "dotnet",
  "elixir",
  "go",
  "haskell",
  "java",
  "kotlin",
  "lua",
  "ocaml",
  "perl",
  "python",
  "ruby",
  "rust",
  "swift",
  "typescript",
] as const);

export type ToolchainName = (typeof CANONICAL_TOOLCHAINS)[number];
export type TargetPlatform = "darwin" | "linux" | "windows" | "win32";

export interface ToolchainPackageSnapshot {
  readonly name: string;
  readonly language: string;
  readonly buildFiles: Readonly<Record<string, string>>;
}

export interface ToolchainSnapshotOptions {
  readonly platform: string;
  readonly forceFull: boolean;
  readonly packages: readonly ToolchainPackageSnapshot[];
  readonly scheduledPackages: readonly string[] | null;
  readonly forcedToolchains: readonly string[] | null;
}

export interface ToolchainDiagnostic {
  readonly code: "TOOLCHAIN_UNSUPPORTED";
  readonly severity: "error";
  readonly package?: string;
}

export interface ToolchainEvaluation {
  readonly outcome: "ok" | "error";
  readonly toolchains: Readonly<Record<string, boolean>>;
  readonly diagnostics: readonly ToolchainDiagnostic[];
}

export type ToolchainSnapshotErrorCode =
  | "BUILD_FRONT_TOO_LARGE"
  | "BUILD_FRONT_TOO_MANY_LINES"
  | "BUILD_SNAPSHOT_TOO_LARGE"
  | "FORCE_FULL_SCHEDULE_INVALID"
  | "PLATFORM_UNSUPPORTED"
  | "SNAPSHOT_INVALID";

/**
 * Resource and shape failures are API errors, not portable diagnostics.
 *
 * Neutral fixtures have already passed their schema limits.  Direct callers
 * still receive a stable typed error if they bypass that runner.  Messages do
 * not echo caller content, package paths, or host details.
 */
export class ToolchainSnapshotError extends Error {
  public readonly code: ToolchainSnapshotErrorCode;

  public constructor(code: ToolchainSnapshotErrorCode, message: string) {
    super(message);
    this.name = "ToolchainSnapshotError";
    this.code = code;
  }
}

interface PreparedPackage {
  readonly name: string;
  readonly language: string;
  readonly buildFiles: Readonly<Record<string, string>>;
}

const DECLARATION_PREFIX = "# needs-toolchain:";
const UTF8_ENCODER = new TextEncoder();
const CANONICAL_SET: ReadonlySet<string> = new Set(CANONICAL_TOOLCHAINS);
const EMPTY_DIAGNOSTICS = Object.freeze([]) as readonly ToolchainDiagnostic[];

function utf8ByteLength(value: string): number {
  return UTF8_ENCODER.encode(value).byteLength;
}

/** `N` LF bytes delimit `N + 1` logical lines, even when the last is empty. */
function logicalLineCount(content: string): number {
  let lines = 1;
  for (let index = 0; index < content.length; index += 1) {
    if (content.charCodeAt(index) === 0x0a) {
      lines += 1;
    }
  }
  return lines;
}

function isAsciiSpaceOrTab(code: number): boolean {
  return code === 0x20 || code === 0x09;
}

/** JavaScript's `trim()` is intentionally too broad for the byte grammar. */
function trimAsciiSpaceAndTab(value: string): string {
  let start = 0;
  let end = value.length;
  while (start < end && isAsciiSpaceOrTab(value.charCodeAt(start))) {
    start += 1;
  }
  while (end > start && isAsciiSpaceOrTab(value.charCodeAt(end - 1))) {
    end -= 1;
  }
  return value.slice(start, end);
}

function isToolchainName(value: string): value is ToolchainName {
  return CANONICAL_SET.has(value);
}

/**
 * Parse one already bounded BUILD front.
 *
 * The public helper repeats the same ceilings as the top-level evaluator so a
 * caller cannot turn direct parser use into an unbounded `split()` allocation.
 * Oversized direct input is inert and therefore contributes no declaration.
 */
export function parseExtraToolchains(
  content: string,
): readonly ToolchainName[] {
  if (
    utf8ByteLength(content) > MAX_BUILD_BYTES ||
    logicalLineCount(content) > MAX_BUILD_LINES
  ) {
    return Object.freeze([]);
  }

  const rawLines = content.split("\n");
  const declarations: ToolchainName[] = [];
  const seen = new Set<ToolchainName>();

  for (let index = 0; index < rawLines.length; index += 1) {
    let line = rawLines[index] ?? "";

    // Only the CR immediately paired with an LF terminator is structural.
    // A final lone CR, a whitespace-separated CR, or the first CR in CRCRLF
    // remains content and therefore makes the declaration invalid.
    if (index < rawLines.length - 1 && line.endsWith("\r")) {
      line = line.slice(0, -1);
    }
    line = trimAsciiSpaceAndTab(line);
    if (!line.startsWith(DECLARATION_PREFIX)) {
      continue;
    }

    const suffix = line.slice(DECLARATION_PREFIX.length);
    if (
      suffix.length === 0 ||
      !isAsciiSpaceOrTab(suffix.charCodeAt(0))
    ) {
      continue;
    }
    const name = trimAsciiSpaceAndTab(suffix);
    if (!isToolchainName(name) || seen.has(name)) {
      continue;
    }
    seen.add(name);
    declarations.push(name);
  }

  return Object.freeze(declarations);
}

function preparePackages(
  packages: readonly ToolchainPackageSnapshot[],
): readonly PreparedPackage[] {
  let aggregateBytes = 0;
  const prepared: PreparedPackage[] = [];

  for (const packageSnapshot of packages) {
    if (
      typeof packageSnapshot.name !== "string" ||
      typeof packageSnapshot.language !== "string" ||
      typeof packageSnapshot.buildFiles !== "object" ||
      packageSnapshot.buildFiles === null ||
      Array.isArray(packageSnapshot.buildFiles)
    ) {
      throw new ToolchainSnapshotError(
        "SNAPSHOT_INVALID",
        "toolchain package snapshots require string names, languages, and BUILD maps",
      );
    }

    const copiedBuildFiles: Record<string, string> = {};
    for (const [filename, content] of Object.entries(
      packageSnapshot.buildFiles,
    )) {
      if (typeof filename !== "string" || typeof content !== "string") {
        throw new ToolchainSnapshotError(
          "SNAPSHOT_INVALID",
          "toolchain BUILD fronts must map strings to strings",
        );
      }
      const byteLength = utf8ByteLength(content);
      if (byteLength > MAX_BUILD_BYTES) {
        throw new ToolchainSnapshotError(
          "BUILD_FRONT_TOO_LARGE",
          "toolchain BUILD front exceeds its UTF-8 byte ceiling",
        );
      }
      if (logicalLineCount(content) > MAX_BUILD_LINES) {
        throw new ToolchainSnapshotError(
          "BUILD_FRONT_TOO_MANY_LINES",
          "toolchain BUILD front exceeds its logical-line ceiling",
        );
      }
      aggregateBytes += byteLength;
      copiedBuildFiles[filename] = content;
    }

    prepared.push({
      name: packageSnapshot.name,
      language: packageSnapshot.language,
      buildFiles: Object.freeze(copiedBuildFiles),
    });
  }

  if (aggregateBytes > MAX_AGGREGATE_BUILD_BYTES) {
    throw new ToolchainSnapshotError(
      "BUILD_SNAPSHOT_TOO_LARGE",
      "toolchain BUILD snapshot exceeds its aggregate byte ceiling",
    );
  }
  return Object.freeze(prepared);
}

function buildFileCandidates(platform: string): readonly string[] {
  switch (platform) {
    case "darwin":
      return ["BUILD_mac", "BUILD_mac_and_linux", "BUILD"];
    case "linux":
      return ["BUILD_linux", "BUILD_mac_and_linux", "BUILD"];
    case "windows":
    case "win32":
      return ["BUILD_windows", "BUILD"];
    default:
      throw new ToolchainSnapshotError(
        "PLATFORM_UNSUPPORTED",
        "unsupported target platform",
      );
  }
}

function selectedFront(
  buildFiles: Readonly<Record<string, string>>,
  candidates: readonly string[],
): string {
  for (const filename of candidates) {
    if (Object.prototype.hasOwnProperty.call(buildFiles, filename)) {
      return buildFiles[filename] ?? "";
    }
  }
  return "";
}

function toolchainForLanguage(language: string): ToolchainName | null {
  if (language === "wasm") {
    return "rust";
  }
  if (language === "c" || language === "cpp") {
    return "cpp";
  }
  if (
    language === "csharp" ||
    language === "fsharp" ||
    language === "dotnet"
  ) {
    return "dotnet";
  }
  return isToolchainName(language) ? language : null;
}

function unsupported(packageName?: string): ToolchainEvaluation {
  const diagnostic: ToolchainDiagnostic = Object.freeze(
    packageName === undefined
      ? { code: "TOOLCHAIN_UNSUPPORTED", severity: "error" }
      : {
          code: "TOOLCHAIN_UNSUPPORTED",
          severity: "error",
          package: packageName,
        },
  );
  return Object.freeze({
    outcome: "error",
    toolchains: Object.freeze({}),
    diagnostics: Object.freeze([diagnostic]),
  });
}

function completeToolchainMap(enabled: boolean): Record<ToolchainName, boolean> {
  return Object.fromEntries(
    CANONICAL_TOOLCHAINS.map((name) => [name, enabled]),
  ) as Record<ToolchainName, boolean>;
}

/** Evaluate one complete caller-owned snapshot without touching host state. */
export function evaluateToolchainSnapshot(
  options: ToolchainSnapshotOptions,
): ToolchainEvaluation {
  // Meter every supplied front, including an unselected platform override,
  // before scheduling can make a package appear irrelevant.
  const packages = preparePackages(options.packages);
  const candidates = buildFileCandidates(options.platform);
  if (options.forceFull && options.scheduledPackages !== null) {
    throw new ToolchainSnapshotError(
      "FORCE_FULL_SCHEDULE_INVALID",
      "force-full snapshots require a null package schedule",
    );
  }

  const scheduled =
    options.scheduledPackages === null
      ? null
      : new Set(options.scheduledPackages);
  const toolchains = completeToolchainMap(options.forceFull);

  for (const packageSnapshot of packages) {
    if (scheduled !== null && !scheduled.has(packageSnapshot.name)) {
      continue;
    }
    const nativeToolchain = toolchainForLanguage(packageSnapshot.language);
    if (nativeToolchain === null) {
      return unsupported(packageSnapshot.name);
    }
    if (options.forceFull) {
      continue;
    }

    toolchains[nativeToolchain] = true;
    const front = selectedFront(packageSnapshot.buildFiles, candidates);
    for (const extraToolchain of parseExtraToolchains(front)) {
      toolchains[extraToolchain] = true;
    }
  }

  for (const forcedToolchain of options.forcedToolchains ?? []) {
    if (!isToolchainName(forcedToolchain)) {
      return unsupported();
    }
    toolchains[forcedToolchain] = true;
  }

  return Object.freeze({
    outcome: "ok",
    toolchains: Object.freeze(toolchains),
    diagnostics: EMPTY_DIAGNOSTICS,
  });
}
