import XCTest
import Point2D
import Trig

final class Point2DTests: XCTestCase {
    let eps = 1e-9

    func approx(_ a: Double, _ b: Double) -> Bool { abs(a-b) < eps }
    func ptEq(_ a: Point, _ b: Point) -> Bool { approx(a.x, b.x) && approx(a.y, b.y) }

    func testAdd() { XCTAssert(ptEq(Point(1,2).add(Point(3,4)), Point(4,6))) }
    func testSubtract() { XCTAssert(ptEq(Point(5,3).subtract(Point(2,1)), Point(3,2))) }
    func testScale() { XCTAssert(ptEq(Point(2,3).scale(2), Point(4,6))) }
    func testNegate() { XCTAssert(ptEq(Point(1,-2).negate(), Point(-1,2))) }
    func testDot() { XCTAssertEqual(Point(1,2).dot(Point(3,4)), 11, accuracy: eps) }
    func testCross() { XCTAssertEqual(Point(1,2).cross(Point(3,4)), -2, accuracy: eps) }
    func testMagnitude() { XCTAssertEqual(Point(3,4).magnitude, 5, accuracy: eps) }
    func testMagnitudeSquared() { XCTAssertEqual(Point(3,4).magnitudeSquared, 25, accuracy: eps) }
    func testNormalize() { XCTAssertEqual(Point(3,4).normalize().magnitude, 1, accuracy: eps) }
    func testNormalizeZero() {
        let n = Point(0,0).normalize()
        XCTAssertEqual(n.x, 0, accuracy: eps)
    }
    func testDistance() { XCTAssertEqual(Point(0,0).distance(to: Point(3,4)), 5, accuracy: eps) }
    func testLerp() { XCTAssertEqual(Point(0,0).lerp(Point(10,0), 0.5).x, 5, accuracy: eps) }
    func testPerpendicular() { XCTAssert(ptEq(Point(1,0).perpendicular, Point(0,1))) }
    func testAngle() { XCTAssertEqual(Point(1,1).angle, PI/4, accuracy: eps) }

    func testContains() {
        let r = Rect(0,0,10,10)
        XCTAssertTrue(r.containsPoint(Point(5,5)))
        XCTAssertFalse(r.containsPoint(Point(10,5)))
    }
    func testUnion() {
        let u = Rect(0,0,10,10).union(Rect(5,5,10,10))
        XCTAssertEqual(u.x, 0, accuracy: eps)
        XCTAssertEqual(u.width, 15, accuracy: eps)
    }
    func testIntersection() {
        let i = Rect(0,0,10,10).intersection(Rect(5,5,10,10))
        XCTAssertNotNil(i)
        XCTAssertEqual(i!.x, 5, accuracy: eps)
    }
    func testIntersectionDisjoint() {
        XCTAssertNil(Rect(0,0,5,5).intersection(Rect(10,10,5,5)))
    }
    func testExpandBy() {
        let e = Rect(0,0,10,10).expandedBy(2)
        XCTAssertEqual(e.x, -2, accuracy: eps)
        XCTAssertEqual(e.width, 14, accuracy: eps)
    }
}
