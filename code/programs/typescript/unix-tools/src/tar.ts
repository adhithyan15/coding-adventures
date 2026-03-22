/**
 * tar -- an archiving utility.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the POSIX `tar` utility in TypeScript.
 * It creates, extracts, and lists archive files in the standard tar format.
 *
 * === How the tar Format Works ===
 *
 * A tar archive is a sequence of 512-byte blocks. Each file in the
 * archive is represented by:
 *
 * 1. A **header block** (512 bytes) containing metadata:
 *    - File name (100 bytes, null-terminated)
 *    - File mode (8 bytes, octal ASCII)
 *    - Owner UID (8 bytes, octal ASCII)
 *    - Owner GID (8 bytes, octal ASCII)
 *    - File size (12 bytes, octal ASCII)
 *    - Modification time (12 bytes, octal ASCII, Unix timestamp)
 *    - Checksum (8 bytes, octal ASCII)
 *    - Type flag (1 byte: '0'=file, '5'=directory)
 *    - Link name (100 bytes)
 *    - ... plus other fields up to 512 bytes total
 *
 * 2. **Data blocks** containing the file contents, padded to a
 *    multiple of 512 bytes.
 *
 * The archive ends with two consecutive 512-byte blocks of zeros
 * (the "end-of-archive" marker).
 *
 * === Example: A Small Archive ===
 *
 *     Block 0:   Header for "hello.txt" (size=13, mode=0o644)
 *     Block 1:   "Hello, world!\n" + 499 bytes of padding
 *     Block 2:   Header for "subdir/" (type=directory)
 *     Block 3-4: Two zero blocks (end-of-archive)
 *
 * === Header Layout ===
 *
 *     Offset  Size  Field
 *     ------  ----  -----
 *     0       100   File name
 *     100     8     File mode (octal)
 *     108     8     Owner UID (octal)
 *     116     8     Group GID (octal)
 *     124     12    File size (octal)
 *     136     12    Modification time (octal, Unix epoch)
 *     148     8     Header checksum
 *     156     1     Type flag ('0'=file, '5'=directory)
 *     157     100   Link name
 *     257     6     Magic ("ustar")
 *     263     2     Version ("00")
 *     265     32    Owner name
 *     297     32    Group name
 *     329     8     Device major
 *     337     8     Device minor
 *     345     155   Filename prefix (for names > 100 chars)
 *     500     12    Padding (zeros)
 *
 * @module tar
 */

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

const __filename_tar = fileURLToPath(import.meta.url);
const __dirname_tar = path.dirname(__filename_tar);
const SPEC_FILE = path.resolve(__dirname_tar, "..", "tar.json");

// ---------------------------------------------------------------------------
// Constants: Tar format details.
// ---------------------------------------------------------------------------

/** Every block in a tar archive is exactly 512 bytes. */
const BLOCK_SIZE = 512;

/** The magic string for POSIX/ustar format. */
const USTAR_MAGIC = "ustar";

/** Type flag for regular files. */
const TYPE_FILE = "0";

/** Type flag for directories. */
const TYPE_DIRECTORY = "5";

// ---------------------------------------------------------------------------
// Types: Options that control tar's behavior.
// ---------------------------------------------------------------------------

/**
 * Configuration options for tar operations.
 *
 *     Flag   Option      Meaning
 *     ----   ------      -------
 *     -c     create      Create a new archive
 *     -x     extract     Extract files from an archive
 *     -t     list        List archive contents
 *     -f     file        Archive file name
 *     -v     verbose     List files processed
 *     -C     directory   Change to directory first
 */
export interface TarOptions {
  /** Operation mode. */
  operation: "create" | "extract" | "list";
  /** Archive file path (null for stdin/stdout). */
  archiveFile: string | null;
  /** Verbose output. */
  verbose: boolean;
  /** Change to this directory before operating. */
  directory: string | null;
  /** Files to include/extract. */
  files: string[];
  /** Preserve permissions on extraction. */
  preservePermissions: boolean;
  /** Don't overwrite existing files on extraction. */
  keepOldFiles: boolean;
  /** Strip this many leading path components on extraction. */
  stripComponents: number;
}

// ---------------------------------------------------------------------------
// Types: Tar header structure.
// ---------------------------------------------------------------------------

/**
 * Parsed tar header for a single entry.
 */
export interface TarHeader {
  /** File name (possibly with path). */
  name: string;
  /** File mode (permission bits). */
  mode: number;
  /** Owner UID. */
  uid: number;
  /** Group GID. */
  gid: number;
  /** File size in bytes. */
  size: number;
  /** Modification time (Unix timestamp). */
  mtime: number;
  /** Type: '0' for file, '5' for directory. */
  typeflag: string;
  /** Link target name (for symbolic links). */
  linkname: string;
}

// ---------------------------------------------------------------------------
// Business Logic: Write octal values into a buffer.
// ---------------------------------------------------------------------------

/**
 * Write an octal ASCII value into a buffer at the specified offset.
 *
 * Tar headers store numeric values as ASCII octal strings, padded
 * with leading zeros and terminated with a space or null.
 *
 *     writeOctal(buf, 100, 8, 0o755)
 *     => writes "0000755\0" at offset 100
 *
 * @param buf     The buffer to write into.
 * @param offset  Starting position in the buffer.
 * @param length  Total field length in bytes.
 * @param value   The numeric value to write.
 */
function writeOctal(buf: Buffer, offset: number, length: number, value: number): void {
  const str = value.toString(8).padStart(length - 1, "0");
  buf.write(str, offset, length - 1, "ascii");
  buf[offset + length - 1] = 0; // null terminator
}

/**
 * Read an octal ASCII value from a buffer.
 *
 * @param buf     The buffer to read from.
 * @param offset  Starting position.
 * @param length  Field length.
 * @returns       The parsed numeric value.
 */
function readOctal(buf: Buffer, offset: number, length: number): number {
  const str = buf.subarray(offset, offset + length).toString("ascii").replace(/\0/g, "").trim();
  if (str.length === 0) return 0;
  return parseInt(str, 8);
}

/**
 * Read a null-terminated ASCII string from a buffer.
 *
 * @param buf     The buffer to read from.
 * @param offset  Starting position.
 * @param length  Maximum field length.
 * @returns       The parsed string.
 */
function readString(buf: Buffer, offset: number, length: number): string {
  const raw = buf.subarray(offset, offset + length);
  const nullIdx = raw.indexOf(0);
  if (nullIdx === -1) return raw.toString("ascii");
  return raw.subarray(0, nullIdx).toString("ascii");
}

// ---------------------------------------------------------------------------
// Business Logic: Create a tar header block.
// ---------------------------------------------------------------------------

/**
 * Create a 512-byte tar header block for a file or directory.
 *
 * The header follows the POSIX/ustar format. The checksum is
 * calculated by treating the checksum field itself as spaces,
 * summing all bytes, and storing the result as an octal value.
 *
 * @param header  The header metadata to encode.
 * @returns       A 512-byte Buffer containing the header.
 *
 * @example
 * ```ts
 * const hdr = createHeaderBlock({
 *   name: "hello.txt",
 *   mode: 0o644,
 *   uid: 1000,
 *   gid: 1000,
 *   size: 13,
 *   mtime: Math.floor(Date.now() / 1000),
 *   typeflag: "0",
 *   linkname: "",
 * });
 * // hdr is a 512-byte buffer
 * ```
 */
export function createHeaderBlock(header: TarHeader): Buffer {
  const buf = Buffer.alloc(BLOCK_SIZE, 0);

  // --- Write the header fields -----------------------------------------

  // Name (0-99, 100 bytes).
  buf.write(header.name, 0, Math.min(header.name.length, 100), "ascii");

  // Mode (100-107, 8 bytes).
  writeOctal(buf, 100, 8, header.mode);

  // UID (108-115, 8 bytes).
  writeOctal(buf, 108, 8, header.uid);

  // GID (116-123, 8 bytes).
  writeOctal(buf, 116, 8, header.gid);

  // Size (124-135, 12 bytes).
  writeOctal(buf, 124, 12, header.size);

  // Mtime (136-147, 12 bytes).
  writeOctal(buf, 136, 12, header.mtime);

  // Leave checksum as spaces for now (148-155, 8 bytes).
  buf.fill(0x20, 148, 156); // spaces

  // Typeflag (156, 1 byte).
  buf.write(header.typeflag || "0", 156, 1, "ascii");

  // Linkname (157-256, 100 bytes).
  if (header.linkname) {
    buf.write(header.linkname, 157, Math.min(header.linkname.length, 100), "ascii");
  }

  // Magic (257-262, 6 bytes): "ustar\0".
  buf.write(USTAR_MAGIC, 257, 5, "ascii");
  buf[262] = 0;

  // Version (263-264, 2 bytes): "00".
  buf.write("00", 263, 2, "ascii");

  // --- Calculate and write the checksum --------------------------------
  // The checksum is the sum of all bytes in the header, with the
  // checksum field itself treated as all spaces (0x20).

  let checksum = 0;
  for (let i = 0; i < BLOCK_SIZE; i++) {
    checksum += buf[i];
  }

  // Write the checksum as 6 octal digits + null + space.
  const checksumStr = checksum.toString(8).padStart(6, "0");
  buf.write(checksumStr, 148, 6, "ascii");
  buf[154] = 0;
  buf[155] = 0x20;

  return buf;
}

// ---------------------------------------------------------------------------
// Business Logic: Parse a tar header block.
// ---------------------------------------------------------------------------

/**
 * Parse a 512-byte tar header block into a TarHeader structure.
 *
 * Returns null if the block is all zeros (end-of-archive marker).
 *
 * @param buf  A 512-byte buffer containing the header.
 * @returns    The parsed header, or null for end-of-archive.
 */
export function parseHeaderBlock(buf: Buffer): TarHeader | null {
  // Check for end-of-archive (all zeros).
  let allZeros = true;
  for (let i = 0; i < BLOCK_SIZE; i++) {
    if (buf[i] !== 0) {
      allZeros = false;
      break;
    }
  }
  if (allZeros) return null;

  return {
    name: readString(buf, 0, 100),
    mode: readOctal(buf, 100, 8),
    uid: readOctal(buf, 108, 8),
    gid: readOctal(buf, 116, 8),
    size: readOctal(buf, 124, 12),
    mtime: readOctal(buf, 136, 12),
    typeflag: readString(buf, 156, 1) || "0",
    linkname: readString(buf, 157, 100),
  };
}

// ---------------------------------------------------------------------------
// Business Logic: Create a tar archive.
// ---------------------------------------------------------------------------

/**
 * Create a tar archive from a list of file paths.
 *
 * This function walks the list of files and directories, creating
 * header blocks and data blocks for each one. Directories are
 * recursively included.
 *
 * The archive format:
 *     [header1][data1...][header2][data2...][zero][zero]
 *
 * @param files     List of file/directory paths to archive.
 * @param baseDir   Base directory to resolve relative paths from.
 * @param verbose   Whether to return file names for verbose output.
 * @returns         An object with the archive buffer and list of archived names.
 *
 * @example
 * ```ts
 * const result = createArchive(["hello.txt", "subdir"], "/tmp");
 * fs.writeFileSync("archive.tar", result.buffer);
 * ```
 */
export function createArchive(
  files: string[],
  baseDir: string,
  verbose: boolean = false
): { buffer: Buffer; entries: string[] } {
  const blocks: Buffer[] = [];
  const entries: string[] = [];

  function addFile(filePath: string, archiveName: string): void {
    const fullPath = path.resolve(baseDir, filePath);
    const stat = fs.statSync(fullPath);

    if (stat.isDirectory()) {
      // Add directory entry.
      const dirName = archiveName.endsWith("/") ? archiveName : archiveName + "/";
      const header: TarHeader = {
        name: dirName,
        mode: stat.mode & 0o7777,
        uid: stat.uid,
        gid: stat.gid,
        size: 0,
        mtime: Math.floor(stat.mtimeMs / 1000),
        typeflag: TYPE_DIRECTORY,
        linkname: "",
      };

      blocks.push(createHeaderBlock(header));
      entries.push(dirName);

      // Recurse into directory contents.
      const contents = fs.readdirSync(fullPath).sort();
      for (const child of contents) {
        addFile(path.join(filePath, child), dirName + child);
      }
    } else if (stat.isFile()) {
      // Add file entry.
      const content = fs.readFileSync(fullPath);

      const header: TarHeader = {
        name: archiveName,
        mode: stat.mode & 0o7777,
        uid: stat.uid,
        gid: stat.gid,
        size: content.length,
        mtime: Math.floor(stat.mtimeMs / 1000),
        typeflag: TYPE_FILE,
        linkname: "",
      };

      blocks.push(createHeaderBlock(header));

      // Add data blocks, padded to 512-byte boundary.
      if (content.length > 0) {
        const paddedSize = Math.ceil(content.length / BLOCK_SIZE) * BLOCK_SIZE;
        const paddedContent = Buffer.alloc(paddedSize, 0);
        content.copy(paddedContent);
        blocks.push(paddedContent);
      }

      entries.push(archiveName);
    }
  }

  for (const file of files) {
    addFile(file, file);
  }

  // Add two zero blocks as end-of-archive marker.
  blocks.push(Buffer.alloc(BLOCK_SIZE, 0));
  blocks.push(Buffer.alloc(BLOCK_SIZE, 0));

  return {
    buffer: Buffer.concat(blocks),
    entries,
  };
}

// ---------------------------------------------------------------------------
// Business Logic: List archive contents.
// ---------------------------------------------------------------------------

/**
 * List the entries in a tar archive.
 *
 * Reads through the archive headers without extracting any files.
 *
 * @param archiveData  The raw archive buffer.
 * @returns            Array of TarHeader objects for each entry.
 *
 * @example
 * ```ts
 * const data = fs.readFileSync("archive.tar");
 * const entries = listArchive(data);
 * for (const entry of entries) {
 *   console.log(entry.name);
 * }
 * ```
 */
export function listArchive(archiveData: Buffer): TarHeader[] {
  const entries: TarHeader[] = [];
  let offset = 0;

  while (offset + BLOCK_SIZE <= archiveData.length) {
    const headerBuf = archiveData.subarray(offset, offset + BLOCK_SIZE);
    const header = parseHeaderBlock(headerBuf);

    if (!header) break; // End-of-archive.

    entries.push(header);
    offset += BLOCK_SIZE;

    // Skip over data blocks.
    if (header.size > 0) {
      const dataBlocks = Math.ceil(header.size / BLOCK_SIZE);
      offset += dataBlocks * BLOCK_SIZE;
    }
  }

  return entries;
}

// ---------------------------------------------------------------------------
// Business Logic: Extract archive contents.
// ---------------------------------------------------------------------------

/**
 * Strip leading path components from a file name.
 *
 *     stripComponents("a/b/c/file.txt", 2) => "c/file.txt"
 *     stripComponents("file.txt", 1)       => ""  (stripped away entirely)
 *
 * @param name   The original file name.
 * @param count  Number of leading components to strip.
 * @returns      The stripped name.
 */
export function stripPathComponents(name: string, count: number): string {
  if (count <= 0) return name;
  const parts = name.split("/").filter(p => p.length > 0);
  if (count >= parts.length) return "";
  return parts.slice(count).join("/");
}

/**
 * Extract files from a tar archive.
 *
 * Reads through the archive, creating directories and writing files
 * to the specified output directory.
 *
 * @param archiveData   The raw archive buffer.
 * @param outputDir     Directory to extract files into.
 * @param opts          Extraction options.
 * @returns             Array of extracted file names.
 *
 * @example
 * ```ts
 * const data = fs.readFileSync("archive.tar");
 * const extracted = extractArchive(data, "/tmp/output", {
 *   preservePermissions: false,
 *   keepOldFiles: false,
 *   stripComponents: 0,
 *   verbose: false,
 *   filterFiles: [],
 * });
 * ```
 */
export function extractArchive(
  archiveData: Buffer,
  outputDir: string,
  opts: {
    preservePermissions: boolean;
    keepOldFiles: boolean;
    stripComponents: number;
    verbose: boolean;
    filterFiles: string[];
  }
): string[] {
  const extracted: string[] = [];
  let offset = 0;

  while (offset + BLOCK_SIZE <= archiveData.length) {
    const headerBuf = archiveData.subarray(offset, offset + BLOCK_SIZE);
    const header = parseHeaderBlock(headerBuf);

    if (!header) break; // End-of-archive.

    offset += BLOCK_SIZE;

    // Read data blocks.
    let data: Buffer | null = null;
    if (header.size > 0) {
      data = archiveData.subarray(offset, offset + header.size);
      const dataBlocks = Math.ceil(header.size / BLOCK_SIZE);
      offset += dataBlocks * BLOCK_SIZE;
    }

    // Apply strip-components.
    let name = header.name;
    if (opts.stripComponents > 0) {
      name = stripPathComponents(name, opts.stripComponents);
      if (name === "") continue; // Name was stripped away entirely.
    }

    // Apply file filter (if specific files were requested).
    if (opts.filterFiles.length > 0) {
      const matches = opts.filterFiles.some(f =>
        name === f || name.startsWith(f + "/") || f === name.replace(/\/$/, "")
      );
      if (!matches) continue;
    }

    const fullPath = path.join(outputDir, name);

    // Create the file or directory.
    if (header.typeflag === TYPE_DIRECTORY || name.endsWith("/")) {
      fs.mkdirSync(fullPath, { recursive: true });
      if (opts.preservePermissions) {
        try {
          fs.chmodSync(fullPath, header.mode);
        } catch {
          // Ignore permission errors.
        }
      }
    } else {
      // Regular file.
      if (opts.keepOldFiles && fs.existsSync(fullPath)) {
        // Skip existing files.
        extracted.push(name);
        continue;
      }

      // Ensure parent directory exists.
      const parentDir = path.dirname(fullPath);
      fs.mkdirSync(parentDir, { recursive: true });

      fs.writeFileSync(fullPath, data || Buffer.alloc(0));

      if (opts.preservePermissions) {
        try {
          fs.chmodSync(fullPath, header.mode);
        } catch {
          // Ignore permission errors.
        }
      }
    }

    extracted.push(name);
  }

  return extracted;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then run tar.
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
        process.stderr.write(`tar: ${error.message}\n`);
      }
      process.exit(2);
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

  const flags = result.flags || {};
  const args = result.args || {};

  // Determine operation.
  let operation: "create" | "extract" | "list";
  if (flags.create) operation = "create";
  else if (flags.extract) operation = "extract";
  else if (flags.list) operation = "list";
  else {
    process.stderr.write("tar: You must specify one of -c, -x, or -t\n");
    process.exit(2);
    return;
  }

  const archiveFile = (flags.file as string) || null;
  const directory = (flags.directory as string) || null;
  const verbose = flags.verbose || false;
  const files: string[] = Array.isArray(args.files) ? args.files : (args.files ? [args.files] : []);

  const baseDir = directory || process.cwd();

  try {
    switch (operation) {
      case "create": {
        const result = createArchive(files, baseDir, verbose as boolean);
        if (archiveFile) {
          fs.writeFileSync(archiveFile, result.buffer);
        } else {
          process.stdout.write(result.buffer);
        }
        if (verbose) {
          for (const entry of result.entries) {
            process.stderr.write(entry + "\n");
          }
        }
        break;
      }

      case "list": {
        const data = archiveFile
          ? fs.readFileSync(archiveFile)
          : fs.readFileSync("/dev/stdin");
        const entries = listArchive(data);
        for (const entry of entries) {
          if (verbose) {
            const modeStr = (entry.mode & 0o7777).toString(8).padStart(4, "0");
            const sizeStr = entry.size.toString().padStart(7, " ");
            process.stdout.write(`${modeStr} ${entry.uid}/${entry.gid} ${sizeStr} ${entry.name}\n`);
          } else {
            process.stdout.write(entry.name + "\n");
          }
        }
        break;
      }

      case "extract": {
        const data = archiveFile
          ? fs.readFileSync(archiveFile)
          : fs.readFileSync("/dev/stdin");
        const outputDir = directory || process.cwd();
        const extracted = extractArchive(data, outputDir, {
          preservePermissions: flags.preserve_permissions || false,
          keepOldFiles: flags.keep_old_files || false,
          stripComponents: (flags.strip_components as number) || 0,
          verbose: verbose as boolean,
          filterFiles: files,
        });
        if (verbose) {
          for (const name of extracted) {
            process.stderr.write(name + "\n");
          }
        }
        break;
      }
    }
  } catch (err: unknown) {
    if (err instanceof Error) {
      process.stderr.write(`tar: ${err.message}\n`);
    }
    process.exit(2);
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

if (!process.env.VITEST) {
  main();
}
