"""Independent bounded-heap histories, boundary checks, and fuzz regressions."""

from __future__ import annotations

import itertools
import operator
import random
import sys
from collections import Counter
from collections.abc import Callable, Iterator
from dataclasses import dataclass
from math import ceil

import pytest

from heap import BoundedHeap, CheckedPriorityQueue, HeapFullError, MaxHeap, MinHeap
from heap.functions import heap_sort, nlargest, nsmallest

IntComparator = Callable[[int, int], bool]
Operation = tuple[str, int | None]
QueueFactory = Callable[[int, IntComparator], BoundedHeap[int]]


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


def replay_history(
    history: list[Operation],
    *,
    reverse: bool,
    capacity: int,
    queue_factory: QueueFactory = BoundedHeap,
) -> None:
    """Replay one operation sequence against a fresh independent model."""

    less = operator.gt if reverse else operator.lt
    queue = queue_factory(capacity, less)
    model: list[int] = []
    for operation in history:
        apply_operation(queue, model, operation, reverse)
        assert_heap_state(queue, model, less, reverse)


def history_fails(
    history: list[Operation],
    *,
    reverse: bool,
    capacity: int,
    queue_factory: QueueFactory,
) -> bool:
    """Whether a fresh replay still triggers a model assertion."""

    try:
        replay_history(
            history,
            reverse=reverse,
            capacity=capacity,
            queue_factory=queue_factory,
        )
    except Exception:
        return True
    return False


def reduce_failing_history(
    history: list[Operation],
    still_fails: Callable[[list[Operation]], bool],
    *,
    max_checks: int = 1200,
) -> tuple[list[Operation], int]:
    """Boundedly remove chunks and simplify actions while preserving a failure."""

    reduced = list(history)
    checks = 0

    def check(candidate: list[Operation]) -> bool:
        nonlocal checks
        if checks >= max_checks:
            return False
        checks += 1
        return still_fails(candidate)

    granularity = 2
    while reduced and checks < max_checks:
        chunk_size = ceil(len(reduced) / granularity)
        changed = False
        for start in range(0, len(reduced), chunk_size):
            candidate = reduced[:start] + reduced[start + chunk_size :]
            if check(candidate):
                reduced = candidate
                granularity = max(2, granularity - 1)
                changed = True
                break
        if not changed:
            if granularity >= len(reduced):
                break
            granularity = min(len(reduced), granularity * 2)

    # Simplify each remaining action. The order makes short, ordinary examples
    # preferable to extreme-value examples while preserving reproducibility.
    for index, (name, value) in enumerate(list(reduced)):
        if checks >= max_checks:
            break
        if name == "push":
            assert value is not None
            candidates = [0, 1, -1, value // 2]
            for simpler in dict.fromkeys(candidates):
                if (abs(simpler), simpler != 0) >= (abs(value), value != 0):
                    continue
                candidate = list(reduced)
                candidate[index] = ("push", simpler)
                if check(candidate):
                    reduced = candidate
                    value = simpler
        else:
            for simpler_name in ("peek", "pop", "clear"):
                if simpler_name == name:
                    continue
                candidate = list(reduced)
                candidate[index] = (simpler_name, None)
                if check(candidate):
                    reduced = candidate
                    name = simpler_name

    return reduced, checks


@pytest.mark.parametrize("reverse", [False, True], ids=["min", "max"])
def test_seeded_stateful_histories_match_independent_model(reverse: bool) -> None:
    seed = 0xD04C + int(reverse)
    rng = random.Random(seed)
    special_values = [-(2**63), -1, 0, 1, 2**63 - 1]
    total_steps = 0

    for history_index in range(200):
        capacity = rng.choice([0, 1, 3, 12])
        operation_count = rng.randrange(101)
        history: list[Operation] = []
        for _ in range(operation_count):
            choice = rng.randrange(100)
            if choice < 45:
                value = (
                    rng.choice(special_values)
                    if rng.randrange(4) == 0
                    else rng.randint(-(2**31), 2**31 - 1)
                )
                history.append(("push", value))
            elif choice < 70:
                history.append(("pop", None))
            elif choice < 90:
                history.append(("peek", None))
            else:
                history.append(("clear", None))

        total_steps += len(history)
        try:
            replay_history(history, reverse=reverse, capacity=capacity)
        except Exception as error:

            def predicate(
                candidate: list[Operation],
                *,
                replay_reverse: bool = reverse,
                replay_capacity: int = capacity,
            ) -> bool:
                return history_fails(
                    candidate,
                    reverse=replay_reverse,
                    capacity=replay_capacity,
                    queue_factory=BoundedHeap,
                )

            minimized, checks = reduce_failing_history(history, predicate)
            raise AssertionError(
                f"seed={seed} order={'max' if reverse else 'min'} "
                f"history={history_index} capacity={capacity} "
                f"reduction_checks={checks} original={history!r} "
                f"minimized={minimized!r}"
            ) from error

    assert 0 <= total_steps <= 20_000
    print(
        f"stdlib model {('max' if reverse else 'min')}: seed={seed}, "
        f"histories=200, operations={total_steps}, max_history=100"
    )


def test_history_reducer_minimizes_a_deliberately_wrong_pop_adapter() -> None:
    class WrongPopHeap(BoundedHeap[int]):
        def pop(self) -> int:
            return super().pop() + 1

    original: list[Operation] = [
        ("push", 41),
        ("peek", None),
        ("push", 7),
        ("pop", None),
        ("clear", None),
    ]

    def fails(candidate: list[Operation]) -> bool:
        return history_fails(
            candidate,
            reverse=False,
            capacity=8,
            queue_factory=WrongPopHeap,
        )

    assert fails(original)
    reduced, checks = reduce_failing_history(original, fails)
    assert checks > 0
    assert reduced == [("push", 0), ("pop", None)]
    assert fails(reduced)


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
