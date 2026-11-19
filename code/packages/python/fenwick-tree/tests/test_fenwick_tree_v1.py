"""DT06 v1 direct consumer, exhaustive oracle, histories, and mutation fuzzing."""

from __future__ import annotations

import itertools
import json
import os
from pathlib import Path
from typing import Any

import pytest
from coding_adventures_test_bootstrap.fuzz import fuzz_bytes
from coding_adventures_test_bootstrap.pbt import for_all, gen_int, gen_list

from fenwick_tree import FenwickError, FenwickTree, ValueOutOfDomainError
from fenwick_tree.fenwick_tree import EmptyTreeError, IndexOutOfRangeError

LIMIT = (1 << 53) - 1
FIXTURE = Path(
    os.environ.get(
        "FENWICK_FIXTURE",
        Path(__file__).resolve().parents[4]
        / "specs/fixtures/fenwick-tree-v1/cases.json",
    )
)


def _error_type(name: str) -> type[Exception]:
    return {
        "out_of_domain": ValueOutOfDomainError,
        "index_out_of_range": IndexOutOfRangeError,
        "empty": EmptyTreeError,
        "negative_size": FenwickError,
        "invalid_range": FenwickError,
        "nonpositive_target": FenwickError,
        "exceeds_total": FenwickError,
    }[name]


def _audit(tree: FenwickTree, values: list[int]) -> None:
    assert len(tree) == len(values)
    assert [tree.point_query(i) for i in range(1, len(values) + 1)] == values
    for end in range(len(values) + 1):
        assert tree.prefix_sum(end) == sum(values[:end])
    for left in range(1, len(values) + 1):
        for right in range(left, len(values) + 1):
            assert tree.range_sum(left, right) == sum(values[left - 1 : right])


def test_shared_fixture_uses_public_api_and_preserves_errors() -> None:
    fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
    assert fixture["version"] == 1
    for case in fixture["cases"]:
        name = case["name"]
        values = case["values"]
        if "constructor_error" in case:
            error = case["constructor_error"]
            with pytest.raises(_error_type(error)) as caught:
                if error == "negative_size":
                    FenwickTree(case["size"], portable_integer_domain=True)
                else:
                    FenwickTree.from_list(values, portable_integer_domain=True)
            assert type(caught.value) is _error_type(error), name
            continue
        model = list(values)
        tree = FenwickTree.from_list(model, portable_integer_domain=True)
        for operation in case["ops"]:
            op = operation["op"]
            if "error" in operation:
                before = model.copy()
                error = _error_type(operation["error"])
                with pytest.raises(error) as caught:
                    _run_operation(tree, model, operation)
                assert type(caught.value) is error, (
                    f"{name}: {op} mapped to {type(caught.value).__name__}, "
                    f"expected {error.__name__}"
                )
                assert model == before, f"{name}: rejected {op} mutated model"
                if op == "update":
                    assert operation.get("after", before) == before, name
                    _audit(tree, before)
                continue
            result = _run_operation(tree, model, operation)
            assert result == operation["result"], f"{name}: {op}"
            if op == "update":
                assert model == operation["result"], name
                assert model == operation.get("after", model), name
                _audit(tree, model)


def _run_operation(
    tree: FenwickTree, model: list[int], op: dict[str, Any]
) -> int | list[int]:
    match op["op"]:
        case "prefix_sum":
            return tree.prefix_sum(op["i"])
        case "range_sum":
            return tree.range_sum(op["l"], op["r"])
        case "point_query":
            return tree.point_query(op["i"])
        case "update":
            tree.update(op["i"], op["delta"])
            model[op["i"] - 1] += op["delta"]
            return model.copy()
        case "find_kth":
            return tree.find_kth(op["k"])
        case other:
            raise AssertionError(f"unknown fixture operation: {other}")


def test_exhaustive_signed_arrays_updates_and_all_queries() -> None:
    checked_arrays = 0
    checked_updates = 0
    for length in range(7):
        for values_tuple in itertools.product((-1, 0, 1), repeat=length):
            values = list(values_tuple)
            tree = FenwickTree.from_list(values, portable_integer_domain=True)
            _audit(tree, values)
            checked_arrays += 1
            for index in range(length):
                for delta in (-1, 0, 1):
                    changed = values.copy()
                    changed[index] += delta
                    if sum(map(abs, changed)) > LIMIT:
                        continue
                    updated = FenwickTree.from_list(
                        values, portable_integer_domain=True
                    )
                    updated.update(index + 1, delta)
                    _audit(updated, changed)
                    checked_updates += 1
    assert checked_arrays == 1093
    assert checked_updates == 18_045


def test_exhaustive_nonnegative_kth_targets() -> None:
    checked_arrays = 0
    checked_targets = 0
    for length in range(7):
        for values_tuple in itertools.product((0, 1, 2), repeat=length):
            values = list(values_tuple)
            tree = FenwickTree.from_list(values, portable_integer_domain=True)
            total = sum(values)
            for target in range(1, total + 2):
                if length == 0:
                    with pytest.raises(EmptyTreeError):
                        tree.find_kth(target)
                elif target == total + 1:
                    with pytest.raises(FenwickError):
                        tree.find_kth(target)
                else:
                    expected = next(
                        i
                        for i in range(1, length + 1)
                        if sum(values[:i]) >= target
                    )
                    assert tree.find_kth(target) == expected
                checked_targets += 1
            checked_arrays += 1
    assert checked_arrays == 1093
    assert checked_targets == 7108


@pytest.mark.parametrize("length", [7, 8, 9])
def test_power_boundary_lengths(length: int) -> None:
    values = [(-1) ** i * (i + 1) for i in range(length)]
    tree = FenwickTree.from_list(values, portable_integer_domain=True)
    _audit(tree, values)
    tree.update(length, -values[-1])
    values[-1] = 0
    _audit(tree, values)


def test_domain_boundaries_precedence_atomicity_and_instance_profile() -> None:
    strict = FenwickTree.from_list([LIMIT], portable_integer_domain=True)
    legacy = FenwickTree.from_list([LIMIT])
    with pytest.raises(ValueOutOfDomainError):
        strict.update(1, 1)
    assert strict.point_query(1) == LIMIT
    strict.update(1, -1)
    assert strict.point_query(1) == LIMIT - 1
    strict.update(1, 1)
    assert strict.point_query(1) == LIMIT
    legacy.update(1, 1)
    assert legacy.point_query(1) == LIMIT + 1
    for bad in (True, 1.0, LIMIT + 1):
        with pytest.raises(ValueOutOfDomainError):
            FenwickTree.from_list([bad], portable_integer_domain=True)
    with pytest.raises(IndexOutOfRangeError):
        strict.update(0, LIMIT + 1)
    with pytest.raises(ValueOutOfDomainError):
        strict.update(1, LIMIT + 1)
    with pytest.raises(TypeError):
        FenwickTree(1, portable_integer_domain=1)  # type: ignore[arg-type]
    with pytest.raises(ValueOutOfDomainError):
        FenwickTree.from_list([0], portable_integer_domain=True).update(1, True)


def test_default_from_list_preserves_subclass_constructor() -> None:
    class CustomFenwickTree(FenwickTree):
        def __init__(self, n: int) -> None:
            super().__init__(n)

    tree = CustomFenwickTree.from_list([2, 3])
    assert isinstance(tree, CustomFenwickTree)
    assert tree.prefix_sum(2) == 5


def _history_property(encoded: list[int]) -> bool:
    values = [0] * 9
    tree = FenwickTree.from_list(values, portable_integer_domain=True)
    for offset in range(0, len(encoded) - 1, 2):
        index = encoded[offset] % len(values)
        delta = encoded[offset + 1] % 9 - 4
        values[index] += delta
        tree.update(index + 1, delta)
        _audit(tree, values)
    return True


def test_seeded_histories_use_first_party_generators_and_shrinkers() -> None:
    seed = 0xD706
    result = for_all(
        _history_property,
        gen_list(gen_int(-10_000, 10_000), min_len=0, max_len=80),
        count=500,
        seed=seed,
    )
    assert result.passed, (
        f"seed={seed} cases={result.cases_run} counterexample={result.counterexample} "
        f"shrunken={result.shrunken} error={result.error} "
        f"shrink_steps={result.shrink_steps}"
    )
    assert result.cases_run == 500


def test_seeded_byte_operation_fuzz_has_no_unexpected_crashes() -> None:
    def target(data: bytes) -> None:
        values = [0] * 10
        tree = FenwickTree.from_list(values, portable_integer_domain=True)
        for offset in range(0, len(data) - 1, 2):
            index = data[offset] % len(values)
            delta = data[offset + 1] - 128
            values[index] += delta
            tree.update(index + 1, delta)
            _audit(tree, values)

    seed = 0xD706B17
    result = fuzz_bytes(
        target,
        seed=seed,
        iterations=5000,
        max_input_bytes=128,
        initial_corpus=[b"\x00\x80", b"\x09\xff", bytes(range(32))],
    )
    assert result.iterations_run == 5000
    assert result.crashes == [], (
        f"seed={seed} iterations={result.iterations_run} "
        f"crashes={result.crashes[:3]} unique={result.unique_crashes}"
    )
