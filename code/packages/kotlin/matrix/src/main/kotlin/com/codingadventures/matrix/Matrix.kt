// ============================================================================
// Matrix.kt — Matrix Mathematics Library
// ============================================================================
//
// A matrix is a rectangular grid of numbers arranged in rows and columns.
// This class provides an immutable Matrix type with operator overloading
// for natural algebraic syntax: A + B, A - B, A * 2.0, etc.
//
// Layer: ML03 (machine-learning layer 3 — leaf package, zero dependencies)
// Spec:  code/specs/ML03-matrix.md
// ============================================================================

package com.codingadventures.matrix

/**
 * An immutable matrix of [Double] values.
 *
 * All arithmetic operations return new [Matrix] instances — the original
 * is never mutated.
 */
class Matrix private constructor(private val data: Array<DoubleArray>) {

    val rows: Int get() = data.size
    val cols: Int get() = if (data.isEmpty()) 0 else data[0].size

    companion object {
        /** Create a matrix from a 2D array (deep-copied, validated). */
        fun of(data: Array<DoubleArray>): Matrix {
            require(data.isNotEmpty()) { "Matrix must have at least one row" }
            val cols = data[0].size
            require(cols > 0) { "Matrix must have at least one column" }
            for ((i, row) in data.withIndex()) {
                require(row.size == cols) { "Row $i has ${row.size} columns, expected $cols" }
            }
            return Matrix(Array(data.size) { data[it].copyOf() })
        }

        /** Create a 1x1 matrix from a scalar. */
        fun fromScalar(value: Double): Matrix = of(arrayOf(doubleArrayOf(value)))

        /** Create a 1xN row vector from a 1D array. */
        fun fromArray(array: DoubleArray): Matrix {
            require(array.isNotEmpty()) { "Array must not be empty" }
            return of(arrayOf(array.copyOf()))
        }

        /** Create an MxN matrix filled with zeros. */
        fun zeros(rows: Int, cols: Int): Matrix {
            require(rows > 0 && cols > 0) { "Dimensions must be positive" }
            return Matrix(Array(rows) { DoubleArray(cols) })
        }
    }

    /** Access element at (row, col). */
    operator fun get(row: Int, col: Int): Double = data[row][col]

    /** Return a deep copy of the data. */
    fun getData(): Array<DoubleArray> = Array(rows) { data[it].copyOf() }

    // ========================================================================
    // Element-wise Arithmetic
    // ========================================================================

    /** Add two matrices element-wise. */
    operator fun plus(other: Matrix): Matrix {
        checkDimensions(other, "add")
        return Matrix(Array(rows) { i ->
            DoubleArray(cols) { j -> data[i][j] + other.data[i][j] }
        })
    }

    /** Subtract another matrix element-wise. */
    operator fun minus(other: Matrix): Matrix {
        checkDimensions(other, "subtract")
        return Matrix(Array(rows) { i ->
            DoubleArray(cols) { j -> data[i][j] - other.data[i][j] }
        })
    }

    /** Add a scalar to every element. */
    operator fun plus(scalar: Double): Matrix =
        Matrix(Array(rows) { i -> DoubleArray(cols) { j -> data[i][j] + scalar } })

    /** Subtract a scalar from every element. */
    operator fun minus(scalar: Double): Matrix = plus(-scalar)

    // ========================================================================
    // Scale
    // ========================================================================

    /** Multiply every element by a scalar. */
    operator fun times(scalar: Double): Matrix =
        Matrix(Array(rows) { i -> DoubleArray(cols) { j -> data[i][j] * scalar } })

    /** Alias for times(). */
    fun scale(scalar: Double): Matrix = times(scalar)

    // ========================================================================
    // Transpose
    // ========================================================================

    /** Swap rows and columns: M×N becomes N×M. */
    fun transpose(): Matrix =
        Matrix(Array(cols) { j -> DoubleArray(rows) { i -> data[i][j] } })

    // ========================================================================
    // Dot Product (Matrix Multiplication)
    // ========================================================================

    /**
     * Matrix multiplication: (M×K) · (K×N) = (M×N).
     *
     * C[i][j] = Σ(k) A[i][k] * B[k][j]
     */
    fun dot(other: Matrix): Matrix {
        require(cols == other.rows) {
            "Dot dimension mismatch: ${rows}×${cols} · ${other.rows}×${other.cols}"
        }
        return Matrix(Array(rows) { i ->
            DoubleArray(other.cols) { j ->
                var sum = 0.0
                for (k in 0 until cols) sum += data[i][k] * other.data[k][j]
                sum
            }
        })
    }

    // ========================================================================
    // Equality
    // ========================================================================

    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is Matrix) return false
        if (rows != other.rows || cols != other.cols) return false
        for (i in 0 until rows) {
            if (!data[i].contentEquals(other.data[i])) return false
        }
        return true
    }

    override fun hashCode(): Int {
        var result = rows
        for (row in data) result = 31 * result + row.contentHashCode()
        return result
    }

    override fun toString(): String = "Matrix(${rows}×${cols})"

    private fun checkDimensions(other: Matrix, op: String) {
        require(rows == other.rows && cols == other.cols) {
            "$op dimension mismatch: ${rows}×${cols} vs ${other.rows}×${other.cols}"
        }
    }
}
