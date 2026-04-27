from hash_set import (
    HashSet,
    add,
    contains,
    difference,
    discard,
    equals,
    from_list,
    intersection,
    is_disjoint,
    is_subset,
    is_superset,
    remove,
    symmetric_difference,
    union,
)


def sorted_values(set_: HashSet[int]) -> list[int]:
    return sorted(set_.to_list())


def test_basic_membership_and_duplicates() -> None:
    set_ = HashSet.from_list([1, 1, 2, 2, 3])

    assert set_.contains(1)
    assert not set_.contains(4)
    assert set_.size() == 3
    assert len(set_) == 3
    assert 2 in set_
    assert sorted(set_) == [1, 2, 3]
    assert not set_.is_empty()
    assert repr(set_).startswith("HashSet(")


def test_add_remove_are_persistent() -> None:
    set_ = HashSet.from_list([1, 2])
    added = set_.add(3)
    removed = added.remove(2)

    assert sorted_values(set_) == [1, 2]
    assert sorted_values(added) == [1, 2, 3]
    assert sorted_values(removed) == [1, 3]
    assert sorted_values(removed.discard(99)) == [1, 3]


def test_set_algebra() -> None:
    a = HashSet.from_list([1, 2, 3, 4, 5])
    b = HashSet.from_list([3, 4, 5, 6, 7])

    assert sorted_values(a.union(b)) == [1, 2, 3, 4, 5, 6, 7]
    assert sorted_values(a.intersection(b)) == [3, 4, 5]
    assert sorted_values(a.difference(b)) == [1, 2]
    assert sorted_values(a.symmetric_difference(b)) == [1, 2, 6, 7]


def test_relation_checks() -> None:
    a = HashSet.from_list([1, 2, 3])
    b = HashSet.from_list([1, 2, 3, 4, 5])
    c = HashSet.from_list([10, 20])

    assert a.is_subset(b)
    assert b.is_superset(a)
    assert a.is_disjoint(c)
    assert not a.is_disjoint(b)
    assert a.equals(HashSet.from_list([3, 2, 1]))
    assert not a.equals(b)


def test_options_are_preserved() -> None:
    set_ = HashSet.with_options(4, "open_addressing", "murmur3").add(1).add(5)

    assert set_.capacity >= 4
    assert set_.strategy == "open_addressing"
    assert set_.hash_algorithm == "murmur3"
    assert sorted_values(set_) == [1, 5]

    seeded = HashSet.from_list_with_options([1, 2, 2, 3], 2, "open", "djb2")
    assert seeded.capacity >= 3
    assert seeded.strategy == "open"
    assert seeded.hash_algorithm == "djb2"
    assert sorted_values(seeded) == [1, 2, 3]


def test_free_function_wrappers() -> None:
    set_ = from_list([10, 20])
    assert contains(set_, 10)

    set_ = add(set_, 30)
    assert contains(set_, 30)

    set_ = remove(set_, 20)
    assert not contains(set_, 20)

    set_ = discard(set_, 99)
    assert contains(set_, 10)

    other = from_list([30, 40])
    unioned = union(set_, other)
    assert sorted_values(unioned) == [10, 30, 40]

    assert sorted_values(intersection(set_, other)) == [30]
    assert sorted_values(difference(set_, other)) == [10]
    assert sorted_values(symmetric_difference(set_, other)) == [10, 40]
    assert is_subset(set_, unioned)
    assert is_superset(unioned, set_)
    assert not is_subset(unioned, set_)
    assert not is_superset(set_, unioned)
    assert is_disjoint(set_, from_list([999]))
    assert not is_disjoint(set_, other)
    assert equals(set_, set_)
    assert not equals(set_, other)


def test_empty_constructor_and_negative_capacity() -> None:
    empty = HashSet.new()
    assert empty.is_empty()
    assert empty.to_list() == []

    options = HashSet.with_options(-10, "strategy", "hash")
    assert options.capacity == 0
