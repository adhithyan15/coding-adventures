// ============================================================================
// FuzzSignature.swift -- crash signatures, token sanitising, the one public
// failure line (VER06 sections 5.2 and 6, VER06l sections 5 and 7)
// ============================================================================

import Foundation

public enum FuzzSignature {
    /// Caps the target id in a TAP line, in UTF-8 bytes.
    static let maxTargetLen = 128
    /// The innermost frames kept in a signature.
    static let maxFrames = 64
    /// Caps a crash kind.
    static let maxKindLen = 64
    /// Caps one normalised frame.
    static let maxFrameLen = 128

    /// The low 32 bits of `fnv1a64(kind ":" frames joined by ":")`, as 8
    /// lowercase hex digits.
    ///
    /// Frames are NOT escaped (VER06 section 9.4 item 6): one frame `"x:y"` and
    /// two frames `"x", "y"` hash alike. This lane never produces a frame or a
    /// kind with a colon (`token`), so the ambiguity cannot arise from Swift
    /// targets.
    public static func crashSignature(kind: String, frames: [String]) -> String {
        hex8(UInt32(truncatingIfNeeded: fnv1a64(kind + ":" + frames.joined(separator: ":"))))
    }

    /// Whether two crash reports share one signature.
    public static func isDuplicateCrash(kindA: String, framesA: [String], kindB: String, framesB: [String]) -> Bool {
        crashSignature(kind: kindA, frames: framesA) == crashSignature(kind: kindB, frames: framesB)
    }

    static func hex8(_ v: UInt32) -> String {
        let digits = Array("0123456789abcdef".utf8)
        var out = [UInt8](repeating: 0, count: 8)
        for i in 0..<8 { out[7 - i] = digits[Int((v >> UInt32(4 * i)) & 0xF)] }
        return String(decoding: out, as: UTF8.self)
    }

    // MARK: TAP: the only public failure line ---------------------------------------

    private static func isIdByte(_ c: UInt8) -> Bool {
        (c >= 0x61 && c <= 0x7A) || (c >= 0x41 && c <= 0x5A) || (c >= 0x30 && c <= 0x39)
            || c == 0x2E || c == 0x5F || c == 0x2F || c == 0x2D
    }

    private static func isHexChar(_ c: UInt8) -> Bool { (c >= 0x30 && c <= 0x39) || (c >= 0x61 && c <= 0x66) }

    /// Reduces text to the characters its grammar allows; every other UTF-8
    /// BYTE becomes `_`. Without this a target id holding a newline would let a
    /// caller forge a second TAP line inside "an opaque token". Multi-byte
    /// characters are replaced byte for byte, so the result is pure ASCII. At
    /// most `limit` bytes are produced and only that many are read, so a hostile
    /// megabyte-long id costs O(limit).
    static func token(_ s: String, limit: Int, extra: Bool) -> String {
        var out = [UInt8]()
        out.reserveCapacity(limit)
        for c in s.utf8.prefix(limit) {
            let ok = isIdByte(c) || (extra && (c == 0x3C || c == 0x3E || c == 0x24)) // < > $
            out.append(ok ? c : 0x5F)
        }
        return String(decoding: out, as: UTF8.self)
    }

    /// The only public failure line:
    ///
    ///     not ok N - target [failure-id:hex8]
    ///
    /// It carries no input bytes, seed, message or stack trace (VER00 7.3).
    /// `failureId` must be one of the five registered ids and `sig` exactly 8
    /// lowercase hex digits, else `FuzzError.invalidArgument`. The target is
    /// reduced to `[A-Za-z0-9._/-]` (every other byte becoming `_`) and
    /// truncated to 128 bytes. Well-formed arguments pass through unchanged.
    public static func formatTapFailure(_ testNum: Int, target: String, failureId: String, sig: String) throws -> String {
        guard FuzzFailureIds.isFuzzFailureId(failureId) else {
            throw FuzzError.invalidArgument("failure id is not one of the registered fuzz failure ids")
        }
        let s = Array(sig.utf8)
        guard s.count == 8, s.allSatisfy(isHexChar) else {
            throw FuzzError.invalidArgument("sig hash must be exactly 8 lowercase hex digits")
        }
        return "not ok \(testNum) - \(token(target, limit: maxTargetLen, extra: false)) [\(failureId):\(sig)]"
    }

    /// Renders a report as public, opaque failure lines (none for a clean one):
    /// one per distinct crash signature, then one for timeouts and one for OOMs.
    /// A timeout or OOM has no stack to hash, so its hash is the signature of the
    /// bare kind.
    public static func tapFailures(_ r: FuzzReport, target: String, firstTest: Int) -> [String] {
        var lines: [String] = []
        let invalid = crashSignature(kind: "invalid", frames: [])
        func line(_ id: String, _ sig: String) {
            let n = firstTest &+ lines.count // a hostile firstTest cannot trap
            lines.append((try? formatTapFailure(n, target: target, failureId: id, sig: sig))
                ?? ((try? formatTapFailure(n, target: target, failureId: id, sig: invalid)) ?? ""))
        }
        for rec in r.crashes { line(FuzzFailureIds.crash, rec.signature) }
        if r.timeouts > 0 { line(FuzzFailureIds.timeout, crashSignature(kind: "timeout", frames: [])) }
        if r.ooms > 0 { line(FuzzFailureIds.oom, crashSignature(kind: "oom", frames: [])) }
        return lines
    }

    // MARK: kind and frame normalisation (VER06l section 5.1) ----------------------

    /// A local type's reflected name carries the address of its context:
    /// `Module.(unknown context at $104a3c1f8).Boom`. That differs per run (ASLR),
    /// so it would make a campaign's signatures unstable; it becomes `(local)`.
    static func stripUnstable(_ s: String) -> String {
        let marker = "(unknown context at $"
        guard s.contains(marker) else { return s }
        var out = ""
        var rest = Substring(s)
        while let r = rest.range(of: marker) {
            out += rest[rest.startIndex..<r.lowerBound]
            let after = rest[r.upperBound...]
            if let close = after.firstIndex(of: ")") {
                out += "(local)"
                rest = after[after.index(after: close)...]
            } else {
                out += "(local)"
                rest = Substring("")
            }
        }
        out += rest
        return out
    }

    /// Turns a type's reflected name into a signature kind: stable, sanitised, at most 64 bytes.
    static func normaliseKind(_ typeName: String) -> String {
        token(stripUnstable(typeName.count > 4096 ? String(typeName.prefix(4096)) : typeName), limit: maxKindLen, extra: true)
    }

    /// Turns a caller-supplied scope label into a frame: sanitised, at most 128
    /// bytes, never empty (an empty label is `_`).
    static func normaliseFrame(_ label: String) -> String {
        let t = token(label, limit: maxFrameLen, extra: true)
        return t.isEmpty ? "_" : t
    }

    /// Removes consecutive duplicate frames, so a recursive crash hashes alike at
    /// any depth, and keeps the innermost `maxFrames`.
    static func collapse(_ frames: [String]) -> [String] {
        var out: [String] = []
        for f in frames where out.last != f {
            out.append(f)
            if out.count >= maxFrames { break }
        }
        return out
    }
}
