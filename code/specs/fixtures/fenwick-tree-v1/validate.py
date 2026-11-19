"""Validate DT06 fixture expectations with a flat-array reference oracle.

This script verifies fixture integrity; language adapters must execute these
cases against their own implementation to establish conformance.
"""
import itertools
import json
from pathlib import Path

FIXTURE = Path(__file__).with_name("cases.json")
LIMIT = 2**53 - 1


def check_array(values):
    """Check direct prefix/range/point definitions without Fenwick logic."""
    n = len(values)
    prefixes = [sum(values[:i]) for i in range(n + 1)]
    for i in range(n + 1):
        assert prefixes[i] == sum(values[:i])
    for left in range(1, n + 1):
        for right in range(left, n + 1):
            assert prefixes[right] - prefixes[left - 1] == sum(values[left - 1:right])
        assert values[left - 1] == sum(values[left - 1:left])


def validate_case(case):
    values = list(case["values"])
    if "constructor_error" in case:
        assert case["constructor_error"] in {"negative_size", "out_of_domain"}
        if case["constructor_error"] == "negative_size":
            assert case["size"] < 0
        else:
            assert sum(abs(v) for v in values) > LIMIT
        assert not case["ops"]
        return
    assert sum(abs(v) for v in values) <= LIMIT
    for action in case["ops"]:
        op = action["op"]
        error = action.get("error")
        n = len(values)
        if op == "prefix_sum":
            i = action["i"]
            if error:
                assert error == "index_out_of_range" and not 0 <= i <= n
            else:
                assert 0 <= i <= n and action["result"] == sum(values[:i])
        elif op == "range_sum":
            left, right = action["l"], action["r"]
            if error == "invalid_range":
                assert left > right
            elif error:
                assert error == "index_out_of_range" and left <= right
                assert not (1 <= left <= right <= n)
            else:
                assert 1 <= left <= right <= n
                assert action["result"] == sum(values[left - 1:right])
        elif op == "point_query":
            i = action["i"]
            if error:
                assert error == "index_out_of_range" and not 1 <= i <= n
            else:
                assert 1 <= i <= n and action["result"] == values[i - 1]
        elif op == "update":
            i, delta = action["i"], action["delta"]
            if error == "index_out_of_range":
                assert not 1 <= i <= n and action["after"] == values
            elif error == "out_of_domain":
                assert 1 <= i <= n
                proposed = values.copy()
                proposed[i - 1] += delta
                assert abs(delta) > LIMIT or sum(abs(v) for v in proposed) > LIMIT
                assert action["after"] == values
            else:
                assert error is None and 1 <= i <= n and abs(delta) <= LIMIT
                proposed = values.copy()
                proposed[i - 1] += delta
                assert sum(abs(v) for v in proposed) <= LIMIT
                assert action["result"] == proposed
                values = proposed
        elif op == "find_kth":
            k = action["k"]
            if not values:
                assert error == "empty"
            elif k <= 0:
                assert error == "nonpositive_target"
            else:
                assert all(v >= 0 for v in values)
                prefixes = list(itertools.accumulate(values))
                expected = next((i + 1 for i, value in enumerate(prefixes) if value >= k), None)
                if expected is None:
                    assert error == "exceeds_total"
                else:
                    assert error is None and action["result"] == expected
        else:
            raise AssertionError(f"unknown fixture operation: {op}")


def main():
    cases = json.loads(FIXTURE.read_text())
    assert cases["version"] == 1 and isinstance(cases["cases"], list)
    names = [case["name"] for case in cases["cases"]]
    assert len(names) == len(set(names)), "duplicate fixture case name"
    for case in cases["cases"]:
        validate_case(case)

    # Bounded independent oracle domain: all signed arrays through length 6,
    # then each point update by -1/0/1, plus order-statistic expectations on
    # all nonnegative arrays. This checks the local contract oracle itself;
    # consumers must additionally run their implementation against this corpus.
    checked = 0
    for n in range(7):
        for values in itertools.product((-1, 0, 1), repeat=n):
            values = list(values)
            check_array(values)
            for index in range(n):
                for delta in (-1, 0, 1):
                    changed = values.copy()
                    changed[index] += delta
                    check_array(changed)
            checked += 1
    kth_cases = 0
    for n in range(7):
        for values in itertools.product((0, 1, 2), repeat=n):
            prefixes = list(itertools.accumulate(values))
            total = prefixes[-1] if prefixes else 0
            for k in range(1, total + 2):
                result = next((i + 1 for i, value in enumerate(prefixes) if value >= k), None)
                assert (result is not None) == (k <= total)
                kth_cases += 1
    print(f"fixture valid; oracle covered {checked} signed arrays and {kth_cases} kth targets")


if __name__ == "__main__":
    main()
