// ============================================================================
// Tests for Matrix — 2D matrix type with arithmetic and extensions
// ============================================================================
//
// These tests cover all public API of the Matrix module:
//   - Constructors and factories
//   - Element access (subscript, get, set)
//   - Arithmetic (add, subtract, scale, transpose, dot)
//   - Reductions (sum, sum_rows, sum_cols, mean, min, max, argmin, argmax)
//   - Element-wise math (map, sqrt, abs, pow)
//   - Shape operations (flatten, reshape, row, col, slice)
//   - Equality and comparison (==, close)
//   - Operator overloads (+, -, *)
//   - Factory methods (identity, fromDiagonal)
//   - Parity test vectors (identical across all 9 languages)

import Testing
@testable import Matrix

// ---------------------------------------------------------------------------
// Helper: floating-point comparison with tolerance
// ---------------------------------------------------------------------------

func near(_ a: Double, _ b: Double, tol: Double = 1e-10) -> Bool {
    Swift.abs(a - b) < tol
}

// ============================================================================
// 1. Constructors
// ============================================================================

@Suite("Constructors")
struct ConstructorTests {
    @Test func zeros() {
        let A = Matrix.zeros(rows: 3, cols: 4)
        #expect(A.rows == 3)
        #expect(A.cols == 4)
        for i in 0..<3 {
            for j in 0..<4 {
                #expect(near(A[i, j], 0.0))
            }
        }
    }

    @Test func from2D() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        #expect(A.rows == 2)
        #expect(A.cols == 2)
        #expect(near(A[0, 0], 1.0))
        #expect(near(A[0, 1], 2.0))
        #expect(near(A[1, 0], 3.0))
        #expect(near(A[1, 1], 4.0))
    }

    @Test func from1D() {
        let v = Matrix(from1D: [5, 6, 7, 8])
        #expect(v.rows == 1)
        #expect(v.cols == 4)
        #expect(near(v[0, 0], 5.0))
        #expect(near(v[0, 3], 8.0))
    }

    @Test func scalar() {
        let s = Matrix(scalar: 3.14)
        #expect(s.rows == 1)
        #expect(s.cols == 1)
        #expect(near(s[0, 0], 3.14))
    }
}

// ============================================================================
// 2. Element access
// ============================================================================

@Suite("Element Access")
struct ElementAccessTests {
    @Test func subscriptAccess() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        #expect(near(A[0, 0], 1.0))
        #expect(near(A[1, 1], 4.0))
    }

    @Test func getMethod() throws {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        #expect(near(try A.get(row: 0, col: 0), 1.0))
        #expect(near(try A.get(row: 1, col: 1), 4.0))
    }

    @Test func setReturnsNewMatrix() throws {
        let A = Matrix.zeros(rows: 2, cols: 2)
        let B = try A.set(row: 0, col: 1, value: 99.0)
        #expect(near(B[0, 1], 99.0))
        #expect(near(B[0, 0], 0.0))
        // Original not mutated (value semantics).
        #expect(near(A[0, 1], 0.0))
    }
}

// ============================================================================
// 3. Arithmetic
// ============================================================================

@Suite("Arithmetic")
struct ArithmeticTests {
    @Test func addElementWise() throws {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let B = Matrix(from2D: [[5, 6], [7, 8]])
        let C = try A.add(B)
        #expect(near(C[0, 0], 6.0))
        #expect(near(C[0, 1], 8.0))
        #expect(near(C[1, 0], 10.0))
        #expect(near(C[1, 1], 12.0))
    }

    @Test func addZerosIsIdentity() throws {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let Z = Matrix.zeros(rows: 2, cols: 2)
        let C = try A.add(Z)
        #expect(C == A)
    }

    @Test func addDimensionMismatchThrows() {
        let A = Matrix.zeros(rows: 2, cols: 3)
        let B = Matrix.zeros(rows: 3, cols: 2)
        #expect(throws: (any Error).self) { try A.add(B) }
    }

    @Test func addScalar() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let B = A.addScalar(10)
        #expect(near(B[0, 0], 11.0))
        #expect(near(B[1, 1], 14.0))
    }

    @Test func subtractElementWise() throws {
        let A = Matrix(from2D: [[5, 6], [7, 8]])
        let B = Matrix(from2D: [[1, 2], [3, 4]])
        let C = try A.subtract(B)
        #expect(near(C[0, 0], 4.0))
        #expect(near(C[1, 1], 4.0))
    }

    @Test func scaleMultiplies() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let B = A.scale(3.0)
        #expect(near(B[0, 0], 3.0))
        #expect(near(B[0, 1], 6.0))
        #expect(near(B[1, 0], 9.0))
        #expect(near(B[1, 1], 12.0))
    }

    @Test func scaleByZeroGivesZeros() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let B = A.scale(0.0)
        #expect(B == Matrix.zeros(rows: 2, cols: 2))
    }
}

// ============================================================================
// 4. Transpose
// ============================================================================

@Suite("Transpose")
struct TransposeTests {
    @Test func transposes2x3to3x2() {
        let A = Matrix(from2D: [[1, 2, 3], [4, 5, 6]])
        let AT = A.transpose()
        #expect(AT.rows == 3)
        #expect(AT.cols == 2)
        #expect(near(AT[0, 0], 1.0))
        #expect(near(AT[0, 1], 4.0))
        #expect(near(AT[2, 0], 3.0))
        #expect(near(AT[2, 1], 6.0))
    }

    @Test func doubleTransposeIsIdentity() {
        let A = Matrix(from2D: [[1, 2, 3], [4, 5, 6]])
        #expect(A.transpose().transpose() == A)
    }

    @Test func rowVectorTransposesToColumnVector() {
        let v = Matrix(from1D: [1, 2, 3])
        let vT = v.transpose()
        #expect(vT.rows == 3)
        #expect(vT.cols == 1)
    }
}

// ============================================================================
// 5. Dot product
// ============================================================================

@Suite("Dot Product")
struct DotProductTests {
    @Test func dot2x2() throws {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let B = Matrix(from2D: [[5, 6], [7, 8]])
        let C = try A.dot(B)
        #expect(near(C[0, 0], 19.0))
        #expect(near(C[0, 1], 22.0))
        #expect(near(C[1, 0], 43.0))
        #expect(near(C[1, 1], 50.0))
    }

    @Test func dot2x3times3x2() throws {
        let A = Matrix(from2D: [[1, 2, 3], [4, 5, 6]])
        let B = Matrix(from2D: [[7, 8], [9, 10], [11, 12]])
        let C = try A.dot(B)
        #expect(near(C[0, 0], 58.0))
        #expect(near(C[0, 1], 64.0))
        #expect(near(C[1, 0], 139.0))
        #expect(near(C[1, 1], 154.0))
    }

    @Test func identityDotAEqualsA() throws {
        let I = Matrix.identity(n: 2)
        let A = Matrix(from2D: [[3, 7], [2, 5]])
        let IA = try I.dot(A)
        #expect(IA == A)
    }

    @Test func dotDimensionMismatchThrows() {
        let A = Matrix.zeros(rows: 2, cols: 3)
        let B = Matrix.zeros(rows: 2, cols: 2)
        #expect(throws: (any Error).self) { try A.dot(B) }
    }

    @Test func innerProductViaMatrixMultiply() throws {
        let row = Matrix(from1D: [1, 2, 3])
        let col = Matrix(from1D: [4, 5, 6]).transpose()
        let C = try row.dot(col)
        #expect(C.rows == 1)
        #expect(C.cols == 1)
        #expect(near(C[0, 0], 32.0))
    }
}

// ============================================================================
// 6. Reductions
// ============================================================================

@Suite("Reductions")
struct ReductionTests {
    @Test func sumAllElements() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        #expect(near(A.sum(), 10.0))
    }

    @Test func sumOfZeros() {
        #expect(near(Matrix.zeros(rows: 3, cols: 3).sum(), 0.0))
    }

    @Test func sumRows() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let sr = A.sumRows()
        #expect(sr.rows == 2)
        #expect(sr.cols == 1)
        #expect(near(sr[0, 0], 3.0))
        #expect(near(sr[1, 0], 7.0))
    }

    @Test func sumCols() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let sc = A.sumCols()
        #expect(sc.rows == 1)
        #expect(sc.cols == 2)
        #expect(near(sc[0, 0], 4.0))
        #expect(near(sc[0, 1], 6.0))
    }

    @Test func meanValue() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        #expect(near(A.mean(), 2.5))
    }

    @Test func minValue() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        #expect(near(A.min(), 1.0))
    }

    @Test func maxValue() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        #expect(near(A.max(), 4.0))
    }

    @Test func minWithNegatives() {
        let A = Matrix(from2D: [[-5, 2], [3, -1]])
        #expect(near(A.min(), -5.0))
    }

    @Test func maxWithNegatives() {
        let A = Matrix(from2D: [[-5, 2], [3, -1]])
        #expect(near(A.max(), 3.0))
    }

    @Test func argminPosition() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let (r, c) = A.argmin()
        #expect(r == 0)
        #expect(c == 0)
    }

    @Test func argmaxPosition() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let (r, c) = A.argmax()
        #expect(r == 1)
        #expect(c == 1)
    }

    @Test func argmaxFirstOccurrenceOnTie() {
        let A = Matrix(from2D: [[3, 1], [3, 2]])
        let (r, c) = A.argmax()
        #expect(r == 0)
        #expect(c == 0)
    }

    @Test func argminFirstOccurrenceOnTie() {
        let A = Matrix(from2D: [[1, 1], [2, 3]])
        let (r, c) = A.argmin()
        #expect(r == 0)
        #expect(c == 0)
    }
}

// ============================================================================
// 7. Element-wise math
// ============================================================================

@Suite("Element-wise Math")
struct ElementWiseMathTests {
    @Test func mapDoubles() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let B = A.map { $0 * 2 }
        #expect(near(B[0, 0], 2.0))
        #expect(near(B[1, 1], 8.0))
    }

    @Test func sqrtElements() {
        let A = Matrix(from2D: [[4, 9], [16, 25]])
        let B = A.sqrt()
        #expect(near(B[0, 0], 2.0))
        #expect(near(B[0, 1], 3.0))
        #expect(near(B[1, 0], 4.0))
        #expect(near(B[1, 1], 5.0))
    }

    @Test func absElements() {
        let A = Matrix(from2D: [[-1, 2], [-3, 4]])
        let B = A.abs()
        #expect(near(B[0, 0], 1.0))
        #expect(near(B[1, 0], 3.0))
    }

    @Test func powSquares() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let B = A.pow(2.0)
        #expect(near(B[0, 0], 1.0))
        #expect(near(B[0, 1], 4.0))
        #expect(near(B[1, 0], 9.0))
        #expect(near(B[1, 1], 16.0))
    }

    @Test func sqrtPowRoundtrip() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        #expect(A.close(A.sqrt().pow(2.0), tolerance: 1e-9))
    }
}

// ============================================================================
// 8. Shape operations
// ============================================================================

@Suite("Shape Operations")
struct ShapeOperationTests {
    @Test func flattenTo1xN() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let F = A.flatten()
        #expect(F.rows == 1)
        #expect(F.cols == 4)
        #expect(near(F[0, 0], 1.0))
        #expect(near(F[0, 1], 2.0))
        #expect(near(F[0, 2], 3.0))
        #expect(near(F[0, 3], 4.0))
    }

    @Test func reshapeValid() throws {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let R = try A.reshape(rows: 1, cols: 4)
        #expect(R.rows == 1)
        #expect(R.cols == 4)
    }

    @Test func flattenReshapeRoundtrip() throws {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let rt = try A.flatten().reshape(rows: A.rows, cols: A.cols)
        #expect(A == rt)
    }

    @Test func reshapeIncompatibleThrows() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        #expect(throws: (any Error).self) { try A.reshape(rows: 3, cols: 3) }
    }

    @Test func extractRow() throws {
        let A = Matrix(from2D: [[1, 2, 3], [4, 5, 6]])
        let r = try A.row(0)
        #expect(r.rows == 1)
        #expect(r.cols == 3)
        #expect(near(r[0, 0], 1.0))
        #expect(near(r[0, 2], 3.0))
    }

    @Test func extractCol() throws {
        let A = Matrix(from2D: [[1, 2, 3], [4, 5, 6]])
        let c = try A.col(1)
        #expect(c.rows == 2)
        #expect(c.cols == 1)
        #expect(near(c[0, 0], 2.0))
        #expect(near(c[1, 0], 5.0))
    }

    @Test func rowOutOfBoundsThrows() {
        let A = Matrix(from2D: [[1, 2]])
        #expect(throws: (any Error).self) { try A.row(3) }
    }

    @Test func colOutOfBoundsThrows() {
        let A = Matrix(from2D: [[1, 2]])
        #expect(throws: (any Error).self) { try A.col(3) }
    }

    @Test func sliceSubMatrix() throws {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let S = try A.slice(r0: 0, r1: 2, c0: 0, c1: 1)
        #expect(S.rows == 2)
        #expect(S.cols == 1)
        #expect(near(S[0, 0], 1.0))
        #expect(near(S[1, 0], 3.0))
    }

    @Test func sliceFullMatrixEqualsOriginal() throws {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let S = try A.slice(r0: 0, r1: 2, c0: 0, c1: 2)
        #expect(A == S)
    }
}

// ============================================================================
// 9. Equality and comparison
// ============================================================================

@Suite("Equality")
struct EqualityTests {
    @Test func equalMatrices() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let B = Matrix(from2D: [[1, 2], [3, 4]])
        #expect(A == B)
    }

    @Test func differentMatrices() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let B = Matrix(from2D: [[1, 2], [3, 5]])
        #expect(A != B)
    }

    @Test func differentShapes() {
        let A = Matrix(from2D: [[1, 2]])
        let B = Matrix(from2D: [[1], [2]])
        #expect(A != B)
    }

    @Test func closeWithinTolerance() {
        let A = Matrix(from2D: [[1.0, 2.0]])
        let B = Matrix(from2D: [[1.0 + 1e-10, 2.0 - 1e-10]])
        #expect(A.close(B, tolerance: 1e-9))
    }

    @Test func closeOutsideTolerance() {
        let A = Matrix(from2D: [[1.0, 2.0]])
        let B = Matrix(from2D: [[1.1, 2.0]])
        #expect(!A.close(B, tolerance: 1e-9))
    }
}

// ============================================================================
// 10. Operator overloads
// ============================================================================

@Suite("Operators")
struct OperatorTests {
    @Test func plusOperator() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let B = Matrix(from2D: [[5, 6], [7, 8]])
        let C = A + B
        #expect(near(C[0, 0], 6.0))
        #expect(near(C[1, 1], 12.0))
    }

    @Test func minusOperator() {
        let A = Matrix(from2D: [[5, 6], [7, 8]])
        let B = Matrix(from2D: [[1, 2], [3, 4]])
        let C = A - B
        #expect(near(C[0, 0], 4.0))
    }

    @Test func scalarMultiplication() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let B = A * 3.0
        #expect(near(B[0, 0], 3.0))
        let C = 3.0 * A
        #expect(B == C)
    }
}

// ============================================================================
// 11. Factory methods
// ============================================================================

@Suite("Factory Methods")
struct FactoryTests {
    @Test func identity3x3() {
        let I = Matrix.identity(n: 3)
        #expect(I.rows == 3)
        #expect(I.cols == 3)
        for i in 0..<3 {
            for j in 0..<3 {
                let expected: Double = (i == j) ? 1.0 : 0.0
                #expect(near(I[i, j], expected))
            }
        }
    }

    @Test func identityDotM() throws {
        let I = Matrix.identity(n: 3)
        let A = Matrix(from2D: [[1, 2], [3, 4], [5, 6]])
        let IA = try I.dot(A)
        #expect(IA == A)
    }

    @Test func fromDiagonal() {
        let D = Matrix.fromDiagonal([2, 3])
        #expect(D.rows == 2)
        #expect(D.cols == 2)
        #expect(near(D[0, 0], 2.0))
        #expect(near(D[0, 1], 0.0))
        #expect(near(D[1, 0], 0.0))
        #expect(near(D[1, 1], 3.0))
    }

    @Test func fromDiagonalOnesEqualsIdentity() {
        #expect(Matrix.fromDiagonal([1, 1, 1]) == Matrix.identity(n: 3))
    }
}

// ============================================================================
// 12. Combined / property tests
// ============================================================================

@Suite("Properties")
struct PropertyTests {
    @Test func transposeDistributesOverAddition() throws {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let B = Matrix(from2D: [[5, 6], [7, 8]])
        let lhs = try A.add(B).transpose()
        let rhs = try A.transpose().add(B.transpose())
        #expect(lhs == rhs)
    }

    @Test func scaleDistributesOverAddition() throws {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let B = Matrix(from2D: [[5, 6], [7, 8]])
        let s = 3.0
        let lhs = try A.add(B).scale(s)
        let rhs = try A.scale(s).add(B.scale(s))
        #expect(lhs == rhs)
    }

    @Test func subtractEqualsAddNegated() throws {
        let A = Matrix(from2D: [[10, 20], [30, 40]])
        let B = Matrix(from2D: [[1, 2], [3, 4]])
        let sub = try A.subtract(B)
        let add = try A.add(B.scale(-1.0))
        #expect(sub == add)
    }
}

// ============================================================================
// 13. Parity test vectors (identical across all 9 languages)
// ============================================================================

@Suite("Parity Test Vectors")
struct ParityTests {
    @Test func sumAndMean() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        #expect(near(A.sum(), 10.0))
        #expect(near(A.mean(), 2.5))
    }

    @Test func sumRowsAndSumCols() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let sr = A.sumRows()
        let sc = A.sumCols()
        #expect(near(sr[0, 0], 3.0))
        #expect(near(sr[1, 0], 7.0))
        #expect(near(sc[0, 0], 4.0))
        #expect(near(sc[0, 1], 6.0))
    }

    @Test func identityDot() throws {
        let I = Matrix.identity(n: 3)
        let A = Matrix(from2D: [[1, 2, 3], [4, 5, 6], [7, 8, 9]])
        let IA = try I.dot(A)
        #expect(IA == A)
    }

    @Test func flattenReshapeRoundtrip() throws {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let rt = try A.flatten().reshape(rows: A.rows, cols: A.cols)
        #expect(A == rt)
    }

    @Test func closeAfterSqrtPow() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        #expect(A.close(A.sqrt().pow(2.0), tolerance: 1e-9))
    }

    @Test func getElement() throws {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        #expect(near(try A.get(row: 0, col: 0), 1.0))
    }

    @Test func argmaxPosition() {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let (r, c) = A.argmax()
        #expect(r == 1)
        #expect(c == 1)
    }

    @Test func sliceFirstColumn() throws {
        let A = Matrix(from2D: [[1, 2], [3, 4]])
        let S = try A.slice(r0: 0, r1: 2, c0: 0, c1: 1)
        #expect(S.rows == 2)
        #expect(S.cols == 1)
        #expect(near(S[0, 0], 1.0))
        #expect(near(S[1, 0], 3.0))
    }
}
