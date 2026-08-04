// Md5Native.swift — Swift wrapper over the Rust md5-c C ABI.
//
// Native-through-Rust MD5, companion to the pure-Swift `md5` package. Links the
// Rust `coding_adventures_md5` crate at compile time via the `md5-c` static
// library and calls it through the `CMd5` C module.
//
// SECURITY: MD5 is cryptographically broken — checksum use only.
//
// PREREQUISITE: `libmd5_c.a` must be compiled and copied into `Sources/CMd5/`
// before building — see the BUILD file.

import CMd5

/// MD5 backed by the Rust `md5-c` static library.
public enum Md5Native {

    /// The 16-byte MD5 digest of `data` (computed in Rust).
    public static func digest(_ data: [UInt8]) -> [UInt8] {
        var out = [UInt8](repeating: 0, count: 16)
        data.withUnsafeBufferPointer { input in
            out.withUnsafeMutableBufferPointer { output in
                md5_c_digest(input.baseAddress, data.count, output.baseAddress)
            }
        }
        return out
    }

    /// The 32-character lowercase hex digest of `data` (computed in Rust).
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

    /// A streaming MD5 hasher backed by a native Rust hasher handle, freed in
    /// `deinit`. `digest()` is non-destructive.
    public final class Hasher {
        private let handle: OpaquePointer

        public init() {
            guard let h = md5_c_hasher_new() else { fatalError("md5_c_hasher_new returned null") }
            handle = h
        }
        private init(handle: OpaquePointer) { self.handle = handle }

        public func update(_ data: [UInt8]) {
            data.withUnsafeBufferPointer { input in
                md5_c_hasher_update(handle, input.baseAddress, data.count)
            }
        }
        public func digest() -> [UInt8] {
            var out = [UInt8](repeating: 0, count: 16)
            out.withUnsafeMutableBufferPointer { output in
                md5_c_hasher_digest(handle, output.baseAddress)
            }
            return out
        }
        public func hexDigest() -> String { Md5Native.hex(digest()) }
        public func copy() -> Hasher {
            guard let h = md5_c_hasher_clone(handle) else { fatalError("md5_c_hasher_clone returned null") }
            return Hasher(handle: h)
        }
        deinit { md5_c_hasher_free(handle) }
    }
}
