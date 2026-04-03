import XCTest
import Arc2D
import Point2D
import Trig

final class Arc2DTests: XCTestCase {
    let eps = 1e-6

    func approx(_ a: Double, _ b: Double, _ tol: Double = 1e-6) -> Bool { Swift.abs(a-b) < tol }
    func ptEq(_ a: Point, _ b: Point, _ tol: Double = 1e-6) -> Bool {
        Swift.abs(a.x - b.x) < tol && Swift.abs(a.y - b.y) < tol
    }

    // =========================================================================
    // CenterArc — eval
    // =========================================================================

    func testEvalStartPoint() {
        // Unit circle arc starting at angle 0
        let arc = CenterArc(center: Point(0,0), rx: 1, ry: 1,
                            startAngle: 0, sweepAngle: PI/2, xRotation: 0)
        let p = arc.eval(0)
        XCTAssertEqual(p.x, 1, accuracy: eps)
        XCTAssertEqual(p.y, 0, accuracy: eps)
    }

    func testEvalEndPoint() {
        // Quarter arc from 0 to π/2: end point should be (0, 1)
        let arc = CenterArc(center: Point(0,0), rx: 1, ry: 1,
                            startAngle: 0, sweepAngle: PI/2, xRotation: 0)
        let p = arc.eval(1)
        XCTAssertEqual(p.x, 0, accuracy: eps)
        XCTAssertEqual(p.y, 1, accuracy: eps)
    }

    func testEvalMidpoint() {
        // Mid of quarter arc: angle = π/4 → (cos π/4, sin π/4)
        let arc = CenterArc(center: Point(0,0), rx: 1, ry: 1,
                            startAngle: 0, sweepAngle: PI/2, xRotation: 0)
        let p = arc.eval(0.5)
        let expected = Trig.sqrt(0.5)
        XCTAssertEqual(p.x, expected, accuracy: eps)
        XCTAssertEqual(p.y, expected, accuracy: eps)
    }

    func testEvalWithOffset() {
        // Arc centered at (2, 3)
        let arc = CenterArc(center: Point(2,3), rx: 1, ry: 1,
                            startAngle: 0, sweepAngle: PI/2, xRotation: 0)
        let p = arc.eval(0)
        XCTAssertEqual(p.x, 3, accuracy: eps)
        XCTAssertEqual(p.y, 3, accuracy: eps)
    }

    // =========================================================================
    // CenterArc — boundingBox
    // =========================================================================

    func testBboxFullCircle() {
        // Full circle radius 1: bbox should be (-1,-1,2,2) approximately
        let arc = CenterArc(center: Point(0,0), rx: 1, ry: 1,
                            startAngle: 0, sweepAngle: 2*PI, xRotation: 0)
        let bb = arc.boundingBox
        XCTAssertEqual(bb.x, -1, accuracy: 0.02)
        XCTAssertEqual(bb.y, -1, accuracy: 0.02)
        XCTAssertEqual(bb.width, 2, accuracy: 0.02)
        XCTAssertEqual(bb.height, 2, accuracy: 0.02)
    }

    func testBboxQuarterArc() {
        // Quarter arc from 0 to π/2: from (1,0) to (0,1)
        let arc = CenterArc(center: Point(0,0), rx: 1, ry: 1,
                            startAngle: 0, sweepAngle: PI/2, xRotation: 0)
        let bb = arc.boundingBox
        XCTAssertEqual(bb.x, 0, accuracy: 0.02)
        XCTAssertEqual(bb.y, 0, accuracy: 0.02)
        XCTAssertEqual(bb.width, 1, accuracy: 0.02)
        XCTAssertEqual(bb.height, 1, accuracy: 0.02)
    }

    // =========================================================================
    // CenterArc — toCubicBeziers
    // =========================================================================

    func testQuarterArcOneSegment() {
        let arc = CenterArc(center: Point(0,0), rx: 1, ry: 1,
                            startAngle: 0, sweepAngle: PI/2, xRotation: 0)
        let curves = arc.toCubicBeziers()
        XCTAssertEqual(curves.count, 1)
    }

    func testFullCircleFourSegments() {
        let arc = CenterArc(center: Point(0,0), rx: 1, ry: 1,
                            startAngle: 0, sweepAngle: 2*PI, xRotation: 0)
        let curves = arc.toCubicBeziers()
        XCTAssertEqual(curves.count, 4)
    }

    func testCubicBezierEndpoints() {
        // First and last cubic should touch arc start/end
        let arc = CenterArc(center: Point(0,0), rx: 1, ry: 1,
                            startAngle: 0, sweepAngle: PI/2, xRotation: 0)
        let curves = arc.toCubicBeziers()
        XCTAssert(ptEq(curves.first!.p0, arc.eval(0)))
        XCTAssert(ptEq(curves.last!.p3, arc.eval(1)))
    }

    func testCubicBezierContinuity() {
        // For a multi-segment arc, consecutive beziers must be C0-continuous
        let arc = CenterArc(center: Point(0,0), rx: 1, ry: 1,
                            startAngle: 0, sweepAngle: 2*PI, xRotation: 0)
        let curves = arc.toCubicBeziers()
        for i in 0..<(curves.count-1) {
            XCTAssert(ptEq(curves[i].p3, curves[i+1].p0))
        }
    }

    // =========================================================================
    // SvgArc — toCenterArc
    // =========================================================================

    func testDegenerateSamePoint() {
        let svg = SvgArc(from: Point(0,0), to: Point(0,0),
                         rx: 1, ry: 1, xRotation: 0,
                         largeArc: false, sweep: true)
        XCTAssertNil(svg.toCenterArc())
    }

    func testSemicircle() {
        // Semicircle from (1,0) to (-1,0) on unit circle, sweep=true (CCW)
        let svg = SvgArc(from: Point(1,0), to: Point(-1,0),
                         rx: 1, ry: 1, xRotation: 0,
                         largeArc: false, sweep: true)
        let center = svg.toCenterArc()
        XCTAssertNotNil(center)
        // Center should be at origin
        XCTAssertEqual(center!.center.x, 0, accuracy: eps)
        XCTAssertEqual(center!.center.y, 0, accuracy: eps)
        // Sweep should be π (semicircle)
        XCTAssertEqual(Swift.abs(center!.sweepAngle), PI, accuracy: eps)
    }

    func testRoundtripEndpoints() {
        // Start and end points should match the original svg arc endpoints
        let svg = SvgArc(from: Point(1,0), to: Point(0,1),
                         rx: 1, ry: 1, xRotation: 0,
                         largeArc: false, sweep: true)
        let ca = svg.toCenterArc()!
        XCTAssert(ptEq(ca.eval(0), svg.from, eps))
        XCTAssert(ptEq(ca.eval(1), svg.to, eps))
    }
}
