"""Symbolic SAT checks for the HWEB09 C++ append/link transition."""
from __future__ import annotations

import os
import subprocess
from pathlib import Path


class CNF:
    """DIMACS builder with bit-level unsigned arithmetic and comparisons."""

    def __init__(self) -> None:
        self.variable_count = 0
        self.clauses: list[tuple[int, ...]] = []

    def variable(self) -> int:
        self.variable_count += 1
        return self.variable_count

    def bits(self, width: int) -> tuple[int, ...]:
        return tuple(self.variable() for _ in range(width))

    def clause(self, *literals: int) -> None:
        self.clauses.append(tuple(literals))

    def dimacs(self) -> str:
        lines = [f"p cnf {self.variable_count} {len(self.clauses)}"]
        lines.extend(" ".join(map(str, clause)) + " 0" for clause in self.clauses)
        return "\n".join(lines) + "\n"


def _fix(formula: CNF, bits: tuple[int, ...], value: int) -> None:
    for bit, variable in enumerate(bits):
        formula.clause(variable if value & (1 << bit) else -variable)


def _equal(formula: CNF, left: tuple[int, ...], right: tuple[int, ...]) -> None:
    for a, b in zip(left, right, strict=True):
        formula.clause(-a, b)
        formula.clause(a, -b)


def _xor(formula: CNF, left: int, right: int) -> int:
    result = formula.variable()
    formula.clause(left, right, -result)
    formula.clause(-left, -right, -result)
    formula.clause(left, -right, result)
    formula.clause(-left, right, result)
    return result


def _and(formula: CNF, left: int, right: int) -> int:
    result = formula.variable()
    formula.clause(-left, -right, result)
    formula.clause(left, -result)
    formula.clause(right, -result)
    return result


def _or(formula: CNF, left: int, right: int) -> int:
    result = formula.variable()
    formula.clause(left, right, -result)
    formula.clause(-left, result)
    formula.clause(-right, result)
    return result


def _add_one(formula: CNF, value: tuple[int, ...]) -> tuple[tuple[int, ...], int]:
    """Return value+1 and its carry-out, without imposing no-wrap."""
    result: list[int] = []
    carry = value[0]
    bit_zero = formula.variable()
    formula.clause(value[0], bit_zero)
    formula.clause(-value[0], -bit_zero)
    result.append(bit_zero)
    for bit in range(1, len(value)):
        next_bit = _xor(formula, value[bit], carry)
        next_carry = _and(formula, value[bit], carry)
        result.append(next_bit)
        carry = next_carry
    return tuple(result), carry


def _unsigned_less(
    formula: CNF, left: tuple[int, ...], right: tuple[int, ...]
) -> int:
    """Encode unsigned `left < right`, with bit tuples in little-endian order."""
    if len(left) != len(right):
        raise ValueError("unsigned comparison widths differ")
    less = formula.variable()
    formula.clause(-less)
    for bit in range(len(left)):
        a, b = left[bit], right[bit]
        at_bit = _and(formula, -a, b)
        equal_bit = _variable_equal(formula, a, b)
        lower_less = _and(formula, equal_bit, less)
        less = _or(formula, at_bit, lower_less)
    return less


def _variable_equal(formula: CNF, left: int, right: int) -> int:
    equal = formula.variable()
    formula.clause(-left, right, -equal)
    formula.clause(left, -right, -equal)
    formula.clause(-left, -right, equal)
    formula.clause(left, right, equal)
    return equal


def _model(
    width: int,
    case: str,
    *,
    link_mutant: bool = False,
    omit_source_bound: bool = False,
    self_parent: bool = False,
    omit_no_wrap: bool = False,
    query: str = "legal",
) -> CNF:
    if width not in {32, 64}:
        raise ValueError(f"unsupported size_t width: {width}")
    if case not in {"root", "first_child", "previous_sibling"}:
        raise ValueError(f"unknown append case: {case}")

    formula = CNF()
    old_count = formula.bits(width)
    append_index = formula.bits(width)
    post_count = formula.bits(width)
    _equal(formula, append_index, old_count)
    incremented, carry_out = _add_one(formula, old_count)
    _equal(formula, post_count, incremented)
    if not omit_no_wrap:
        formula.clause(-carry_out)

    # Successful vector append returns the old vector size as the new index;
    # count after append is old count + 1. This is the exact index arithmetic.
    link_target = formula.bits(width)
    _equal(formula, link_target, post_count if link_mutant else append_index)

    source: tuple[int, ...] | None = None
    parent: tuple[int, ...] | None = None
    if case in {"first_child", "previous_sibling"}:
        source = formula.bits(width)
        if not omit_source_bound:
            source_below_old_count = _unsigned_less(formula, source, old_count)
            formula.clause(source_below_old_count)
    if case == "first_child":
        parent = source
    elif case == "previous_sibling":
        parent = formula.bits(width)
        parent_below_old_count = _unsigned_less(formula, parent, old_count)
        formula.clause(parent_below_old_count)

    node_parent = formula.bits(width)
    if case == "root":
        _fix(formula, node_parent, (1 << width) - 1)
    elif case == "first_child":
        if source is None:
            raise ValueError("first-child transition needs a parent source")
        _equal(formula, node_parent, source)
    else:
        if parent is None:
            raise ValueError("previous-sibling transition needs a parent")
        _equal(formula, node_parent, parent)

    # HtmlNode defaults its outgoing traversal links to no_index. A first-child
    # write aliases the new node only when a caller violates the stack-parent
    # invariant and supplies the just-appended index as its own parent.
    new_first_child = formula.bits(width)
    new_next_sibling = formula.bits(width)
    no_index = (1 << width) - 1
    _fix(formula, new_next_sibling, no_index)
    if self_parent:
        if case != "first_child" or source is None:
            raise ValueError("self-parent control requires first_child case")
        _equal(formula, source, link_target)
        _equal(formula, new_first_child, link_target)
    else:
        _fix(formula, new_first_child, no_index)

    if query == "target_ge_post_count":
        target_below_count = _unsigned_less(formula, link_target, post_count)
        formula.clause(-target_below_count)
    elif query == "source_ge_target":
        if source is None:
            raise ValueError("source comparison requires child/sibling case")
        source_below_target = _unsigned_less(formula, source, link_target)
        formula.clause(-source_below_target)
    elif query == "old_count_is_max":
        _fix(formula, old_count, no_index)
    elif query != "legal":
        raise ValueError(f"unknown model query: {query}")

    return formula


def _solve(formula: CNF) -> str:
    repository_root = Path(__file__).resolve().parents[5]
    bridge = repository_root / "code/packages/python/historical-web/verification/go_sat"
    go = os.environ.get("HWEB_GO", "go")
    result = subprocess.run(
        [go, "run", "."],
        cwd=bridge,
        input=formula.dimacs(),
        text=True,
        capture_output=True,
        check=True,
        timeout=60,
    )
    return result.stdout.strip().upper()


def _expect(label: str, formula: CNF, expected: str) -> None:
    actual = _solve(formula)
    if actual != expected:
        raise RuntimeError(f"{label}: solver returned {actual}, expected {expected}")


def run() -> None:
    for width in (32, 64):
        for case in ("root", "first_child", "previous_sibling"):
            _expect(f"{width}-bit legal {case} append", _model(width, case), "SAT")
            _expect(
                f"{width}-bit correct append cannot link at/after post count",
                _model(width, case, query="target_ge_post_count"),
                "UNSAT",
            )
        for case in ("first_child", "previous_sibling"):
            _expect(
                f"{width}-bit old source cannot be at/after appended target",
                _model(width, case, query="source_ge_target"),
                "UNSAT",
            )
        _expect(
            f"{width}-bit one-past-link mutant",
            _model(
                width,
                "first_child",
                link_mutant=True,
                query="target_ge_post_count",
            ),
            "SAT",
        )
        _expect(
            f"{width}-bit stack parent cannot equal append target",
            _model(width, "first_child", self_parent=True),
            "UNSAT",
        )
        _expect(
            f"{width}-bit missing stack-parent bound creates self-child",
            _model(
                width,
                "first_child",
                omit_source_bound=True,
                self_parent=True,
            ),
            "SAT",
        )
        _expect(
            f"{width}-bit successful append excludes wrapped max count",
            _model(width, "root", query="old_count_is_max"),
            "UNSAT",
        )
        _expect(
            f"{width}-bit no-wrap mutant admits max-count wrap",
            _model(
                width,
                "root",
                omit_no_wrap=True,
                query="old_count_is_max",
            ),
            "SAT",
        )

    print("Go CDCL symbolic HWEB09 arena transition: size_t widths 32 and 64")
    print("Positive witnesses: root, first-child, and previous-sibling append SAT")
    print(
        "Derived invariant: target=old_count < post_count=old_count+1 "
        "(target >= post_count is UNSAT)"
    )
    print(
        "Derived invariant: old source < old_count cannot reach or exceed "
        "the appended target"
    )
    print(
        "Negative controls: one-past link, missing parent bound, and wrapped "
        "count are SAT"
    )
    print(
        "This models the one-step append transition, not std::vector "
        "allocation or the whole parser."
    )


if __name__ == "__main__":
    run()
