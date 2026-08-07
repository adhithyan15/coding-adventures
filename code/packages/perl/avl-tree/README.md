# Perl AVL tree

A dependency-free persistent AVL tree with duplicate suppression, balanced
insertion and deletion, lookup, predecessor/successor queries, rank and k-th
order statistics, custom comparison, and metadata validation.

```perl
my $tree = CodingAdventures::AVLTree->from_values([5, 1, 8, 3]);
my $updated = $tree->delete(5);
print $tree->contains(5);        # true
print $updated->contains(5);     # false
print $updated->is_valid_avl;    # true
```

`insert` and `delete` return new trees and preserve the original. Supply a
three-way comparison code reference to `empty` or `from_values` for custom
value ordering. The default comparator handles numeric and string scalars.

## Development

```bash
bash BUILD
```
