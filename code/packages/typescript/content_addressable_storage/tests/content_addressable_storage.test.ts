/**
 * Tests for @coding-adventures/content-addressable-storage
 *
 * Test strategy
 * ─────────────
 * We test the CAS package at three levels:
 *
 *   1. Unit: hex utilities (keyToHex, hexToKey, decodeHexPrefix).
 *   2. Integration: ContentAddressableStore + LocalDiskStore (the main path).
 *   3. Compile-time: BlobStore as an interface — we define a minimal MemStore
 *      to verify that any class satisfying the interface compiles and runs
 *      correctly.
 *
 * Filesystem isolation
 * ────────────────────
 * Each test gets its own temporary directory, created fresh and deleted after
 * the test. This avoids cross-test contamination and keeps the test suite
 * idempotent (re-runnable without manual cleanup).
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import {
  BlobStore,
  CasAmbiguousPrefixError,
  CasCorruptedError,
  CasInvalidPrefixError,
  CasNotFoundError,
  CasPrefixNotFoundError,
  ContentAddressableStore,
  LocalDiskStore,
  decodeHexPrefix,
  hexToKey,
  keyToHex,
} from "../src/index.js";

// ─── Helpers ─────────────────────────────────────────────────────────────────

/**
 * Create a fresh temporary directory for a test.
 * The directory is guaranteed not to exist before this call.
 * Returns the absolute path.
 */
function makeTmpDir(label: string): string {
  const dir = path.join(os.tmpdir(), `cas-ts-test-${label}-${process.pid}`);
  // Remove any leftovers from a previous test run (defensive).
  fs.rmSync(dir, { recursive: true, force: true });
  fs.mkdirSync(dir, { recursive: true });
  return dir;
}

/** Remove a temporary directory created by makeTmpDir. */
function cleanTmpDir(dir: string): void {
  fs.rmSync(dir, { recursive: true, force: true });
}

// ─── Hex Utilities ───────────────────────────────────────────────────────────

describe("keyToHex / hexToKey", () => {
  it("round-trips a known 20-byte key", () => {
    // The SHA-1 of "abc" is a well-known test vector (FIPS 180-4).
    const hex = "a9993e364706816aba3e25717850c26c9cd0d89d";
    const key = hexToKey(hex);
    expect(key).toHaveLength(20);
    expect(keyToHex(key)).toBe(hex);
  });

  it("round-trips all-zero key", () => {
    const hex = "0000000000000000000000000000000000000000";
    expect(keyToHex(hexToKey(hex))).toBe(hex);
  });

  it("round-trips all-ff key", () => {
    const hex = "ffffffffffffffffffffffffffffffffffffffff";
    expect(keyToHex(hexToKey(hex))).toBe(hex);
  });

  it("hexToKey accepts uppercase hex", () => {
    const lower = "a9993e364706816aba3e25717850c26c9cd0d89d";
    const upper = lower.toUpperCase();
    // Both should decode to the same bytes.
    expect(hexToKey(lower).equals(hexToKey(upper))).toBe(true);
  });

  it("hexToKey rejects a string shorter than 40 chars", () => {
    expect(() => hexToKey("a3f4")).toThrow("expected 40 hex chars");
  });

  it("hexToKey rejects a string longer than 40 chars", () => {
    const tooLong = "a".repeat(41);
    expect(() => hexToKey(tooLong)).toThrow("expected 40 hex chars");
  });

  it("hexToKey rejects non-hex characters", () => {
    const bad = "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6zz";
    expect(() => hexToKey(bad)).toThrow();
  });
});

// ─── decodeHexPrefix ─────────────────────────────────────────────────────────

describe("decodeHexPrefix", () => {
  it("decodes an even-length prefix", () => {
    // "a3f4" → 2 bytes [0xa3, 0xf4]
    const result = decodeHexPrefix("a3f4");
    expect(result).toEqual(Buffer.from([0xa3, 0xf4]));
  });

  it("right-pads odd-length prefix with nibble 0", () => {
    // "a3f" → pad to "a3f0" → 2 bytes [0xa3, 0xf0]
    // This means "starts with byte 0xa3 AND high nibble of next byte is 0xf"
    const result = decodeHexPrefix("a3f");
    expect(result).toEqual(Buffer.from([0xa3, 0xf0]));
  });

  it("decodes a single character (half a byte)", () => {
    // "a" → pad to "a0" → 1 byte [0xa0]
    const result = decodeHexPrefix("a");
    expect(result).toEqual(Buffer.from([0xa0]));
  });

  it("decodes a full 40-char hex string", () => {
    const hex = "a9993e364706816aba3e25717850c26c9cd0d89d";
    const result = decodeHexPrefix(hex);
    expect(result).toHaveLength(20);
    expect(result.toString("hex")).toBe(hex);
  });

  it("rejects an empty string", () => {
    expect(() => decodeHexPrefix("")).toThrow("cannot be empty");
  });

  it("rejects non-hex characters", () => {
    expect(() => decodeHexPrefix("a3g4")).toThrow("invalid hex");
  });

  it("rejects a string with spaces", () => {
    expect(() => decodeHexPrefix("a3 f4")).toThrow("invalid hex");
  });
});

// ─── CasError Class Hierarchy ─────────────────────────────────────────────────

describe("CasError subclasses", () => {
  const key = hexToKey("a9993e364706816aba3e25717850c26c9cd0d89d");

  it("CasNotFoundError instanceof chain", () => {
    const err = new CasNotFoundError(key);
    expect(err).toBeInstanceOf(CasNotFoundError);
    expect(err).toBeInstanceOf(Error);
    expect(err.name).toBe("CasNotFoundError");
    expect(err.message).toContain("a9993e");
    expect(err.key.equals(key)).toBe(true);
  });

  it("CasCorruptedError instanceof chain", () => {
    const err = new CasCorruptedError(key);
    expect(err).toBeInstanceOf(CasCorruptedError);
    expect(err).toBeInstanceOf(Error);
    expect(err.name).toBe("CasCorruptedError");
    expect(err.message).toContain("a9993e");
    expect(err.key.equals(key)).toBe(true);
  });

  it("CasAmbiguousPrefixError instanceof chain", () => {
    const err = new CasAmbiguousPrefixError("a3f4");
    expect(err).toBeInstanceOf(CasAmbiguousPrefixError);
    expect(err).toBeInstanceOf(Error);
    expect(err.name).toBe("CasAmbiguousPrefixError");
    expect(err.message).toContain("a3f4");
    expect(err.prefix).toBe("a3f4");
  });

  it("CasPrefixNotFoundError instanceof chain", () => {
    const err = new CasPrefixNotFoundError("a3f4");
    expect(err).toBeInstanceOf(CasPrefixNotFoundError);
    expect(err).toBeInstanceOf(Error);
    expect(err.name).toBe("CasPrefixNotFoundError");
    expect(err.prefix).toBe("a3f4");
  });

  it("CasInvalidPrefixError instanceof chain", () => {
    const err = new CasInvalidPrefixError("");
    expect(err).toBeInstanceOf(CasInvalidPrefixError);
    expect(err).toBeInstanceOf(Error);
    expect(err.name).toBe("CasInvalidPrefixError");
    expect(err.prefix).toBe("");
  });
});

// ─── BlobStore Interface: MemStore compile-time + runtime test ────────────────
//
// TypeScript interfaces only exist at compile time. To verify that the
// BlobStore interface is sound (any conforming implementation works with
// ContentAddressableStore), we define a minimal in-memory store here.
//
// If this file compiles and these tests pass, the interface is correct.

/**
 * Minimal in-memory BlobStore for testing the interface contract.
 *
 * This is NOT a production implementation — it has no atomic writes,
 * no fanout layout, and no persistence. Its only job is to satisfy the
 * BlobStore interface and let us test ContentAddressableStore in isolation
 * from the filesystem.
 */
class MemStore implements BlobStore {
  // Map from hex-encoded key to blob bytes.
  // We use hex strings as Map keys because Buffer equality is by reference,
  // not by value — two different Buffer objects with the same bytes would be
  // treated as different keys in a Map<Buffer, Buffer>.
  private readonly blobs = new Map<string, Buffer>();

  put(key: Buffer, data: Buffer): void {
    // Idempotent: overwriting with the same content is fine.
    this.blobs.set(key.toString("hex"), Buffer.from(data));
  }

  get(key: Buffer): Buffer {
    const value = this.blobs.get(key.toString("hex"));
    if (value === undefined) {
      throw new CasNotFoundError(key);
    }
    return Buffer.from(value);
  }

  exists(key: Buffer): boolean {
    return this.blobs.has(key.toString("hex"));
  }

  keysWithPrefix(prefix: Buffer): Buffer[] {
    const prefixHex = prefix.toString("hex");
    const results: Buffer[] = [];
    for (const hexKey of this.blobs.keys()) {
      // A key matches the prefix if its byte representation starts with
      // the prefix bytes. Comparing hex strings works because 2 hex chars = 1 byte,
      // so a hex prefix of length 2n corresponds to n prefix bytes.
      if (hexKey.startsWith(prefixHex)) {
        results.push(Buffer.from(hexKey, "hex"));
      }
    }
    return results;
  }

  /** Return the number of stored blobs (test helper). */
  size(): number {
    return this.blobs.size;
  }
}

describe("ContentAddressableStore + MemStore (interface compile-time test)", () => {
  let cas: ContentAddressableStore<MemStore>;
  let store: MemStore;

  beforeEach(() => {
    store = new MemStore();
    cas = new ContentAddressableStore(store);
  });

  it("round-trips a small blob", () => {
    const data = Buffer.from("hello, cas");
    const key = cas.put(data);
    expect(key).toHaveLength(20);
    expect(cas.get(key).equals(data)).toBe(true);
  });

  it("round-trips an empty blob", () => {
    // SHA-1("") = da39a3ee5e6b4b0d3255bfef95601890afd80709
    const data = Buffer.alloc(0);
    const key = cas.put(data);
    expect(cas.get(key).equals(data)).toBe(true);
    expect(keyToHex(key)).toBe("da39a3ee5e6b4b0d3255bfef95601890afd80709");
  });

  it("put is idempotent", () => {
    const data = Buffer.from("idempotent data");
    const key1 = cas.put(data);
    const key2 = cas.put(data);
    // Same key, and the store should deduplicate (MemStore allows overwrite,
    // but the CAS key is always the same).
    expect(key1.equals(key2)).toBe(true);
  });

  it("exists returns false before put, true after", () => {
    const data = Buffer.from("existence check");
    const key = cas.put(data);
    // We need a different key that was never put — compute it manually.
    const notStored = hexToKey("0000000000000000000000000000000000000000");
    expect(cas.exists(notStored)).toBe(false);
    expect(cas.exists(key)).toBe(true);
  });

  it("get throws CasNotFoundError for unknown key", () => {
    const unknown = hexToKey("0000000000000000000000000000000000000000");
    expect(() => cas.get(unknown)).toThrow(CasNotFoundError);
  });

  it("inner() returns the underlying MemStore", () => {
    expect(cas.inner()).toBe(store);
  });

  it("findByPrefix finds unique match", () => {
    const data = Buffer.from("prefix test data");
    const key = cas.put(data);
    const hex = keyToHex(key);

    // Use a 10-char prefix (5 bytes) — should be unique in a 1-item store.
    const resolved = cas.findByPrefix(hex.slice(0, 10));
    expect(resolved.equals(key)).toBe(true);
  });

  it("findByPrefix with full 40-char hex", () => {
    const data = Buffer.from("full prefix");
    const key = cas.put(data);
    const resolved = cas.findByPrefix(keyToHex(key));
    expect(resolved.equals(key)).toBe(true);
  });

  it("findByPrefix throws CasPrefixNotFoundError when nothing matches", () => {
    // Put something to give the store a non-empty state.
    cas.put(Buffer.from("something"));
    // Request a prefix that definitely doesn't match.
    expect(() => cas.findByPrefix("0000000000000000000000000000000000000000"))
      .toThrow(CasPrefixNotFoundError);
  });

  it("findByPrefix throws CasAmbiguousPrefixError when two objects share a prefix", () => {
    // We'll manufacture two keys that share a prefix by putting them directly
    // into the MemStore, bypassing SHA-1. This lets us control the exact bytes.
    //
    // key1 = 00112233...  key2 = 00112244...  share prefix "001122"
    const key1 = Buffer.from("00112233445566778899aabbccddeeff00112233", "hex");
    const key2 = Buffer.from("00112244556677889900aabbccddeeff00112244", "hex");
    store.put(key1, Buffer.from("data1"));
    store.put(key2, Buffer.from("data2"));

    expect(() => cas.findByPrefix("001122")).toThrow(CasAmbiguousPrefixError);
  });

  it("findByPrefix throws CasInvalidPrefixError for empty string", () => {
    expect(() => cas.findByPrefix("")).toThrow(CasInvalidPrefixError);
  });

  it("findByPrefix throws CasInvalidPrefixError for non-hex chars", () => {
    expect(() => cas.findByPrefix("a3g4")).toThrow(CasInvalidPrefixError);
  });

  it("findByPrefix odd-length prefix works (nibble padding)", () => {
    // An odd-length prefix like "a3f" is padded to "a3f0" (byte 0xa3, 0xf0).
    // It therefore only matches keys starting with EXACTLY those 2 bytes.
    // We store a key that starts with 0xa3, 0xf0 to guarantee a match.
    const key = hexToKey("a3f0000000000000000000000000000000000000");
    store.put(key, Buffer.from("nibble data"));

    // "a3f" pads to "a3f0" → matches keys starting with 0xa3, 0xf0
    const resolved = cas.findByPrefix("a3f");
    expect(resolved.equals(key)).toBe(true);
  });
});

// ─── LocalDiskStore ───────────────────────────────────────────────────────────

describe("LocalDiskStore", () => {
  let tmpDir: string;
  let store: LocalDiskStore;

  beforeEach(() => {
    tmpDir = makeTmpDir("local-disk");
    store = new LocalDiskStore(tmpDir);
  });

  afterEach(() => {
    cleanTmpDir(tmpDir);
  });

  // ── Path layout ─────────────────────────────────────────────────────────────

  it("creates the 2/38 fanout directory structure", () => {
    // SHA-1("hello") = aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d
    // → dir:  tmpDir/aa/
    // → file: tmpDir/aa/f4c61ddcc5e8a2dabede0f3b482cd9aea9434d
    const data = Buffer.from("hello");
    store.put(hexToKey("aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"), data);

    const dirPath = path.join(tmpDir, "aa");
    const filePath = path.join(dirPath, "f4c61ddcc5e8a2dabede0f3b482cd9aea9434d");
    expect(fs.existsSync(dirPath)).toBe(true);
    expect(fs.existsSync(filePath)).toBe(true);
  });

  it("objectPath splits at position 2 of the hex key", () => {
    const key = hexToKey("a9993e364706816aba3e25717850c26c9cd0d89d");
    const p = store.objectPath(key);
    // The directory should be the first 2 hex chars of the key.
    expect(path.basename(path.dirname(p))).toBe("a9");
    // The filename should be the remaining 38 hex chars.
    expect(path.basename(p)).toBe("993e364706816aba3e25717850c26c9cd0d89d");
  });

  it("stored file contains the exact bytes written", () => {
    const key = hexToKey("a9993e364706816aba3e25717850c26c9cd0d89d");
    const data = Buffer.from("abc");
    store.put(key, data);
    const filePath = store.objectPath(key);
    const onDisk = fs.readFileSync(filePath);
    expect(onDisk.equals(data)).toBe(true);
  });

  // ── put ─────────────────────────────────────────────────────────────────────

  it("put is idempotent — no error on second put with same key", () => {
    const key = hexToKey("a9993e364706816aba3e25717850c26c9cd0d89d");
    const data = Buffer.from("abc");
    expect(() => store.put(key, data)).not.toThrow();
    expect(() => store.put(key, data)).not.toThrow(); // second call
  });

  // ── get ─────────────────────────────────────────────────────────────────────

  it("get retrieves what was put", () => {
    const key = hexToKey("a9993e364706816aba3e25717850c26c9cd0d89d");
    const data = Buffer.from("abc");
    store.put(key, data);
    expect(store.get(key).equals(data)).toBe(true);
  });

  it("get throws CasNotFoundError for a key that was never stored", () => {
    const key = hexToKey("0000000000000000000000000000000000000000");
    expect(() => store.get(key)).toThrow(CasNotFoundError);
  });

  // ── exists ───────────────────────────────────────────────────────────────────

  it("exists returns false before put", () => {
    const key = hexToKey("a9993e364706816aba3e25717850c26c9cd0d89d");
    expect(store.exists(key)).toBe(false);
  });

  it("exists returns true after put", () => {
    const key = hexToKey("a9993e364706816aba3e25717850c26c9cd0d89d");
    store.put(key, Buffer.from("abc"));
    expect(store.exists(key)).toBe(true);
  });

  // ── keysWithPrefix ───────────────────────────────────────────────────────────

  it("keysWithPrefix returns empty when bucket directory doesn't exist", () => {
    const prefix = Buffer.from([0xde, 0xad]);
    expect(store.keysWithPrefix(prefix)).toEqual([]);
  });

  it("keysWithPrefix returns empty for empty prefix", () => {
    expect(store.keysWithPrefix(Buffer.alloc(0))).toEqual([]);
  });

  it("keysWithPrefix finds a key by 2-byte prefix", () => {
    const key = hexToKey("a9993e364706816aba3e25717850c26c9cd0d89d");
    store.put(key, Buffer.from("abc"));

    // prefix = first 2 bytes of the key
    const prefix = Buffer.from([0xa9, 0x99]);
    const results = store.keysWithPrefix(prefix);
    expect(results).toHaveLength(1);
    expect(results[0].equals(key)).toBe(true);
  });

  it("keysWithPrefix returns empty when prefix doesn't match any stored key", () => {
    const key = hexToKey("a9993e364706816aba3e25717850c26c9cd0d89d");
    store.put(key, Buffer.from("abc"));

    // prefix that shares the first byte but not the second
    const prefix = Buffer.from([0xa9, 0x00]);
    expect(store.keysWithPrefix(prefix)).toEqual([]);
  });

  it("keysWithPrefix finds multiple keys sharing a prefix", () => {
    // Manually store two keys that share the first byte (same "a9/" bucket).
    const key1 = hexToKey("a9993e364706816aba3e25717850c26c9cd0d89d");
    const key2 = hexToKey("a9aabbccddeeff001122334455667788990a1b2c");
    store.put(key1, Buffer.from("key1 data"));
    store.put(key2, Buffer.from("key2 data"));

    // prefix = first byte only → both keys should match
    const prefix = Buffer.from([0xa9]);
    const results = store.keysWithPrefix(prefix);
    expect(results).toHaveLength(2);
  });

  it("keysWithPrefix with 1-byte prefix matches only keys in that bucket", () => {
    const keyInA9 = hexToKey("a9993e364706816aba3e25717850c26c9cd0d89d");
    const keyInFF = hexToKey("ffffffffffffffffffffffffffffffffffffffff");

    // Write a fake blob for the all-ff key (content won't hash to this key —
    // that's fine for testing the store layer independently of the CAS layer).
    store.put(keyInA9, Buffer.from("in a9 bucket"));
    store.put(keyInFF, Buffer.from("in ff bucket"));

    const prefixA9 = Buffer.from([0xa9]);
    const prefixFF = Buffer.from([0xff]);

    const inA9 = store.keysWithPrefix(prefixA9);
    const inFF = store.keysWithPrefix(prefixFF);

    expect(inA9).toHaveLength(1);
    expect(inA9[0].equals(keyInA9)).toBe(true);
    expect(inFF).toHaveLength(1);
    expect(inFF[0].equals(keyInFF)).toBe(true);
  });

  it("keysWithPrefix skips non-file directory entries (subdirectories, etc.)", () => {
    // Create a bucket directory manually and add a subdirectory inside it.
    // The scan should skip non-file entries.
    const bucket = path.join(tmpDir, "a9");
    fs.mkdirSync(bucket, { recursive: true });
    // Create a subdirectory whose name is 38 chars — it should be skipped.
    const subdir = path.join(bucket, "9".repeat(38));
    fs.mkdirSync(subdir);

    const prefix = Buffer.from([0xa9]);
    const results = store.keysWithPrefix(prefix);
    expect(results).toHaveLength(0); // the subdirectory should be ignored
  });

  it("keysWithPrefix skips files with non-hex names in the bucket", () => {
    // Create a bucket with a file whose name is 38 chars but contains non-hex chars.
    const bucket = path.join(tmpDir, "a9");
    fs.mkdirSync(bucket, { recursive: true });
    // 38 chars, but last char is 'z' (not valid hex)
    const badName = "9".repeat(37) + "z";
    fs.writeFileSync(path.join(bucket, badName), Buffer.from("bad file"));

    const prefix = Buffer.from([0xa9]);
    const results = store.keysWithPrefix(prefix);
    expect(results).toHaveLength(0); // non-hex filenames are skipped
  });

  it("keysWithPrefix returns empty when bucket is a file instead of a directory", () => {
    // Place a regular file at the bucket path ("a9"). This is not a normal
    // situation but could occur if the store root is corrupted. The catch branch
    // in keysWithPrefix handles this gracefully by returning [].
    //
    // We must use a different root to avoid conflicting with other tests in this
    // describe block that share `store` and `tmpDir`.
    const specialRoot = makeTmpDir("readdir-catch");
    const badStore = new LocalDiskStore(specialRoot);

    // Create a file at the "a9" bucket path — readdirSync will throw ENOTDIR.
    const bucketPath = path.join(specialRoot, "a9");
    fs.writeFileSync(bucketPath, Buffer.from("not a directory"));

    const results = badStore.keysWithPrefix(Buffer.from([0xa9]));
    expect(results).toHaveLength(0);
    cleanTmpDir(specialRoot);
  });

  it("get re-throws non-ENOENT errors — EISDIR when object path is a directory", () => {
    // The get() method catches ENOENT and wraps it in CasNotFoundError.
    // Any OTHER error code (EISDIR, EACCES, etc.) must be re-thrown as-is
    // so callers can decide how to handle unexpected I/O failures.
    //
    // We trigger EISDIR by creating a directory at the object path (the CAS
    // layer never creates directories there, so this is a corrupted store).
    const key = hexToKey("a9993e364706816aba3e25717850c26c9cd0d89d");
    const filePath = store.objectPath(key);

    // Create the parent directory, then a directory where the file should be.
    fs.mkdirSync(path.dirname(filePath), { recursive: true });
    fs.mkdirSync(filePath); // directory instead of a file

    // readFileSync on a directory throws with code EISDIR (not ENOENT).
    let thrownCode: string | undefined;
    try {
      store.get(key);
    } catch (err) {
      const nodeErr = err as NodeJS.ErrnoException;
      thrownCode = nodeErr.code;
    }
    // Must NOT be CasNotFoundError — must propagate the original error.
    expect(thrownCode).toBe("EISDIR");
  });

  it("put rethrows I/O errors and leaves no temp files behind", () => {
    // We trigger a write error by making the object's parent directory read-only
    // (on POSIX). On Windows this test is skipped because Windows permission
    // semantics are different and child processes run as admin in CI.
    //
    // Skip on Windows.
    if (process.platform === "win32") {
      return;
    }
    const key = hexToKey("bb0000000000000000000000000000000000bbbb");
    const bucketDir = path.join(tmpDir, "bb");
    fs.mkdirSync(bucketDir);
    // Remove write permission from the bucket directory.
    fs.chmodSync(bucketDir, 0o555); // r-xr-xr-x

    let didThrow = false;
    try {
      store.put(key, Buffer.from("will fail"));
    } catch {
      didThrow = true;
    }

    expect(didThrow).toBe(true);

    // Restore permissions so cleanup works.
    fs.chmodSync(bucketDir, 0o755);

    // Verify no temp files were left behind (the catch block should unlink them).
    const entries = fs.readdirSync(bucketDir);
    expect(entries.filter((e) => e.endsWith(".tmp"))).toHaveLength(0);
  });
});

// ─── ContentAddressableStore + LocalDiskStore (integration) ──────────────────

describe("ContentAddressableStore + LocalDiskStore", () => {
  let tmpDir: string;
  let cas: ContentAddressableStore<LocalDiskStore>;

  beforeEach(() => {
    tmpDir = makeTmpDir("cas-integration");
    cas = new ContentAddressableStore(new LocalDiskStore(tmpDir));
  });

  afterEach(() => {
    cleanTmpDir(tmpDir);
  });

  // ── Round-trip tests ────────────────────────────────────────────────────────

  it("round-trips an empty blob", () => {
    // The empty buffer is a valid object. SHA-1("") is a known test vector.
    const data = Buffer.alloc(0);
    const key = cas.put(data);
    // Known SHA-1 of the empty string:
    expect(keyToHex(key)).toBe("da39a3ee5e6b4b0d3255bfef95601890afd80709");
    const retrieved = cas.get(key);
    expect(retrieved.equals(data)).toBe(true);
  });

  it("round-trips a small blob", () => {
    const data = Buffer.from("hello, content-addressable world");
    const key = cas.put(data);
    expect(cas.get(key).equals(data)).toBe(true);
  });

  it("round-trips a 1 MiB blob", () => {
    // A large blob exercises the streaming path of the SHA-1 hasher and ensures
    // that readFileSync handles large files correctly.
    const data = Buffer.alloc(1024 * 1024);
    // Fill with a non-trivial pattern to catch truncation bugs.
    for (let i = 0; i < data.length; i++) {
      data[i] = i & 0xff;
    }
    const key = cas.put(data);
    const retrieved = cas.get(key);
    expect(retrieved.equals(data)).toBe(true);
  });

  it("round-trips binary data including null bytes", () => {
    const data = Buffer.from([0x00, 0x01, 0x02, 0xff, 0xfe, 0x00, 0x00]);
    const key = cas.put(data);
    expect(cas.get(key).equals(data)).toBe(true);
  });

  // ── Idempotent put ─────────────────────────────────────────────────────────

  it("put is idempotent — same key returned on second call", () => {
    const data = Buffer.from("idempotent");
    const key1 = cas.put(data);
    const key2 = cas.put(data); // second call
    expect(key1.equals(key2)).toBe(true);
  });

  it("second put does not corrupt the stored blob", () => {
    const data = Buffer.from("idempotent safety");
    const key = cas.put(data);
    cas.put(data); // second put
    expect(cas.get(key).equals(data)).toBe(true);
  });

  // ── exists ─────────────────────────────────────────────────────────────────

  it("exists returns false before put", () => {
    // We need a key that was never put. Compute SHA-1("not stored") and check.
    // Rather than hard-coding the hash, just use a known all-zero key.
    const notStored = hexToKey("0000000000000000000000000000000000000000");
    expect(cas.exists(notStored)).toBe(false);
  });

  it("exists returns true after put", () => {
    const data = Buffer.from("existence");
    const key = cas.put(data);
    expect(cas.exists(key)).toBe(true);
  });

  // ── get not found ─────────────────────────────────────────────────────────

  it("get throws CasNotFoundError for an unknown key", () => {
    const unknown = hexToKey("0000000000000000000000000000000000000000");
    expect(() => cas.get(unknown)).toThrow(CasNotFoundError);
  });

  // ── Corrupted file detection ───────────────────────────────────────────────

  it("get throws CasCorruptedError when the on-disk file is mutated", () => {
    const data = Buffer.from("pristine data");
    const key = cas.put(data);

    // Mutate the stored file directly — bypassing the CAS layer.
    const filePath = cas.inner().objectPath(key);
    fs.writeFileSync(filePath, Buffer.from("tampered!"));

    // Now get() should detect the hash mismatch and throw CasCorruptedError.
    expect(() => cas.get(key)).toThrow(CasCorruptedError);

    // Verify the error carries the correct key.
    try {
      cas.get(key);
    } catch (err) {
      expect(err).toBeInstanceOf(CasCorruptedError);
      if (err instanceof CasCorruptedError) {
        expect(err.key.equals(key)).toBe(true);
      }
    }
  });

  // ── findByPrefix ──────────────────────────────────────────────────────────

  it("findByPrefix resolves an 8-char (even-length) prefix uniquely", () => {
    const data = Buffer.from("prefix resolution");
    const key = cas.put(data);
    const hex = keyToHex(key);

    // 8-char = 4-byte even-length prefix — no nibble padding ambiguity.
    const resolved = cas.findByPrefix(hex.slice(0, 8));
    expect(resolved.equals(key)).toBe(true);
  });

  it("findByPrefix resolves the full 40-char hex", () => {
    const data = Buffer.from("full hex");
    const key = cas.put(data);
    const resolved = cas.findByPrefix(keyToHex(key));
    expect(resolved.equals(key)).toBe(true);
  });

  it("findByPrefix resolves an odd-length (nibble) prefix when key matches the padded value", () => {
    // Odd-length prefixes are right-padded with '0'. "a3f" → bytes [0xa3, 0xf0].
    // We store a key starting with 0xa3, 0xf0 so the match is guaranteed.
    // This exercises the nibble-padding code path end-to-end on disk.
    const diskStore = cas.inner();
    const key = hexToKey("a3f0000000000000000000000000000000000000");
    diskStore.put(key, Buffer.from("nibble prefix on disk"));

    // "a3f" padded to "a3f0" → [0xa3, 0xf0] — matches our key exactly.
    const resolved = cas.findByPrefix("a3f");
    expect(resolved.equals(key)).toBe(true);
  });

  it("findByPrefix throws CasPrefixNotFoundError for no match", () => {
    cas.put(Buffer.from("something"));
    expect(() => cas.findByPrefix("0000000000000000000000000000000000000000"))
      .toThrow(CasPrefixNotFoundError);
  });

  it("findByPrefix throws CasAmbiguousPrefixError when two objects share a prefix", () => {
    // Write two objects whose keys share a prefix by writing them directly to
    // the disk store (bypassing SHA-1 hashing so we control the key bytes).
    const diskStore = cas.inner();

    // key1 and key2 share the first 4 hex chars (2 bytes): "0011"
    const key1 = hexToKey("0011223344556677889900aabbccddeeff001122");
    const key2 = hexToKey("0011aabbccddeeff001122334455667788990011");
    diskStore.put(key1, Buffer.from("data1"));
    diskStore.put(key2, Buffer.from("data2"));

    expect(() => cas.findByPrefix("0011")).toThrow(CasAmbiguousPrefixError);

    // Verify the error carries the prefix string.
    try {
      cas.findByPrefix("0011");
    } catch (err) {
      expect(err).toBeInstanceOf(CasAmbiguousPrefixError);
      if (err instanceof CasAmbiguousPrefixError) {
        expect(err.prefix).toBe("0011");
      }
    }
  });

  it("findByPrefix throws CasInvalidPrefixError for empty string", () => {
    expect(() => cas.findByPrefix("")).toThrow(CasInvalidPrefixError);
  });

  it("findByPrefix throws CasInvalidPrefixError for non-hex characters", () => {
    expect(() => cas.findByPrefix("a3g4")).toThrow(CasInvalidPrefixError);

    try {
      cas.findByPrefix("a3g4");
    } catch (err) {
      if (err instanceof CasInvalidPrefixError) {
        expect(err.prefix).toBe("a3g4");
      }
    }
  });

  it("findByPrefix throws CasInvalidPrefixError for string with spaces", () => {
    expect(() => cas.findByPrefix("a3 f4")).toThrow(CasInvalidPrefixError);
  });

  // ── inner() ───────────────────────────────────────────────────────────────

  it("inner() returns the LocalDiskStore with the correct root", () => {
    const inner = cas.inner();
    expect(inner).toBeInstanceOf(LocalDiskStore);
    expect(inner.root).toBe(tmpDir);
  });

  // ── LocalDiskStore 2/38 path layout verification ─────────────────────────

  it("LocalDiskStore creates the exact 2/38 directory structure on disk", () => {
    // SHA-1("abc") = a9993e364706816aba3e25717850c26c9cd0d89d
    const data = Buffer.from("abc");
    const key = cas.put(data);
    expect(keyToHex(key)).toBe("a9993e364706816aba3e25717850c26c9cd0d89d");

    // Verify: the directory "a9" exists under tmpDir.
    const dir = path.join(tmpDir, "a9");
    expect(fs.existsSync(dir)).toBe(true);
    expect(fs.statSync(dir).isDirectory()).toBe(true);

    // Verify: the file "993e364706816aba3e25717850c26c9cd0d89d" exists in "a9/".
    const file = path.join(dir, "993e364706816aba3e25717850c26c9cd0d89d");
    expect(fs.existsSync(file)).toBe(true);
    expect(fs.statSync(file).isFile()).toBe(true);

    // Verify: the file contains exactly the bytes we stored.
    expect(fs.readFileSync(file).equals(data)).toBe(true);
  });

  it("multiple objects with different first bytes create different buckets", () => {
    // "abc" → a9..., "": da..., two different buckets
    const key1 = cas.put(Buffer.from("abc"));
    const key2 = cas.put(Buffer.alloc(0));

    // They must land in different directories.
    const dir1 = keyToHex(key1).slice(0, 2); // "a9"
    const dir2 = keyToHex(key2).slice(0, 2); // "da"
    expect(dir1).not.toBe(dir2);
    expect(fs.existsSync(path.join(tmpDir, dir1))).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, dir2))).toBe(true);
  });

  // ── Multiple objects in the same store ───────────────────────────────────

  it("stores and retrieves multiple different blobs independently", () => {
    const blobs: Buffer[] = [
      Buffer.from("first blob"),
      Buffer.from("second blob"),
      Buffer.from(""),
      Buffer.alloc(512, 0xab),
    ];
    const keys = blobs.map((b) => cas.put(b));

    for (let i = 0; i < blobs.length; i++) {
      expect(cas.get(keys[i]).equals(blobs[i])).toBe(true);
    }
  });
});
