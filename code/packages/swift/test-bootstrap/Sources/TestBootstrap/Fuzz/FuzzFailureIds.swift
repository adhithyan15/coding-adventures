// ============================================================================
// FuzzFailureIds.swift -- the closed vocabulary of fuzz failure ids (VER06 s6)
// ============================================================================
//
// Only these five strings may appear in a public failure line. `fuzz-leak` and
// `fuzz-panic` are registered but this engine cannot produce them (VER06 section
// 9.4 item 9; VER06l section 9 item 10).

public enum FuzzFailureIds {
    public static let crash = "fuzz-crash"
    public static let timeout = "fuzz-timeout"
    public static let oom = "fuzz-oom"
    /// Registered; unproducible here.
    public static let leak = "fuzz-leak"
    /// Registered; unproducible here.
    public static let panic = "fuzz-panic"

    /// Every registered id, in registration order.
    public static let all = [crash, timeout, oom, leak, panic]

    /// True only for the five registered strings (compared byte for byte).
    ///
    ///     isFuzzFailureId("fuzz-crash")           -> true
    ///     isFuzzFailureId("fuzz-arbitrary-error") -> false
    public static func isFuzzFailureId(_ candidate: String?) -> Bool {
        guard let c = candidate else { return false }
        return all.contains { $0.utf8.elementsEqual(c.utf8) }
    }
}
