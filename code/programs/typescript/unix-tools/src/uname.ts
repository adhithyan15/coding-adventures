/**
 * uname -- print system information.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `uname` utility in TypeScript. It
 * prints information about the current system: kernel name, hostname,
 * kernel release, kernel version, machine architecture, and operating
 * system name.
 *
 * === How uname Works ===
 *
 * With no flags, uname prints just the kernel name (equivalent to `-s`):
 *
 *     $ uname
 *     Darwin
 *
 * With `-a` (all), it prints everything in this order:
 *
 *     kernel-name nodename kernel-release kernel-version machine processor
 *     hardware-platform operating-system
 *
 * === Flag-to-Field Mapping ===
 *
 *     Flag    Field               Node.js API
 *     -s      kernel name         os.type()
 *     -n      nodename            os.hostname()
 *     -r      kernel release      os.release()
 *     -v      kernel version      os.version() (Node 13+) or fallback
 *     -m      machine             os.machine() or os.arch()
 *     -p      processor           os.arch()
 *     -i      hardware platform   os.arch() (non-portable)
 *     -o      operating system    os.platform() mapped to name
 *
 * @module uname
 */

import * as os from "node:os";
import * as path from "node:path";
import { fileURLToPath } from "node:url";

// ---------------------------------------------------------------------------
// Import CLI Builder.
// ---------------------------------------------------------------------------

import { Parser } from "@coding-adventures/cli-builder";

// ---------------------------------------------------------------------------
// Locate the JSON spec file.
// ---------------------------------------------------------------------------

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const SPEC_FILE = path.resolve(__dirname, "..", "uname.json");

// ---------------------------------------------------------------------------
// Types: System information.
// ---------------------------------------------------------------------------

/**
 * All the system information fields that uname can report.
 *
 * Each field corresponds to one of the uname flags. We gather all
 * the information upfront and then selectively print based on which
 * flags were requested.
 */
export interface UnameInfo {
  /** Kernel name, e.g., "Darwin" or "Linux". */
  kernelName: string;
  /** Network node hostname. */
  nodename: string;
  /** Kernel release version string. */
  kernelRelease: string;
  /** Kernel version string (build info). */
  kernelVersion: string;
  /** Machine hardware name, e.g., "x86_64" or "arm64". */
  machine: string;
  /** Processor type (non-portable). */
  processor: string;
  /** Hardware platform (non-portable). */
  hardwarePlatform: string;
  /** Operating system name. */
  operatingSystem: string;
}

// ---------------------------------------------------------------------------
// Business Logic: Map platform names to OS names.
// ---------------------------------------------------------------------------

/**
 * Map Node.js platform strings to human-readable OS names.
 *
 * GNU uname -o outputs "GNU/Linux" on Linux and "Darwin" on macOS.
 * We follow the same convention.
 */
const PLATFORM_NAMES: Record<string, string> = {
  linux: "GNU/Linux",
  darwin: "Darwin",
  win32: "Windows",
  freebsd: "FreeBSD",
  openbsd: "OpenBSD",
  sunos: "SunOS",
  aix: "AIX",
};

// ---------------------------------------------------------------------------
// Business Logic: Gather system information.
// ---------------------------------------------------------------------------

/**
 * Gather all system information from the Node.js `os` module.
 *
 * We use various `os` module functions to populate the UnameInfo
 * structure. Some fields (like kernel version) may not be available
 * on older Node.js versions, so we provide fallbacks.
 *
 * @returns A populated UnameInfo object.
 */
export function getSystemInfo(): UnameInfo {
  const arch = os.arch();
  // os.machine() returns the actual hardware name (e.g., "arm64" even
  // on x86 emulation). Available since Node 16.18.
  const machine =
    typeof os.machine === "function" ? os.machine() : arch;

  // os.version() returns the kernel version string (e.g., "Darwin Kernel
  // Version 23.1.0: ..."). Available since Node 13.11.
  let kernelVersion: string;
  try {
    kernelVersion =
      typeof os.version === "function" ? os.version() : "unknown";
  } catch {
    kernelVersion = "unknown";
  }

  return {
    kernelName: os.type(),
    nodename: os.hostname(),
    kernelRelease: os.release(),
    kernelVersion,
    machine,
    processor: arch,
    hardwarePlatform: arch,
    operatingSystem: PLATFORM_NAMES[os.platform()] ?? os.platform(),
  };
}

// ---------------------------------------------------------------------------
// Business Logic: Format output based on selected fields.
// ---------------------------------------------------------------------------

/**
 * Format the uname output based on which flags are selected.
 *
 * Each flag selects a field from the UnameInfo object. The fields are
 * printed in a fixed order (matching GNU uname -a order), separated
 * by spaces.
 *
 * @param info    - The system information.
 * @param flags   - Which fields to include.
 * @returns A string with the selected fields, space-separated.
 */
export function formatUname(
  info: UnameInfo,
  flags: {
    kernelName: boolean;
    nodename: boolean;
    kernelRelease: boolean;
    kernelVersion: boolean;
    machine: boolean;
    processor: boolean;
    hardwarePlatform: boolean;
    operatingSystem: boolean;
  }
): string {
  const parts: string[] = [];

  // The order matches GNU uname -a output order.
  if (flags.kernelName) parts.push(info.kernelName);
  if (flags.nodename) parts.push(info.nodename);
  if (flags.kernelRelease) parts.push(info.kernelRelease);
  if (flags.kernelVersion) parts.push(info.kernelVersion);
  if (flags.machine) parts.push(info.machine);
  if (flags.processor) parts.push(info.processor);
  if (flags.hardwarePlatform) parts.push(info.hardwarePlatform);
  if (flags.operatingSystem) parts.push(info.operatingSystem);

  return parts.join(" ");
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then print system info.
// ---------------------------------------------------------------------------

function main(): void {
  let result;

  try {
    const parser = new Parser(SPEC_FILE, process.argv);
    result = parser.parse();
  } catch (err: unknown) {
    if (err && typeof err === "object" && "errors" in err) {
      const errors = (err as { errors: Array<{ message: string }> }).errors;
      for (const error of errors) {
        process.stderr.write(`uname: ${error.message}\n`);
      }
      process.exit(1);
    }
    throw err;
  }

  if ("text" in result) {
    process.stdout.write(result.text + "\n");
    process.exit(0);
  }

  if ("version" in result && !("flags" in result)) {
    process.stdout.write(result.version + "\n");
    process.exit(0);
  }

  const flags = (result as { flags: Record<string, unknown> }).flags;
  const all = !!flags["all"];

  // Determine which fields to show. If no flag is set, default to -s.
  const anyFlagSet =
    all ||
    !!flags["kernel_name"] ||
    !!flags["nodename"] ||
    !!flags["kernel_release"] ||
    !!flags["kernel_version"] ||
    !!flags["machine"] ||
    !!flags["processor"] ||
    !!flags["hardware_platform"] ||
    !!flags["operating_system"];

  const showFlags = {
    kernelName: all || !!flags["kernel_name"] || !anyFlagSet,
    nodename: all || !!flags["nodename"],
    kernelRelease: all || !!flags["kernel_release"],
    kernelVersion: all || !!flags["kernel_version"],
    machine: all || !!flags["machine"],
    processor: all || !!flags["processor"],
    hardwarePlatform: all || !!flags["hardware_platform"],
    operatingSystem: all || !!flags["operating_system"],
  };

  const info = getSystemInfo();
  process.stdout.write(formatUname(info, showFlags) + "\n");
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

if (!process.env.VITEST) {
  main();
}
