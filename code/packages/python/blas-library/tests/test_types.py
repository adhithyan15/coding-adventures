"""Tests for BLAS data types — Matrix, Vector, enums, and converters."""

from __future__ import annotations

import pytest

from blas_library import Matrix, Side, StorageOrder, Transpose, Vector

# =========================================================================
# Vector tests
# =========================================================================


class TestVector:
    """Tests for the Vector dataclass."""

    def test_create_vector(self) -> None:
        """Create a simple vector and verify fields."""
        v = Vector(data=[1.0, 2.0, 3.0], size=3)
        assert v.data == [1.0, 2.0, 3.0]
        assert v.size == 3

    def test_empty_vector(self) -> None:
        """Create an empty vector."""
        v = Vector(data=[], size=0)
        assert v.size == 0
        assert v.data == []

    def test_single_element_vector(self) -> None:
        """Create a vector with one element."""
        v = Vector(data=[42.0], size=1)
        assert v.size == 1
        assert v.data[0] == 42.0

    def test_size_mismatch_raises(self) -> None:
        """Data length != size should raise ValueError."""
        with pytest.raises(ValueError, match="3 elements but size=2"):
            Vector(data=[1.0, 2.0, 3.0], size=2)

    def test_size_mismatch_too_few(self) -> None:
        """Too few elements for declared size."""
        with pytest.raises(ValueError):
            Vector(data=[1.0], size=5)

    def test_negative_values(self) -> None:
        """Vectors can contain negative values."""
        v = Vector(data=[-1.0, -2.0, 0.0, 3.0], size=4)
        assert v.data[0] == -1.0


# =========================================================================
# Matrix tests
# =========================================================================


class TestMatrix:
    """Tests for the Matrix dataclass."""

    def test_create_matrix(self) -> None:
        """Create a 2x3 matrix and verify fields."""
        m = Matrix(data=[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows=2, cols=3)
        assert m.rows == 2
        assert m.cols == 3
        assert len(m.data) == 6

    def test_element_access(self) -> None:
        """Access elements using row-major indexing: data[i*cols+j]."""
        m = Matrix(data=[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows=2, cols=3)
        # Element at row 0, col 2 should be 3.0
        assert m.data[0 * 3 + 2] == 3.0
        # Element at row 1, col 0 should be 4.0
        assert m.data[1 * 3 + 0] == 4.0

    def test_1x1_matrix(self) -> None:
        """Create a scalar-like 1x1 matrix."""
        m = Matrix(data=[42.0], rows=1, cols=1)
        assert m.rows == 1
        assert m.cols == 1
        assert m.data[0] == 42.0

    def test_shape_mismatch_raises(self) -> None:
        """Data length != rows*cols should raise ValueError."""
        with pytest.raises(ValueError, match="5 elements"):
            Matrix(data=[1.0, 2.0, 3.0, 4.0, 5.0], rows=2, cols=3)

    def test_default_order_is_row_major(self) -> None:
        """Default storage order should be row-major."""
        m = Matrix(data=[1.0, 2.0, 3.0, 4.0], rows=2, cols=2)
        assert m.order == StorageOrder.ROW_MAJOR

    def test_column_major_order(self) -> None:
        """Can create a column-major matrix."""
        m = Matrix(
            data=[1.0, 4.0, 2.0, 5.0, 3.0, 6.0],
            rows=2,
            cols=3,
            order=StorageOrder.COLUMN_MAJOR,
        )
        assert m.order == StorageOrder.COLUMN_MAJOR

    def test_square_matrix(self) -> None:
        """Create a square matrix."""
        m = Matrix(data=[1.0, 0.0, 0.0, 1.0], rows=2, cols=2)
        assert m.rows == m.cols == 2

    def test_zero_matrix(self) -> None:
        """Create a matrix of all zeros."""
        m = Matrix(data=[0.0] * 6, rows=2, cols=3)
        assert all(v == 0.0 for v in m.data)


# =========================================================================
# Enum tests
# =========================================================================


class TestEnums:
    """Tests for StorageOrder, Transpose, and Side enums."""

    def test_storage_order_values(self) -> None:
        """StorageOrder has ROW_MAJOR and COLUMN_MAJOR."""
        assert StorageOrder.ROW_MAJOR.value == "row_major"
        assert StorageOrder.COLUMN_MAJOR.value == "column_major"

    def test_transpose_values(self) -> None:
        """Transpose has NO_TRANS and TRANS."""
        assert Transpose.NO_TRANS.value == "no_trans"
        assert Transpose.TRANS.value == "trans"

    def test_side_values(self) -> None:
        """Side has LEFT and RIGHT."""
        assert Side.LEFT.value == "left"
        assert Side.RIGHT.value == "right"

    def test_enums_are_distinct(self) -> None:
        """Each enum member is distinct."""
        assert Transpose.NO_TRANS != Transpose.TRANS
        assert Side.LEFT != Side.RIGHT
        assert StorageOrder.ROW_MAJOR != StorageOrder.COLUMN_MAJOR
