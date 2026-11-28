"""One-step SAT check for HWEB09's flat-arena append/link transition."""
from __future__ import annotations

import os
import subprocess
from pathlib import Path

WIDTH = 3
NO_INDEX = (1 << WIDTH) - 1
OLD_COUNT = 2
NEW_INDEX = OLD_COUNT
NEW_COUNT = OLD_COUNT + 1


class CNF:
    def __init__(self) -> None:
        self.variable_count = 0
        self.clauses: list[tuple[int, ...]] = []

    def variable(self) -> int:
        self.variable_count += 1
        return self.variable_count

    def bits(self) -> tuple[int, ...]:
        return tuple(self.variable() for _ in range(WIDTH))

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


def _not_equal_value(formula: CNF, bits: tuple[int, ...], value: int) -> None:
    # This clause is false only when `bits` encode the excluded value.
    formula.clause(
        *( -variable if value & (1 << bit) else variable
           for bit, variable in enumerate(bits) )
    )


def _index_field(
    formula: CNF, fields: dict[str, tuple[int, ...]], name: str, value: int
) -> tuple[int, ...]:
    bits = formula.bits()
    fields[name] = bits
    _fix(formula, bits, value)
    # Every published index is either no_index or an existing/appended node.
    # For this model node indices are 0..new_count-1 and no_index is 7.
    for invalid in range(NEW_COUNT, NO_INDEX):
        _not_equal_value(formula, bits, invalid)
    return bits


def _model(
    case: str,
    *,
    parent_override: int | None = None,
    require_parent_precondition: bool = True,
    require_out_of_range_link: bool = False,
) -> CNF:
    """Encode the C++ push_back then root/child/sibling link transition.

    The three-bit universe is deliberately bounded: old node count is 2,
    append position is 2, count afterwards is 3, and no_index is 7. Thus the
    one-step count/index arithmetic cannot wrap in this abstraction.
    """
    if case not in {"root", "first_child", "previous_sibling"}:
        raise ValueError(f"unknown append case: {case}")

    formula = CNF()
    fields: dict[str, tuple[int, ...]] = {}
    old_count = formula.bits()
    append_index = formula.bits()
    count_after = formula.bits()
    _fix(formula, old_count, OLD_COUNT)
    _fix(formula, count_after, NEW_COUNT)
    _equal(formula, append_index, old_count)
    fields["append_index"] = append_index
    for invalid in range(NEW_COUNT, NO_INDEX):
        _not_equal_value(formula, append_index, invalid)
    fields["count_after"] = count_after

    parent = (
        NO_INDEX if case == "root" else 0
    ) if parent_override is None else parent_override
    if require_parent_precondition:
        # The parser obtains this from stack_.back(): it is an older node or
        # the root sentinel. This is a caller invariant, not an append guard.
        for invalid in range(OLD_COUNT, NO_INDEX):
            parent_bits = fields.get("parent")
            if parent_bits is None:
                parent_bits = formula.bits()
                fields["parent"] = parent_bits
                _fix(formula, parent_bits, parent)
            _not_equal_value(formula, parent_bits, invalid)
        if "parent" not in fields:
            fields["parent"] = formula.bits()
            _fix(formula, fields["parent"], parent)
    else:
        fields["parent"] = formula.bits()
        _fix(formula, fields["parent"], parent)

    node_parent = _index_field(formula, fields, "node_parent", parent)
    first_child_value = (
        NEW_INDEX
        if case == "first_child" and parent == NEW_INDEX
        else NO_INDEX
    )
    node_first = _index_field(
        formula, fields, "node_first_child", first_child_value
    )
    _index_field(formula, fields, "node_next_sibling", NO_INDEX)
    _equal(formula, node_parent, fields["parent"])

    if case == "root":
        # Two preexisting roots: root_last=1. append_node links its sibling to
        # the appended root index and updates root_last/root_count.
        _index_field(formula, fields, "old_root_last", 1)
        link_source = _index_field(formula, fields, "link_source", 1)
        link_target = _index_field(formula, fields, "link_target", NEW_INDEX)
        _equal(formula, link_target, append_index)
        _equal(formula, link_source, fields["old_root_last"])
        _index_field(formula, fields, "root_last_after", NEW_INDEX)
    elif case == "first_child":
        # Existing parent 0 has no child; first_child becomes append index.
        _index_field(formula, fields, "old_parent_first_child", NO_INDEX)
        link_source = _index_field(formula, fields, "link_source", parent)
        link_target = _index_field(formula, fields, "link_target", NEW_INDEX)
        _equal(formula, link_target, append_index)
        _equal(formula, link_source, fields["parent"])
    else:
        # Parent 0 already has child 1; its previous last child's sibling link
        # is updated to the newly appended index.
        _index_field(formula, fields, "old_parent_first_child", 1)
        _index_field(formula, fields, "old_last_child", 1)
        link_source = _index_field(formula, fields, "link_source", 1)
        link_target = _index_field(formula, fields, "link_target", NEW_INDEX)
        _equal(formula, link_target, append_index)
        _equal(formula, link_source, fields["old_last_child"])
        _index_field(formula, fields, "parent_last_child_after", NEW_INDEX)

    if require_out_of_range_link:
        # Ask whether the append link can land exactly one past the new vector.
        _fix(formula, fields["link_target"], NEW_COUNT)

    return formula


def _solve(formula: CNF) -> str:
    bridge = Path(__file__).with_name("go_sat")
    go = os.environ.get("HWEB_GO", "go")
    result = subprocess.run(
        [go, "run", "."],
        cwd=bridge,
        input=formula.dimacs(),
        text=True,
        capture_output=True,
        check=True,
        timeout=30,
    )
    return result.stdout.strip().upper()


def _expect(label: str, formula: CNF, expected: str) -> None:
    actual = _solve(formula)
    if actual != expected:
        raise RuntimeError(f"{label}: solver returned {actual}, expected {expected}")


def run() -> None:
    for case in ("root", "first_child", "previous_sibling"):
        _expect(f"legal {case} append/link", _model(case), "SAT")
        _expect(
            f"{case} link one-past-count miter",
            _model(case, require_out_of_range_link=True),
            "UNSAT",
        )

    _expect(
        "valid-stack parent cannot self-link new child",
        _model(
            "first_child",
            parent_override=NEW_INDEX,
            require_parent_precondition=True,
        ),
        "UNSAT",
    )
    _expect(
        "missing parent-from-stack precondition self-cycle mutant",
        _model(
            "first_child",
            parent_override=NEW_INDEX,
            require_parent_precondition=False,
        ),
        "SAT",
    )

    print("Go CDCL HWEB09 one-step arena append model: 3-bit indexes")
    print("Assumptions: old_count=2, append_index=2, new_count=3, no_index=7; no wrap")
    print("Legal root, first-child, and previous-sibling append/link transitions: SAT")
    print("Each append link at one-past-count: UNSAT")
    print(
        "Weakening valid stack-parent precondition permits a new-node "
        "self-child: SAT"
    )
    print(
        "Forward traversal edges are first-child/next-sibling; "
        "parent_index is a backlink."
    )
    print(
        "This local transition model does not prove arbitrary-size vector "
        "behavior or the whole parser."
    )


if __name__ == "__main__":
    run()
