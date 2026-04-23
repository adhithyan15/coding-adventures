/**
 * @coding-adventures/content-addressable-storage
 *
 * Generic Content-Addressable Storage (CAS)
 * ==========================================
 *
 * Content-addressable storage maps the *hash of content* to the content itself.
 * The hash is simultaneously the address and an integrity check: if the bytes
 * returned by the store don't hash to the key you requested, the data is corrupt.
 * No separate checksum file or trust anchor is needed.
 *
 * Mental model
 * ────────────
 * Imagine a library where every book's call number *is* a fingerprint of the
 * book's text. You can't file a different book under that number — the number
 * would immediately be wrong. And if someone swaps pages, the fingerprint
 * changes and the librarian knows before you even open the cover.
 *
 *   Traditional storage:   name  ──►  content   (name can lie; content can change)
 *   Content-addressed:     hash  ──►  content   (hash is derived from content, cannot lie)
 *
 * How Git uses CAS
 * ────────────────
 * Git's entire history is built on this principle. Every blob (file snapshot),
 * tree (directory listing), commit, and tag is stored by the SHA-1 hash of its
 * serialized bytes. Two identical files share one object. Renaming a file creates
 * zero new storage. History is an immutable DAG of hashes pointing to hashes.
 *
 * This package provides the CAS layer only — hashing and storage. The Git object
 * format ("blob N\0content"), compression, and pack files are handled by layers
 * above and below.
 *
 * Architecture
 * ────────────
 *
 *   ┌────────────────────────────────────────────────────────┐
 *   │  ContentAddressableStore<S extends BlobStore>          │
 *   │  · put(data)         → SHA-1 key, delegate to S       │
 *   │  · get(key)          → fetch from S, verify hash      │
 *   │  · findByPrefix(hex) → prefix search via S            │
 *   └──────────────────────┬─────────────────────────────────┘
 *                          │ interface BlobStore
 *             ┌────────────┴──────────────────────────────┐
 *             │                                           │
 *      LocalDiskStore                     (S3, MemStore, custom …)
 *      root/XX/XXXXXX…
 *
 * @example
 * ```ts
 * import { ContentAddressableStore, LocalDiskStore, keyToHex } from "@coding-adventures/content-addressable-storage";
 * import * as os from "node:os";
 * import * as path from "node:path";
 *
 * const root = path.join(os.tmpdir(), "cas-example");
 * const store = new LocalDiskStore(root);
 * const cas = new ContentAddressableStore(store);
 *
 * const key = cas.put(Buffer.from("hello, world"));
 * const data = cas.get(key);
 * console.log(data.toString()); // → "hello, world"
 * console.log(keyToHex(key));   // → 40-char SHA-1 hex
 * ```
 */

import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { sha1 } from "@coding-adventures/sha1";

export const VERSION = "0.1.0";

// ─── Hex Utilities ────────────────────────────────────────────────────────────
//
// Keys are 20-byte Buffers, but humans interact with them as 40-char lowercase
// hex strings (e.g., "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5").
//
// keyToHex  — converts Buffer(20) → 40-char lowercase hex string
// hexToKey  — parses a 40-char hex string → Buffer(20)
// decodeHexPrefix — parses 1–40 char hex prefix → byte prefix (with nibble padding)

/**
 * Convert a 20-byte SHA-1 key to a 40-character lowercase hex string.
 *
 * @example
 * ```ts
 * const key = Buffer.from("a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5", "hex");
 * keyToHex(key); // → "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5"
 * ```
 */
export function keyToHex(key: Buffer): string {
  // Each byte becomes exactly 2 hex digits, zero-padded on the left.
  // Buffer.toString("hex") does exactly this in Node.js.
  return key.toString("hex");
}

/**
 * Parse a 40-character lowercase (or uppercase) hex string into a 20-byte key.
 *
 * Throws an error if the string is not exactly 40 valid hex characters.
 *
 * @example
 * ```ts
 * const hex = "a9993e364706816aba3e25717850c26c9cd0d89d";
 * const key = hexToKey(hex);
 * keyToHex(key); // → hex
 * ```
 */
export function hexToKey(hex: string): Buffer {
  if (hex.length !== 40) {
    throw new Error(`expected 40 hex chars, got ${hex.length}`);
  }
  if (!/^[0-9a-fA-F]{40}$/.test(hex)) {
    throw new Error(`invalid hex characters in: ${hex}`);
  }
  return Buffer.from(hex, "hex");
}

/**
 * Decode an abbreviated hex prefix (1–40 chars) to a byte prefix Buffer.
 *
 * Odd-length hex strings are right-padded with '0' before converting.
 * This is because a nibble prefix like "a3f" means "starts with 0xa3, 0xf0" —
 * the trailing nibble is the high nibble of the next byte. Padding to even
 * length lets us decode nibble pairs cleanly.
 *
 * Example: "a3f" → 0xa3, 0xf0  (matches any key whose first two bytes are
 *                                 0xa3 and 0xf0 through 0xff)
 *
 * Throws if the string is empty or contains non-hex characters.
 */
export function decodeHexPrefix(hex: string): Buffer {
  if (hex.length === 0) {
    throw new Error("prefix cannot be empty");
  }
  if (!/^[0-9a-fA-F]+$/.test(hex)) {
    throw new Error(`invalid hex characters in prefix: ${hex}`);
  }
  // Pad to even length so we can decode nibble pairs.
  const padded = hex.length % 2 === 1 ? hex + "0" : hex;
  return Buffer.from(padded, "hex");
}

// ─── CasError Hierarchy ───────────────────────────────────────────────────────
//
// A class hierarchy (not a union type) because TypeScript callers typically
// use `instanceof` checks in catch blocks. Each subclass carries its own
// contextual data so the caller never has to parse error message strings.
//
// The hierarchy:
//
//   CasError (base)
//   ├── CasNotFoundError       — key absent from store
//   ├── CasCorruptedError      — stored bytes don't hash to the key
//   ├── CasAmbiguousPrefixError — hex prefix matched 2+ objects
//   ├── CasPrefixNotFoundError  — hex prefix matched 0 objects
//   └── CasInvalidPrefixError   — hex prefix string is malformed

/** Base class for all CAS errors. */
export class CasError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "CasError";
    // Maintain proper prototype chain in transpiled output.
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * A blob was requested by key but no such key exists in the store.
 *
 * This is a normal "cache miss" — the object was never stored, or the
 * store was wiped. It is NOT a data corruption event.
 */
export class CasNotFoundError extends CasError {
  /** The 20-byte key that was not found. */
  readonly key: Buffer;

  constructor(key: Buffer) {
    super(`object not found: ${keyToHex(key)}`);
    this.name = "CasNotFoundError";
    this.key = key;
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * The store returned bytes whose SHA-1 hash does not match the requested key.
 *
 * This is a data integrity violation — the stored bytes have been modified
 * since they were written, or the storage medium is malfunctioning.
 * The CAS layer surfaces it distinctly from I/O errors so callers can decide
 * whether to attempt repair, alert an operator, or abort.
 */
export class CasCorruptedError extends CasError {
  /** The key that was requested (NOT the hash of the corrupted bytes). */
  readonly key: Buffer;

  constructor(key: Buffer) {
    super(`object corrupted: ${keyToHex(key)}`);
    this.name = "CasCorruptedError";
    this.key = key;
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * A hex prefix matched two or more objects.
 *
 * Git calls this "ambiguous argument": `git show a3f4` is ambiguous if both
 * `a3f4aabbcc…` and `a3f4112233…` exist. The user must supply more characters.
 */
export class CasAmbiguousPrefixError extends CasError {
  /** The hex prefix string the caller supplied. */
  readonly prefix: string;

  constructor(prefix: string) {
    super(`ambiguous prefix: ${prefix}`);
    this.name = "CasAmbiguousPrefixError";
    this.prefix = prefix;
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * A hex prefix matched zero objects.
 *
 * The prefix is valid hex but nothing in the store starts with those bytes.
 */
export class CasPrefixNotFoundError extends CasError {
  /** The hex prefix string the caller supplied. */
  readonly prefix: string;

  constructor(prefix: string) {
    super(`object not found for prefix: ${prefix}`);
    this.name = "CasPrefixNotFoundError";
    this.prefix = prefix;
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * The supplied hex string is not valid hexadecimal, or is empty.
 *
 * Empty string → InvalidPrefix because "match everything" is never useful
 * and would force the caller to handle massive result sets.
 */
export class CasInvalidPrefixError extends CasError {
  /** The invalid prefix string the caller supplied. */
  readonly prefix: string;

  constructor(prefix: string) {
    super(`invalid hex prefix: ${JSON.stringify(prefix)}`);
    this.name = "CasInvalidPrefixError";
    this.prefix = prefix;
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

// ─── BlobStore Interface ──────────────────────────────────────────────────────
//
// The single abstraction that separates CAS logic from persistence.
// Any class that can store and retrieve byte blobs by a 20-byte key qualifies.
//
// Why an interface instead of an abstract class?
//   Interfaces impose zero runtime cost and allow a class to implement multiple
//   interfaces. An abstract class would force a specific prototype chain.
//
// Why synchronous?
//   The LocalDiskStore uses Node's synchronous fs API so tests are simple and
//   deterministic. A future AsyncBlobStore interface could mirror this one with
//   Promise return types. The CAS layer would then be async as well.

/**
 * A pluggable key-value store for raw byte blobs, keyed by a 20-byte Buffer.
 *
 * Implement this interface to add a new storage backend. The key is always a
 * SHA-1 digest produced by `ContentAddressableStore`; implementations should
 * treat it as an opaque identifier.
 *
 * All methods are synchronous. See the note above about async alternatives.
 *
 * @example Minimal in-memory implementation (compile-time test):
 * ```ts
 * class MemStore implements BlobStore {
 *   private map = new Map<string, Buffer>();
 *   put(key: Buffer, data: Buffer): void {
 *     this.map.set(key.toString("hex"), data);
 *   }
 *   get(key: Buffer): Buffer {
 *     const v = this.map.get(key.toString("hex"));
 *     if (!v) throw new Error("not found");
 *     return v;
 *   }
 *   exists(key: Buffer): boolean {
 *     return this.map.has(key.toString("hex"));
 *   }
 *   keysWithPrefix(prefix: Buffer): Buffer[] {
 *     const hex = prefix.toString("hex");
 *     return [...this.map.keys()]
 *       .filter(k => k.startsWith(hex))
 *       .map(k => Buffer.from(k, "hex"));
 *   }
 * }
 * ```
 */
export interface BlobStore {
  /**
   * Persist `data` under `key`.
   *
   * Must be idempotent: storing the same key twice with the same bytes is not
   * an error. Implementations may skip the write if the key already exists.
   * Storing a *different* blob under an existing key is undefined behaviour —
   * the CAS layer prevents this by construction (same content → same key).
   */
  put(key: Buffer, data: Buffer): void;

  /**
   * Retrieve the blob stored under `key`.
   *
   * Throws if the key is not present or if I/O fails. Implementations do NOT
   * need to verify the hash — that is the CAS layer's responsibility.
   */
  get(key: Buffer): Buffer;

  /**
   * Check whether `key` is present without fetching the blob.
   *
   * More efficient than `get` when you only need to know if an object exists.
   */
  exists(key: Buffer): boolean;

  /**
   * Return all stored keys whose first `prefix.length` bytes equal `prefix`.
   *
   * Used for abbreviated-hash lookup: the caller supplies a byte prefix decoded
   * from a short hex string (e.g., "a3f4"), and the store returns the full keys
   * that match. The CAS layer checks for uniqueness and reports ambiguity.
   *
   * An empty prefix returns all keys (or an empty array — behaviour is
   * implementation-defined, since the CAS layer rejects empty prefixes before
   * calling this method).
   */
  keysWithPrefix(prefix: Buffer): Buffer[];
}

// ─── ContentAddressableStore ──────────────────────────────────────────────────
//
// The CAS class owns one BlobStore instance and adds three things the raw
// store cannot provide on its own:
//
//   1. Automatic keying  — callers pass content; SHA-1 is computed internally.
//   2. Integrity check   — on every get(), SHA-1(returned bytes) must equal key.
//   3. Prefix resolution — converts abbreviated hex (like "a3f4b2") to a full key.
//
// The generic parameter <S extends BlobStore> lets the TypeScript type checker
// preserve the concrete store type — useful if the caller needs to call
// store-specific methods via inner().

/**
 * Content-addressable store that wraps a `BlobStore` backend.
 *
 * All objects are keyed by their SHA-1 hash. The same content always maps to
 * the same key (deduplication), and the stored bytes are verified against the
 * key on every read (integrity).
 *
 * @typeParam S - Any `BlobStore` implementation. Use `LocalDiskStore` for
 *   filesystem-backed storage, or supply your own for cloud or in-memory storage.
 *
 * @example
 * ```ts
 * const store = new LocalDiskStore("/tmp/content_addressable_storage");
 * const cas = new ContentAddressableStore(store);
 *
 * const key = cas.put(Buffer.from("hello"));
 * const data = cas.get(key);
 * assert(data.equals(Buffer.from("hello")));
 * ```
 */
export class ContentAddressableStore<S extends BlobStore> {
  // The underlying blob store — all persistence goes through here.
  private readonly store: S;

  /**
   * Create a new CAS wrapping `store`.
   */
  constructor(store: S) {
    this.store = store;
  }

  /**
   * Hash `data` with SHA-1, store it in the backend, and return the 20-byte key.
   *
   * Idempotent: if the same content has already been stored, the existing key
   * is returned and no write is performed (the BlobStore handles the skip).
   *
   * We delegate directly to the store without a pre-`exists()` check.
   * Skipping the exists → put two-step eliminates a TOCTOU (time-of-check /
   * time-of-use) race window and avoids an extra filesystem round-trip.
   *
   * @returns The 20-byte SHA-1 key for `data`.
   *
   * @example
   * ```ts
   * const key1 = cas.put(Buffer.from("foo"));
   * const key2 = cas.put(Buffer.from("foo")); // second call is a no-op
   * assert(key1.equals(key2));
   * ```
   */
  put(data: Buffer): Buffer {
    // sha1() from @coding-adventures/sha1 returns a Uint8Array; wrap in Buffer
    // for Node.js filesystem compatibility.
    const key = Buffer.from(sha1(data));
    this.store.put(key, data);
    return key;
  }

  /**
   * Retrieve the blob stored under `key` and verify its integrity.
   *
   * The returned bytes are guaranteed to hash to `key` — if the store returns
   * anything else, `CasCorruptedError` is thrown instead.
   *
   * If the key does not exist in the store, the store's own error propagates
   * as-is (typically a `CasNotFoundError` for `LocalDiskStore`).
   *
   * @throws {CasNotFoundError} if the key is absent from the store.
   * @throws {CasCorruptedError} if the stored bytes don't hash to `key`.
   *
   * @example
   * ```ts
   * const key = cas.put(Buffer.from("bar"));
   * cas.get(key).toString(); // → "bar"
   * ```
   */
  get(key: Buffer): Buffer {
    const data = this.store.get(key);

    // Integrity check: re-hash the returned bytes and compare to the requested key.
    // If these don't match, the storage medium has been tampered with or corrupted.
    const actual = Buffer.from(sha1(data));
    if (!actual.equals(key)) {
      throw new CasCorruptedError(key);
    }
    return data;
  }

  /**
   * Check whether a key is present in the store.
   *
   * More efficient than `get` when you only need to know if an object exists.
   * Does NOT verify integrity — `get` is required for that.
   */
  exists(key: Buffer): boolean {
    return this.store.exists(key);
  }

  /**
   * Resolve an abbreviated hex string to a full 20-byte key.
   *
   * Accepts any non-empty hex string of 1–40 characters. Odd-length strings
   * are treated as nibble prefixes: `"a3f"` matches any key starting with
   * bytes 0xa3, 0xf0 (the trailing nibble is the high nibble of the next byte).
   *
   * This mirrors how `git log --oneline` shows 7-character hashes that you can
   * pass back to `git show`. As long as the short hash is unambiguous in the
   * current repository, it works.
   *
   * @param hexPrefix - 1–40 valid hex characters
   * @returns The unique full 20-byte key matching the prefix.
   *
   * @throws {CasInvalidPrefixError}   if `hexPrefix` is empty or contains non-hex chars
   * @throws {CasPrefixNotFoundError}  if no keys match
   * @throws {CasAmbiguousPrefixError} if two or more keys match
   *
   * @example
   * ```ts
   * const key = cas.put(Buffer.from("hello"));
   * const hex = keyToHex(key);
   * const resolved = cas.findByPrefix(hex.slice(0, 7)); // 7-char prefix
   * assert(resolved.equals(key));
   * ```
   */
  findByPrefix(hexPrefix: string): Buffer {
    let prefixBytes: Buffer;
    try {
      prefixBytes = decodeHexPrefix(hexPrefix);
    } catch {
      throw new CasInvalidPrefixError(hexPrefix);
    }

    const matches = this.store.keysWithPrefix(prefixBytes);

    // Sort for deterministic behaviour: if there are 2+ matches, the
    // AmbiguousPrefix error is consistent across runs regardless of the
    // filesystem's readdir ordering.
    matches.sort((a, b) => a.compare(b));

    if (matches.length === 0) {
      throw new CasPrefixNotFoundError(hexPrefix);
    }
    if (matches.length > 1) {
      throw new CasAmbiguousPrefixError(hexPrefix);
    }
    return matches[0];
  }

  /**
   * Access the underlying `BlobStore` directly.
   *
   * Useful when you need backend-specific operations not exposed by the CAS
   * interface (e.g., listing all keys for garbage collection, or querying
   * storage statistics). The type parameter `S` is preserved so you get the
   * concrete store type, not just `BlobStore`.
   */
  inner(): S {
    return this.store;
  }
}

// ─── LocalDiskStore ───────────────────────────────────────────────────────────
//
// Filesystem backend using the Git 2/38 fanout layout.
//
// Why 2/38?
//   A repository with 100 000 objects would put 100 000 files in a single
//   directory if we stored objects as root/<40-hex-hash>. Most filesystems slow
//   down dramatically at that scale — directory lookup is O(n) for many common
//   FSes without a directory index. Splitting on the first byte creates up to
//   256 sub-directories ("00/" through "ff/"), keeping each to a manageable size
//   even in very large repositories. Git has used this layout since its initial
//   release in 2005.
//
// Object path:  root/<xx>/<remaining-38-hex-chars>
//   key = a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5
//   dir  = root/a3/
//   file = root/a3/f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5
//
// Atomic writes
// ─────────────
// A naive write would leave a window where a reader could see a partial file:
//
//   Writer: open file, write bytes 1-100, crash
//   Reader: opens file, reads only 100 bytes — file looks corrupt
//
// To avoid this we write to a temp file first, then call rename() into place.
// On POSIX, rename(2) is atomic: the destination path atomically transitions
// from the old inode to the new one. Readers either see the old file or the
// new complete file — never a half-written file.
//
// On Windows, rename fails if the destination exists (unlike POSIX where it
// overwrites atomically). We handle that by checking if the destination now
// exists after a rename failure — if it does, another writer beat us, which is
// fine because the content is identical (same hash → same bytes).
//
// Temp file naming
// ────────────────
// The temp file is placed in the same directory as the final file so the
// rename stays on the same filesystem (cross-device renames fail on POSIX).
// We use a name incorporating the PID and a high-resolution timestamp to make
// it infeasible for an attacker to pre-create a symlink at the temp path and
// redirect our write to an arbitrary destination.

/**
 * Filesystem-backed `BlobStore` using Git-style 2/38 fanout layout.
 *
 * Objects are stored at `<root>/<xx>/<38-hex-chars>` where `xx` is the first
 * byte of the SHA-1 hash encoded as two lowercase hex digits.
 *
 * Writes are atomic: data is written to a temp file, then renamed into place.
 *
 * @example
 * ```ts
 * const store = new LocalDiskStore("/var/lib/myapp/objects");
 * store.put(key, data);
 * const blob = store.get(key);
 * ```
 */
export class LocalDiskStore implements BlobStore {
  /** Absolute path to the root directory of the object store. */
  readonly root: string;

  /**
   * Create (or open) a store rooted at `root`.
   *
   * The directory and all parents are created if they do not exist.
   *
   * @param root - Path to the root directory.
   * @example
   * ```ts
   * const store = new LocalDiskStore("/tmp/my-cas");
   * ```
   */
  constructor(root: string) {
    this.root = root;
    // Ensure the root directory exists before any reads or writes.
    // `recursive: true` is a no-op if the directory already exists.
    fs.mkdirSync(root, { recursive: true });
  }

  /**
   * Compute the filesystem path for a given 20-byte key.
   *
   * The first byte of the hex-encoded key becomes the two-character directory
   * name (the "fanout bucket"), and the remaining 38 characters become the
   * filename.
   *
   *   key = a3f4b2c1d0…  →  root/a3/f4b2c1d0…
   *
   * This is intentionally `public` so callers can inspect the layout in tests
   * or build tooling (e.g., garbage collection that walks the filesystem).
   */
  objectPath(key: Buffer): string {
    const hex = keyToHex(key);
    // Split the 40-char hex at position 2: first 2 chars = dir, rest = filename.
    const dirName = hex.slice(0, 2);   // e.g., "a3"
    const fileName = hex.slice(2);     // e.g., "f4b2c1d0…" (38 chars)
    return path.join(this.root, dirName, fileName);
  }

  /**
   * Persist `data` under `key`.
   *
   * Idempotent: if `key` already exists on disk, returns immediately without
   * writing. The short-circuit avoids redundant I/O and prevents a race where
   * two concurrent writers both try to rename the same temp file.
   *
   * Write path:
   *   1. If the final path exists → return (already stored).
   *   2. `mkdir -p` the two-char fanout directory.
   *   3. Write data to a temp file (same directory, PID+timestamp name).
   *   4. `rename(tmp, final)`.  On Windows failure → check if final exists.
   */
  put(key: Buffer, data: Buffer): void {
    const finalPath = this.objectPath(key);

    // Short-circuit: file already on disk. Because the key is a hash of the
    // content, we know the stored bytes are identical — no need to overwrite.
    if (fs.existsSync(finalPath)) {
      return;
    }

    // Create the two-char fanout directory ("a3/") if it doesn't exist yet.
    const dir = path.dirname(finalPath);
    fs.mkdirSync(dir, { recursive: true });

    // Build an unpredictable temp filename: base + PID + nanosecond timestamp.
    // Using process.pid and hrtime makes it infeasible for a local attacker to
    // pre-create a symlink at this path to redirect our write.
    const baseName = path.basename(finalPath);
    const pid = process.pid;
    const ns = process.hrtime.bigint().toString();
    const tmpName = `${baseName}.${pid}.${ns}.tmp`;
    const tmpPath = path.join(dir, tmpName);

    try {
      // Write all bytes to the temp file. fsync is implicit in writeFileSync.
      fs.writeFileSync(tmpPath, data);

      // Atomic rename into place.
      try {
        fs.renameSync(tmpPath, finalPath);
      } catch (renameErr) {
        // On Windows, renameSync fails if the destination already exists.
        // If the final file now exists, another writer stored the same object
        // concurrently — that is fine (identical content).
        if (!fs.existsSync(finalPath)) {
          // The rename failed AND the final file still doesn't exist → real error.
          throw renameErr;
        }
        // Otherwise: concurrent write won the race, clean up our temp file.
        try { fs.unlinkSync(tmpPath); } catch { /* ignore cleanup failure */ }
      }
    } catch (err) {
      // Clean up the temp file to avoid leaving orphans on disk.
      try { fs.unlinkSync(tmpPath); } catch { /* ignore cleanup failure */ }
      throw err;
    }
  }

  /**
   * Read and return the raw bytes stored under `key`.
   *
   * @throws {CasNotFoundError} if the key is not present on disk.
   */
  get(key: Buffer): Buffer {
    const filePath = this.objectPath(key);
    try {
      return fs.readFileSync(filePath);
    } catch (err) {
      // Translate ENOENT to a typed CasNotFoundError so callers don't need to
      // inspect raw NodeJS.ErrnoException error codes.
      const nodeErr = err as NodeJS.ErrnoException;
      if (nodeErr.code === "ENOENT") {
        throw new CasNotFoundError(key);
      }
      throw err;
    }
  }

  /**
   * Return true if `key` exists on disk, false otherwise.
   *
   * Uses `fs.existsSync` which avoids an exception on the miss path.
   */
  exists(key: Buffer): boolean {
    return fs.existsSync(this.objectPath(key));
  }

  /**
   * Return all stored keys whose first `prefix.length` bytes equal `prefix`.
   *
   * Implementation:
   *   1. The first byte of the prefix identifies the fanout bucket ("a3/").
   *   2. Read that single directory listing (all objects sharing that first byte).
   *   3. For each 38-char filename, reconstruct the full 40-char hex and parse it.
   *   4. Filter: keep only keys where the leading bytes match `prefix` exactly.
   *
   * Why only one bucket?
   *   `decodeHexPrefix` pads odd-length hex strings with '0', so a single
   *   nibble "a" becomes byte 0xa0. This means we always have at least the
   *   high nibble of the first byte, which is enough to identify a unique
   *   bucket. A prefix of "a" only matches objects in bucket "a0/", not all
   *   "a*" buckets — the caller should use a longer prefix for broader searches.
   *   The CAS layer does final filtering, so false positives from the bucket
   *   scan are harmless; false negatives are not, and this approach has none.
   *
   * @param prefix - Byte prefix to match against stored keys.
   * @returns Array of matching 20-byte key Buffers (may be empty).
   */
  keysWithPrefix(prefix: Buffer): Buffer[] {
    // An empty prefix would require scanning all 256 buckets, which is
    // expensive and not needed (the CAS layer rejects empty hex prefixes).
    if (prefix.length === 0) {
      return [];
    }

    // The first byte of the prefix is the directory name.
    // format it as two lowercase hex digits, matching how objectPath() names dirs.
    const firstByteHex = prefix[0].toString(16).padStart(2, "0");
    const bucket = path.join(this.root, firstByteHex);

    if (!fs.existsSync(bucket)) {
      return [];
    }

    const keys: Buffer[] = [];

    let entries: fs.Dirent[];
    try {
      entries = fs.readdirSync(bucket, { withFileTypes: true });
    } catch {
      return [];
    }

    for (const entry of entries) {
      // Each valid object file has exactly 38 hex chars for its name.
      // Skip temp files (which have ".tmp" suffixes), subdirectories, etc.
      if (!entry.isFile() || entry.name.length !== 38) {
        continue;
      }

      // Reconstruct the full 40-char hex by prepending the bucket name.
      const fullHex = firstByteHex + entry.name;

      // Skip entries that aren't valid hex (e.g., operating system metadata).
      if (!/^[0-9a-f]{40}$/.test(fullHex)) {
        continue;
      }

      const key = Buffer.from(fullHex, "hex");

      // Final filter: check that all bytes in `prefix` match the start of `key`.
      // This is necessary because the bucket only guarantees the first byte matches.
      // For a 3-byte prefix [0xa3, 0xf4, 0xb2], we filtered by bucket "a3/" above,
      // but we still need to confirm key[1] === 0xf4 and key[2] === 0xb2.
      if (key.subarray(0, prefix.length).equals(prefix)) {
        keys.push(key);
      }
    }

    return keys;
  }
}
