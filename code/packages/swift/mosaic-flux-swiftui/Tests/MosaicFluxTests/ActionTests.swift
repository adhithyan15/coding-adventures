import XCTest
@testable import MosaicFlux

private struct ActionTestState: Equatable {
    var count: Int
}

private struct ActionTestIncrement: MosaicAction {
    typealias State = ActionTestState
    func apply(to state: ActionTestState) -> ActionTestState {
        var s = state
        s.count += 1
        return s
    }
}

private struct ActionTestAdd: MosaicAction {
    typealias State = ActionTestState
    let amount: Int
    func apply(to state: ActionTestState) -> ActionTestState {
        var s = state
        s.count += amount
        return s
    }
}

final class ActionTests: XCTestCase {
    func testApplyReturnsNextStateWithoutMutatingInput() {
        let initial = ActionTestState(count: 5)
        let next = ActionTestIncrement().apply(to: initial)
        XCTAssertEqual(next.count, 6)
        XCTAssertEqual(initial.count, 5) // Swift value semantics
    }

    func testPayloadAccessible() {
        let action = ActionTestAdd(amount: 7)
        XCTAssertEqual(action.amount, 7)
        XCTAssertEqual(action.apply(to: ActionTestState(count: 3)).count, 10)
    }

    func testDeterministic() {
        let state = ActionTestState(count: 0)
        let action = ActionTestAdd(amount: 5)
        XCTAssertEqual(action.apply(to: state), action.apply(to: state))
    }
}
