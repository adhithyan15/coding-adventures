// ============================================================================
// Matrix.swift — Matrix Mathematics Library
// ============================================================================
//
// A matrix is a rectangular grid of numbers arranged in rows and columns.
// Matrices are the fundamental data structure of linear algebra and the
// computational backbone of neural networks.
//
// In a neural network, every layer performs: output = activation(W · input + b)
// where W is a weight matrix, input is a column vector, and · is matrix
// multiplication (the "dot product").
//
// This module provides an immutable Matrix type with:
//   • Dynamic instantiation from scalars, 1D arrays, or 2D arrays
//   • Element-wise add/subtract (matrix-matrix and matrix-scalar)
//   • Scalar multiplication (scale)
//   • Transpose (swap rows and columns)
//   • Dot product (true matrix multiplication)
//
// Layer: ML03 (machine-learning layer 3 — leaf package, zero dependencies)
// Spec:  code/specs/ML03-matrix.md
// ============================================================================

import Foundation

/// An immutable matrix of Double values.
///
/// All arithmetic operations return new `Matrix` instances — the original
/// is never mutated.  This makes matrices safe to share across threads
/// and easy to reason about in mathematical proofs.
public struct Matrix: Equatable {

    // ========================================================================
    // MARK: - Storage
    // ========================================================================

    /// The underlying 2D array.  Row-major: `data[row][col]`.
    public let data: [[Double]]

    /// Number of rows (M in an M×N matrix).
    public var rows: Int { data.count }

    /// Number of columns (N in an M×N matrix).
    public var cols: Int { data.isEmpty ? 0 : data[0].count }

    // ========================================================================
    // MARK: - Initializers
    // ========================================================================

    /// Create a matrix from a 2D array.
    ///
    /// Precondition: all rows must have the same length.
    public init(_ data: [[Double]]) {
        precondition(!data.isEmpty, "Matrix must have at least one row")
        let colCount = data[0].count
        precondition(colCount > 0, "Matrix must have at least one column")
        for (i, row) in data.enumerated() {
            precondition(row.count == colCount,
                         "Row \(i) has \(row.count) columns, expected \(colCount)")
        }
        self.data = data
    }

    /// Create a 1×N matrix (row vector) from a 1D array.
    public init(_ array: [Double]) {
        precondition(!array.isEmpty, "Array must not be empty")
        self.data = [array]
    }

    /// Create a 1×1 matrix from a scalar.
    public init(_ scalar: Double) {
        self.data = [[scalar]]
    }

    // ========================================================================
    // MARK: - Factories
    // ========================================================================

    /// Create an M×N matrix filled with zeros.
    ///
    /// ```swift
    /// let z = Matrix.zeros(rows: 3, cols: 2)
    /// // [[0, 0],
    /// //  [0, 0],
    /// //  [0, 0]]
    /// ```
    public static func zeros(rows: Int, cols: Int) -> Matrix {
        precondition(rows > 0 && cols > 0, "Dimensions must be positive")
        let row = Array(repeating: 0.0, count: cols)
        let data = Array(repeating: row, count: rows)
        return Matrix(data)
    }

    // ========================================================================
    // MARK: - Element-wise Arithmetic
    // ========================================================================
    //
    // Element-wise operations apply the same operation to every corresponding
    // pair of elements.  For A + B, element (i,j) of the result is
    // A[i][j] + B[i][j].  Both matrices must have the same dimensions.
    // ========================================================================

    /// Add two matrices element-wise.
    public func add(_ other: Matrix) -> Matrix {
        precondition(rows == other.rows && cols == other.cols,
                     "Dimension mismatch: \(rows)×\(cols) vs \(other.rows)×\(other.cols)")
        var result = data
        for i in 0..<rows {
            for j in 0..<cols {
                result[i][j] += other.data[i][j]
            }
        }
        return Matrix(result)
    }

    /// Subtract another matrix element-wise.
    public func subtract(_ other: Matrix) -> Matrix {
        precondition(rows == other.rows && cols == other.cols,
                     "Dimension mismatch: \(rows)×\(cols) vs \(other.rows)×\(other.cols)")
        var result = data
        for i in 0..<rows {
            for j in 0..<cols {
                result[i][j] -= other.data[i][j]
            }
        }
        return Matrix(result)
    }

    /// Add a scalar to every element.
    public func add(_ scalar: Double) -> Matrix {
        let result = data.map { row in row.map { $0 + scalar } }
        return Matrix(result)
    }

    /// Subtract a scalar from every element.
    public func subtract(_ scalar: Double) -> Matrix {
        let result = data.map { row in row.map { $0 - scalar } }
        return Matrix(result)
    }

    // ========================================================================
    // MARK: - Scalar Multiplication
    // ========================================================================

    /// Multiply every element by a scalar.
    public func scale(_ scalar: Double) -> Matrix {
        let result = data.map { row in row.map { $0 * scalar } }
        return Matrix(result)
    }

    // ========================================================================
    // MARK: - Transpose
    // ========================================================================
    //
    // Transposing swaps rows and columns: the element at (i, j) moves to
    // (j, i).  An M×N matrix becomes N×M.
    //
    // Transpose is critical in backpropagation: to compute weight gradients,
    // you multiply by the transpose of the input matrix.
    // ========================================================================

    /// Return the transpose of this matrix (swap rows and columns).
    public func transpose() -> Matrix {
        var result = Array(repeating: Array(repeating: 0.0, count: rows), count: cols)
        for i in 0..<rows {
            for j in 0..<cols {
                result[j][i] = data[i][j]
            }
        }
        return Matrix(result)
    }

    // ========================================================================
    // MARK: - Dot Product (Matrix Multiplication)
    // ========================================================================
    //
    // The dot product of A (M×K) and B (K×N) produces C (M×N) where:
    //
    //   C[i][j] = Σ(k=0..K-1) A[i][k] · B[k][j]
    //
    // Each element of the result is the sum of element-wise products of a
    // row from A and a column from B.  This is the fundamental operation
    // of neural networks — every layer computes weights · inputs.
    //
    // Time complexity: O(M · N · K).  For large matrices, optimised
    // algorithms (Strassen, BLAS) can do better, but the naive triple
    // loop is correct and easy to understand.
    // ========================================================================

    /// Multiply this matrix by another (true matrix multiplication).
    ///
    /// Precondition: `self.cols == other.rows`.
    public func dot(_ other: Matrix) -> Matrix {
        precondition(cols == other.rows,
                     "Dot product dimension mismatch: \(rows)×\(cols) · \(other.rows)×\(other.cols)")
        var result = Array(repeating: Array(repeating: 0.0, count: other.cols), count: rows)
        for i in 0..<rows {
            for j in 0..<other.cols {
                var sum = 0.0
                for k in 0..<cols {
                    sum += data[i][k] * other.data[k][j]
                }
                result[i][j] = sum
            }
        }
        return Matrix(result)
    }

    // ========================================================================
    // MARK: - Subscript
    // ========================================================================

    /// Access element at (row, col).
    public subscript(row: Int, col: Int) -> Double {
        return data[row][col]
    }
}
