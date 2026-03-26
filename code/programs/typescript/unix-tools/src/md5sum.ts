/**
 * md5sum -- compute and check MD5 message digest.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `md5sum` utility in TypeScript.
 * It computes MD5 checksums for files and can verify previously computed
 * checksums.
 *
 * === How MD5 Works (Simplified) ===
 *
 * MD5 (Message Digest Algorithm 5) produces a 128-bit (16-byte) hash
 * value, conventionally displayed as a 32-character hexadecimal string.
 *
 * While MD5 is cryptographically broken (collisions can be crafted),
 * it's still widely used for:
 * - Verifying file integrity after transfer
 * - Detecting accidental corruption
 * - Checksumming in non-security contexts
 *
 * === Output Format ===
 *
 * For each file, md5sum prints:
 *
 *     d41d8cd98f00b204e9800998ecf8427e  filename
 *
 * The format is: 32-char hex hash, two spaces, filename.
 * In binary mode, the two spaces become a space and asterisk: " *".
 *
 * === Check Mode (-c) ===
 *
 * With `-c`, md5sum reads a file containing previously computed checksums
 * and verifies each one:
 *
 *     $ md5sum -c checksums.md5
 *     file1.txt: OK
 *     file2.txt: FAILED
 *
 * @module md5sum
 */

import * as crypto from "node:crypto";
import * as fs from "node:fs";
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
const SPEC_FILE = path.resolve(__dirname, "..", "md5sum.json");

// ---------------------------------------------------------------------------
// Business Logic: Compute MD5 hash.
// ---------------------------------------------------------------------------

/**
 * Compute the MD5 hash of a Buffer.
 *
 * We use Node.js's built-in `crypto` module, which delegates to
 * OpenSSL's MD5 implementation. The result is a 32-character
 * hexadecimal string.
 *
 * === Why Buffer Instead of String? ===
 *
 * We accept a Buffer because checksums operate on raw bytes, not
 * text. A string would need to be encoded first, and the encoding
 * choice affects the hash. By taking a Buffer, the caller controls
 * the encoding explicitly.
 *
 * @param data - The raw bytes to hash.
 * @returns A 32-character lowercase hexadecimal string.
 */
export function computeMd5(data: Buffer): string {
  return crypto.createHash("md5").update(data).digest("hex");
}

// ---------------------------------------------------------------------------
// Business Logic: Format a checksum line.
// ---------------------------------------------------------------------------

/**
 * Format a checksum result as a single output line.
 *
 * The format follows GNU md5sum conventions:
 * - Text mode:   "hash  filename"  (two spaces)
 * - Binary mode:  "hash *filename"  (space + asterisk)
 *
 * @param hash     - The hex digest string.
 * @param filename - The file path.
 * @param binary   - Whether binary mode is active.
 * @returns The formatted checksum line (without newline).
 */
export function formatChecksum(
  hash: string,
  filename: string,
  binary: boolean
): string {
  const separator = binary ? " *" : "  ";
  return hash + separator + filename;
}

// ---------------------------------------------------------------------------
// Business Logic: Parse a checksum line for verification.
// ---------------------------------------------------------------------------

/**
 * Parse a checksum line into its components.
 *
 * Expected format: "hash  filename" or "hash *filename".
 *
 * @param line - The checksum line to parse.
 * @returns An object with hash and filename, or null if malformed.
 */
export function parseChecksumLine(
  line: string
): { hash: string; filename: string; binary: boolean } | null {
  // Match: 32 hex chars, then either "  " or " *", then filename.
  const match = line.match(/^([a-fA-F0-9]{32})(  | \*)(.+)$/);
  if (!match) return null;

  return {
    hash: match[1].toLowerCase(),
    filename: match[3],
    binary: match[2] === " *",
  };
}

// ---------------------------------------------------------------------------
// Business Logic: Verify checksums from a file.
// ---------------------------------------------------------------------------

/**
 * Verify checksums listed in a check file.
 *
 * Each line of the check file should be in md5sum output format.
 * We recompute the hash for each referenced file and compare.
 *
 * @param checkContent - The content of the checksum file.
 * @param quiet        - Don't print OK for successful verifications.
 * @param statusOnly   - Don't print anything; just return success/failure.
 * @param strict       - Fail on improperly formatted lines.
 * @param warn         - Warn about improperly formatted lines.
 * @returns True if all checksums matched, false otherwise.
 */
export function verifyChecksums(
  checkContent: string,
  quiet: boolean,
  statusOnly: boolean,
  strict: boolean,
  warn: boolean
): boolean {
  const lines = checkContent.split("\n");
  let allOk = true;
  let badFormat = 0;

  for (const line of lines) {
    if (line.trim() === "") continue;

    const parsed = parseChecksumLine(line);
    if (!parsed) {
      badFormat++;
      if (warn && !statusOnly) {
        process.stderr.write(`md5sum: ${line}: improperly formatted MD5 checksum line\n`);
      }
      if (strict) allOk = false;
      continue;
    }

    try {
      const data = fs.readFileSync(parsed.filename);
      const computed = computeMd5(data);
      const match = computed === parsed.hash;

      if (!match) allOk = false;

      if (!statusOnly) {
        if (match && !quiet) {
          process.stdout.write(`${parsed.filename}: OK\n`);
        } else if (!match) {
          process.stdout.write(`${parsed.filename}: FAILED\n`);
        }
      }
    } catch {
      allOk = false;
      if (!statusOnly) {
        process.stderr.write(
          `md5sum: ${parsed.filename}: No such file or directory\n`
        );
      }
    }
  }

  return allOk;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then compute/check MD5.
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
        process.stderr.write(`md5sum: ${error.message}\n`);
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
  const args = (result as { arguments: Record<string, unknown> }).arguments;

  const checkMode = !!flags["check"];
  const binary = !!flags["binary"];
  const quiet = !!flags["quiet"];
  const statusOnly = !!flags["status"];
  const strict = !!flags["strict"];
  const warn = !!flags["warn"];
  const zeroTerminated = !!flags["zero"];
  const lineEnd = zeroTerminated ? "\0" : "\n";

  let files = args["files"] as string[] | string | undefined;
  if (!files) {
    files = ["-"];
  } else if (typeof files === "string") {
    files = [files];
  }

  if (checkMode) {
    // Verification mode.
    let allOk = true;
    for (const file of files) {
      let content: string;
      if (file === "-") {
        try {
          content = fs.readFileSync(0, "utf-8");
        } catch {
          continue;
        }
      } else {
        try {
          content = fs.readFileSync(file, "utf-8");
        } catch (err: unknown) {
          const message = err instanceof Error ? err.message : String(err);
          process.stderr.write(`md5sum: ${file}: ${message}\n`);
          process.exitCode = 1;
          continue;
        }
      }

      if (!verifyChecksums(content, quiet, statusOnly, strict, warn)) {
        allOk = false;
      }
    }

    if (!allOk) {
      process.exit(1);
    }
  } else {
    // Computation mode.
    for (const file of files) {
      let data: Buffer;
      let displayName = file;

      if (file === "-") {
        try {
          data = fs.readFileSync(0);
          displayName = "-";
        } catch {
          continue;
        }
      } else {
        try {
          data = fs.readFileSync(file);
        } catch (err: unknown) {
          const message = err instanceof Error ? err.message : String(err);
          process.stderr.write(`md5sum: ${file}: ${message}\n`);
          process.exitCode = 1;
          continue;
        }
      }

      const hash = computeMd5(data);
      process.stdout.write(formatChecksum(hash, displayName, binary) + lineEnd);
    }
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

if (!process.env.VITEST) {
  main();
}
