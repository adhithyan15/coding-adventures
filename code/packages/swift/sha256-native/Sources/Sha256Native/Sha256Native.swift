// ============================================================================
// Sha256Native.swift — Swift wrapper over the Rust sha256-c C ABI
// ============================================================================
//
// This is the **native-through-Rust** companion to the pure-Swift `sha256`
// package. Instead of reimplementing SHA-256, it links the Rust
// `coding_adventures_sha256` crate at compile time (via the `sha256-c` static
// library) and calls it through a C ABI imported as the `CSha256` module.
//
// The digest functions write into a caller-owned 32-byte buffer, so no memory
// crosses the boundary on the one-shot path. The streaming `Hasher` owns an
// opaque native handle and frees it in `deinit`.
//
// PREREQUISITE: `libsha256_c.a` must be compiled and copied into
// `Sources/CSha256/` before building — see the BUILD file.

import CSha256

/// SHA-256 backed by the Rust `sha256-c` static library.
public enum Sha256Native {

    /// The 32-byte SHA-256 digest of `data` (computed in Rust).
    public static func digest(_ data: [UInt8]) -> [UInt8] {
        var out = [UInt8](repeating: 0, count: 32)
        data.withUnsafeBufferPointer { input in
            out.withUnsafeMutableBufferPointer { output in
                sha256_c_digest(input.baseAddress, data.count, output.baseAddress)
            }
        }
        return out
    }

    /// The 64-character lowercase hex digest of `data` (computed in Rust).
    public static func hexString(_ data: [UInt8]) -> String { hex(digest(data)) }

    /// Format bytes as a lowercase hex string.
    public static func hex(_ bytes: [UInt8]) -> String {
        let table = Array("0123456789abcdef")
        var s = ""
        s.reserveCapacity(bytes.count * 2)
        for b in bytes {
            s.append(table[Int(b >> 4)])
            s.append(table[Int(b & 0x0f)])
        }
        return s
    }

    /// A streaming SHA-256 hasher backed by a native Rust hasher handle.
    ///
    /// The handle is freed automatically in `deinit`. `digest()` is
    /// non-destructive, so the hasher can keep receiving `update`s afterwards.
    public final class Hasher {
        private let handle: OpaquePointer

        /// Create a new streaming hasher.
        public init() {
            guard let h = sha256_c_hasher_new() else {
                fatalError("sha256_c_hasher_new returned null")
            }
            handle = h
        }

        private init(handle: OpaquePointer) { self.handle = handle }

        /// Feed more bytes into the hash.
        public func update(_ data: [UInt8]) {
            data.withUnsafeBufferPointer { input in
                sha256_c_hasher_update(handle, input.baseAddress, data.count)
            }
        }

        /// The 32-byte digest of all data fed so far (non-destructive).
        public func digest() -> [UInt8] {
            var out = [UInt8](repeating: 0, count: 32)
            out.withUnsafeMutableBufferPointer { output in
                sha256_c_hasher_digest(handle, output.baseAddress)
            }
            return out
        }

        /// The 64-character lowercase hex digest string.
        public func hexDigest() -> String { Sha256Native.hex(digest()) }

        /// An independent copy of this hasher (its own native handle).
        public func copy() -> Hasher {
            guard let h = sha256_c_hasher_clone(handle) else {
                fatalError("sha256_c_hasher_clone returned null")
            }
            return Hasher(handle: h)
        }

        deinit { sha256_c_hasher_free(handle) }
    }
}
