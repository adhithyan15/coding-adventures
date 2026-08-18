"""Unit tests for the SIR22 array/matrix Python runtime."""

from __future__ import annotations

import math

import pytest

import coding_adventures_sir_runtime_array.array as array_mod
from coding_adventures_sir_runtime_array import (
    MAX_ELEMENTS,
    NDArray,
    apply_op,
    checked_shape_size,
    elementwise,
    from_rows,
    from_vec,
    get,
    index_get,
    index_range,
    index_scalar,
    index_set,
    index_whole,
    is_scalar,
    matmul,
    ncols,
    ndarray,
    ndims,
    nrows,
    scalar,
    to_array_value,
    transpose,
    zeros,
)
from coding_adventures_sir_runtime_array import range as sir_range
from coding_adventures_sir_runtime_array import set as sir_set


class TestCheckedShapeSize:
    def test_scalar_shape_is_one_element(self) -> None:
        assert checked_shape_size(()) == 1

    def test_matrix_shape_multiplies_dims(self) -> None:
        assert checked_shape_size((3, 4)) == 12

    def test_rejects_negative_dimension(self) -> None:
        with pytest.raises(ValueError, match="negative or non-integer"):
            checked_shape_size((-1, 2))

    def test_rejects_non_integer_dimension(self) -> None:
        with pytest.raises(ValueError, match="negative or non-integer"):
            checked_shape_size((1.5, 2))  # type: ignore[arg-type]

    def test_rejects_bool_dimension(self) -> None:
        # bool is an int subclass in Python; a shape dimension must be a
        # real int, not a stray True/False.
        with pytest.raises(ValueError, match="negative or non-integer"):
            checked_shape_size((True, 2))  # type: ignore[arg-type]

    def test_rejects_shape_exceeding_max_elements_cap(self) -> None:
        with pytest.raises(ValueError, match="exceeds the .*-element cap"):
            checked_shape_size((MAX_ELEMENTS + 1,))

    def test_accepts_shape_at_exactly_the_cap(self) -> None:
        assert checked_shape_size((MAX_ELEMENTS,)) == MAX_ELEMENTS


class TestNdarrayConstruction:
    def test_ndarray_rejects_length_mismatch(self) -> None:
        with pytest.raises(ValueError, match="implies 4 elements, got 3"):
            ndarray((2, 2), [1, 2, 3])

    def test_ndarray_rejects_non_list_data(self) -> None:
        with pytest.raises(TypeError, match="data must be a list"):
            ndarray((2,), (1, 2))  # type: ignore[arg-type]

    def test_scalar_factory(self) -> None:
        a = scalar(7)
        assert a.shape == ()
        assert a.data == [7]
        assert is_scalar(a)

    def test_from_vec_factory(self) -> None:
        a = from_vec([1, 2, 3])
        assert a.shape == (3,)
        assert a.data == [1, 2, 3]

    def test_zeros_factory(self) -> None:
        a = zeros(2, 3)
        assert a.shape == (2, 3)
        assert a.data == [0] * 6

    def test_from_rows_empty(self) -> None:
        a = from_rows([])
        assert a.shape == (0, 0)
        assert a.data == []

    def test_from_rows_rejects_ragged_rows(self) -> None:
        with pytest.raises(ValueError, match="ragged rows"):
            from_rows([[1, 2], [3]])

    def test_from_rows_stores_column_major(self) -> None:
        # [[1, 2], [3, 4]] -- row 0 = (1, 2), row 1 = (3, 4). Column-major
        # layout is data[c * nrows + r], so element order is
        # (0,0)=1, (1,0)=3, (0,1)=2, (1,1)=4 -> [1, 3, 2, 4].
        a = from_rows([[1, 2], [3, 4]])
        assert a.shape == (2, 2)
        assert a.data == [1, 3, 2, 4]

    def test_ndarray_equality(self) -> None:
        assert from_rows([[1, 2]]) == from_rows([[1, 2]])
        assert from_rows([[1, 2]]) != from_rows([[1, 3]])
        assert from_rows([[1, 2]]).__eq__(42) is NotImplemented

    def test_ndarray_repr_mentions_shape_and_data(self) -> None:
        r = repr(scalar(5))
        assert "shape=" in r
        assert "data=" in r


class TestShapeQueries:
    def test_ndims(self) -> None:
        assert ndims(scalar(1)) == 0
        assert ndims(from_vec([1, 2])) == 1
        assert ndims(from_rows([[1, 2]])) == 2

    def test_nrows_ncols_scalar_treated_as_1x1(self) -> None:
        a = scalar(1)
        assert nrows(a) == 1
        assert ncols(a) == 1

    def test_nrows_ncols_vector_treated_as_nx1(self) -> None:
        a = from_vec([1, 2, 3])
        assert nrows(a) == 3
        assert ncols(a) == 1

    def test_nrows_ncols_matrix(self) -> None:
        a = from_rows([[1, 2, 3], [4, 5, 6]])
        assert nrows(a) == 2
        assert ncols(a) == 3


class TestGetSet:
    def test_get_reads_column_major_element(self) -> None:
        a = from_rows([[1, 2], [3, 4]])
        assert get(a, 0, 0) == 1
        assert get(a, 0, 1) == 2
        assert get(a, 1, 0) == 3
        assert get(a, 1, 1) == 4

    def test_get_out_of_bounds_returns_none(self) -> None:
        a = from_rows([[1, 2], [3, 4]])
        assert get(a, 2, 0) is None
        assert get(a, 0, -1) is None

    def test_set_mutates_in_place(self) -> None:
        a = from_rows([[1, 2], [3, 4]])
        sir_set(a, 0, 1, 99)
        assert get(a, 0, 1) == 99

    def test_set_out_of_bounds_raises(self) -> None:
        a = from_rows([[1, 2], [3, 4]])
        with pytest.raises(ValueError, match="out of bounds"):
            sir_set(a, 5, 0, 1)

    def test_set_with_nan_index_raises_instead_of_silently_writing(self) -> None:
        # SECURITY regression: an OR-form bounds check (`r < 0 or ...`)
        # would let r=NaN sail through (every relational comparison with
        # NaN is False), so this must raise, not silently misbehave.
        a = from_rows([[1, 2], [3, 4]])
        with pytest.raises(ValueError, match="out of bounds"):
            sir_set(a, float("nan"), 0, 1)


class TestApplyOp:
    @pytest.mark.parametrize(
        ("op", "a", "b", "expected"),
        [
            ("Add", 2, 3, 5),
            ("Sub", 5, 3, 2),
            ("Mul", 4, 3, 12),
            ("Pow", 2, 3, 8),
            ("Max", 2, 3, 3),
            ("Min", 2, 3, 2),
        ],
    )
    def test_arithmetic_ops(self, op: str, a: int, b: int, expected: int) -> None:
        result = apply_op(op, a, b)  # type: ignore[arg-type]
        assert result == expected
        assert type(result) is int  # int-preserving, not forced float

    def test_div_always_true_divides(self) -> None:
        result = apply_op("Div", 7, 2)
        assert result == 3.5
        assert isinstance(result, float)

    def test_div_of_evenly_divisible_ints_is_still_float(self) -> None:
        # MATLAB `./` is always real division -- 4/2 must be 2.0, not
        # Python's floor-division 2.
        result = apply_op("Div", 4, 2)
        assert result == 2.0
        assert isinstance(result, float)

    @pytest.mark.parametrize(
        ("op", "a", "b", "expected"),
        [
            ("Eq", 3, 3, 1),
            ("Eq", 3, 4, 0),
            ("Ne", 3, 4, 1),
            ("Ne", 3, 3, 0),
            ("Lt", 2, 3, 1),
            ("Lt", 3, 2, 0),
            ("Le", 3, 3, 1),
            ("Ge", 3, 3, 1),
            ("Gt", 4, 3, 1),
        ],
    )
    def test_comparisons_return_int_1_or_0_never_bool(
        self, op: str, a: int, b: int, expected: int
    ) -> None:
        result = apply_op(op, a, b)  # type: ignore[arg-type]
        assert result == expected
        assert type(result) is int
        assert not isinstance(result, bool)

    def test_unrecognised_op_raises(self) -> None:
        with pytest.raises(ValueError, match="unrecognised ElementwiseOpKind"):
            apply_op("Bogus", 1, 2)  # type: ignore[arg-type]


class TestToArrayValue:
    def test_bare_number_is_wrapped_as_scalar(self) -> None:
        wrapped = to_array_value(2)
        assert isinstance(wrapped, NDArray)
        assert wrapped.shape == ()
        assert wrapped.data == [2]

    def test_ndarray_passes_through_unchanged(self) -> None:
        a = scalar(5)
        assert to_array_value(a) is a


class TestElementwise:
    def test_elementwise_matching_shapes(self) -> None:
        a = from_rows([[1, 2], [3, 4]])
        b = from_rows([[10, 20], [30, 40]])
        result = elementwise("Add", a, b)
        assert result.data == [11, 33, 22, 44]

    def test_elementwise_scalar_rhs_broadcasts(self) -> None:
        # MATLAB `A .* 2` -- the `2` arrives as a bare int, not an
        # ArrayLit/scalar-array. Regression coverage for `to_array_value`.
        a = from_rows([[1, 2], [3, 4]])
        result = elementwise("Mul", a, 2)
        assert result.shape == (2, 2)
        assert get(result, 0, 0) == 2
        assert get(result, 1, 1) == 8

    def test_elementwise_scalar_lhs_broadcasts(self) -> None:
        a = from_rows([[1, 2], [3, 4]])
        result = elementwise("Add", 10, a)
        assert result.shape == (2, 2)
        assert get(result, 0, 0) == 11

    def test_elementwise_both_scalars(self) -> None:
        result = elementwise("Add", 1, 2)
        assert result.shape == ()
        assert result.data == [3]

    def test_elementwise_shape_mismatch_raises(self) -> None:
        a = from_rows([[1, 2]])
        b = from_rows([[1], [2], [3]])
        with pytest.raises(ValueError, match="non-conformable"):
            elementwise("Add", a, b)


class TestMatmul:
    def test_known_two_by_two_product(self) -> None:
        # [1 2; 3 4] * [5 6; 7 8] = [19 22; 43 50]
        a = from_rows([[1, 2], [3, 4]])
        b = from_rows([[5, 6], [7, 8]])
        p = matmul(a, b)
        assert p.shape == (2, 2)
        assert get(p, 0, 0) == 19
        assert get(p, 0, 1) == 22
        assert get(p, 1, 0) == 43
        assert get(p, 1, 1) == 50

    def test_matmul_preserves_int_type_for_all_integer_inputs(self) -> None:
        p = matmul(from_rows([[1, 2]]), from_rows([[3], [4]]))
        assert p.data == [11]
        assert type(p.data[0]) is int

    def test_matmul_inner_dimension_mismatch_raises(self) -> None:
        a = from_rows([[1, 2]])  # 1x2
        b = from_rows([[1, 2]])  # 1x2, inner dims (2 vs 1) disagree
        with pytest.raises(ValueError, match="inner dimensions disagree"):
            matmul(a, b)

    def test_matmul_rejects_outer_product_shaped_dos_before_allocating(self) -> None:
        # Each operand is individually small, but their product (an
        # outer-product shape) would exceed MAX_ELEMENTS -- must be
        # rejected before `[0] * out_len` ever allocates.
        a = zeros(9000, 1)
        b = zeros(1, 9000)
        with pytest.raises(ValueError, match="exceeds the .*-element cap"):
            matmul(a, b)


class TestTranspose:
    def test_transpose_swaps_rows_and_columns(self) -> None:
        # [1 2 3; 4 5 6]' = [1 4; 2 5; 3 6]
        a = from_rows([[1, 2, 3], [4, 5, 6]])
        t = transpose(a, conjugate=True)
        assert t.shape == (3, 2)
        assert get(t, 0, 0) == 1
        assert get(t, 0, 1) == 4
        assert get(t, 2, 1) == 6

    def test_transpose_conjugate_is_a_no_op_on_real_data(self) -> None:
        a = from_rows([[1, 2], [3, 4]])
        assert transpose(a, conjugate=True) == transpose(a, conjugate=False)

    def test_transpose_default_conjugate_is_false(self) -> None:
        a = from_rows([[1, 2], [3, 4]])
        assert transpose(a) == transpose(a, conjugate=False)


class TestRange:
    def test_basic_ascending_range(self) -> None:
        r = sir_range(1, 5)
        assert r.shape == (1, 5)
        assert r.data == [1, 2, 3, 4, 5]

    def test_stepped_range(self) -> None:
        r = sir_range(1, 9, 2)
        assert r.data == [1, 3, 5, 7, 9]

    def test_descending_range(self) -> None:
        r = sir_range(5, 1, -1)
        assert r.data == [5, 4, 3, 2, 1]

    def test_inclusive_stop_with_float_step_epsilon_tolerance(self) -> None:
        r = sir_range(0, 1, 0.25)
        assert r.data == pytest.approx([0, 0.25, 0.5, 0.75, 1.0])

    def test_empty_range_when_direction_disagrees(self) -> None:
        r = sir_range(5, 1, 1)
        assert r.shape == (1, 0)
        assert r.data == []

    def test_zero_step_raises(self) -> None:
        with pytest.raises(ValueError, match="step cannot be zero"):
            sir_range(1, 5, 0)

    def test_non_finite_bound_raises(self) -> None:
        with pytest.raises(ValueError, match="must be finite"):
            sir_range(float("nan"), 5)
        with pytest.raises(ValueError, match="must be finite"):
            sir_range(1, float("inf"))

    def test_range_exceeding_max_elements_cap_raises(self, monkeypatch: pytest.MonkeyPatch) -> None:
        monkeypatch.setattr(array_mod, "MAX_ELEMENTS", 3)
        with pytest.raises(ValueError, match="produces more than"):
            sir_range(1, 10)


class TestIndexArgConstructors:
    def test_index_scalar(self) -> None:
        arg = index_scalar(3)
        assert arg.kind == "scalar"
        assert arg.value == 3

    def test_index_whole(self) -> None:
        arg = index_whole()
        assert arg.kind == "whole"
        assert arg.value is None

    def test_index_range(self) -> None:
        indices = from_vec([0, 2])
        arg = index_range(indices)
        assert arg.kind == "range"
        assert arg.value is indices

    def test_repr(self) -> None:
        assert "kind='scalar'" in repr(index_scalar(1))


class TestResolvePositionsAndAssertValidPosition:
    def test_assert_valid_position_accepts_int(self) -> None:
        assert array_mod._assert_valid_position(3) == 3

    def test_assert_valid_position_accepts_integer_valued_float(self) -> None:
        assert array_mod._assert_valid_position(3.0) == 3

    def test_assert_valid_position_rejects_fractional_float(self) -> None:
        with pytest.raises(ValueError, match="not a finite integer"):
            array_mod._assert_valid_position(3.5)

    def test_assert_valid_position_rejects_nan(self) -> None:
        # NaN.is_integer() is False, closing the same "comparison-based
        # check silently passes NaN" hazard the module docstring covers.
        with pytest.raises(ValueError, match="not a finite integer"):
            array_mod._assert_valid_position(float("nan"))

    def test_assert_valid_position_rejects_infinity(self) -> None:
        with pytest.raises(ValueError, match="not a finite integer"):
            array_mod._assert_valid_position(float("inf"))

    def test_assert_valid_position_rejects_bool(self) -> None:
        with pytest.raises(ValueError, match="not a finite integer"):
            array_mod._assert_valid_position(True)

    def test_assert_valid_position_rejects_non_numeric(self) -> None:
        with pytest.raises(ValueError, match="not a finite integer"):
            array_mod._assert_valid_position("3")  # type: ignore[arg-type]

    def test_resolve_positions_scalar(self) -> None:
        assert array_mod._resolve_positions(index_scalar(2), 5) == [2]

    def test_resolve_positions_whole(self) -> None:
        assert array_mod._resolve_positions(index_whole(), 4) == [0, 1, 2, 3]

    def test_resolve_positions_range_truncates(self) -> None:
        indices = from_vec([0.0, 1.9, 3.0])
        assert array_mod._resolve_positions(index_range(indices), 5) == [0, 1, 3]

    def test_resolve_positions_range_with_nan_element_raises(self) -> None:
        indices = from_vec([0.0, float("nan")])
        with pytest.raises(ValueError, match="not a finite integer"):
            array_mod._resolve_positions(index_range(indices), 5)

    def test_resolve_positions_unrecognised_kind_raises(self) -> None:
        bad = array_mod.IndexArg("bogus", None)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="unrecognised IndexArg"):
            array_mod._resolve_positions(bad, 5)


class TestIndexGet:
    def test_rank_one_scalar_read(self) -> None:
        a = from_rows([[1, 2], [3, 4]])  # data column-major: [1, 3, 2, 4]
        assert index_get(a, [index_scalar(2)]) == 2

    def test_rank_one_out_of_bounds_raises(self) -> None:
        a = from_vec([1, 2, 3])
        with pytest.raises(ValueError, match="linear index .* out of bounds"):
            index_get(a, [index_scalar(10)])

    def test_rank_one_range_reads_sub_array(self) -> None:
        a = from_vec([10, 20, 30, 40])
        sub = index_get(a, [index_range(from_vec([0, 2]))])
        assert isinstance(sub, NDArray)
        assert sub.data == [10, 30]

    def test_rank_two_scalar_scalar_reads_element(self) -> None:
        a = from_rows([[1, 2], [3, 4]])
        assert index_get(a, [index_scalar(1), index_scalar(1)]) == 4

    def test_rank_two_whole_selector_reads_entire_row(self) -> None:
        # A(1, :) on [[1, 2], [3, 4]] reads the whole second row [3, 4].
        a = from_rows([[1, 2], [3, 4]])
        row = index_get(a, [index_scalar(1), index_whole()])
        assert isinstance(row, NDArray)
        assert row.shape == (1, 2)
        assert row.data == [3, 4]

    def test_rank_two_out_of_bounds_raises(self) -> None:
        a = from_rows([[1, 2], [3, 4]])
        with pytest.raises(ValueError, match=r"\(5, 0\) out of bounds"):
            index_get(a, [index_scalar(5), index_scalar(0)])

    def test_unsupported_rank_raises(self) -> None:
        a = from_rows([[1, 2], [3, 4]])
        with pytest.raises(ValueError, match="only 1 or 2 index arguments"):
            index_get(a, [index_scalar(0), index_scalar(0), index_scalar(0)])

    def test_rank_two_outer_product_shaped_dos_before_allocating(self) -> None:
        # `a` itself is tiny -- the hazard is the *selection* sizes
        # multiplying to exceed the cap, not `a`'s own size, so this must
        # be rejected before `read2` ever attempts an out-of-bounds read.
        a = zeros(1, 1)
        rows = index_range(from_vec(list(range(9000))))
        cols = index_range(from_vec(list(range(9000))))
        with pytest.raises(ValueError, match="exceeds the .*-element cap"):
            index_get(a, [rows, cols])


class TestIndexSet:
    def test_rank_two_scalar_scalar_mutates_in_place(self) -> None:
        a = from_rows([[1, 2], [3, 4]])
        index_set(a, [index_scalar(1), index_scalar(1)], 99)
        assert get(a, 1, 1) == 99

    def test_rank_one_scalar_mutates_in_place(self) -> None:
        a = from_vec([1, 2, 3])
        index_set(a, [index_scalar(1)], 99)
        assert a.data == [1, 99, 3]

    def test_broadcast_scalar_value_to_whole_selection(self) -> None:
        a = from_rows([[1, 2], [3, 4]])
        index_set(a, [index_whole(), index_scalar(0)], 0)
        assert get(a, 0, 0) == 0
        assert get(a, 1, 0) == 0
        assert get(a, 0, 1) == 2  # untouched column

    def test_broadcast_scalar_ndarray_value_to_whole_selection(self) -> None:
        # A value that is itself a scalar-shaped NDArray (not a bare
        # number) must broadcast the same way a bare scalar does.
        a = from_rows([[1, 2], [3, 4]])
        index_set(a, [index_whole(), index_scalar(0)], scalar(0))
        assert get(a, 0, 0) == 0
        assert get(a, 1, 0) == 0
        assert get(a, 0, 1) == 2  # untouched column

    def test_broadcast_ndarray_value_of_matching_length(self) -> None:
        a = from_rows([[1, 2], [3, 4]])
        index_set(a, [index_whole(), index_scalar(0)], from_vec([10, 20]))
        assert get(a, 0, 0) == 10
        assert get(a, 1, 0) == 20

    def test_broadcast_ndarray_value_length_mismatch_raises(self) -> None:
        a = from_rows([[1, 2], [3, 4]])
        with pytest.raises(ValueError, match="expected 2"):
            index_set(a, [index_whole(), index_scalar(0)], from_vec([10, 20, 30]))

    def test_rank_one_out_of_bounds_raises(self) -> None:
        a = from_vec([1, 2, 3])
        with pytest.raises(ValueError, match="linear index .* out of bounds"):
            index_set(a, [index_scalar(10)], 1)

    def test_unsupported_rank_raises(self) -> None:
        a = from_rows([[1, 2], [3, 4]])
        with pytest.raises(ValueError, match="only 1 or 2 index arguments"):
            index_set(a, [index_scalar(0), index_scalar(0), index_scalar(0)], 1)


class TestPublicAliases:
    def test_range_alias_is_range_(self) -> None:
        assert array_mod.range_ is sir_range

    def test_set_alias_is_set_(self) -> None:
        assert array_mod.set_ is sir_set

    def test_builtin_range_still_usable_inside_the_module(self) -> None:
        # Regression guard for the exact hazard the trailing-underscore
        # convention avoids: if `array.py` had bound its function to the
        # bare name `range`, its own internal loops (`from_rows`,
        # `matmul`, `transpose`) would break. This factory only works if
        # the module can still call the *builtin* `range()` internally.
        a = from_rows([[1, 2], [3, 4]])
        assert a.data == [1, 3, 2, 4]


class TestMathIsfiniteSanity:
    def test_isfinite_matches_expectations(self) -> None:
        # Cheap sanity check that this module's NaN/inf reasoning rests on
        # correct assumptions about `math.isfinite`.
        assert math.isfinite(1.0)
        assert not math.isfinite(float("nan"))
        assert not math.isfinite(float("inf"))
