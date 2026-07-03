import XCTest
@testable import MosaicFlux

private struct S { var v: Int }

private struct Bump: MosaicAction {
    typealias State = S
    func apply(to state: S) -> S { var s = state; s.v += 1; return s }
}

final class MiddlewareTests: XCTestCase {
    func testEmptyComposeNoOp() {
        let m = composeMiddleware([Middleware<S>]())
        // Should not throw or crash
        m(Bump(), S(v: 0), S(v: 1))
    }

    func testSingleMiddlewareReturnedVerbatim() {
        var called = false
        let m1: Middleware<S> = { _, _, _ in called = true }
        let composed = composeMiddleware([m1])
        composed(Bump(), S(v: 0), S(v: 1))
        XCTAssertTrue(called)
    }

    func testRunsInRegistrationOrder() {
        var calls: [String] = []
        let middlewares: [Middleware<S>] = [
            { _, _, _ in calls.append("a") },
            { _, _, _ in calls.append("b") },
            { _, _, _ in calls.append("c") },
        ]
        let composed = composeMiddleware(middlewares)
        composed(Bump(), S(v: 0), S(v: 1))
        XCTAssertEqual(calls, ["a", "b", "c"])
    }

    func testLoggerMiddlewareDoesNotThrow() {
        let m: Middleware<S> = loggerMiddleware()
        m(Bump(), S(v: 0), S(v: 1))
        // Just verifies the middleware shape; output is to stdout
    }
}
