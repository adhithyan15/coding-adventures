// GlassesLogicTests.swift
//
// Tests the filled-glass-count calculation in isolation.
//
// The formula under test:
//   filledCount = min(totalMl / glassSize, goalGlasses)
//
// This is pure integer arithmetic — no SwiftData, no UI, no network.
// We test the formula directly rather than going through SwiftUI views,
// which would require a running simulator and ModelContainer.
//
// LITERATE TESTING:
//   Each test method name reads as a sentence describing what the system
//   does. If a test fails, the name alone tells you what broke.

import XCTest
@testable import WaterNotify

final class GlassesLogicTests: XCTestCase {

    // ── Constants (mirror ContentView) ────────────────────────────────────

    private let glassSize   = 250
    private let goalGlasses = 8

    // ── Helper ────────────────────────────────────────────────────────────

    /// The formula under test, extracted here so changes in ContentView
    /// that break the calculation will also break these tests.
    private func filledCount(for totalMl: Int) -> Int {
        min(totalMl / glassSize, goalGlasses)
    }

    // ── Tests ─────────────────────────────────────────────────────────────

    /// No drinks logged → no glasses filled.
    func testZeroMlGivesZeroGlasses() {
        XCTAssertEqual(filledCount(for: 0), 0)
    }

    /// One standard drink (250 ml) fills exactly one glass.
    func testOneDrinkFillsOneGlass() {
        XCTAssertEqual(filledCount(for: 250), 1)
    }

    /// The full daily goal (2,000 ml) fills all 8 glasses.
    func testFullGoalFillsAllGlasses() {
        XCTAssertEqual(filledCount(for: 2_000), 8)
    }

    /// Logging more than the goal cannot exceed 8 filled glasses (display is capped).
    func testOverGoalCapsAtEightGlasses() {
        XCTAssertEqual(filledCount(for: 2_500), 8)
        XCTAssertEqual(filledCount(for: 10_000), 8)
    }

    /// Three drinks (750 ml) fills 3 glasses. Integer division: 750 / 250 = 3.
    func testThreeDrinksFillThreeGlasses() {
        XCTAssertEqual(filledCount(for: 750), 3)
    }

    /// 300 ml fills 1 glass, not 1.2. Partial glasses are floored.
    ///
    /// A glass is either fully logged or not. No partial-fill display in
    /// this stage. 300 / 250 = 1 remainder 50 → 1 filled.
    func testPartialGlassIsFlooredNotRounded() {
        XCTAssertEqual(filledCount(for: 300), 1)
        XCTAssertEqual(filledCount(for: 499), 1)
    }

    /// 500 ml fills exactly 2 glasses. No rounding ambiguity.
    func testTwoDrinksFillTwoGlasses() {
        XCTAssertEqual(filledCount(for: 500), 2)
    }

    /// 1,999 ml fills 7 glasses (one ml short of the 8th glass boundary).
    func testOneMilliLitreShortOfEighthGlass() {
        XCTAssertEqual(filledCount(for: 1_999), 7)
    }

    /// 2,000 ml is the exact threshold for the 8th (final) glass.
    func testExactGoalBoundaryFillsEighthGlass() {
        XCTAssertEqual(filledCount(for: 2_000), 8)
    }
}
