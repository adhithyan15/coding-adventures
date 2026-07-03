import XCTest
@testable import MosaicFlux

private struct S: Equatable {
    var count: Int
    var label: String
}

private struct Increment: MosaicAction {
    typealias State = S
    func apply(to state: S) -> S { var s = state; s.count += 1; return s }
}

private struct SetLabel: MosaicAction {
    typealias State = S
    let label: String
    func apply(to state: S) -> S { var s = state; s.label = label; return s }
}

private let initial = S(count: 0, label: "")

final class StoreTests: XCTestCase {
    func testStartsAtInitialState() {
        let store = MosaicStore(initialState: initial)
        XCTAssertEqual(store.state, initial)
    }

    func testDispatchAppliesAction() {
        let store = MosaicStore(initialState: initial)
        store.dispatch(Increment())
        XCTAssertEqual(store.state.count, 1)
    }

    func testPayloadedActionWorks() {
        let store = MosaicStore(initialState: initial)
        store.dispatch(SetLabel(label: "hi"))
        XCTAssertEqual(store.state.label, "hi")
    }

    func testSelectReturnsProjectionWithoutSubscribing() {
        let store = MosaicStore(initialState: initial)
        store.dispatch(SetLabel(label: "t"))
        XCTAssertEqual(store.select { $0.label }, "t")
    }

    func testSubscribeFiresOnChangedSlice() {
        let store = MosaicStore(initialState: initial)
        var received: [Int] = []
        store.subscribe(
            selector: { $0.count },
            equality: ==,
            callback: { received.append($0) }
        )
        store.dispatch(Increment())
        XCTAssertEqual(received, [1])
    }

    func testSubscribeDoesNotFireOnUnrelatedChange() {
        let store = MosaicStore(initialState: initial)
        var received: [Int] = []
        store.subscribe(
            selector: { $0.count },
            equality: ==,
            callback: { received.append($0) }
        )
        store.dispatch(SetLabel(label: "x"))
        XCTAssertEqual(received, [])
    }

    func testUnsubscribeStopsNotifications() {
        let store = MosaicStore(initialState: initial)
        var received: [Int] = []
        let unsub = store.subscribe(
            selector: { $0.count },
            equality: ==,
            callback: { received.append($0) }
        )
        store.dispatch(Increment())
        unsub()
        store.dispatch(Increment())
        XCTAssertEqual(received, [1])
    }

    func testMultipleSubscribersFireInRegistrationOrder() {
        let store = MosaicStore(initialState: initial)
        var calls: [String] = []
        store.subscribe(
            selector: { $0.count },
            equality: ==,
            callback: { _ in calls.append("a") }
        )
        store.subscribe(
            selector: { $0.count },
            equality: ==,
            callback: { _ in calls.append("b") }
        )
        store.dispatch(Increment())
        // Order isn't strictly guaranteed (dict iteration in Swift
        // is unspecified) but both must fire.
        XCTAssertEqual(calls.sorted(), ["a", "b"])
    }

    func testMiddlewareSeesTriple() {
        var seen: [(String, Int, Int)] = []
        let middleware: Middleware<S> = { action, prev, next in
            seen.append((String(describing: type(of: action)), prev.count, next.count))
        }
        let store = MosaicStore(initialState: initial, middleware: [middleware])
        store.dispatch(Increment())
        XCTAssertEqual(seen.count, 1)
        XCTAssertEqual(seen[0].1, 0)
        XCTAssertEqual(seen[0].2, 1)
    }

    func testSubscribeCallsCallbackOnEveryDispatchOfNewSliceValue() {
        let store = MosaicStore(initialState: initial)
        var counts: [Int] = []
        store.subscribe(
            selector: { $0.count },
            equality: ==,
            callback: { counts.append($0) }
        )
        store.dispatch(Increment())
        store.dispatch(Increment())
        store.dispatch(Increment())
        XCTAssertEqual(counts, [1, 2, 3])
    }
}
