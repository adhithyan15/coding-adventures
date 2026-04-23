// ============================================================================
// Cas.swift — Generic Content-Addressable Storage
// ============================================================================
//
// Content-addressable storage (CAS) maps the *hash of content* to the content
// itself. The hash is simultaneously the address and an integrity check: if the
// bytes returned by the store don't hash to the key you requested, the data is
// corrupt. No separate checksum file or trust anchor is needed.
//
// Mental model
// ─────────────
// Imagine a library where every book's call number IS a fingerprint of the
// book's text. You cannot file a different book under that number — the number
// would immediately be wrong. And if someone swaps pages, the fingerprint
// changes and the librarian detects the tampering before you open the cover.
//
//   Traditional storage:  name ──► content   (name can lie; content can change)
//   Content-addressed:    hash ──► content   (hash is derived from content, cannot lie)
//
// How Git uses CAS
// ─────────────────
// Git's entire history is built on this principle. Every blob (file snapshot),
// tree (directory listing), commit, and tag is stored by the SHA-1 hash of its
// serialized bytes. Two identical files share one object. Renaming a file
// creates zero new storage. History is an immutable DAG of hashes pointing to
// hashes.
//
// Architecture
// ─────────────
//
//   ┌──────────────────────────────────────────────────┐
//   │  ContentAddressableStore<S: BlobStore>            │
//   │  · put(data:)       → [UInt8] key (20 bytes)     │
//   │  · get(key:)        → fetch from S, verify hash  │
//   │  · findByPrefix(_:) → prefix search via S        │
//   └─────────────────┬────────────────────────────────┘
//                     │ protocol BlobStore
//          ┌──────────┴──────────────────────────────┐
//          │                                         │
//   LocalDiskStore                       (mem, S3, custom, …)
//   root/XX/XXXXXX…
//
// Layer: CAS01 — depends on SHA1
// ============================================================================

import Foundation
import SHA1

// ============================================================================
// MARK: - Hex Utilities
// ============================================================================
//
// Keys are 20-byte arrays ([UInt8] with count 20), but humans interact with
// them as 40-character lowercase hex strings (e.g., "a3f4b2c1d0…").
//
//   keyToHex(_:) — converts [UInt8] (20 bytes) → 40-char hex string
//   hexToKey(_:) — parses a 40-char hex string → [UInt8], throws on bad input

/// Convert a 20-byte SHA-1 key to a 40-character lowercase hex string.
///
/// Each byte becomes two hex digits, e.g., 0xa3 → "a3".
///
/// - Parameter key: A 20-element byte array (SHA-1 digest).
/// - Returns: A 40-character lowercase hexadecimal string.
public func keyToHex(_ key: [UInt8]) -> String {
    key.map { String(format: "%02x", $0) }.joined()
}

/// Parse a 40-character hex string into a 20-byte key array.
///
/// Returns `nil` if the string is not exactly 40 valid hex characters.
///
/// - Parameter hex: A 40-character lowercase (or uppercase) hexadecimal string.
/// - Returns: A 20-element `[UInt8]` on success, `nil` on failure.
public func hexToKey(_ hex: String) -> [UInt8]? {
    guard hex.count == 40 else { return nil }
    var key = [UInt8](repeating: 0, count: 20)
    let chars = Array(hex.unicodeScalars)
    for i in 0..<20 {
        guard let hi = hexNibble(chars[i * 2]),
              let lo = hexNibble(chars[i * 2 + 1]) else { return nil }
        key[i] = (hi << 4) | lo
    }
    return key
}

// Decode a single Unicode scalar hex digit ('0'–'9', 'a'–'f', 'A'–'F') to
// its numeric value (0–15). Returns nil for any non-hex character.
private func hexNibble(_ scalar: Unicode.Scalar) -> UInt8? {
    let v = scalar.value
    switch v {
    case 48...57:  return UInt8(v - 48)         // '0'–'9'
    case 97...102: return UInt8(v - 97 + 10)    // 'a'–'f'
    case 65...70:  return UInt8(v - 65 + 10)    // 'A'–'F'
    default:       return nil
    }
}

// Decode an arbitrary-length hex string (1–40 characters) to a byte prefix.
//
// Odd-length strings are right-padded with '0' before converting to bytes.
// This matches how Git abbreviates object hashes: "a3f" means "starts with
// 0xa3, 0xf0" — the trailing nibble is the high nibble of the next byte.
//
// Returns nil if the string is empty or contains any non-hex character.
func decodeHexPrefix(_ hex: String) -> [UInt8]? {
    guard !hex.isEmpty else { return nil }
    let scalars = Array(hex.unicodeScalars)
    // Validate all characters are hex before doing any byte work.
    for s in scalars {
        guard hexNibble(s) != nil else { return nil }
    }
    // Pad to even length so we can decode in 2-character chunks.
    let padded = hex.count % 2 == 1 ? hex + "0" : hex
    let paddedScalars = Array(padded.unicodeScalars)
    var result = [UInt8]()
    result.reserveCapacity(paddedScalars.count / 2)
    var i = 0
    while i < paddedScalars.count {
        let hi = hexNibble(paddedScalars[i])!
        let lo = hexNibble(paddedScalars[i + 1])!
        result.append((hi << 4) | lo)
        i += 2
    }
    return result
}

// ============================================================================
// MARK: - Internal Utilities
// ============================================================================

// Check whether an error represents "file not found" from Foundation.
//
// `Data(contentsOf:)` throws `CocoaError(.fileReadNoSuchFile)` for missing
// files. We also accept the equivalent NSError representation so that test
// helpers (like InMemoryStore) can throw the same domain/code manually.
func isNotFoundError(_ error: Error) -> Bool {
    let nsError = error as NSError
    return nsError.domain == NSCocoaErrorDomain
        && nsError.code == NSFileReadNoSuchFileError
}

// ============================================================================
// MARK: - CasError
// ============================================================================
//
// A typed error enum covering both backend failures and CAS-level integrity
// problems. Using an enum instead of a single `Error` type lets callers match
// specific cases without unsafe downcasting.

/// Errors that can arise from `ContentAddressableStore` operations.
///
/// - `notFound`: a key was requested but no such key exists in the store.
/// - `corrupted`: the stored bytes don't hash to the key — data was tampered.
/// - `ambiguousPrefix`: a hex prefix matched two or more objects.
/// - `prefixNotFound`: a hex prefix matched zero objects.
/// - `invalidPrefix`: the hex string is malformed (empty or non-hex chars).
/// - `storeError`: a lower-level error from the `BlobStore` backend.
public enum CasError: Error, CustomStringConvertible {
    /// The requested key is not in the store.
    case notFound([UInt8])

    /// The bytes returned by the store don't hash to the requested key.
    ///
    /// This is a data integrity violation: the file on disk (or the network
    /// response) has been modified since it was written.
    case corrupted([UInt8])

    /// The hex prefix matched two or more stored objects.
    case ambiguousPrefix(String)

    /// The hex prefix matched zero stored objects.
    case prefixNotFound(String)

    /// The supplied string is not valid hexadecimal, or is empty.
    case invalidPrefix(String)

    /// A failure in the underlying `BlobStore` (I/O error, network error, …).
    case storeError(Error)

    public var description: String {
        switch self {
        case .notFound(let key):
            return "object not found: \(keyToHex(key))"
        case .corrupted(let key):
            return "object corrupted: \(keyToHex(key))"
        case .ambiguousPrefix(let p):
            return "ambiguous prefix: \(p)"
        case .prefixNotFound(let p):
            return "object not found for prefix: \(p)"
        case .invalidPrefix(let p):
            return "invalid hex prefix: \"\(p)\""
        case .storeError(let e):
            return "store error: \(e)"
        }
    }
}

// ============================================================================
// MARK: - BlobStore Protocol
// ============================================================================
//
// The single abstraction that separates the CAS logic from persistence.
//
// Any type that can store and retrieve byte blobs by a 20-byte key qualifies.
// Conforming to `BlobStore` is sufficient to use `ContentAddressableStore`.
//
// Design note: all methods use `throws` rather than an associated Error type.
// This keeps the protocol simple — in Swift, protocol associated types for
// errors require complex generic constraints at every call site. Since
// `ContentAddressableStore` wraps errors in `CasError.storeError` anyway, the
// concrete error type need not propagate to callers.

/// A pluggable key-value store for raw byte blobs, keyed by a 20-byte hash.
///
/// Implement this protocol to add a new storage backend. The key is always a
/// SHA-1 digest produced by `ContentAddressableStore`; implementations should
/// treat it as an opaque 20-byte identifier.
///
/// All methods may throw. The `ContentAddressableStore` wraps any thrown error
/// in `CasError.storeError` before surfacing it to callers.
public protocol BlobStore {

    /// Persist `data` under `key`.
    ///
    /// Implementations must be idempotent: storing the same key twice with the
    /// same bytes is not an error. The CAS layer prevents hash collisions by
    /// construction, so storing a different blob under an existing key cannot
    /// happen in practice.
    mutating func put(key: [UInt8], data: [UInt8]) throws

    /// Retrieve the blob stored under `key`.
    ///
    /// Throws if the key is not present or if I/O fails.
    /// Implementations do NOT verify the hash — that is the CAS layer's job.
    func get(key: [UInt8]) throws -> [UInt8]

    /// Return whether `key` is present without fetching the blob.
    func exists(key: [UInt8]) throws -> Bool

    /// Return all stored keys whose first `prefix.count` bytes equal `prefix`.
    ///
    /// Used for abbreviated-hash lookup. The caller supplies a byte prefix
    /// decoded from a short hex string, and the store returns the full 20-byte
    /// keys that match. The CAS layer then checks for uniqueness.
    func keysWithPrefix(_ prefix: [UInt8]) throws -> [[UInt8]]
}

// ============================================================================
// MARK: - ContentAddressableStore
// ============================================================================
//
// The main CAS struct. It wraps any `BlobStore` and adds three things the
// store alone cannot provide:
//
//   1. Automatic keying — callers pass content; SHA-1 is computed internally.
//   2. Integrity check  — on every get, SHA-1(returned bytes) must equal key.
//   3. Prefix resolution — converts abbreviated hex (like `a3f4b2`) to a full key.

/// Content-addressable store that wraps a `BlobStore` backend.
///
/// All objects are keyed by their SHA-1 hash. The same content always maps to
/// the same key (deduplication), and the stored bytes are verified against the
/// key on every read (integrity).
///
/// ## Type parameter
///
/// `S` is any `BlobStore`. Use `LocalDiskStore` for filesystem-backed storage,
/// or supply your own implementation for cloud or in-memory storage.
///
/// ## Example
///
/// ```swift
/// let store = try LocalDiskStore(root: URL(fileURLWithPath: "/tmp/my-cas"))
/// var cas = ContentAddressableStore(store: store)
/// let key = try cas.put(data: Array("hello".utf8))
/// let data = try cas.get(key: key)
/// // data == Array("hello".utf8)
/// ```
public struct ContentAddressableStore<S: BlobStore> {

    // The underlying BlobStore. Marked `var` because `BlobStore.put` takes
    // `mutating` — struct conformances need a mutable receiver.
    private var store: S

    /// Create a new CAS wrapping `store`.
    public init(store: S) {
        self.store = store
    }

    /// Hash `data` with SHA-1, store it in the backend, and return the 20-byte key.
    ///
    /// Idempotent: if the same content has already been stored, the existing
    /// key is returned and no new write is performed.
    ///
    /// - Parameter data: The bytes to store.
    /// - Returns: The 20-byte SHA-1 key.
    /// - Throws: `CasError.storeError` if the backend write fails.
    @discardableResult
    public mutating func put(data: [UInt8]) throws -> [UInt8] {
        // Compute SHA-1 using our own sha1() function from the SHA1 package.
        // sha1(_:) takes Foundation.Data and returns Foundation.Data (20 bytes).
        let dataObj = Data(data)
        let digestData = sha1(dataObj)
        let key = [UInt8](digestData)

        // Delegate to the backend. BlobStore.put is required to be idempotent,
        // so no pre-check is needed here. Skipping the exists→put two-step
        // eliminates a TOCTOU race and saves a round-trip on the happy path.
        do {
            try store.put(key: key, data: data)
        } catch {
            throw CasError.storeError(error)
        }
        return key
    }

    /// Retrieve the blob stored under `key` and verify its integrity.
    ///
    /// The returned bytes are guaranteed to hash to `key`. If the store
    /// returns corrupted bytes, `CasError.corrupted` is thrown rather than
    /// silently returning bad data.
    ///
    /// - Parameter key: A 20-byte SHA-1 key.
    /// - Returns: The stored bytes.
    /// - Throws: `CasError.notFound`, `CasError.corrupted`, or `CasError.storeError`.
    public func get(key: [UInt8]) throws -> [UInt8] {
        let data: [UInt8]
        do {
            data = try store.get(key: key)
        } catch {
            // Translate backend "not found" into a typed CasError so callers
            // don't have to inspect the opaque backend error.
            //
            // `Data(contentsOf:)` throws CocoaError(.fileReadNoSuchFile) when
            // the file does not exist. We also accept NSCocoaErrorDomain +
            // NSFileReadNoSuchFileError for any backend that constructs the
            // error manually (e.g., InMemoryStore in tests).
            if isNotFoundError(error) {
                throw CasError.notFound(key)
            }
            throw CasError.storeError(error)
        }

        // Integrity check: re-hash the returned bytes.
        // If SHA-1(returned_bytes) ≠ key, the file has been corrupted on disk.
        let actual = [UInt8](sha1(Data(data)))
        guard actual == key else {
            throw CasError.corrupted(key)
        }
        return data
    }

    /// Check whether a key is present in the store.
    ///
    /// - Parameter key: A 20-byte SHA-1 key.
    /// - Returns: `true` if the object is stored, `false` if not.
    /// - Throws: `CasError.storeError` if the backend check fails.
    public func exists(key: [UInt8]) throws -> Bool {
        do {
            return try store.exists(key: key)
        } catch {
            throw CasError.storeError(error)
        }
    }

    /// Resolve an abbreviated hex string to a full 20-byte key.
    ///
    /// Accepts any non-empty hex string of 1–40 characters. Odd-length strings
    /// are treated as nibble prefixes: "a3f" matches any key starting with
    /// 0xa3, 0xf0 (the trailing nibble is the high nibble of the next byte).
    ///
    /// - Parameter hexPrefix: A 1–40 character hex string (upper or lowercase).
    /// - Returns: The unique matching 20-byte key.
    /// - Throws:
    ///   - `CasError.invalidPrefix`  — empty string or non-hex characters.
    ///   - `CasError.prefixNotFound` — no keys match.
    ///   - `CasError.ambiguousPrefix` — two or more keys match.
    ///   - `CasError.storeError`     — backend failure during the scan.
    public func findByPrefix(_ hexPrefix: String) throws -> [UInt8] {
        // Validate and decode the hex prefix to a byte prefix.
        guard let prefixBytes = decodeHexPrefix(hexPrefix) else {
            throw CasError.invalidPrefix(hexPrefix)
        }

        var matches: [[UInt8]]
        do {
            matches = try store.keysWithPrefix(prefixBytes)
        } catch {
            throw CasError.storeError(error)
        }

        // Sort for deterministic behaviour when there are multiple matches.
        // This makes test assertions against AmbiguousPrefix reliable.
        matches.sort { $0.lexicographicallyPrecedes($1) }

        switch matches.count {
        case 0:
            throw CasError.prefixNotFound(hexPrefix)
        case 1:
            return matches[0]
        default:
            throw CasError.ambiguousPrefix(hexPrefix)
        }
    }

    /// Access the underlying `BlobStore` directly.
    ///
    /// Useful when you need backend-specific operations not exposed by the CAS
    /// interface (e.g., listing all keys for garbage collection).
    public var inner: S { store }
}

// ============================================================================
// MARK: - LocalDiskStore
// ============================================================================
//
// Filesystem backend using the Git 2/38 fanout layout.
//
// Why 2/38?
// ──────────
// A repository with 100 000 objects would put 100 000 files in a single
// directory if we stored objects as root/<40-hex-hash>. Most filesystems slow
// down dramatically at that scale. Splitting on the first byte creates up to
// 256 sub-directories — each holding at most ~390 entries for a 100 k object
// repo. Git has used this layout since its initial release.
//
// Object path: root/<xx>/<remaining-38-hex-chars>
//
//   key = [0xa3, 0xf4, …]
//   dir  = root/a3/
//   file = root/a3/f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5
//
// Atomic writes
// ──────────────
// We write to a temp file in the same fanout directory, then rename into place.
// On POSIX, rename(2) is atomic. On platforms where the destination may already
// exist (Windows), we treat a rename failure as a successful idempotent write —
// another writer stored the same object concurrently and the stored bytes are
// identical.
//
// The temp filename includes the process ID and a nanosecond timestamp to make
// it unpredictable. A fixed suffix like ".tmp" could be targeted by a local
// attacker who pre-places a symlink at that path, redirecting the write to an
// arbitrary location. Mixing PID + nanoseconds makes the name infeasible to
// predict without privileged access to the process.

/// Filesystem-backed `BlobStore` using Git-style 2/38 fanout layout.
///
/// Objects are stored at `<root>/<xx>/<38-hex-chars>` where `xx` is the first
/// byte of the SHA-1 hash encoded as two lowercase hex digits.
///
/// Writes are atomic: content is written to a temp file in the same bucket
/// directory, then renamed into place.
public struct LocalDiskStore: BlobStore {

    /// The root directory of the object store.
    public let root: URL

    /// Create (or open) a store rooted at `root`.
    ///
    /// The directory is created if it does not exist.
    ///
    /// - Parameter root: The filesystem URL for the store root.
    /// - Throws: If the directory cannot be created.
    public init(root: URL) throws {
        self.root = root
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    }

    // Compute the storage URL for a given 20-byte key.
    //
    // key[0] encodes as a two-char directory name.
    // key[1..] encodes as the 38-char filename.
    //
    //   key = [0xa3, 0xf4, 0xb2, …]
    //   dir  = root/a3/
    //   file = root/a3/f4b2…
    private func objectURL(for key: [UInt8]) -> URL {
        let hex = keyToHex(key)
        // Split at position 2: first 2 chars = directory, rest = filename.
        let dirName  = String(hex.prefix(2))
        let fileName = String(hex.dropFirst(2))
        return root.appendingPathComponent(dirName).appendingPathComponent(fileName)
    }

    public mutating func put(key: [UInt8], data: [UInt8]) throws {
        let finalURL = objectURL(for: key)

        // Short-circuit: if the file already exists, the object is stored.
        // The key is a SHA-1 hash of the content, so the stored bytes are
        // guaranteed to be identical — no need to overwrite.
        if FileManager.default.fileExists(atPath: finalURL.path) {
            return
        }

        // Create the two-char fanout directory (e.g., "a3/") if needed.
        let bucketURL = finalURL.deletingLastPathComponent()
        try FileManager.default.createDirectory(at: bucketURL, withIntermediateDirectories: true)

        // Atomic write: write to a temp file in the same bucket directory,
        // then rename into place. Same-directory rename avoids cross-device
        // failures (POSIX rename(2) fails across mount points).
        //
        // Security: include PID and nanosecond timestamp in the temp name so
        // it cannot be predicted by a local attacker. A fixed suffix like
        // ".tmp" would let an attacker pre-place a symlink at that path,
        // redirecting our write to an arbitrary destination.
        // Build an unpredictable temp filename using PID and a nanosecond
        // timestamp derived from the system uptime. ProcessInfo is part of
        // Foundation (available on all platforms); we multiply to nanoseconds
        // to get sufficient entropy.
        let pid = ProcessInfo.processInfo.processIdentifier
        let ns  = UInt64(ProcessInfo.processInfo.systemUptime * 1_000_000_000)
        let tmpName = "\(finalURL.lastPathComponent).\(pid).\(ns).tmp"
        let tmpURL  = bucketURL.appendingPathComponent(tmpName)

        // Write to temp file.
        try Data(data).write(to: tmpURL, options: .atomic)

        // Rename into place.
        do {
            // FileManager.moveItem throws if destination already exists.
            // That means another writer stored the same object first — fine.
            try FileManager.default.moveItem(at: tmpURL, to: finalURL)
        } catch {
            // Clean up the temp file so we don't leave orphans.
            try? FileManager.default.removeItem(at: tmpURL)
            // If the final file now exists, a concurrent writer stored the same
            // object. That is correct and idempotent — not an error.
            if !FileManager.default.fileExists(atPath: finalURL.path) {
                throw error
            }
        }
    }

    public func get(key: [UInt8]) throws -> [UInt8] {
        let url = objectURL(for: key)
        // Foundation throws NSError with NSFileReadNoSuchFileError when the
        // file does not exist. The CAS layer translates this to CasError.notFound.
        let data = try Data(contentsOf: url)
        return [UInt8](data)
    }

    public func exists(key: [UInt8]) -> Bool {
        FileManager.default.fileExists(atPath: objectURL(for: key).path)
    }

    // Protocol conformance (throws version — the non-throwing overload above
    // satisfies the protocol because a non-throwing function satisfies a
    // throwing requirement in Swift).
    public func keysWithPrefix(_ prefix: [UInt8]) throws -> [[UInt8]] {
        // An empty prefix would match everything. The CAS layer already rejects
        // empty hex strings (InvalidPrefix), but we guard here defensively.
        guard !prefix.isEmpty else { return [] }

        // The first byte of the prefix tells us which fanout bucket to scan.
        // If the prefix is at least one byte, the first byte is the directory name.
        let firstByteHex = String(format: "%02x", prefix[0])
        let bucketURL = root.appendingPathComponent(firstByteHex)

        // If the bucket directory does not exist, no objects with this prefix
        // have been stored yet.
        guard FileManager.default.fileExists(atPath: bucketURL.path) else { return [] }

        // List all entries in the bucket directory.
        let entries = try FileManager.default.contentsOfDirectory(
            at: bucketURL,
            includingPropertiesForKeys: nil,
            options: .skipsHiddenFiles
        )

        var keys: [[UInt8]] = []
        for entry in entries {
            let name = entry.lastPathComponent
            // Each object file has a 38-character name (the last 38 hex chars
            // of the 40-char hash). Skip temp files or any other artifacts.
            guard name.count == 38 else { continue }

            // Reconstruct the full 40-char hex and parse it back to a key.
            let fullHex = firstByteHex + name
            guard let key = hexToKey(fullHex) else { continue }

            // Check that this key actually starts with the requested prefix.
            // We need this because the first byte determines the bucket but
            // subsequent bytes narrow the match further.
            if key.prefix(prefix.count).elementsEqual(prefix) {
                keys.append(key)
            }
        }
        return keys
    }
}
