"""Independent bounded-heap histories, boundary checks, and fuzz regressions."""

from __future__ import annotations

import itertools
import operator
import random
import sys
from collections import Counter
from collections.abc import Callable, Iterator
from dataclasses import dataclass

import pytest
from hypothesis import settings
from hypothesis import strategies as st
from hypothesis.stateful import RuleBasedStateMachine, invariant, rule

from heap import BoundedHeap, CheckedPriorityQueue, HeapFullError, MaxHeap, MinHeap
from heap.functions import heap_sort, nlargest, nsmallest

IntComparator = Callable[[int, int], bool]


def assert_heap_state(
    heap: BoundedHeap[int],
    model: list[int],
    less: IntComparator,
    reverse: bool = False,
) -> None:
    values = heap.to_array()
    assert len(heap) == len(model)
    assert len(heap) <= heap.capacity
    assert sorted(values) == sorted(model)
    if model:
        expected = max(model) if reverse else min(model)
        assert heap.peek() == expected
    else:
        with pytest.raises(IndexError):
            heap.peek()

    for index in range(1, len(values)):
        parent = (index - 1) // 2
        assert not less(values[index], values[parent])


def apply_operation(
    heap: BoundedHeap[int],
    model: list[int],
    operation: tuple[str, int | None],
    reverse: bool = False,
) -> None:
    name, value = operation
    before = heap.to_array()
    if name == "push":
        assert value is not None
        if len(model) == heap.capacity:
            with pytest.raises(HeapFullError):
                heap.push(value)
            assert heap.to_array() == before
        else:
            heap.push(value)
            model.append(value)
    elif name == "pop":
        if not model:
            with pytest.raises(IndexError):
                heap.pop()
            assert heap.to_array() == before
        else:
            expected = max(model) if reverse else min(model)
            assert heap.pop() == expected
            model.remove(expected)
    elif name == "peek":
        if not model:
            with pytest.raises(IndexError):
                heap.peek()
        else:
            expected = max(model) if reverse else min(model)
            assert heap.peek() == expected
    elif name == "clear":
        heap.clear()
        model.clear()
    else:
        raise AssertionError(f"unknown operation {name}")


@pytest.mark.parametrize("less", [operator.lt, operator.gt], ids=["min", "max"])
def test_every_five_step_history_over_six_operations(less: IntComparator) -> None:
    alphabet = [
        ("push", -1),
        ("push", 0),
        ("push", 1),
        ("pop", None),
        ("peek", None),
        ("clear", None),
    ]
    # 9,331 histories through length five; replay each prefix and inspect the
    # full heap/model relation after every operation.
    for length in range(6):
        for history in itertools.product(alphabet, repeat=length):
            heap = BoundedHeap(3, less)
            model: list[int] = []
            reverse = less is operator.gt
            assert_heap_state(heap, model, less, reverse)
            for operation in history:
                apply_operation(heap, model, operation, reverse)
                assert_heap_state(heap, model, less, reverse)


@pytest.mark.parametrize("less", [operator.lt, operator.gt], ids=["min", "max"])
@pytest.mark.parametrize("seed", [0xD04C, 723_901])
def test_seeded_mixed_histories_include_extreme_values(
    less: IntComparator, seed: int
) -> None:
    rng = random.Random(seed)
    heap = BoundedHeap(17, less)
    model: list[int] = []
    special = [-(2**63), -1, 0, 1, 2**63 - 1]
    for _ in range(1200):
        operation = rng.randrange(5)
        if operation <= 1:
            value = (
                rng.choice(special)
                if rng.randrange(5) == 0
                else rng.randint(-1000, 1000)
            )
            apply_operation(heap, model, ("push", value), less is operator.gt)
        elif operation == 2:
            apply_operation(heap, model, ("pop", None), less is operator.gt)
        elif operation == 3:
            apply_operation(heap, model, ("peek", None), less is operator.gt)
        else:
            apply_operation(heap, model, ("clear", None), less is operator.gt)
        assert_heap_state(heap, model, less, less is operator.gt)


@pytest.mark.parametrize("less", [operator.lt, operator.gt], ids=["min", "max"])
def test_bounded_input_byte_programs_are_replayable(less: IntComparator) -> None:
    seed = 0xD504C
    rng = random.Random(seed)
    for case_index in range(256):
        payload = rng.randbytes(64)
        heap = BoundedHeap(8, less)
        model: list[int] = []
        try:
            for byte in payload:
                opcode = byte & 0b11
                value = (byte >> 2) - 32
                if opcode == 0:
                    apply_operation(heap, model, ("push", value), less is operator.gt)
                elif opcode == 1:
                    apply_operation(heap, model, ("pop", None), less is operator.gt)
                elif opcode == 2:
                    apply_operation(heap, model, ("peek", None), less is operator.gt)
                else:
                    apply_operation(heap, model, ("clear", None), less is operator.gt)
                assert_heap_state(heap, model, less, less is operator.gt)
        except Exception as error:
            raise AssertionError(
                f"seed={seed} case={case_index} bytes={payload.hex()}"
            ) from error


def test_capacity_and_comparator_types_are_checked_without_consuming_input() -> None:
    class MustNotIterate:
        def __iter__(self) -> Iterator[int]:
            raise AssertionError("input consumed before validation")

    invalid_capacities = [True, False, 1.0, "3", None]
    for capacity in invalid_capacities:
        with pytest.raises(TypeError):
            BoundedHeap(capacity, operator.lt)  # type: ignore[arg-type]
        with pytest.raises(TypeError):
            BoundedHeap.from_iterable(MustNotIterate(), capacity, operator.lt)  # type: ignore[arg-type]
    for capacity in [-1, sys.maxsize + 1]:
        with pytest.raises(ValueError):
            BoundedHeap(capacity, operator.lt)
        with pytest.raises(ValueError):
            BoundedHeap.from_iterable(MustNotIterate(), capacity, operator.lt)
    with pytest.raises(TypeError, match="callable"):
        BoundedHeap(2, None)  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="capacity"):
        BoundedHeap.from_iterable(MustNotIterate(), -1, None)  # type: ignore[arg-type]
    maximum = BoundedHeap(sys.maxsize, operator.lt)
    assert maximum.capacity == sys.maxsize

    class IntSubclass(int):
        pass

    with pytest.raises(TypeError, match="built-in int"):
        BoundedHeap(IntSubclass(2), operator.lt)


def test_zero_capacity_full_rejection_clear_and_reuse() -> None:
    empty = BoundedHeap[int](0, operator.lt)
    assert empty.capacity == 0
    assert not empty
    with pytest.raises(HeapFullError):
        empty.push(1)
    empty.clear()
    assert len(empty) == 0

    queue = BoundedHeap(2, operator.lt)
    queue.push(3)
    queue.push(2)
    before = queue.to_array()
    with pytest.raises(HeapFullError):
        queue.push(1)
    assert queue.to_array() == before
    queue.clear()
    assert queue.capacity == 2
    queue.push(9)
    assert queue.pop() == 9


def test_from_iterable_consumes_once_caps_oversize_and_owns_storage() -> None:
    class OnePass:
        def __init__(self, values: list[int]) -> None:
            self.values = values
            self.iterations = 0

        def __iter__(self) -> Iterator[int]:
            self.iterations += 1
            if self.iterations > 1:
                raise AssertionError("iterable consumed twice")
            yield from self.values

    source = OnePass([8, 1, 5, 1, -4])
    queue = BoundedHeap.from_iterable(source, 5, operator.lt)
    assert source.iterations == 1
    assert queue.to_array() is not source.values
    source.values[0] = 999
    assert [queue.pop() for _ in range(len(queue))] == [-4, 1, 1, 5, 8]

    requests = []

    def infinite() -> Iterator[int]:
        for value in itertools.count():
            requests.append(value)
            yield value

    with pytest.raises(ValueError, match="capacity"):
        BoundedHeap.from_iterable(infinite(), 3, operator.lt)
    assert requests == [0, 1, 2, 3]


def test_array_copy_is_shallow_and_independent() -> None:
    item = Job(2, "compile")
    queue: BoundedHeap[Job] = BoundedHeap(
        2, lambda left, right: left.priority < right.priority
    )
    queue.push(item)
    copied = queue.to_array()
    assert copied is not queue._data
    assert copied[0] is item
    copied.clear()
    assert len(queue) == 1


@pytest.mark.parametrize("less,reverse", [(operator.lt, False), (operator.gt, True)])
@pytest.mark.parametrize("values", [[], [42], [7, -4, 7, 0, -(2**63), 2**63 - 1]])
def test_bounded_min_max_floyd_build_matches_sorted_model(
    less: IntComparator, reverse: bool, values: list[int]
) -> None:
    source = values[:]
    queue = BoundedHeap.from_iterable(values, len(values), less)
    assert values == source
    assert_heap_state(queue, source, less, reverse)
    expected = sorted(source, reverse=reverse)
    assert [queue.pop() for _ in range(len(queue))] == expected


@pytest.mark.parametrize("fail_at", [1, 2, 3])
def test_comparator_exceptions_propagate_at_sift_depths(fail_at: int) -> None:
    calls = 0
    armed = False

    def flaky(left: int, right: int) -> bool:
        nonlocal calls
        if armed:
            calls += 1
        if armed and calls == fail_at:
            raise RuntimeError("comparator failed")
        return left < right

    queue = BoundedHeap(8, flaky)
    for value in [1, 2, 3, 4, 5, 6, 7]:
        queue.push(value)
    armed = True
    calls = 0
    with pytest.raises(RuntimeError, match="comparator failed"):
        queue.push(-1)
    # The documented comparator-error policy is propagation without rollback;
    # do not assert the queue remains usable after a callback failure.


@pytest.mark.parametrize("fail_at", [1, 3, 5])
def test_comparator_exceptions_during_pop_propagate(fail_at: int) -> None:
    calls = 0
    armed = False

    def flaky(left: int, right: int) -> bool:
        nonlocal calls
        if armed:
            calls += 1
        if armed and calls == fail_at:
            raise RuntimeError("pop comparator failed")
        return left < right

    queue = BoundedHeap.from_iterable(list(range(15, 0, -1)), 16, flaky)
    armed = True
    with pytest.raises(RuntimeError, match="pop comparator failed"):
        queue.pop()


@pytest.mark.parametrize("fail_at", [1, 2, 3])
def test_comparator_exceptions_during_construction_propagate(fail_at: int) -> None:
    calls = 0

    def flaky(left: int, right: int) -> bool:
        nonlocal calls
        calls += 1
        if calls == fail_at:
            raise RuntimeError("construction comparator failed")
        return left < right

    with pytest.raises(RuntimeError, match="construction comparator failed"):
        BoundedHeap.from_iterable([7, 6, 5, 4, 3, 2, 1], 7, flaky)


def test_non_boolean_comparator_result_is_rejected() -> None:
    def wrong_result(_left: int, _right: int) -> bool:
        return 1  # type: ignore[return-value]

    queue: BoundedHeap[int] = BoundedHeap(2, wrong_result)
    queue.push(2)
    with pytest.raises(TypeError, match="return bool"):
        queue.push(1)


@dataclass(frozen=True)
class Job:
    priority: int
    name: str


def test_generic_comparator_supports_non_integer_values_and_ties() -> None:
    jobs = [Job(4, "compile"), Job(1, "lint"), Job(4, "test"), Job(-2, "format")]
    queue = BoundedHeap.from_iterable(
        jobs, 4, lambda left, right: left.priority < right.priority
    )
    actual = [queue.pop() for _ in range(len(queue))]
    assert [job.priority for job in actual] == [-2, 1, 4, 4]
    assert Counter(actual) == Counter(jobs)


def test_public_protocol_and_read_only_capacity() -> None:
    queue: CheckedPriorityQueue[int] = BoundedHeap(2, operator.lt)
    queue.push(5)
    assert queue.peek() == 5
    assert len(queue) == 1
    with pytest.raises(AttributeError):
        queue.capacity = 4  # type: ignore[misc]


def test_repr_reports_empty_and_populated_state() -> None:
    queue = BoundedHeap(3, operator.lt)
    assert repr(queue) == "BoundedHeap(size=0, capacity=3, root=empty)"
    queue.push(6)
    assert repr(queue) == "BoundedHeap(size=1, capacity=3, root=6)"


class _ModelMachine(RuleBasedStateMachine):
    reverse = False
    stateful_steps: dict[str, list[int]] = {"min": [], "max": []}

    def comparator(self, left: int, right: int) -> bool:
        return left > right if self.reverse else left < right

    def __init__(self) -> None:
        super().__init__()
        self.queue = BoundedHeap(12, self.comparator)
        self.model: list[int] = []
        self.steps = 0

    def teardown(self) -> None:
        label = "max" if self.reverse else "min"
        self.stateful_steps[label].append(self.steps)

    @rule(value=st.integers(min_value=-(2**63), max_value=2**63 - 1))
    def insert(self, value: int) -> None:
        self.steps += 1
        apply_operation(self.queue, self.model, ("push", value), self.reverse)

    @rule()
    def remove(self) -> None:
        self.steps += 1
        apply_operation(self.queue, self.model, ("pop", None), self.reverse)

    @rule()
    def inspect(self) -> None:
        self.steps += 1
        apply_operation(self.queue, self.model, ("peek", None), self.reverse)

    @rule()
    def reset(self) -> None:
        self.steps += 1
        apply_operation(self.queue, self.model, ("clear", None), self.reverse)

    @invariant()
    def state_matches_independent_model(self) -> None:
        assert_heap_state(self.queue, self.model, self.comparator, self.reverse)


class _MinHeapMachine(_ModelMachine):
    reverse = False


class _MaxHeapMachine(_ModelMachine):
    reverse = True


_STATEFUL_SETTINGS = settings(
    max_examples=200,
    stateful_step_count=100,
    derandomize=True,
    deadline=None,
)
TestMinHeapStateMachine = _MinHeapMachine.TestCase
TestMinHeapStateMachine.settings = _STATEFUL_SETTINGS
TestMaxHeapStateMachine = _MaxHeapMachine.TestCase
TestMaxHeapStateMachine.settings = _STATEFUL_SETTINGS


def test_hypothesis_history_step_counts_are_bounded_and_reported() -> None:
    for label in ("min", "max"):
        counts = _ModelMachine.stateful_steps[label]
        assert len(counts) >= 200
        assert all(0 <= count <= 100 for count in counts)
        print(
            f"Hypothesis {label}: completed histories={len(counts)}, "
            f"total steps={sum(counts)}, min steps={min(counts)}, "
            f"max steps={max(counts)}"
        )


@pytest.mark.parametrize("seed", [17, 91, 4_044])
def test_legacy_min_max_heap_public_apis_keep_model_behavior(seed: int) -> None:
    rng = random.Random(seed)
    for heap_type, less in ((MinHeap, operator.lt), (MaxHeap, operator.gt)):
        heap = heap_type()
        model: list[int] = []
        for _ in range(240):
            if not model or (len(model) < 30 and rng.random() < 0.62):
                value = rng.randint(-(2**31), 2**31 - 1)
                heap.push(value)
                model.append(value)
            else:
                expected = min(model) if less is operator.lt else max(model)
                assert heap.pop() == expected
                model.remove(expected)
            assert len(heap) == len(model)
            assert sorted(heap.to_array()) == sorted(model)
            if model:
                expected = min(model) if less is operator.lt else max(model)
                assert heap.peek() == expected

        source = [rng.randint(-10_000, 10_000) for _ in range(rng.randrange(80))]
        bulk = heap_type.from_iterable(source)
        sorted_expected = (
            sorted(source) if less is operator.lt else sorted(source, reverse=True)
        )
        assert [bulk.pop() for _ in range(len(bulk))] == sorted_expected


@pytest.mark.parametrize("seed", [12, 44, 2026])
def test_legacy_sort_and_top_k_helpers_against_sorted_oracles(seed: int) -> None:
    rng = random.Random(seed)
    for _ in range(80):
        values = [rng.randint(-100, 100) for _ in range(rng.randrange(40))]
        count = rng.randrange(45)
        assert heap_sort(values) == sorted(values)
        assert nlargest(values, count) == sorted(values, reverse=True)[:count]
        assert nsmallest(values, count) == sorted(values)[:count]


def test_test_only_negative_controls_trip_the_independent_oracle() -> None:
    # These adapters deliberately model two faulty outcomes; they do not patch
    # production code and make no source-mutation claim.
    class Observation:
        def __init__(self, capacity: int, values: list[int]) -> None:
            self.capacity = capacity
            self.values = values

        def __len__(self) -> int:
            return len(self.values)

        def to_array(self) -> list[int]:
            return list(self.values)

        def peek(self) -> int:
            return self.values[0]

    less = operator.lt
    with pytest.raises(AssertionError):
        assert_heap_state(Observation(1, [1, 2]), [1, 2], less)  # type: ignore[arg-type]
    with pytest.raises(AssertionError):
        assert_heap_state(Observation(2, [2, 1]), [1, 2], less)  # type: ignore[arg-type]

    class WrongPop:
        capacity = 2

        def to_array(self) -> list[int]:
            return [1, 2]

        def pop(self) -> int:
            return 2

    with pytest.raises(AssertionError):
        apply_operation(WrongPop(), [1, 2], ("pop", None))  # type: ignore[arg-type]
