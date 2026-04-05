// ============================================================================
// Matrix — 2D matrix type with arithmetic and linear-algebra operations
// ============================================================================
//
// This module implements a two-dimensional matrix in pure Swift.  It is
// designed to be readable and educational rather than optimised for raw
// throughput.  Every algorithm is explained at the level of the mathematics
// behind it so that a reader new to linear algebra can follow along.
//
// ## What Is a Matrix?
//
// A matrix is a rectangular grid of numbers arranged in rows and columns.
// We write an m x n matrix (m rows, n columns) as:
//
//       | a11  a12  ...  a1n |
//   A = | a21  a22  ...  a2n |
//       | am1  am2  ...  amn |
//
// Elements are accessed as A[i, j] where i is the row (0-based) and j is
// the column (0-based).
//
// ## Representation
//
// The matrix stores its elements in a flat array of Doubles in row-major
// order.  Element (i, j) lives at index i * cols + j.  This is both
// cache-friendly and straightforward to index.
//
// ## Design
//
// - Value semantics: Matrix is a struct, so copies are independent.
// - Equatable: Two matrices are equal if same shape and same elements.
// - Sendable: Safe to pass across concurrency boundaries.
// - CustomStringConvertible: Pretty-prints for debugging.
// - All operations return new matrices; nothing is mutated.
//
// ============================================================================

import Foundation

// MARK: - MatrixError

/// Errors that matrix operations can throw.
///
/// Rather than returning optional values, we use Swift's throwing mechanism
/// so callers get descriptive error messages when something goes wrong.
public enum MatrixError: Error, CustomStringConvertible {
    case dimensionMismatch(String)
    case indexOutOfBounds(String)
    case invalidReshape(String)
    case innerDimensionMismatch(String)

    public var description: String {
        switch self {
        case .dimensionMismatch(let msg): return msg
        case .indexOutOfBounds(let msg): return msg
        case .invalidReshape(let msg): return msg
        case .innerDimensionMismatch(let msg): return msg
        }
    }
}

// MARK: - Matrix

/// A two-dimensional matrix of Double values.
///
/// Matrices are the fundamental data structure of linear algebra.  They
/// represent systems of linear equations, transformations, datasets, and
/// neural network weight tables.
///
/// This implementation uses a flat row-major array for storage:
///
///     Element at row i, column j = data[i * cols + j]
///
/// All operations return new matrices — the original is never mutated.
/// This follows the principle of value semantics that Swift encourages.
public struct Matrix: Equatable, Sendable, CustomStringConvertible {

    // MARK: Stored properties

    /// The number of rows in the matrix.
    public let rows: Int

    /// The number of columns in the matrix.
    public let cols: Int

    /// Flat storage in row-major order.  Element (i, j) is at data[i * cols + j].
    public let data: [Double]

    // MARK: - Initializers

    /// Creates a matrix from a flat array and explicit dimensions.
    ///
    /// - Parameters:
    ///   - rows: Number of rows.
    ///   - cols: Number of columns.
    ///   - data: Flat array of length rows * cols in row-major order.
    /// - Precondition: `data.count == rows * cols`
    public init(rows: Int, cols: Int, data: [Double]) {
        precondition(data.count == rows * cols,
                     "data length (\(data.count)) must equal rows*cols (\(rows * cols))")
        self.rows = rows
        self.cols = cols
        self.data = data
    }

    /// Creates a matrix from a 2D array (array of row arrays).
    ///
    /// The number of rows is inferred from the outer array; the number of
    /// columns from the first inner array.  All inner arrays must have the
    /// same length.
    ///
    /// ## Example
    ///
    ///     let A = Matrix(from2D: [[1, 2, 3],
    ///                             [4, 5, 6]])  // 2x3 matrix
    ///
    public init(from2D array: [[Double]]) {
        let r = array.count
        let c = r > 0 ? array[0].count : 0
        var flat = [Double]()
        flat.reserveCapacity(r * c)
        for row in array {
            flat.append(contentsOf: row)
        }
        self.init(rows: r, cols: c, data: flat)
    }

    /// Creates a 1 x n row vector from a flat array.
    public init(from1D array: [Double]) {
        self.init(rows: 1, cols: array.count, data: array)
    }

    /// Creates a 1 x 1 matrix containing a single scalar value.
    public init(scalar val: Double) {
        self.init(rows: 1, cols: 1, data: [val])
    }

    // MARK: - Factory methods

    /// Creates an (rows x cols) matrix filled with zeros.
    ///
    /// The zero matrix is the additive identity: A + 0 = A.
    public static func zeros(rows: Int, cols: Int) -> Matrix {
        Matrix(rows: rows, cols: cols, data: [Double](repeating: 0.0, count: rows * cols))
    }

    /// Creates an n x n identity matrix.
    ///
    /// The identity matrix has 1.0 on the main diagonal and 0.0 elsewhere.
    /// It is the multiplicative identity: I . A = A . I = A.
    public static func identity(n: Int) -> Matrix {
        var d = [Double](repeating: 0.0, count: n * n)
        for i in 0..<n {
            d[i * n + i] = 1.0
        }
        return Matrix(rows: n, cols: n, data: d)
    }

    /// Creates a square diagonal matrix from an array of values.
    ///
    /// The values go on the main diagonal; all off-diagonal elements are 0.
    ///
    ///     Matrix.fromDiagonal([2, 3])  // [[2, 0], [0, 3]]
    ///
    public static func fromDiagonal(_ values: [Double]) -> Matrix {
        let n = values.count
        var d = [Double](repeating: 0.0, count: n * n)
        for i in 0..<n {
            d[i * n + i] = values[i]
        }
        return Matrix(rows: n, cols: n, data: d)
    }

    // MARK: - Subscript (element access)

    /// Accesses the element at (row, col).
    ///
    /// This subscript provides a natural syntax: `matrix[i, j]`
    ///
    /// - Parameters:
    ///   - row: Row index (0-based).
    ///   - col: Column index (0-based).
    /// - Returns: The element value.
    /// - Precondition: Both indices must be in bounds.
    public subscript(row: Int, col: Int) -> Double {
        precondition(row >= 0 && row < rows && col >= 0 && col < cols,
                     "Index (\(row), \(col)) out of bounds for \(rows)x\(cols) matrix")
        return data[row * cols + col]
    }

    // MARK: - Element access (named methods)

    /// Returns the element at (row, col).  Throws on out-of-bounds.
    public func get(row: Int, col: Int) throws -> Double {
        guard row >= 0, row < rows, col >= 0, col < cols else {
            throw MatrixError.indexOutOfBounds(
                "index (\(row), \(col)) out of bounds for \(rows)x\(cols) matrix")
        }
        return data[row * cols + col]
    }

    /// Returns a new matrix with the element at (row, col) replaced.
    public func set(row: Int, col: Int, value: Double) throws -> Matrix {
        guard row >= 0, row < rows, col >= 0, col < cols else {
            throw MatrixError.indexOutOfBounds(
                "index (\(row), \(col)) out of bounds for \(rows)x\(cols) matrix")
        }
        var newData = data
        newData[row * cols + col] = value
        return Matrix(rows: rows, cols: cols, data: newData)
    }

    // MARK: - Arithmetic

    /// Element-wise addition: self + other.
    ///
    /// Both matrices must have the same dimensions.
    public func add(_ other: Matrix) throws -> Matrix {
        guard rows == other.rows, cols == other.cols else {
            throw MatrixError.dimensionMismatch(
                "add: (\(rows)x\(cols)) vs (\(other.rows)x\(other.cols))")
        }
        let d = zip(data, other.data).map(+)
        return Matrix(rows: rows, cols: cols, data: d)
    }

    /// Adds a scalar to every element.
    public func addScalar(_ s: Double) -> Matrix {
        Matrix(rows: rows, cols: cols, data: data.map { $0 + s })
    }

    /// Element-wise subtraction: self - other.
    public func subtract(_ other: Matrix) throws -> Matrix {
        guard rows == other.rows, cols == other.cols else {
            throw MatrixError.dimensionMismatch(
                "subtract: (\(rows)x\(cols)) vs (\(other.rows)x\(other.cols))")
        }
        let d = zip(data, other.data).map(-)
        return Matrix(rows: rows, cols: cols, data: d)
    }

    /// Multiplies every element by a scalar.
    public func scale(_ s: Double) -> Matrix {
        Matrix(rows: rows, cols: cols, data: data.map { $0 * s })
    }

    // MARK: - Transpose

    /// Returns the transpose of self.
    ///
    /// The transpose of an m x n matrix is an n x m matrix where rows and
    /// columns are swapped: (A^T)[i][j] = A[j][i].
    public func transpose() -> Matrix {
        var d = [Double](repeating: 0.0, count: rows * cols)
        for i in 0..<rows {
            for j in 0..<cols {
                d[j * rows + i] = data[i * cols + j]
            }
        }
        return Matrix(rows: cols, cols: rows, data: d)
    }

    // MARK: - Dot product (matrix multiplication)

    /// Computes the matrix product self . other.
    ///
    /// Defined when self is m x k and other is k x n (self.cols == other.rows).
    /// The result is m x n: C[i][j] = sum_{l=0}^{k-1} self[i][l] * other[l][j].
    public func dot(_ other: Matrix) throws -> Matrix {
        guard cols == other.rows else {
            throw MatrixError.innerDimensionMismatch(
                "dot: (\(rows)x\(cols)) . (\(other.rows)x\(other.cols))")
        }
        let m = rows
        let k = cols
        let n = other.cols
        var d = [Double](repeating: 0.0, count: m * n)
        for i in 0..<m {
            for l in 0..<k {
                let aVal = data[i * k + l]
                for j in 0..<n {
                    d[i * n + j] += aVal * other.data[l * n + j]
                }
            }
        }
        return Matrix(rows: m, cols: n, data: d)
    }

    // MARK: - Reductions

    /// Sum of all elements.
    public func sum() -> Double {
        data.reduce(0.0, +)
    }

    /// Returns an m x 1 column vector: each element is the sum of that row.
    public func sumRows() -> Matrix {
        var d = [Double]()
        d.reserveCapacity(rows)
        for i in 0..<rows {
            var s = 0.0
            for j in 0..<cols {
                s += data[i * cols + j]
            }
            d.append(s)
        }
        return Matrix(rows: rows, cols: 1, data: d)
    }

    /// Returns a 1 x n row vector: each element is the sum of that column.
    public func sumCols() -> Matrix {
        var d = [Double](repeating: 0.0, count: cols)
        for i in 0..<rows {
            for j in 0..<cols {
                d[j] += data[i * cols + j]
            }
        }
        return Matrix(rows: 1, cols: cols, data: d)
    }

    /// Arithmetic mean of all elements.
    public func mean() -> Double {
        sum() / Double(rows * cols)
    }

    /// Minimum element value.
    public func min() -> Double {
        data.min()!
    }

    /// Maximum element value.
    public func max() -> Double {
        data.max()!
    }

    /// (row, col) of the minimum element (0-based, first occurrence).
    public func argmin() -> (Int, Int) {
        var bestIdx = 0
        for i in 1..<data.count {
            if data[i] < data[bestIdx] { bestIdx = i }
        }
        return (bestIdx / cols, bestIdx % cols)
    }

    /// (row, col) of the maximum element (0-based, first occurrence).
    public func argmax() -> (Int, Int) {
        var bestIdx = 0
        for i in 1..<data.count {
            if data[i] > data[bestIdx] { bestIdx = i }
        }
        return (bestIdx / cols, bestIdx % cols)
    }

    // MARK: - Element-wise math

    /// Applies a function to every element, returning a new matrix.
    public func map(_ fn: (Double) -> Double) -> Matrix {
        Matrix(rows: rows, cols: cols, data: data.map(fn))
    }

    /// Element-wise square root.
    public func sqrt() -> Matrix {
        map { Foundation.sqrt($0) }
    }

    /// Element-wise absolute value.
    public func abs() -> Matrix {
        map { Swift.abs($0) }
    }

    /// Element-wise exponentiation.
    public func pow(_ exp: Double) -> Matrix {
        map { Foundation.pow($0, exp) }
    }

    // MARK: - Shape operations

    /// Returns a 1 x n row vector with elements in row-major order.
    public func flatten() -> Matrix {
        Matrix(rows: 1, cols: rows * cols, data: data)
    }

    /// Reshapes the matrix to the given dimensions.
    /// Total elements (newRows * newCols) must equal rows * cols.
    public func reshape(rows newRows: Int, cols newCols: Int) throws -> Matrix {
        guard newRows * newCols == rows * cols else {
            throw MatrixError.invalidReshape(
                "reshape: cannot reshape (\(rows)x\(cols)) = \(rows*cols) into (\(newRows)x\(newCols)) = \(newRows*newCols)")
        }
        return Matrix(rows: newRows, cols: newCols, data: data)
    }

    /// Extracts row i as a 1 x cols matrix (0-based).
    public func row(_ i: Int) throws -> Matrix {
        guard i >= 0, i < rows else {
            throw MatrixError.indexOutOfBounds("row: index \(i) out of bounds for \(rows) rows")
        }
        let start = i * cols
        return Matrix(rows: 1, cols: cols, data: Array(data[start..<start + cols]))
    }

    /// Extracts column j as a rows x 1 matrix (0-based).
    public func col(_ j: Int) throws -> Matrix {
        guard j >= 0, j < cols else {
            throw MatrixError.indexOutOfBounds("col: index \(j) out of bounds for \(cols) cols")
        }
        var d = [Double]()
        d.reserveCapacity(rows)
        for i in 0..<rows {
            d.append(data[i * cols + j])
        }
        return Matrix(rows: rows, cols: 1, data: d)
    }

    /// Extracts a sub-matrix for rows [r0..<r1) and columns [c0..<c1).
    public func slice(r0: Int, r1: Int, c0: Int, c1: Int) throws -> Matrix {
        guard r0 >= 0, r1 <= rows, c0 >= 0, c1 <= cols, r0 < r1, c0 < c1 else {
            throw MatrixError.indexOutOfBounds(
                "slice: bounds (\(r0):\(r1), \(c0):\(c1)) out of range for (\(rows)x\(cols))")
        }
        let nr = r1 - r0
        let nc = c1 - c0
        var d = [Double]()
        d.reserveCapacity(nr * nc)
        for i in r0..<r1 {
            for j in c0..<c1 {
                d.append(data[i * cols + j])
            }
        }
        return Matrix(rows: nr, cols: nc, data: d)
    }

    // MARK: - Equality and comparison

    /// Exact element-wise equality (via Equatable conformance).
    /// Two matrices are equal if they have the same shape and identical elements.
    public static func == (lhs: Matrix, rhs: Matrix) -> Bool {
        lhs.rows == rhs.rows && lhs.cols == rhs.cols && lhs.data == rhs.data
    }

    /// Returns true if same shape and all elements within tolerance.
    public func close(_ other: Matrix, tolerance: Double = 1e-9) -> Bool {
        guard rows == other.rows, cols == other.cols else { return false }
        for i in 0..<data.count {
            if Swift.abs(data[i] - other.data[i]) > tolerance { return false }
        }
        return true
    }

    // MARK: - Operator overloads

    /// Matrix addition: A + B.
    public static func + (lhs: Matrix, rhs: Matrix) -> Matrix {
        try! lhs.add(rhs)
    }

    /// Matrix subtraction: A - B.
    public static func - (lhs: Matrix, rhs: Matrix) -> Matrix {
        try! lhs.subtract(rhs)
    }

    /// Scalar multiplication: matrix * scalar.
    public static func * (lhs: Matrix, rhs: Double) -> Matrix {
        lhs.scale(rhs)
    }

    /// Scalar multiplication: scalar * matrix.
    public static func * (lhs: Double, rhs: Matrix) -> Matrix {
        rhs.scale(lhs)
    }

    // MARK: - CustomStringConvertible

    public var description: String {
        var lines = [String]()
        for i in 0..<rows {
            var rowStr = [String]()
            for j in 0..<cols {
                rowStr.append(String(format: "%.4f", data[i * cols + j]))
            }
            lines.append("[\(rowStr.joined(separator: ", "))]")
        }
        return "Matrix(\(rows)x\(cols)):\n" + lines.joined(separator: "\n")
    }
}
