import XCTest
@testable import MosaicFlux

private struct S { var v: Int }

private struct Bump: MosaicAction {
    typealias State = S
    func apply(to state: S) -> S { var s = state; s.v += 1; return s }
}

final class DevToolsTests: XCTestCase {
    func testDevToolsMiddlewareIsCallable() {
        let m: Middleware<S> = devToolsMiddleware()
        // Should not throw; logs to stdout
        m(Bump(), S(v: 0), S(v: 1))
    }

    func testDevToolsMiddlewareAcceptsCustomStoreName() {
        let m: Middleware<S> = devToolsMiddleware(storeName: "my-grid")
        m(Bump(), S(v: 0), S(v: 1))
    }

    func testIntegratesWithStore() {
        var seen = 0
        let probe: Middleware<S> = { _, _, _ in seen += 1 }
        let store = MosaicStore(
            initialState: S(v: 0),
            middleware: [devToolsMiddleware(), probe]
        )
        store.dispatch(Bump())
        store.dispatch(Bump())
        // The probe runs once per dispatch; devtools middleware
        // doesn't interfere with composition.
        XCTAssertEqual(seen, 2)
        XCTAssertEqual(store.state.v, 2)
    }
}
