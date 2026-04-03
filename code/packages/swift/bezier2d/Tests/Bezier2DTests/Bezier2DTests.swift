import XCTest
import Bezier2D
import Point2D
import Trig

final class Bezier2DTests: XCTestCase {
    let eps = 1e-9

    func approx(_ a: Double, _ b: Double) -> Bool { Swift.abs(a - b) < eps }
    func ptEq(_ a: Point, _ b: Point, _ tol: Double = 1e-9) -> Bool {
        Swift.abs(a.x - b.x) < tol && Swift.abs(a.y - b.y) < tol
    }

    // =========================================================================
    // QuadraticBezier
    // =========================================================================

    func testQuadEvalEndpoints() {
        let q = QuadraticBezier(Point(0,0), Point(1,2), Point(2,0))
        XCTAssert(ptEq(q.eval(0), Point(0,0)))
        XCTAssert(ptEq(q.eval(1), Point(2,0)))
    }

    func testQuadEvalMidpoint() {
        // Midpoint of quad with P0=(0,0), P1=(1,1), P2=(2,0): B(0.5) = (1, 0.5)
        let q = QuadraticBezier(Point(0,0), Point(1,1), Point(2,0))
        let m = q.eval(0.5)
        XCTAssertEqual(m.x, 1, accuracy: eps)
        XCTAssertEqual(m.y, 0.5, accuracy: eps)
    }

    func testQuadDerivativeAtZero() {
        // B'(0) = 2(P1 - P0)
        let q = QuadraticBezier(Point(0,0), Point(1,2), Point(2,0))
        let d = q.derivative(0)
        XCTAssertEqual(d.x, 2, accuracy: eps)
        XCTAssertEqual(d.y, 4, accuracy: eps)
    }

    func testQuadSplitEndpoints() {
        let q = QuadraticBezier(Point(0,0), Point(1,2), Point(4,0))
        let (left, right) = q.split(0.5)
        XCTAssert(ptEq(left.p0, q.p0))
        XCTAssert(ptEq(right.p2, q.p2))
        XCTAssert(ptEq(left.p2, right.p0))  // junction point
    }

    func testQuadSplitJunctionOnCurve() {
        let q = QuadraticBezier(Point(0,0), Point(1,2), Point(2,0))
        let (left, _) = q.split(0.5)
        XCTAssert(ptEq(left.p2, q.eval(0.5)))
    }

    func testQuadPolylineIncludes() {
        let q = QuadraticBezier(Point(0,0), Point(1,1), Point(2,0))
        let pts = q.polyline(tolerance: 0.1)
        XCTAssert(pts.count >= 2)
        XCTAssert(ptEq(pts.first!, q.p0))
        XCTAssert(ptEq(pts.last!, q.p2))
    }

    func testQuadBboxFlat() {
        // Flat curve: P0=(0,0), P1=(1,0), P2=(2,0) — a straight line
        let q = QuadraticBezier(Point(0,0), Point(1,0), Point(2,0))
        let bb = q.boundingBox
        XCTAssertEqual(bb.x, 0, accuracy: eps)
        XCTAssertEqual(bb.width, 2, accuracy: eps)
        XCTAssertEqual(bb.height, 0, accuracy: eps)
    }

    func testQuadBboxCurved() {
        // Upward parabola: P0=(0,0), P1=(1,2), P2=(2,0)
        // Max y occurs at t=0.5: y=0.5
        let q = QuadraticBezier(Point(0,0), Point(1,2), Point(2,0))
        let bb = q.boundingBox
        XCTAssertEqual(bb.y, 0, accuracy: eps)
        XCTAssertEqual(bb.height, 1, accuracy: eps)  // y at t=0.5 is 1
    }

    func testQuadElevate() {
        let q = QuadraticBezier(Point(0,0), Point(1,2), Point(2,0))
        let c = q.elevate()
        // Elevated cubic must have same endpoints
        XCTAssert(ptEq(c.p0, q.p0))
        XCTAssert(ptEq(c.p3, q.p2))
        // Midpoint must match
        XCTAssert(ptEq(c.eval(0.5), q.eval(0.5), 1e-9))
    }

    // =========================================================================
    // CubicBezier
    // =========================================================================

    func testCubicEvalEndpoints() {
        let c = CubicBezier(Point(0,0), Point(1,2), Point(3,2), Point(4,0))
        XCTAssert(ptEq(c.eval(0), Point(0,0)))
        XCTAssert(ptEq(c.eval(1), Point(4,0)))
    }

    func testCubicDerivativeAtZero() {
        // B'(0) = 3(P1 - P0)
        let c = CubicBezier(Point(0,0), Point(1,2), Point(3,2), Point(4,0))
        let d = c.derivative(0)
        XCTAssertEqual(d.x, 3, accuracy: eps)
        XCTAssertEqual(d.y, 6, accuracy: eps)
    }

    func testCubicDerivativeAtOne() {
        // B'(1) = 3(P3 - P2)
        let c = CubicBezier(Point(0,0), Point(1,2), Point(3,2), Point(4,0))
        let d = c.derivative(1)
        XCTAssertEqual(d.x, 3, accuracy: eps)
        XCTAssertEqual(d.y, -6, accuracy: eps)
    }

    func testCubicSplitEndpoints() {
        let c = CubicBezier(Point(0,0), Point(1,3), Point(3,3), Point(4,0))
        let (left, right) = c.split(0.5)
        XCTAssert(ptEq(left.p0, c.p0))
        XCTAssert(ptEq(right.p3, c.p3))
        XCTAssert(ptEq(left.p3, right.p0))
    }

    func testCubicSplitJunctionOnCurve() {
        let c = CubicBezier(Point(0,0), Point(1,3), Point(3,3), Point(4,0))
        let (left, _) = c.split(0.5)
        XCTAssert(ptEq(left.p3, c.eval(0.5), 1e-9))
    }

    func testCubicPolylineIncludes() {
        let c = CubicBezier(Point(0,0), Point(0,4), Point(4,4), Point(4,0))
        let pts = c.polyline(tolerance: 0.1)
        XCTAssert(pts.count >= 2)
        XCTAssert(ptEq(pts.first!, c.p0))
        XCTAssert(ptEq(pts.last!, c.p3))
    }

    func testCubicBboxLine() {
        let c = CubicBezier(Point(0,0), Point(1,0), Point(2,0), Point(3,0))
        let bb = c.boundingBox
        XCTAssertEqual(bb.x, 0, accuracy: eps)
        XCTAssertEqual(bb.width, 3, accuracy: eps)
        XCTAssertEqual(bb.height, 0, accuracy: eps)
    }
}
