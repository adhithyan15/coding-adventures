// ============================================================================
// ConformanceTests.swift — Conformance Test Suite for Hamurabi
// ============================================================================

import XCTest
import Foundation
@testable import Hamurabi

final class ConformanceTests: XCTestCase {
    struct CaseDocument: Codable {
        let version: Int
        let suite: String
        let cases: [TestCase]
    }

    struct TestCase: Codable {
        let id: String
        let description: String?
        let seed: UInt64?
        let initial_state: HamurabiState
        let turns: [TurnCase]?
        let order: TurnOrder?
        let expected_error: String?
    }

    struct TurnCase: Codable {
        let turn: Int
        let order: TurnOrder
        let expected_state: HamurabiState
        let expected_report: String
    }

    func findCasesJsonURL() -> URL? {
        let candidates = [
            "../../../specs/fixtures/hamurabi-v1/cases.json",
            "../../specs/fixtures/hamurabi-v1/cases.json",
            "code/specs/fixtures/hamurabi-v1/cases.json",
            "../specs/fixtures/hamurabi-v1/cases.json"
        ]
        let fm = FileManager.default
        let currentDir = fm.currentDirectoryPath
        for rel in candidates {
            let fullPath = (currentDir as NSString).appendingPathComponent(rel)
            if fm.fileExists(atPath: fullPath) {
                return URL(fileURLWithPath: fullPath)
            }
        }
        let thisFile = URL(fileURLWithPath: #filePath)
        let repoRoot = thisFile.deletingLastPathComponent() // Tests/HamurabiTests
            .deletingLastPathComponent() // Tests
            .deletingLastPathComponent() // hamurabi
            .deletingLastPathComponent() // swift
            .deletingLastPathComponent() // packages
            .deletingLastPathComponent() // code
        let fixtureURL = repoRoot.appendingPathComponent("code/specs/fixtures/hamurabi-v1/cases.json")
        if fm.fileExists(atPath: fixtureURL.path) {
            return fixtureURL
        }
        return nil
    }

    func testConformanceAgainstCasesJson() throws {
        guard let url = findCasesJsonURL() else {
            XCTFail("Could not find code/specs/fixtures/hamurabi-v1/cases.json")
            return
        }

        let data = try Data(contentsOf: url)
        let doc = try JSONDecoder().decode(CaseDocument.self, from: data)
        XCTAssertGreaterThanOrEqual(doc.cases.count, 31, "Corpus must have at least 31 cases")

        var passed = 0
        for c in doc.cases {
            let initialState = c.initial_state

            if let turns = c.turns {
                if let seed = c.seed, initialState.year == 1 {
                    let generated = newGame(seed: seed)
                    XCTAssertEqual(generated, initialState, "newGame mismatch on case \(c.id)")
                }

                var currentState = initialState
                for t in turns {
                    let result = stepYear(state: currentState, order: t.order)
                    switch result {
                    case .failure(let err):
                        XCTFail("case \(c.id) turn \(t.turn) stepYear failed unexpectedly with error: \(err)")
                        return
                    case .success(let nextState):
                        XCTAssertEqual(nextState, t.expected_state, "case \(c.id) turn \(t.turn) state mismatch")
                        let report = formatReport(state: nextState)
                        XCTAssertEqual(report, t.expected_report, "case \(c.id) turn \(t.turn) report mismatch")
                        currentState = nextState
                    }
                }
                passed += 1
            } else if let expectedErrorCode = c.expected_error, let order = c.order {
                guard let expectedErr = HamurabiError(rawValue: expectedErrorCode) else {
                    XCTFail("Unknown expected error code: \(expectedErrorCode) in case \(c.id)")
                    return
                }

                let valRes = validateOrder(state: initialState, order: order)
                switch valRes {
                case .success:
                    XCTFail("case \(c.id) validateOrder succeeded unexpectedly, expected \(expectedErr)")
                case .failure(let valErr):
                    XCTAssertEqual(valErr, expectedErr, "case \(c.id) validateOrder error mismatch")
                }

                let stepRes = stepYear(state: initialState, order: order)
                switch stepRes {
                case .success:
                    XCTFail("case \(c.id) stepYear succeeded unexpectedly, expected \(expectedErr)")
                case .failure(let stepErr):
                    XCTAssertEqual(stepErr, expectedErr, "case \(c.id) stepYear error mismatch")
                }
                passed += 1
            } else {
                XCTFail("case \(c.id) has neither turns nor expected_error")
            }
        }

        XCTAssertEqual(passed, doc.cases.count, "All cases in corpus must execute and pass")
    }
}
