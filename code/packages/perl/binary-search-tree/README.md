# Perl binary search tree

A dependency-free persistent binary search tree with duplicate suppression,
deletion, lookup, predecessor/successor queries, rank and k-th order statistics,
balanced construction from sorted arrays, and metadata validation.

```perl
my $tree = CodingAdventures::BinarySearchTree->empty
    ->insert(5)->insert(1)->insert(8)->insert(3);
my $updated = $tree->delete(5);
print $tree->contains(5);       # true
print $updated->contains(5);    # false
print $updated->kth_smallest(2); # 3
```

`insert` and `delete` return new trees and preserve the original. Supply a
three-way comparison code reference to `empty` or `from_sorted_array` for custom
value ordering. The default comparator handles numeric and string scalars.

## Development

```bash
bash BUILD
```
