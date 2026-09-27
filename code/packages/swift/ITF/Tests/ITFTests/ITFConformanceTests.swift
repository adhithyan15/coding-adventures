import Foundation
import SHA256
import XCTest
@testable import ITF

final class ITFConformanceTests: XCTestCase {
    func testSharedITFCorpusConforms() throws {
        let data = try Data(contentsOf: fixtureURL())
        let root = try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
        let cases = try XCTUnwrap(root["cases"] as? [[String: Any]])

        for testCase in cases where testCase["symbology"] as? String == "itf" {
            let input = try materialize(try XCTUnwrap(testCase["input"] as? [String: Any]))
            let expected = try XCTUnwrap(testCase["expected"] as? [String: Any])
            if let errorID = expected["error"] as? String {
                XCTAssertThrowsError(try normalizeITF(input)) { error in
                    XCTAssertEqual(itfErrorID(error), errorID)
                }
                continue
            }

            let normalized = try normalizeITF(input)
            let modules = "1010" + (try encodeITF(input)).map(\.binaryPattern).joined() + "11101"
            let runs = runLengths(modules)
            if let expectedNormalized = expected["normalized"] as? String {
                XCTAssertEqual(normalized, expectedNormalized)
                XCTAssertEqual(modules, expected["modules"] as? String)
                XCTAssertEqual(runs, expected["run_lengths"] as? [Int])
            } else {
                XCTAssertEqual(sha256Hex(Data(normalized.utf8)), expected["normalized_sha256"] as? String)
                XCTAssertEqual(runs.count, expected["run_count"] as? Int)
                let encodedRuns = "[" + runs.map(String.init).joined(separator: ",") + "]"
                XCTAssertEqual(sha256Hex(Data(encodedRuns.utf8)), expected["run_lengths_sha256"] as? String)
            }
            XCTAssertEqual(modules.utf8.count, expected["module_count"] as? Int)
            XCTAssertEqual(sha256Hex(Data(modules.utf8)), expected["module_sha256"] as? String)
        }
    }

    private func materialize(_ input: [String: Any]) throws -> String {
        if let text = input["text"] as? String { return text }
        let repeatSpec = try XCTUnwrap(input["repeat"] as? [String: Any])
        return String(repeating: try XCTUnwrap(repeatSpec["text"] as? String),
                      count: try XCTUnwrap(repeatSpec["count"] as? Int))
    }

    private func runLengths(_ modules: String) -> [Int] {
        var previous: Character?
        var lengths: [Int] = []
        for bit in modules {
            if bit == previous { lengths[lengths.count - 1] += 1 }
            else { lengths.append(1); previous = bit }
        }
        return lengths
    }

    private func fixtureURL() throws -> URL {
        var cursor = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
        for _ in 0..<12 {
            let candidate = cursor.appendingPathComponent("code/specs/fixtures/barcode-symbologies-v1/cases.json")
            if FileManager.default.fileExists(atPath: candidate.path) { return candidate }
            cursor.deleteLastPathComponent()
        }
        throw CocoaError(.fileNoSuchFile)
    }
}
