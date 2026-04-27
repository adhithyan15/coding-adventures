package com.codingadventures.matrix;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class MatrixTest {

    private static final double EPS = 1e-10;

    // ========================================================================
    // Initialization
    // ========================================================================

    @Test void fromScalar() {
        Matrix m = Matrix.fromScalar(5.0);
        assertEquals(1, m.getRows());
        assertEquals(1, m.getCols());
        assertEquals(5.0, m.get(0, 0));
    }

    @Test void fromArray() {
        Matrix m = Matrix.fromArray(new double[]{1, 2, 3});
        assertEquals(1, m.getRows());
        assertEquals(3, m.getCols());
        assertEquals(2.0, m.get(0, 1));
    }

    @Test void from2DArray() {
        Matrix m = new Matrix(new double[][]{{1, 2}, {3, 4}});
        assertEquals(2, m.getRows());
        assertEquals(2, m.getCols());
        assertEquals(3.0, m.get(1, 0));
    }

    @Test void zeros() {
        Matrix m = Matrix.zeros(3, 2);
        assertEquals(3, m.getRows());
        assertEquals(2, m.getCols());
        for (int i = 0; i < 3; i++)
            for (int j = 0; j < 2; j++)
                assertEquals(0.0, m.get(i, j));
    }

    @Test void raggedArrayThrows() {
        assertThrows(IllegalArgumentException.class, () ->
            new Matrix(new double[][]{{1, 2}, {3}}));
    }

    // ========================================================================
    // Addition
    // ========================================================================

    @Test void addMatrices() {
        Matrix a = new Matrix(new double[][]{{1, 2}, {3, 4}});
        Matrix b = new Matrix(new double[][]{{5, 6}, {7, 8}});
        Matrix c = a.add(b);
        assertEquals(new Matrix(new double[][]{{6, 8}, {10, 12}}), c);
    }

    @Test void addScalar() {
        Matrix m = new Matrix(new double[][]{{1, 2}, {3, 4}});
        assertEquals(new Matrix(new double[][]{{11, 12}, {13, 14}}), m.addScalar(10));
    }

    @Test void addDimensionMismatch() {
        assertThrows(IllegalArgumentException.class, () ->
            new Matrix(new double[][]{{1, 2}}).add(new Matrix(new double[][]{{1}, {2}})));
    }

    // ========================================================================
    // Subtraction
    // ========================================================================

    @Test void subtractMatrices() {
        Matrix a = new Matrix(new double[][]{{5, 6}, {7, 8}});
        Matrix b = new Matrix(new double[][]{{1, 2}, {3, 4}});
        assertEquals(new Matrix(new double[][]{{4, 4}, {4, 4}}), a.subtract(b));
    }

    @Test void subtractSelf() {
        Matrix a = new Matrix(new double[][]{{1, 2}, {3, 4}});
        assertEquals(Matrix.zeros(2, 2), a.subtract(a));
    }

    @Test void subtractScalar() {
        Matrix m = new Matrix(new double[][]{{10, 20}});
        assertEquals(new Matrix(new double[][]{{5, 15}}), m.subtractScalar(5));
    }

    // ========================================================================
    // Scale
    // ========================================================================

    @Test void scale() {
        Matrix m = new Matrix(new double[][]{{1, 2}, {3, 4}});
        assertEquals(new Matrix(new double[][]{{2, 4}, {6, 8}}), m.scale(2));
    }

    @Test void scaleByZero() {
        Matrix m = new Matrix(new double[][]{{1, 2}});
        assertEquals(Matrix.zeros(1, 2), m.scale(0));
    }

    // ========================================================================
    // Transpose
    // ========================================================================

    @Test void transposeRectangular() {
        Matrix m = new Matrix(new double[][]{{1, 2, 3}, {4, 5, 6}});
        Matrix t = m.transpose();
        assertEquals(3, t.getRows());
        assertEquals(2, t.getCols());
        assertEquals(new Matrix(new double[][]{{1, 4}, {2, 5}, {3, 6}}), t);
    }

    @Test void doubleTranspose() {
        Matrix m = new Matrix(new double[][]{{1, 2, 3}, {4, 5, 6}});
        assertEquals(m, m.transpose().transpose());
    }

    // ========================================================================
    // Dot Product
    // ========================================================================

    @Test void dot2x2() {
        Matrix a = new Matrix(new double[][]{{1, 2}, {3, 4}});
        Matrix b = new Matrix(new double[][]{{5, 6}, {7, 8}});
        assertEquals(new Matrix(new double[][]{{19, 22}, {43, 50}}), a.dot(b));
    }

    @Test void dotNonSquare() {
        Matrix a = new Matrix(new double[][]{{1, 2, 3}});
        Matrix b = new Matrix(new double[][]{{4}, {5}, {6}});
        Matrix c = a.dot(b);
        assertEquals(1, c.getRows());
        assertEquals(1, c.getCols());
        assertEquals(32.0, c.get(0, 0), EPS);
    }

    @Test void dotIdentity() {
        Matrix a = new Matrix(new double[][]{{1, 2}, {3, 4}});
        Matrix eye = new Matrix(new double[][]{{1, 0}, {0, 1}});
        assertEquals(a, a.dot(eye));
        assertEquals(a, eye.dot(a));
    }

    @Test void dotDimensionMismatch() {
        assertThrows(IllegalArgumentException.class, () ->
            new Matrix(new double[][]{{1, 2}}).dot(new Matrix(new double[][]{{1, 2}})));
    }

    // ========================================================================
    // Equality & Immutability
    // ========================================================================

    @Test void equality() {
        Matrix a = new Matrix(new double[][]{{1, 2}, {3, 4}});
        Matrix b = new Matrix(new double[][]{{1, 2}, {3, 4}});
        assertEquals(a, b);
        assertEquals(a.hashCode(), b.hashCode());
    }

    @Test void inequality() {
        assertNotEquals(
            new Matrix(new double[][]{{1, 2}}),
            new Matrix(new double[][]{{1, 3}}));
    }

    @Test void immutability() {
        Matrix a = new Matrix(new double[][]{{1, 2}, {3, 4}});
        a.add(new Matrix(new double[][]{{10, 20}, {30, 40}}));
        assertEquals(1.0, a.get(0, 0));
    }

    @Test void getDataReturnsCopy() {
        Matrix m = new Matrix(new double[][]{{1, 2}});
        double[][] copy = m.getData();
        copy[0][0] = 999;
        assertEquals(1.0, m.get(0, 0));
    }
}
