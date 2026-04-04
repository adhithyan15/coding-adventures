import XCTest
@testable import Matrix

final class MatrixTests: XCTestCase {

    private let eps = 1e-10

    // ========================================================================
    // MARK: - Initialization
    // ========================================================================

    func testFromScalar() {
        let m = Matrix(5.0)
        XCTAssertEqual(m.rows, 1)
        XCTAssertEqual(m.cols, 1)
        XCTAssertEqual(m[0, 0], 5.0)
    }

    func testFrom1DArray() {
        let m = Matrix([1.0, 2.0, 3.0])
        XCTAssertEqual(m.rows, 1)
        XCTAssertEqual(m.cols, 3)
        XCTAssertEqual(m[0, 1], 2.0)
    }

    func testFrom2DArray() {
        let m = Matrix([[1.0, 2.0], [3.0, 4.0]])
        XCTAssertEqual(m.rows, 2)
        XCTAssertEqual(m.cols, 2)
        XCTAssertEqual(m[1, 0], 3.0)
    }

    func testZeros() {
        let m = Matrix.zeros(rows: 3, cols: 2)
        XCTAssertEqual(m.rows, 3)
        XCTAssertEqual(m.cols, 2)
        for i in 0..<3 {
            for j in 0..<2 {
                XCTAssertEqual(m[i, j], 0.0)
            }
        }
    }

    // ========================================================================
    // MARK: - Addition
    // ========================================================================

    func testAddMatrices() {
        let a = Matrix([[1.0, 2.0], [3.0, 4.0]])
        let b = Matrix([[5.0, 6.0], [7.0, 8.0]])
        let c = a.add(b)
        XCTAssertEqual(c, Matrix([[6.0, 8.0], [10.0, 12.0]]))
    }

    func testAddScalar() {
        let m = Matrix([[1.0, 2.0], [3.0, 4.0]])
        let r = m.add(10.0)
        XCTAssertEqual(r, Matrix([[11.0, 12.0], [13.0, 14.0]]))
    }

    func testAddZeros() {
        let a = Matrix([[1.0, 2.0]])
        let z = Matrix.zeros(rows: 1, cols: 2)
        XCTAssertEqual(a.add(z), a)
    }

    // ========================================================================
    // MARK: - Subtraction
    // ========================================================================

    func testSubtractMatrices() {
        let a = Matrix([[5.0, 6.0], [7.0, 8.0]])
        let b = Matrix([[1.0, 2.0], [3.0, 4.0]])
        let c = a.subtract(b)
        XCTAssertEqual(c, Matrix([[4.0, 4.0], [4.0, 4.0]]))
    }

    func testSubtractSelf() {
        let a = Matrix([[1.0, 2.0], [3.0, 4.0]])
        let z = a.subtract(a)
        XCTAssertEqual(z, Matrix.zeros(rows: 2, cols: 2))
    }

    func testSubtractScalar() {
        let m = Matrix([[10.0, 20.0]])
        let r = m.subtract(5.0)
        XCTAssertEqual(r, Matrix([[5.0, 15.0]]))
    }

    // ========================================================================
    // MARK: - Scale
    // ========================================================================

    func testScale() {
        let m = Matrix([[1.0, 2.0], [3.0, 4.0]])
        let r = m.scale(2.0)
        XCTAssertEqual(r, Matrix([[2.0, 4.0], [6.0, 8.0]]))
    }

    func testScaleByZero() {
        let m = Matrix([[1.0, 2.0]])
        let r = m.scale(0.0)
        XCTAssertEqual(r, Matrix.zeros(rows: 1, cols: 2))
    }

    func testScaleByNegativeOne() {
        let m = Matrix([[1.0, -2.0]])
        let r = m.scale(-1.0)
        XCTAssertEqual(r, Matrix([[-1.0, 2.0]]))
    }

    // ========================================================================
    // MARK: - Transpose
    // ========================================================================

    func testTransposeRectangular() {
        let m = Matrix([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]])
        let t = m.transpose()
        XCTAssertEqual(t.rows, 3)
        XCTAssertEqual(t.cols, 2)
        XCTAssertEqual(t, Matrix([[1.0, 4.0], [2.0, 5.0], [3.0, 6.0]]))
    }

    func testTransposeSquare() {
        let m = Matrix([[1.0, 2.0], [3.0, 4.0]])
        let t = m.transpose()
        XCTAssertEqual(t, Matrix([[1.0, 3.0], [2.0, 4.0]]))
    }

    func testDoubleTranspose() {
        let m = Matrix([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]])
        XCTAssertEqual(m.transpose().transpose(), m)
    }

    func testTransposeScalar() {
        let m = Matrix(5.0)
        XCTAssertEqual(m.transpose(), m)
    }

    // ========================================================================
    // MARK: - Dot Product
    // ========================================================================

    func testDot2x2() {
        let a = Matrix([[1.0, 2.0], [3.0, 4.0]])
        let b = Matrix([[5.0, 6.0], [7.0, 8.0]])
        let c = a.dot(b)
        // [1*5+2*7, 1*6+2*8] = [19, 22]
        // [3*5+4*7, 3*6+4*8] = [43, 50]
        XCTAssertEqual(c, Matrix([[19.0, 22.0], [43.0, 50.0]]))
    }

    func testDotNonSquare() {
        let a = Matrix([[1.0, 2.0, 3.0]])  // 1×3
        let b = Matrix([[4.0], [5.0], [6.0]])  // 3×1
        let c = a.dot(b)  // 1×1
        XCTAssertEqual(c.rows, 1)
        XCTAssertEqual(c.cols, 1)
        XCTAssertEqual(c[0, 0], 32.0, accuracy: eps)  // 4+10+18
    }

    func testDotIdentity() {
        let a = Matrix([[1.0, 2.0], [3.0, 4.0]])
        let eye = Matrix([[1.0, 0.0], [0.0, 1.0]])
        XCTAssertEqual(a.dot(eye), a)
        XCTAssertEqual(eye.dot(a), a)
    }

    func testDotZero() {
        let a = Matrix([[1.0, 2.0], [3.0, 4.0]])
        let z = Matrix.zeros(rows: 2, cols: 2)
        XCTAssertEqual(a.dot(z), z)
    }

    // ========================================================================
    // MARK: - Equality
    // ========================================================================

    func testEquality() {
        let a = Matrix([[1.0, 2.0], [3.0, 4.0]])
        let b = Matrix([[1.0, 2.0], [3.0, 4.0]])
        XCTAssertEqual(a, b)
    }

    func testInequality() {
        let a = Matrix([[1.0, 2.0]])
        let b = Matrix([[1.0, 3.0]])
        XCTAssertNotEqual(a, b)
    }

    // ========================================================================
    // MARK: - Immutability
    // ========================================================================

    func testImmutability() {
        let a = Matrix([[1.0, 2.0], [3.0, 4.0]])
        let _ = a.add(Matrix([[10.0, 20.0], [30.0, 40.0]]))
        // Original should be unchanged
        XCTAssertEqual(a[0, 0], 1.0)
        XCTAssertEqual(a[1, 1], 4.0)
    }
}
