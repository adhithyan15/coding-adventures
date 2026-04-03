import XCTest
import Affine2D
import Point2D
import Trig

final class Affine2DTests: XCTestCase {
    let eps = 1e-9

    func approx(_ a: Double, _ b: Double) -> Bool { Swift.abs(a - b) < eps }
    func ptEq(_ a: Point, _ b: Point) -> Bool { approx(a.x, b.x) && approx(a.y, b.y) }

    // Identity
    func testIdentityApplyToPoint() {
        let p = Point(3, 4)
        XCTAssert(ptEq(Affine2D.identity.applyToPoint(p), p))
    }
    func testIdentityIsIdentity() {
        XCTAssertTrue(Affine2D.identity.isIdentity)
    }
    func testIdentityDeterminant() {
        XCTAssertEqual(Affine2D.identity.determinant, 1, accuracy: eps)
    }

    // Translation
    func testTranslate() {
        let t = Affine2D.translate(5, -3)
        let r = t.applyToPoint(Point(1, 1))
        XCTAssertEqual(r.x, 6, accuracy: eps)
        XCTAssertEqual(r.y, -2, accuracy: eps)
    }
    func testTranslateIsTranslationOnly() {
        XCTAssertTrue(Affine2D.translate(5, 3).isTranslationOnly)
    }
    func testTranslateVector() {
        // Translation does not affect vectors
        let t = Affine2D.translate(5, -3)
        let v = t.applyToVector(Point(1, 2))
        XCTAssert(ptEq(v, Point(1, 2)))
    }

    // Rotation
    func testRotate90() {
        let r = Affine2D.rotate(PI / 2)
        let p = r.applyToPoint(Point(1, 0))
        XCTAssertEqual(p.x, 0, accuracy: 1e-9)
        XCTAssertEqual(p.y, 1, accuracy: 1e-9)
    }
    func testRotate180() {
        let r = Affine2D.rotate(PI)
        let p = r.applyToPoint(Point(1, 0))
        XCTAssertEqual(p.x, -1, accuracy: 1e-9)
        XCTAssertEqual(p.y, 0, accuracy: 1e-9)
    }
    func testRotateDeterminantIsOne() {
        let r = Affine2D.rotate(PI / 3)
        XCTAssertEqual(r.determinant, 1, accuracy: eps)
    }

    // RotateAround
    func testRotateAround() {
        let pivot = Point(1, 0)
        let r = Affine2D.rotateAround(PI / 2, pivot: pivot)
        // (1, 0) rotated 90° around (1, 0) stays at (1, 0)
        let p = r.applyToPoint(Point(1, 0))
        XCTAssertEqual(p.x, 1, accuracy: 1e-9)
        XCTAssertEqual(p.y, 0, accuracy: 1e-9)
        // (2, 0) rotated 90° CCW around (1, 0) goes to (1, 1)
        let q = r.applyToPoint(Point(2, 0))
        XCTAssertEqual(q.x, 1, accuracy: 1e-9)
        XCTAssertEqual(q.y, 1, accuracy: 1e-9)
    }

    // Scale
    func testScale() {
        let s = Affine2D.scale(2, 3)
        let p = s.applyToPoint(Point(4, 5))
        XCTAssertEqual(p.x, 8, accuracy: eps)
        XCTAssertEqual(p.y, 15, accuracy: eps)
    }
    func testScaleUniform() {
        let s = Affine2D.scaleUniform(4)
        let p = s.applyToPoint(Point(1, 1))
        XCTAssertEqual(p.x, 4, accuracy: eps)
        XCTAssertEqual(p.y, 4, accuracy: eps)
    }

    // Skew
    func testSkewX() {
        // skewX(PI/4): tan(PI/4) = 1, so x' = x + y
        let s = Affine2D.skewX(PI / 4)
        let p = s.applyToPoint(Point(0, 1))
        XCTAssertEqual(p.x, 1, accuracy: 1e-9)
        XCTAssertEqual(p.y, 1, accuracy: 1e-9)
    }
    func testSkewY() {
        let s = Affine2D.skewY(PI / 4)
        let p = s.applyToPoint(Point(1, 0))
        XCTAssertEqual(p.x, 1, accuracy: 1e-9)
        XCTAssertEqual(p.y, 1, accuracy: 1e-9)
    }

    // Composition
    func testThen() {
        // Translate then scale
        let t = Affine2D.translate(1, 0).then(Affine2D.scale(2, 2))
        let p = t.applyToPoint(Point(0, 0))
        // (0,0) → translate → (1,0) → scale → (2,0)
        XCTAssertEqual(p.x, 2, accuracy: eps)
        XCTAssertEqual(p.y, 0, accuracy: eps)
    }
    func testThenRotateTranslate() {
        let t = Affine2D.rotate(PI / 2).then(Affine2D.translate(1, 0))
        let p = t.applyToPoint(Point(1, 0))
        // (1,0) → rotate 90° → (0,1) → translate (1,0) → (1,1)
        XCTAssertEqual(p.x, 1, accuracy: 1e-9)
        XCTAssertEqual(p.y, 1, accuracy: 1e-9)
    }

    // Inversion
    func testInvertIdentity() {
        let inv = Affine2D.identity.inverted
        XCTAssertNotNil(inv)
        XCTAssertTrue(inv!.isIdentity)
    }
    func testInvertTranslate() {
        let inv = Affine2D.translate(3, 4).inverted
        XCTAssertNotNil(inv)
        let p = inv!.applyToPoint(Point(5, 7))
        XCTAssertEqual(p.x, 2, accuracy: eps)
        XCTAssertEqual(p.y, 3, accuracy: eps)
    }
    func testInvertRotate() {
        let rot = Affine2D.rotate(PI / 3)
        let inv = rot.inverted!
        let p = inv.applyToPoint(rot.applyToPoint(Point(2, 3)))
        XCTAssertEqual(p.x, 2, accuracy: 1e-9)
        XCTAssertEqual(p.y, 3, accuracy: 1e-9)
    }
    func testInvertSingular() {
        // scale by 0 → singular
        let s = Affine2D.scale(0, 1)
        XCTAssertNil(s.inverted)
    }

    // toArray
    func testToArray() {
        let arr = Affine2D.identity.toArray()
        XCTAssertEqual(arr, [1, 0, 0, 1, 0, 0])
    }
}
