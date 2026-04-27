// ============================================================================
// Matrix.java — Matrix Mathematics Library
// ============================================================================
//
// A matrix is a rectangular grid of numbers arranged in rows and columns.
// Matrices are the fundamental data structure of linear algebra and the
// computational backbone of neural networks.
//
// This class provides an immutable Matrix type with:
//   • Factory methods from scalars, 1D arrays, or 2D arrays
//   • Element-wise add/subtract (matrix-matrix and matrix-scalar)
//   • Scalar multiplication (scale)
//   • Transpose (swap rows and columns)
//   • Dot product (true matrix multiplication)
//
// Layer: ML03 (machine-learning layer 3 — leaf package, zero dependencies)
// Spec:  code/specs/ML03-matrix.md
// ============================================================================

package com.codingadventures.matrix;

import java.util.Arrays;

/**
 * An immutable matrix of double values.
 *
 * <p>All arithmetic operations return new {@code Matrix} instances — the
 * original is never mutated.</p>
 */
public final class Matrix {

    private final double[][] data;
    private final int rows;
    private final int cols;

    // ========================================================================
    // Constructor
    // ========================================================================

    /** Create a matrix from a 2D array (deep-copied). */
    public Matrix(double[][] data) {
        if (data.length == 0 || data[0].length == 0) {
            throw new IllegalArgumentException("Matrix must have at least one row and column");
        }
        this.rows = data.length;
        this.cols = data[0].length;
        this.data = new double[rows][cols];
        for (int i = 0; i < rows; i++) {
            if (data[i].length != cols) {
                throw new IllegalArgumentException(
                    "Row " + i + " has " + data[i].length + " columns, expected " + cols);
            }
            System.arraycopy(data[i], 0, this.data[i], 0, cols);
        }
    }

    // ========================================================================
    // Factories
    // ========================================================================

    /** Create a 1x1 matrix from a scalar. */
    public static Matrix fromScalar(double value) {
        return new Matrix(new double[][]{{value}});
    }

    /** Create a 1xN matrix (row vector) from a 1D array. */
    public static Matrix fromArray(double[] array) {
        if (array.length == 0) throw new IllegalArgumentException("Array must not be empty");
        return new Matrix(new double[][]{array.clone()});
    }

    /** Create an MxN matrix filled with zeros. */
    public static Matrix zeros(int rows, int cols) {
        if (rows <= 0 || cols <= 0) throw new IllegalArgumentException("Dimensions must be positive");
        double[][] data = new double[rows][cols];
        return new Matrix(data);
    }

    // ========================================================================
    // Accessors
    // ========================================================================

    public int getRows() { return rows; }
    public int getCols() { return cols; }

    /** Return a deep copy of the underlying data. */
    public double[][] getData() {
        double[][] copy = new double[rows][cols];
        for (int i = 0; i < rows; i++) {
            System.arraycopy(data[i], 0, copy[i], 0, cols);
        }
        return copy;
    }

    /** Access element at (row, col). */
    public double get(int row, int col) { return data[row][col]; }

    // ========================================================================
    // Element-wise Arithmetic
    // ========================================================================

    /** Add two matrices element-wise. */
    public Matrix add(Matrix other) {
        checkDimensions(other, "add");
        double[][] result = new double[rows][cols];
        for (int i = 0; i < rows; i++)
            for (int j = 0; j < cols; j++)
                result[i][j] = data[i][j] + other.data[i][j];
        return new Matrix(result);
    }

    /** Subtract another matrix element-wise. */
    public Matrix subtract(Matrix other) {
        checkDimensions(other, "subtract");
        double[][] result = new double[rows][cols];
        for (int i = 0; i < rows; i++)
            for (int j = 0; j < cols; j++)
                result[i][j] = data[i][j] - other.data[i][j];
        return new Matrix(result);
    }

    /** Add a scalar to every element. */
    public Matrix addScalar(double scalar) {
        double[][] result = new double[rows][cols];
        for (int i = 0; i < rows; i++)
            for (int j = 0; j < cols; j++)
                result[i][j] = data[i][j] + scalar;
        return new Matrix(result);
    }

    /** Subtract a scalar from every element. */
    public Matrix subtractScalar(double scalar) {
        return addScalar(-scalar);
    }

    // ========================================================================
    // Scale
    // ========================================================================

    /** Multiply every element by a scalar. */
    public Matrix scale(double scalar) {
        double[][] result = new double[rows][cols];
        for (int i = 0; i < rows; i++)
            for (int j = 0; j < cols; j++)
                result[i][j] = data[i][j] * scalar;
        return new Matrix(result);
    }

    // ========================================================================
    // Transpose
    // ========================================================================

    /** Swap rows and columns: M×N becomes N×M. */
    public Matrix transpose() {
        double[][] result = new double[cols][rows];
        for (int i = 0; i < rows; i++)
            for (int j = 0; j < cols; j++)
                result[j][i] = data[i][j];
        return new Matrix(result);
    }

    // ========================================================================
    // Dot Product (Matrix Multiplication)
    // ========================================================================

    /**
     * Matrix multiplication: (M×K) · (K×N) = (M×N).
     *
     * <p>C[i][j] = Σ(k) A[i][k] * B[k][j]</p>
     */
    public Matrix dot(Matrix other) {
        if (cols != other.rows) {
            throw new IllegalArgumentException(
                "Dot dimension mismatch: " + rows + "×" + cols + " · " + other.rows + "×" + other.cols);
        }
        double[][] result = new double[rows][other.cols];
        for (int i = 0; i < rows; i++)
            for (int j = 0; j < other.cols; j++) {
                double sum = 0.0;
                for (int k = 0; k < cols; k++)
                    sum += data[i][k] * other.data[k][j];
                result[i][j] = sum;
            }
        return new Matrix(result);
    }

    // ========================================================================
    // Helpers
    // ========================================================================

    private void checkDimensions(Matrix other, String op) {
        if (rows != other.rows || cols != other.cols) {
            throw new IllegalArgumentException(
                op + " dimension mismatch: " + rows + "×" + cols + " vs " + other.rows + "×" + other.cols);
        }
    }

    @Override
    public boolean equals(Object obj) {
        if (this == obj) return true;
        if (!(obj instanceof Matrix other)) return false;
        if (rows != other.rows || cols != other.cols) return false;
        return Arrays.deepEquals(data, other.data);
    }

    @Override
    public int hashCode() {
        return Arrays.deepHashCode(data);
    }

    @Override
    public String toString() {
        StringBuilder sb = new StringBuilder("Matrix(");
        sb.append(rows).append("×").append(cols).append(")");
        return sb.toString();
    }
}
