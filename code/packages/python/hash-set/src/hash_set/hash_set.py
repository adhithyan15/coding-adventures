"""Persistent hash set utilities."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Generic, Hashable, Iterable, TypeVar

T = TypeVar("T", bound=Hashable)


@dataclass(frozen=True, slots=True)
class HashSet(Generic[T]):
    """A persistent hash set.

    Python's built-in ``frozenset`` supplies the storage; the option fields
    mirror the Rust package's hash-map-backed constructor surface.
    """

    _elements: frozenset[T] = frozenset()
    capacity: int = 0
    strategy: str = "default"
    hash_algorithm: str = "default"

    @classmethod
    def new(cls) -> HashSet[T]:
        return cls()

    @classmethod
    def with_options(
        cls,
        capacity: int,
        strategy: str,
        hash_fn: str,
    ) -> HashSet[T]:
        return cls(frozenset(), max(0, capacity), strategy, hash_fn)

    @classmethod
    def from_list(cls, elements: Iterable[T]) -> HashSet[T]:
        return cls(frozenset(elements))

    @classmethod
    def from_list_with_options(
        cls,
        elements: Iterable[T],
        capacity: int,
        strategy: str,
        hash_fn: str,
    ) -> HashSet[T]:
        items = frozenset(elements)
        return cls(items, max(capacity, len(items)), strategy, hash_fn)

    def add(self, element: T) -> HashSet[T]:
        return self._replace(self._elements | frozenset([element]))

    def remove(self, element: T) -> HashSet[T]:
        return self._replace(self._elements - frozenset([element]))

    def discard(self, element: T) -> HashSet[T]:
        return self.remove(element)

    def contains(self, element: T) -> bool:
        return element in self._elements

    def size(self) -> int:
        return len(self._elements)

    def is_empty(self) -> bool:
        return not self._elements

    def to_list(self) -> list[T]:
        return list(self._elements)

    def union(self, other: HashSet[T]) -> HashSet[T]:
        return self._replace(self._elements | other._elements, min_capacity=self.size() + other.size())

    def intersection(self, other: HashSet[T]) -> HashSet[T]:
        return self._replace(self._elements & other._elements, min_capacity=min(self.size(), other.size()))

    def difference(self, other: HashSet[T]) -> HashSet[T]:
        return self._replace(self._elements - other._elements)

    def symmetric_difference(self, other: HashSet[T]) -> HashSet[T]:
        return self._replace(self._elements ^ other._elements, min_capacity=self.size() + other.size())

    def is_subset(self, other: HashSet[T]) -> bool:
        return self._elements <= other._elements

    def is_superset(self, other: HashSet[T]) -> bool:
        return self._elements >= other._elements

    def is_disjoint(self, other: HashSet[T]) -> bool:
        return self._elements.isdisjoint(other._elements)

    def equals(self, other: HashSet[T]) -> bool:
        return self._elements == other._elements

    def __len__(self) -> int:
        return self.size()

    def __contains__(self, element: object) -> bool:
        return element in self._elements

    def __iter__(self):
        return iter(self._elements)

    def __repr__(self) -> str:
        return f"HashSet({self.to_list()!r})"

    def _replace(self, elements: frozenset[T], min_capacity: int | None = None) -> HashSet[T]:
        capacity = self.capacity
        if min_capacity is not None:
            capacity = max(capacity, min_capacity, len(elements))
        else:
            capacity = max(capacity, len(elements))
        return HashSet(elements, capacity, self.strategy, self.hash_algorithm)


def from_list(elements: Iterable[T]) -> HashSet[T]:
    return HashSet.from_list(elements)


def add(set_: HashSet[T], element: T) -> HashSet[T]:
    return set_.add(element)


def remove(set_: HashSet[T], element: T) -> HashSet[T]:
    return set_.remove(element)


def discard(set_: HashSet[T], element: T) -> HashSet[T]:
    return set_.discard(element)


def contains(set_: HashSet[T], element: T) -> bool:
    return set_.contains(element)


def union(set_: HashSet[T], other: HashSet[T]) -> HashSet[T]:
    return set_.union(other)


def intersection(set_: HashSet[T], other: HashSet[T]) -> HashSet[T]:
    return set_.intersection(other)


def difference(set_: HashSet[T], other: HashSet[T]) -> HashSet[T]:
    return set_.difference(other)


def symmetric_difference(set_: HashSet[T], other: HashSet[T]) -> HashSet[T]:
    return set_.symmetric_difference(other)


def is_subset(set_: HashSet[T], other: HashSet[T]) -> bool:
    return set_.is_subset(other)


def is_superset(set_: HashSet[T], other: HashSet[T]) -> bool:
    return set_.is_superset(other)


def is_disjoint(set_: HashSet[T], other: HashSet[T]) -> bool:
    return set_.is_disjoint(other)


def equals(set_: HashSet[T], other: HashSet[T]) -> bool:
    return set_.equals(other)
