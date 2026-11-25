import Foundation
import XCTest
@testable import TestBootstrap

/// Shared helpers for the fuzz tests: locating the committed fixtures.
enum FuzzSupport {
    /// Walks up from this source file to the repository root and returns `relative` under it.
    static func repoFile(_ relative: String) -> URL {
        var dir = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
        for _ in 0..<12 {
            let candidate = dir.appendingPathComponent(relative)
            if FileManager.default.fileExists(atPath: candidate.path) { return candidate }
            dir = dir.deletingLastPathComponent()
        }
        fatalError("could not locate \(relative) above \(#filePath)")
    }

    static func fixtureURL() -> URL { repoFile("code/specs/fixtures/fuzz-v1/cases.json") }
    static func corpusBytes() throws -> [UInt8] { [UInt8](try Data(contentsOf: fixtureURL())) }
    static func cases() throws -> [Json] { try FuzzCorpus.parse(try corpusBytes()) }

    /// A deterministic error for targets.
    struct Boom: Error {}
    struct Other: Error {}

    /// Waits for a semaphore with a hard ceiling so a regression FAILS instead of hanging.
    static func wait(_ s: DispatchSemaphore, _ seconds: Double = 10) -> Bool {
        s.wait(timeout: .now() + seconds) == .success
    }

    static func hex(_ s: String) -> [UInt8] {
        let b = Array(s.utf8)
        func v(_ c: UInt8) -> UInt8 { c <= 0x39 ? c - 0x30 : c - 0x61 + 10 }
        return stride(from: 0, to: b.count, by: 2).map { v(b[$0]) << 4 | v(b[$0 + 1]) }
    }
}
