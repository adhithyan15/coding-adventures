import XCTest
@testable import MosaicFlux

private struct S: Equatable {
    var a: Int
    var b: Int
    var label: String
}

final class SelectorTests: XCTestCase {
    // MARK: single-input

    func testSingleInputRecomputesOnChange() {
        var calls = 0
        let doubled = createSelector(
            { (s: S) in s.a },
            { (a: Int) -> Int in calls += 1; return a * 2 }
        )
        XCTAssertEqual(doubled(S(a: 5, b: 0, label: "")), 10)
        XCTAssertEqual(doubled(S(a: 7, b: 0, label: "")), 14)
        XCTAssertEqual(calls, 2)
    }

    func testSingleInputCachesOnStableInput() {
        var calls = 0
        let doubled = createSelector(
            { (s: S) in s.a },
            { (a: Int) -> Int in calls += 1; return a * 2 }
        )
        let state = S(a: 5, b: 0, label: "")
        _ = doubled(state)
        _ = doubled(state)
        _ = doubled(state)
        XCTAssertEqual(calls, 1)
    }

    func testSingleInputCachesAcrossDifferentStateRefs() {
        var calls = 0
        let doubled = createSelector(
            { (s: S) in s.a },
            { (a: Int) -> Int in calls += 1; return a * 2 }
        )
        _ = doubled(S(a: 5, b: 0, label: ""))
        _ = doubled(S(a: 5, b: 99, label: "different"))
        XCTAssertEqual(calls, 1)
    }

    // MARK: two-input

    func testTwoInputRecomputesWhenEitherChanges() {
        var calls = 0
        let sum = createSelector(
            { (s: S) in s.a },
            { (s: S) in s.b },
            { (a: Int, b: Int) -> Int in calls += 1; return a + b }
        )
        XCTAssertEqual(sum(S(a: 1, b: 2, label: "")), 3)
        XCTAssertEqual(sum(S(a: 1, b: 5, label: "")), 6)
        XCTAssertEqual(sum(S(a: 4, b: 5, label: "")), 9)
        XCTAssertEqual(calls, 3)
    }

    func testTwoInputCachesOnStableInputs() {
        var calls = 0
        let sum = createSelector(
            { (s: S) in s.a },
            { (s: S) in s.b },
            { (a: Int, b: Int) -> Int in calls += 1; return a + b }
        )
        let state = S(a: 1, b: 2, label: "")
        _ = sum(state)
        _ = sum(state)
        XCTAssertEqual(calls, 1)
    }

    // MARK: three-input

    func testThreeInputRecomputesWhenAnyChanges() {
        var calls = 0
        let fmt = createSelector(
            { (s: S) in s.a },
            { (s: S) in s.b },
            { (s: S) in s.label },
            { (a: Int, b: Int, lbl: String) -> String in
                calls += 1; return "\(lbl):\(a + b)"
            }
        )
        XCTAssertEqual(fmt(S(a: 1, b: 2, label: "x")), "x:3")
        XCTAssertEqual(fmt(S(a: 1, b: 2, label: "x")), "x:3")
        XCTAssertEqual(fmt(S(a: 1, b: 2, label: "y")), "y:3")
        XCTAssertEqual(calls, 2)
    }
}
