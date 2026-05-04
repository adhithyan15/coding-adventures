"""Phase 19 tests — eigenvalues, eigenvectors, LU, subspaces, norm, charpoly.

This file validates the six new operations added to cas-matrix 0.3.0:

- ``charpoly`` — characteristic polynomial as IR expression
- ``eigenvalues`` — eigenvalue list with algebraic multiplicity
- ``eigenvectors`` — eigenspace bases
- ``lu_decompose`` — LU with partial pivoting (P·A = L·U)
- ``nullspace`` / ``columnspace`` / ``rowspace`` — subspace bases
- ``norm`` — Euclidean and Frobenius norms
"""

from __future__ import annotations

import pytest
from symbolic_ir import ADD, IRApply, IRInteger, IRRational, IRSymbol

from cas_matrix import (
    MatrixError,
    charpoly,
    columnspace,
    eigenvalues,
    eigenvectors,
    identity_matrix,
    lu_decompose,
    matrix,
    norm,
    nullspace,
    rowspace,
    zero_matrix,
)
from cas_matrix.eigenvalues import char_poly_coeffs
from cas_matrix.matrix import _rows_of, num_cols, num_rows
from cas_matrix.rowreduce import _entry_to_fraction

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _i(n: int) -> IRInteger:
    return IRInteger(n)


def _r(p: int, q: int) -> IRRational:
    return IRRational(p, q)


def _mat(*rows):
    """Build a Matrix from int rows."""
    return matrix([[_i(v) for v in row] for row in rows])


def _entries(M):
    """Extract (numer, denom) pairs from a Matrix for easy assertion."""
    result = []
    for row in _rows_of(M):
        result.append([
            (e.value, 1) if isinstance(e, IRInteger)
            else (e.numer, e.denom)
            for e in row
        ])
    return result


def _eval_ir(node, x: float = 1.5) -> float:
    """Numerically evaluate an IR node (for polynomial/eigenvalue checks)."""
    from symbolic_ir import (
        DIV,
        MUL,
        NEG,
        POW,
        SQRT,
        SUB,
        IRApply,
        IRInteger,
        IRRational,
        IRSymbol,
    )
    LAM = IRSymbol("lambda")

    def ev(n):
        if isinstance(n, IRInteger):
            return float(n.value)
        if isinstance(n, IRRational):
            return n.numer / n.denom
        if isinstance(n, IRSymbol):
            if n == LAM:
                return x
            return 0.0
        if not isinstance(n, IRApply):
            return 0.0
        h, args = n.head, n.args
        if h == ADD:
            return sum(ev(a) for a in args)
        if h == SUB:
            return ev(args[0]) - ev(args[1])
        if h == MUL:
            r = 1.0
            for a in args:
                r *= ev(a)
            return r
        if h == DIV:
            d = ev(args[1])
            return ev(args[0]) / d if d else 1e18
        if h == NEG:
            return -ev(args[0])
        if h == POW:
            try:
                return ev(args[0]) ** ev(args[1])
            except (ValueError, ZeroDivisionError):
                return 0.0
        if h == SQRT:
            v = ev(args[0])
            return v**0.5 if v >= 0 else 0.0
        return 0.0

    return ev(node)


def _is_lower_triangular(M) -> bool:
    """Check that M is lower triangular (all entries above diagonal are 0)."""
    rows = _rows_of(M)
    n = len(rows)
    for i in range(n):
        for j in range(i + 1, n):
            e = rows[i][j]
            if isinstance(e, IRInteger) and e.value != 0:
                return False
            if isinstance(e, IRRational) and e.numer != 0:
                return False
    return True


def _is_upper_triangular(M) -> bool:
    """Check that M is upper triangular (all entries below diagonal are 0)."""
    rows = _rows_of(M)
    n = len(rows)
    for i in range(n):
        for j in range(0, i):
            e = rows[i][j]
            if isinstance(e, IRInteger) and e.value != 0:
                return False
            if isinstance(e, IRRational) and e.numer != 0:
                return False
    return True


def _diagonal(M):
    """Return diagonal entries as (numer, denom) pairs."""
    rows = _rows_of(M)
    result = []
    for i, row in enumerate(rows):
        e = row[i]
        if isinstance(e, IRInteger):
            result.append((e.value, 1))
        else:
            result.append((e.numer, e.denom))
    return result


def _mat_mul_fracs(A_rows, B_rows):
    """Fraction matrix multiply: returns list-of-lists of Fraction."""
    m, k = len(A_rows), len(A_rows[0])
    n = len(B_rows[0])
    return [
        [sum(A_rows[i][p] * B_rows[p][j] for p in range(k)) for j in range(n)]
        for i in range(m)
    ]


def _ir_mat_to_fracs(M):
    """Extract matrix entries as Fractions."""
    from cas_matrix.rowreduce import _entry_to_fraction
    return [[_entry_to_fraction(e) for e in row] for row in _rows_of(M)]


# ---------------------------------------------------------------------------
# Section 1 — Characteristic polynomial
# ---------------------------------------------------------------------------


class TestPhase19_CharPoly:
    """charpoly(A, λ) returns det(λI − A) as an IR polynomial in λ."""

    def test_1x1(self):
        A = _mat([5])
        lam = IRSymbol("lambda")
        cp = charpoly(A, lam)
        # char poly = λ − 5; evaluate at λ=7: 7-5=2
        assert abs(_eval_ir(cp, 7.0) - 2.0) < 1e-9

    def test_2x2_integer(self):
        # [[1,2],[2,1]]: char poly = (λ-1)²-4 = λ²-2λ-3
        A = _mat([1, 2], [2, 1])
        lam = IRSymbol("lambda")
        cp = charpoly(A, lam)
        # At λ=3: 9-6-3=0 (root); at λ=-1: 1+2-3=0 (root)
        assert abs(_eval_ir(cp, 3.0)) < 1e-9
        assert abs(_eval_ir(cp, -1.0)) < 1e-9
        # At λ=0: -3
        assert abs(_eval_ir(cp, 0.0) - (-3.0)) < 1e-9

    def test_2x2_rational_entries(self):
        # [[1/2, 1],[0, 2]]: char poly = (λ-1/2)(λ-2) = λ²-5/2·λ+1
        A = matrix([[_r(1, 2), _i(1)], [_i(0), _i(2)]])
        lam = IRSymbol("lambda")
        cp = charpoly(A, lam)
        # At λ=1/2 and λ=2: should be roots
        assert abs(_eval_ir(cp, 0.5)) < 1e-9
        assert abs(_eval_ir(cp, 2.0)) < 1e-9

    def test_2x2_identity(self):
        # identity: char poly = (λ-1)² = λ²-2λ+1
        A = identity_matrix(2)
        lam = IRSymbol("lambda")
        cp = charpoly(A, lam)
        # At λ=1: root; at λ=2: 4-4+1=1
        assert abs(_eval_ir(cp, 1.0)) < 1e-9
        assert abs(_eval_ir(cp, 2.0) - 1.0) < 1e-9

    def test_3x3_diagonal(self):
        # diag(1,3,5): char poly = (λ-1)(λ-3)(λ-5)
        D = _mat([1, 0, 0], [0, 3, 0], [0, 0, 5])
        lam = IRSymbol("lambda")
        cp = charpoly(D, lam)
        assert abs(_eval_ir(cp, 1.0)) < 1e-9
        assert abs(_eval_ir(cp, 3.0)) < 1e-9
        assert abs(_eval_ir(cp, 5.0)) < 1e-9
        # Non-roots should be non-zero
        assert abs(_eval_ir(cp, 2.0)) > 0.5

    def test_3x3_general(self):
        # [[2,1,0],[1,2,1],[0,1,2]]: tridiagonal
        A = _mat([2, 1, 0], [1, 2, 1], [0, 1, 2])
        lam = IRSymbol("lambda")
        cp = charpoly(A, lam)
        # Roots are 2-√2, 2, 2+√2  (≈ 0.586, 2, 3.414)
        assert abs(_eval_ir(cp, 2.0)) < 1e-9
        assert abs(_eval_ir(cp, 2 - 2**0.5)) < 1e-6
        assert abs(_eval_ir(cp, 2 + 2**0.5)) < 1e-6

    def test_char_poly_coeffs_2x2(self):
        from fractions import Fraction
        A = _mat([1, 2], [2, 1])
        coeffs = char_poly_coeffs(A)
        # det(λI-A) = λ²-2λ-3 → [−3, −2, 1]
        assert coeffs == [Fraction(-3), Fraction(-2), Fraction(1)]

    def test_non_square_raises(self):
        A = _mat([1, 2, 3], [4, 5, 6])
        lam = IRSymbol("lambda")
        with pytest.raises(MatrixError):
            charpoly(A, lam)


# ---------------------------------------------------------------------------
# Section 2 — Eigenvalues
# ---------------------------------------------------------------------------


class TestPhase19_Eigenvalues:
    """eigenvalues(A) → List(List(λ, m), …)."""

    def _get_eig_pairs(self, M):
        """Return list of (float_val, int_mult) from eigenvalues(M)."""
        eigs = eigenvalues(M)
        assert isinstance(eigs, IRApply)
        pairs = []
        for pair in eigs.args:
            lam_ir, mult_ir = pair.args
            pairs.append((_eval_ir(lam_ir, 0.0), mult_ir.value))
        return pairs

    def test_1x1(self):
        A = _mat([7])
        pairs = self._get_eig_pairs(A)
        assert len(pairs) == 1
        assert abs(pairs[0][0] - 7.0) < 1e-9
        assert pairs[0][1] == 1

    def test_2x2_integer_distinct(self):
        # [[1,2],[2,1]]: eigs -1, 3
        A = _mat([1, 2], [2, 1])
        pairs = sorted(self._get_eig_pairs(A))
        assert abs(pairs[0][0] - (-1.0)) < 1e-9
        assert pairs[0][1] == 1
        assert abs(pairs[1][0] - 3.0) < 1e-9
        assert pairs[1][1] == 1

    def test_2x2_repeated_eigenvalue(self):
        # [[2,0],[0,2]]: eig 2 with multiplicity 2
        A = _mat([2, 0], [0, 2])
        pairs = self._get_eig_pairs(A)
        assert len(pairs) == 1
        assert abs(pairs[0][0] - 2.0) < 1e-9
        assert pairs[0][1] == 2

    def test_2x2_rational_eigenvalue(self):
        # [[1/2, 0],[0, 3]]: eigs 1/2 and 3
        A = matrix([[_r(1, 2), _i(0)], [_i(0), _i(3)]])
        pairs = sorted(self._get_eig_pairs(A))
        assert abs(pairs[0][0] - 0.5) < 1e-9
        assert abs(pairs[1][0] - 3.0) < 1e-9

    def test_3x3_diagonal(self):
        D = _mat([1, 0, 0], [0, 3, 0], [0, 0, 5])
        pairs = sorted(self._get_eig_pairs(D))
        assert len(pairs) == 3
        expected = [(1.0, 1), (3.0, 1), (5.0, 1)]
        for (pv, pm), (ev, em) in zip(pairs, expected, strict=False):
            assert abs(pv - ev) < 1e-9
            assert pm == em

    def test_3x3_repeated(self):
        # [[3,0,0],[0,3,0],[0,0,5]]: eig 3 (mult 2) and 5 (mult 1)
        A = _mat([3, 0, 0], [0, 3, 0], [0, 0, 5])
        pairs = sorted(self._get_eig_pairs(A))
        assert len(pairs) == 2
        assert abs(pairs[0][0] - 3.0) < 1e-9
        assert pairs[0][1] == 2
        assert abs(pairs[1][0] - 5.0) < 1e-9
        assert pairs[1][1] == 1

    def test_sum_of_multiplicities_equals_n(self):
        """Total multiplicity must equal matrix dimension."""
        for A in [
            _mat([1, 2], [2, 1]),          # 2x2 distinct
            _mat([2, 0], [0, 2]),           # 2x2 repeated
            _mat([1, 0, 0], [0, 3, 0], [0, 0, 5]),  # 3x3 distinct
        ]:
            n = num_rows(A)
            eigs = eigenvalues(A)
            total = sum(pair.args[1].value for pair in eigs.args)
            assert total == n, f"Total multiplicity {total} ≠ {n}"

    def test_2x2_complex_eigenvalues(self):
        # [[0,-1],[1,0]]: char poly = λ²+1, roots ±i
        A = _mat([0, -1], [1, 0])
        eigs = eigenvalues(A)
        # Should return 2 eigenvalues (complex)
        assert isinstance(eigs, IRApply)
        assert len(eigs.args) == 2
        # Sum of multiplicities = 2
        total = sum(pair.args[1].value for pair in eigs.args)
        assert total == 2

    def test_4x4_diagonal(self):
        D = _mat([1, 0, 0, 0], [0, 2, 0, 0], [0, 0, 3, 0], [0, 0, 0, 4])
        pairs = sorted(self._get_eig_pairs(D))
        assert len(pairs) == 4
        for i, (pv, pm) in enumerate(pairs):
            assert abs(pv - (i + 1)) < 1e-9
            assert pm == 1

    def test_5x5_raises(self):
        A = _mat([1, 0, 0, 0, 0],
                 [0, 1, 0, 0, 0],
                 [0, 0, 1, 0, 0],
                 [0, 0, 0, 1, 0],
                 [0, 0, 0, 0, 1])
        with pytest.raises(MatrixError):
            eigenvalues(A)


# ---------------------------------------------------------------------------
# Section 3 — Eigenvectors
# ---------------------------------------------------------------------------


class TestPhase19_Eigenvectors:
    """eigenvectors(A) → List(List(λ, m, List(v₁, v₂, …)), …)."""

    def test_2x2_distinct_eigenvalues(self):
        # [[1,2],[2,1]]: eigs -1, 3
        A = _mat([1, 2], [2, 1])
        evecs = eigenvectors(A)
        assert isinstance(evecs, IRApply)
        triples = evecs.args
        assert len(triples) == 2  # two distinct eigenvalues
        # Each triple has 3 args: (lam, mult, vec_list)
        for triple in triples:
            lam_ir, mult_ir, vec_list = triple.args
            assert isinstance(mult_ir, IRInteger)
            # Each eigenvector list should have at least one vector.
            assert len(vec_list.args) >= 1

    def test_eigenvector_satisfies_Av_equals_lam_v(self):
        # For [[3,1],[0,3]], eig λ=3 (repeated), verify A·v = 3·v
        A = _mat([3, 1], [0, 3])
        evecs = eigenvectors(A)
        for triple in evecs.args:
            lam_ir, mult_ir, vec_list = triple.args
            lam_frac = _entry_to_fraction(lam_ir)
            if lam_frac is None:
                continue
            for v_mat in vec_list.args:
                # Compute A·v and λ·v entry by entry using Fractions.
                from cas_matrix.rowreduce import _entry_to_fraction as etf
                A_f = [[etf(e) for e in row] for row in _rows_of(A)]
                v_f = [etf(_rows_of(v_mat)[r][0]) for r in range(len(A_f))]
                Av = [sum(A_f[i][j] * v_f[j] for j in range(len(v_f)))
                      for i in range(len(A_f))]
                lv = [lam_frac * v_f[i] for i in range(len(v_f))]
                for a, b in zip(Av, lv, strict=False):
                    assert abs(float(a - b)) < 1e-9

    def test_2x2_repeated_eigenvalue_gives_2_vectors(self):
        # [[2,0],[0,2]]: eig 2 (mult 2), eigenspace is all of R²
        A = _mat([2, 0], [0, 2])
        evecs = eigenvectors(A)
        assert len(evecs.args) == 1  # one eigenvalue
        triple = evecs.args[0]
        lam_ir, mult_ir, vec_list = triple.args
        assert lam_ir == _i(2)
        assert mult_ir.value == 2
        assert len(vec_list.args) == 2  # two basis vectors

    def test_3x3_diagonal_standard_basis(self):
        # diag(1,3,5): eigenvectors are standard basis vectors
        D = _mat([1, 0, 0], [0, 3, 0], [0, 0, 5])
        evecs = eigenvectors(D)
        assert len(evecs.args) == 3
        # Collect all eigenvectors — should form identity columns
        all_vecs = []
        for triple in evecs.args:
            vec_list = triple.args[2]
            for v in vec_list.args:
                col = [_entry_to_fraction(row[0]) for row in _rows_of(v)]
                all_vecs.append(col)
        # Sort by first non-zero entry
        all_vecs.sort(key=lambda v: next(x for x in v if x != 0))
        from fractions import Fraction
        expected = [
            [Fraction(1), Fraction(0), Fraction(0)],
            [Fraction(0), Fraction(1), Fraction(0)],
            [Fraction(0), Fraction(0), Fraction(1)],
        ]
        for got, exp in zip(all_vecs, expected, strict=False):
            for g, e in zip(got, exp, strict=False):
                assert g == e

    def test_eigenvectors_are_column_vectors(self):
        """Eigenvectors must be n×1 matrices."""
        A = _mat([1, 2], [2, 1])
        evecs = eigenvectors(A)
        for triple in evecs.args:
            vec_list = triple.args[2]
            for v in vec_list.args:
                assert num_rows(v) == 2
                assert num_cols(v) == 1

    def test_irrational_eigenvalue_gets_empty_vector_list(self):
        # [[0,1],[-1,0]]: char poly = λ²+1, complex eigenvalues
        A = _mat([0, 1], [-1, 0])
        evecs = eigenvectors(A)
        for triple in evecs.args:
            vec_list = triple.args[2]
            # Complex eigenvalue → empty vector list
            assert len(vec_list.args) == 0

    def test_structure_format(self):
        """Result is List(List(λ, m, List(…)), …)."""
        A = _mat([2, 0], [0, 3])
        evecs = eigenvectors(A)
        assert isinstance(evecs, IRApply)
        assert evecs.head == IRSymbol("List")
        for triple in evecs.args:
            assert isinstance(triple, IRApply)
            assert len(triple.args) == 3  # λ, m, vec_list

    def test_defective_matrix(self):
        # [[3,1],[0,3]]: eig λ=3 (mult 2 algebraic, geom 1 — defective)
        A = _mat([3, 1], [0, 3])
        evecs = eigenvectors(A)
        assert len(evecs.args) == 1  # one eigenvalue
        lam_ir, mult_ir, vec_list = evecs.args[0].args
        # Algebraic multiplicity = 2
        assert mult_ir.value == 2
        # Geometric multiplicity = 1 (only one linearly independent eigenvector)
        assert len(vec_list.args) == 1


# ---------------------------------------------------------------------------
# Section 4 — LU decomposition
# ---------------------------------------------------------------------------


class TestPhase19_LU:
    """lu_decompose(A) → List(L, U, P) with P·A = L·U."""

    def _verify_lu(self, A):
        """Assert that P·A = L·U holds entry-by-entry."""
        L, U, P = lu_decompose(A).args
        assert _is_lower_triangular(L), "L must be lower triangular"
        assert _is_upper_triangular(U), "U must be upper triangular"
        # L has 1s on diagonal
        assert all(v == (1, 1) for v in _diagonal(L)), "L diagonal must be 1"
        # Check P·A = L·U numerically
        A_f = _ir_mat_to_fracs(A)
        L_f = _ir_mat_to_fracs(L)
        U_f = _ir_mat_to_fracs(U)
        P_f = _ir_mat_to_fracs(P)
        PA = _mat_mul_fracs(P_f, A_f)
        LU_prod = _mat_mul_fracs(L_f, U_f)
        for i, row in enumerate(PA):
            for j, val in enumerate(row):
                assert abs(float(val - LU_prod[i][j])) < 1e-10, \
                    f"P·A ≠ L·U at ({i},{j})"

    def test_2x2_no_pivoting(self):
        # [[2,1],[1,3]]: no row swap needed
        A = _mat([2, 1], [1, 3])
        L, U, P = lu_decompose(A).args
        self._verify_lu(A)
        # P should be identity
        assert _entries(P) == [[(1, 1), (0, 1)], [(0, 1), (1, 1)]]

    def test_2x2_requires_pivoting(self):
        # [[0,1],[1,0]]: first column starts with 0 → must pivot
        A = _mat([0, 1], [1, 0])
        L, U, P = lu_decompose(A).args
        self._verify_lu(A)
        # P should be [[0,1],[1,0]]
        assert _entries(P) == [[(0, 1), (1, 1)], [(1, 1), (0, 1)]]

    def test_3x3_general(self):
        A = _mat([2, 1, 1], [4, 3, 3], [8, 7, 9])
        self._verify_lu(A)

    def test_3x3_identity(self):
        Id = identity_matrix(3)
        L, U, P = lu_decompose(Id).args
        self._verify_lu(Id)
        # L, U, P all equal identity
        assert _entries(L) == _entries(Id)
        assert _entries(U) == _entries(Id)
        assert _entries(P) == _entries(Id)

    def test_rational_entries(self):
        A = matrix([[_r(1, 2), _i(1)], [_i(1), _r(1, 2)]])
        self._verify_lu(A)

    def test_singular_raises(self):
        # Singular matrix: all zeros
        Z = zero_matrix(2, 2)
        with pytest.raises(MatrixError):
            lu_decompose(Z)

    def test_non_square_raises(self):
        A = _mat([1, 2, 3], [4, 5, 6])
        with pytest.raises(MatrixError):
            lu_decompose(A)

    def test_returns_list_of_three_matrices(self):
        A = _mat([1, 2], [3, 4])
        result = lu_decompose(A)
        assert isinstance(result, IRApply)
        assert len(result.args) == 3
        L, U, P = result.args
        assert num_rows(L) == num_cols(L) == 2
        assert num_rows(U) == num_cols(U) == 2
        assert num_rows(P) == num_cols(P) == 2


# ---------------------------------------------------------------------------
# Section 5 — Subspaces
# ---------------------------------------------------------------------------


class TestPhase19_Subspaces:
    """nullspace, columnspace, rowspace via RREF."""

    # --- nullspace ---

    def test_nullspace_full_rank_is_empty(self):
        Id = identity_matrix(3)
        ns = nullspace(Id)
        assert len(ns.args) == 0

    def test_nullspace_singular_2x2(self):
        # [[1,2],[2,4]]: rank 1, nullspace dimension 1
        A = _mat([1, 2], [2, 4])
        ns = nullspace(A)
        assert len(ns.args) == 1
        # The null vector v satisfies A·v = 0
        from fractions import Fraction

        from cas_matrix.rowreduce import _entry_to_fraction as etf
        A_f = [[etf(e) for e in row] for row in _rows_of(A)]
        v = ns.args[0]
        v_f = [etf(_rows_of(v)[r][0]) for r in range(2)]
        Av = [sum(A_f[i][j] * v_f[j] for j in range(2)) for i in range(2)]
        for entry in Av:
            assert entry == Fraction(0)

    def test_nullspace_2_free_variables(self):
        # [[1,2,3],[4,5,6]]: rank 2, nullity 1
        A = _mat([1, 2, 3], [4, 5, 6])
        ns = nullspace(A)
        assert len(ns.args) == 1  # one free variable (col 2)
        # RREF: [[1,0,-1],[0,1,2]], free col 2
        # v = [1, -2, 1]
        v = ns.args[0]
        # v[0]=1, v[1]=-2, v[2]=1
        from fractions import Fraction

        from cas_matrix.rowreduce import _entry_to_fraction as etf
        v_f = [etf(_rows_of(v)[r][0]) for r in range(3)]
        assert v_f[2] == Fraction(1)  # free variable = 1
        # A·v should be 0
        A_f = [[etf(e) for e in row] for row in _rows_of(A)]
        Av = [sum(A_f[i][j] * v_f[j] for j in range(3)) for i in range(2)]
        for entry in Av:
            assert entry == Fraction(0)

    def test_nullspace_zero_matrix_full_null(self):
        # Zero matrix: entire space is null space
        Z = zero_matrix(2, 3)
        ns = nullspace(Z)
        # 2×3 zero matrix: rank 0, nullity 3
        assert len(ns.args) == 3

    # --- columnspace ---

    def test_columnspace_full_rank(self):
        # 2×2 identity: columnspace is R² (both columns)
        A = identity_matrix(2)
        cs = columnspace(A)
        assert len(cs.args) == 2

    def test_columnspace_rank_deficient(self):
        # [[1,2],[2,4]]: rank 1, col space = span of col 0
        A = _mat([1, 2], [2, 4])
        cs = columnspace(A)
        assert len(cs.args) == 1
        # The single basis vector should be col 0: [1, 2]
        col = cs.args[0]
        col_entries = [_entry_to_fraction(row[0]) for row in _rows_of(col)]
        from fractions import Fraction
        assert col_entries[0] == Fraction(1)
        assert col_entries[1] == Fraction(2)

    def test_columnspace_vectors_are_column_vectors(self):
        A = _mat([1, 2, 3], [4, 5, 6])
        cs = columnspace(A)
        for col in cs.args:
            assert num_rows(col) == 2
            assert num_cols(col) == 1

    # --- rowspace ---

    def test_rowspace_full_rank(self):
        A = identity_matrix(2)
        rs = rowspace(A)
        assert len(rs.args) == 2

    def test_rowspace_rank_deficient(self):
        # [[1,2,3],[2,4,6]]: rank 1, row space = one non-zero RREF row
        A = _mat([1, 2, 3], [2, 4, 6])
        rs = rowspace(A)
        assert len(rs.args) == 1

    def test_rowspace_general_3x4(self):
        # [[1,2,3,4],[0,1,2,3],[0,0,0,1]]: rank 3
        A = _mat([1, 2, 3, 4], [0, 1, 2, 3], [0, 0, 0, 1])
        rs = rowspace(A)
        assert len(rs.args) == 3

    def test_rowspace_vectors_are_row_vectors(self):
        A = _mat([1, 2, 3], [4, 5, 6])
        rs = rowspace(A)
        for row_vec in rs.args:
            assert num_rows(row_vec) == 1
            assert num_cols(row_vec) == 3


# ---------------------------------------------------------------------------
# Section 6 — Norm
# ---------------------------------------------------------------------------


class TestPhase19_Norm:
    """norm(v) — Euclidean; norm(A, 'frobenius') — Frobenius."""

    def test_345_vector_gives_5(self):
        # [3, 4]: ‖v‖ = √(9+16) = √25 = 5
        v = matrix([[_i(3)], [_i(4)]])
        result = norm(v)
        assert isinstance(result, IRInteger)
        assert result.value == 5

    def test_euclidean_non_perfect_square(self):
        # [1, 1]: ‖v‖ = √2
        v = matrix([[_i(1)], [_i(1)]])
        result = norm(v)
        # Should be Sqrt(2) = IRApply(SQRT, (IRInteger(2),))
        from symbolic_ir import SQRT
        assert isinstance(result, IRApply)
        assert result.head == SQRT

    def test_frobenius_perfect_square(self):
        # [[1,1],[1,1]]: ‖A‖_F = √(1+1+1+1) = √4 = 2
        A = _mat([1, 1], [1, 1])
        result = norm(A, "frobenius")
        assert isinstance(result, IRInteger)
        assert result.value == 2

    def test_frobenius_3x3_identity(self):
        # ‖I_3‖_F = √3
        Id = identity_matrix(3)
        result = norm(Id, "frobenius")
        from symbolic_ir import SQRT
        assert isinstance(result, IRApply)
        assert result.head == SQRT

    def test_row_vector_norm(self):
        # Row vector [3, 4]: ‖v‖ = 5
        v = matrix([[_i(3), _i(4)]])
        result = norm(v)
        assert isinstance(result, IRInteger)
        assert result.value == 5

    def test_rational_entries(self):
        # [3/5, 4/5]: ‖v‖ = √(9/25 + 16/25) = √(25/25) = 1
        v = matrix([[_r(3, 5)], [_r(4, 5)]])
        result = norm(v)
        assert isinstance(result, IRInteger)
        assert result.value == 1

    def test_norm_matrix_without_frobenius_raises(self):
        # Passing a non-vector without 'frobenius' flag should raise.
        A = _mat([1, 2], [3, 4])
        with pytest.raises(MatrixError):
            norm(A)

    def test_unknown_norm_kind_raises(self):
        A = _mat([1, 2], [3, 4])
        with pytest.raises(MatrixError):
            norm(A, "spectral")


# ---------------------------------------------------------------------------
# Section 7 — Fallthrough and error cases
# ---------------------------------------------------------------------------


class TestPhase19_Fallthrough:
    """Operations that should raise MatrixError for invalid inputs."""

    def test_eigenvalues_non_square_raises(self):
        A = _mat([1, 2, 3], [4, 5, 6])
        with pytest.raises(MatrixError):
            eigenvalues(A)

    def test_eigenvectors_5x5_raises(self):
        I5 = identity_matrix(5)
        with pytest.raises(MatrixError):
            eigenvectors(I5)

    def test_lu_singular_raises(self):
        # All-zero row → singular at first pivot column
        A = _mat([0, 0], [0, 0])
        with pytest.raises(MatrixError):
            lu_decompose(A)

    def test_symbolic_entry_eigenvalues_raises(self):
        A = matrix([[IRSymbol("a"), _i(1)], [_i(0), _i(2)]])
        with pytest.raises(MatrixError):
            eigenvalues(A)


# ---------------------------------------------------------------------------
# Section 8 — Regressions (Phase 0.2.0 ops still work)
# ---------------------------------------------------------------------------


class TestPhase19_Regressions:
    """Ensure Phase 0.2.0 operations are unaffected."""

    def test_rank_still_works(self):
        from cas_matrix import rank
        M = _mat([1, 2, 3], [4, 5, 6], [7, 8, 9])
        assert rank(M) == _i(2)

    def test_row_reduce_still_works(self):
        from cas_matrix import row_reduce
        M = _mat([1, 2], [3, 4])
        R = row_reduce(M)
        e = _entries(R)
        assert e[0][0] == (1, 1) and e[1][1] == (1, 1)

    def test_determinant_still_works(self):
        from cas_matrix import determinant
        M = _mat([1, 2], [3, 4])
        d = determinant(M)
        # det = 1*4 - 2*3 = -2; result is unsimplified IR
        # Just check it's non-trivial IR
        assert d is not None

    def test_inverse_still_works(self):
        from cas_matrix import inverse
        I2 = identity_matrix(2)
        inv = inverse(I2)
        # inv of identity is identity
        assert num_rows(inv) == 2

    def test_trace_still_works(self):
        from cas_matrix import trace
        A = _mat([1, 2], [3, 4])
        t = trace(A)
        # trace = 1 + 4 = Add(1, 4) — unsimplified
        assert t is not None
