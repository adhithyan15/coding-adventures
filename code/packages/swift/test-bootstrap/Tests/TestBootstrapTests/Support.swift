import Foundation
import XCTest
@testable import TestBootstrap

/// Shared helpers: locating the committed fixture and building corrupted copies.
enum Support {
    /// Walks up from this source file to the repository root and reads the
    /// COMMITTED fixture (never a copy).
    static func fixtureURL() -> URL {
        var dir = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
        for _ in 0..<12 {
            let candidate = dir.appendingPathComponent("code/specs/fixtures/pbt-v1/cases.json")
            if FileManager.default.fileExists(atPath: candidate.path) { return candidate }
            dir = dir.deletingLastPathComponent()
        }
        fatalError("could not locate code/specs/fixtures/pbt-v1/cases.json above \(#filePath)")
    }

    static func corpusBytes() throws -> [UInt8] { [UInt8](try Data(contentsOf: fixtureURL())) }
    static func cases() throws -> [Json] { try Corpus.parse(try corpusBytes()) }

    static func str(_ s: String) -> Json { .string(s) }
    static func int(_ n: String) -> Json { .object([("kind", .string("int")), ("value", .string(n))]) }
    static func bytes(_ hex: String) -> Json { .object([("kind", .string("bytes")), ("hex", .string(hex))]) }

    /// Replaces `key` of an object (adding it when absent).
    static func setting(_ j: Json, _ key: String, _ value: Json) -> Json {
        guard case .object(var pairs) = j else { return j }
        if let i = pairs.firstIndex(where: { $0.key == key }) { pairs[i].value = value } else { pairs.append((key, value)) }
        return .object(pairs)
    }

    static func id(_ j: Json) -> String { if case .string(let s)? = j["id"] { return s } else { return "" } }
    static func op(_ j: Json) -> String { if case .string(let s)? = j["op"] { return s } else { return "" } }

    /// The decimal text of an `{"kind":"int"}` node plus one.
    static func intPlus(_ j: Json, _ delta: Int) -> Json {
        guard case .string(let t)? = j["value"], let v = try? BigInt.parse(t) else { return j }
        return int((v + BigInt(delta)).description)
    }
}
