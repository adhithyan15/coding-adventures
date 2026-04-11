from __future__ import annotations

from collections.abc import Iterable, Iterator
from typing import Callable, Generic, TypeVar

T = TypeVar("T")
Comparator = Callable[[T, T], int]


def _default_compare(left: object, right: object) -> int:
    if left < right:  # type: ignore[operator]
        return -1
    if left > right:  # type: ignore[operator]
        return 1
    return 0


def _lower_bound(items: list[T], value: T, compare: Comparator[T]) -> int:
    low = 0
    high = len(items)
    while low < high:
        mid = (low + high) // 2
        if compare(items[mid], value) < 0:
            low = mid + 1
        else:
            high = mid
    return low


def _upper_bound(items: list[T], value: T, compare: Comparator[T]) -> int:
    low = 0
    high = len(items)
    while low < high:
        mid = (low + high) // 2
        if compare(items[mid], value) <= 0:
            low = mid + 1
        else:
            high = mid
    return low


def _merge_unique(left: list[T], right: list[T], compare: Comparator[T]) -> list[T]:
    result: list[T] = []
    li = 0
    ri = 0
    while li < len(left) and ri < len(right):
        order = compare(left[li], right[ri])
        if order < 0:
            result.append(left[li])
            li += 1
        elif order > 0:
            result.append(right[ri])
            ri += 1
        else:
            result.append(left[li])
            li += 1
            ri += 1
    result.extend(left[li:])
    result.extend(right[ri:])
    return result


def _intersection_sorted(left: list[T], right: list[T], compare: Comparator[T]) -> list[T]:
    result: list[T] = []
    li = 0
    ri = 0
    while li < len(left) and ri < len(right):
        order = compare(left[li], right[ri])
        if order < 0:
            li += 1
        elif order > 0:
            ri += 1
        else:
            result.append(left[li])
            li += 1
            ri += 1
    return result


def _difference_sorted(left: list[T], right: list[T], compare: Comparator[T]) -> list[T]:
    result: list[T] = []
    li = 0
    ri = 0
    while li < len(left) and ri < len(right):
        order = compare(left[li], right[ri])
        if order < 0:
            result.append(left[li])
            li += 1
        elif order > 0:
            ri += 1
        else:
            li += 1
            ri += 1
    result.extend(left[li:])
    return result


def _symmetric_difference_sorted(
    left: list[T],
    right: list[T],
    compare: Comparator[T],
) -> list[T]:
    result: list[T] = []
    li = 0
    ri = 0
    while li < len(left) and ri < len(right):
        order = compare(left[li], right[ri])
        if order < 0:
            result.append(left[li])
            li += 1
        elif order > 0:
            result.append(right[ri])
            ri += 1
        else:
            li += 1
            ri += 1
    result.extend(left[li:])
    result.extend(right[ri:])
    return result


def _is_subset_sorted(left: list[T], right: list[T], compare: Comparator[T]) -> bool:
    li = 0
    ri = 0
    while li < len(left) and ri < len(right):
        order = compare(left[li], right[ri])
        if order < 0:
            return False
        if order > 0:
            ri += 1
        else:
            li += 1
            ri += 1
    return li == len(left)


def _is_disjoint_sorted(left: list[T], right: list[T], compare: Comparator[T]) -> bool:
    li = 0
    ri = 0
    while li < len(left) and ri < len(right):
        order = compare(left[li], right[ri])
        if order < 0:
            li += 1
        elif order > 0:
            ri += 1
        else:
            return False
    return True


class TreeSet(Generic[T], Iterable[T]):
    def __init__(
        self,
        values: Iterable[T] = (),
        compare: Comparator[T] | None = None,
    ) -> None:
        self._values: list[T] = []
        self._compare: Comparator[T] = compare or _default_compare  # type: ignore[assignment]
        for value in values:
            self.add(value)

    @classmethod
    def from_values(
        cls,
        values: Iterable[T],
        compare: Comparator[T] | None = None,
    ) -> TreeSet[T]:
        return cls(values, compare)

    def add(self, value: T) -> TreeSet[T]:
        index = _lower_bound(self._values, value, self._compare)
        if index < len(self._values) and self._compare(self._values[index], value) == 0:
            return self
        self._values.insert(index, value)
        return self

    def delete(self, value: T) -> bool:
        index = _lower_bound(self._values, value, self._compare)
        if index >= len(self._values) or self._compare(self._values[index], value) != 0:
            return False
        del self._values[index]
        return True

    def discard(self, value: T) -> bool:
        return self.delete(value)

    def has(self, value: T) -> bool:
        index = _lower_bound(self._values, value, self._compare)
        return index < len(self._values) and self._compare(self._values[index], value) == 0

    def contains(self, value: T) -> bool:
        return self.has(value)

    def __contains__(self, value: object) -> bool:
        try:
            return self.has(value)  # type: ignore[arg-type]
        except TypeError:
            return False

    def size(self) -> int:
        return len(self._values)

    @property
    def length(self) -> int:
        return self.size()

    def __len__(self) -> int:
        return self.size()

    def is_empty(self) -> bool:
        return not self._values

    isEmpty = is_empty

    def min(self) -> T | None:
        return self._values[0] if self._values else None

    def max(self) -> T | None:
        return self._values[-1] if self._values else None

    def first(self) -> T | None:
        return self.min()

    def last(self) -> T | None:
        return self.max()

    def predecessor(self, value: T) -> T | None:
        index = _lower_bound(self._values, value, self._compare)
        return self._values[index - 1] if index > 0 else None

    def successor(self, value: T) -> T | None:
        index = _upper_bound(self._values, value, self._compare)
        return self._values[index] if index < len(self._values) else None

    def rank(self, value: T) -> int:
        return _lower_bound(self._values, value, self._compare)

    def by_rank(self, rank: int) -> T | None:
        return self._values[rank] if 0 <= rank < len(self._values) else None

    def kth_smallest(self, k: int) -> T | None:
        if k <= 0:
            return None
        return self.by_rank(k - 1)

    def to_list(self) -> list[T]:
        return self._values.copy()

    def to_sorted_array(self) -> list[T]:
        return self.to_list()

    def to_array(self) -> list[T]:
        return self.to_list()

    def range(self, minimum: T, maximum: T, inclusive: bool = True) -> list[T]:
        if self._compare(minimum, maximum) > 0:
            return []
        start = (
            _lower_bound(self._values, minimum, self._compare)
            if inclusive
            else _upper_bound(self._values, minimum, self._compare)
        )
        end = (
            _upper_bound(self._values, maximum, self._compare)
            if inclusive
            else _lower_bound(self._values, maximum, self._compare)
        )
        return self._values[start:end]

    def union(self, other: TreeSet[T]) -> TreeSet[T]:
        return TreeSet(_merge_unique(self._values, other._values, self._compare), self._compare)

    def intersection(self, other: TreeSet[T]) -> TreeSet[T]:
        return TreeSet(
            _intersection_sorted(self._values, other._values, self._compare),
            self._compare,
        )

    def difference(self, other: TreeSet[T]) -> TreeSet[T]:
        return TreeSet(
            _difference_sorted(self._values, other._values, self._compare),
            self._compare,
        )

    def symmetric_difference(self, other: TreeSet[T]) -> TreeSet[T]:
        return TreeSet(
            _symmetric_difference_sorted(self._values, other._values, self._compare),
            self._compare,
        )

    def is_subset(self, other: TreeSet[T]) -> bool:
        return _is_subset_sorted(self._values, other._values, self._compare)

    def is_superset(self, other: TreeSet[T]) -> bool:
        return other.is_subset(self)

    def is_disjoint(self, other: TreeSet[T]) -> bool:
        return _is_disjoint_sorted(self._values, other._values, self._compare)

    def equals(self, other: TreeSet[T]) -> bool:
        if len(self._values) != len(other._values):
            return False
        for left, right in zip(self._values, other._values):
            if self._compare(left, right) != 0:
                return False
        return True

    def __iter__(self) -> Iterator[T]:
        return iter(self._values)

    def __eq__(self, other: object) -> bool:
        return isinstance(other, TreeSet) and self.equals(other)

    def __repr__(self) -> str:
        return f"TreeSet({self._values!r})"

    def __str__(self) -> str:
        return self.__repr__()


def from_values(
    values: Iterable[T],
    compare: Comparator[T] | None = None,
) -> TreeSet[T]:
    return TreeSet(values, compare)
