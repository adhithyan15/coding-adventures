# Perl binary tree

A dependency-free generic binary tree with node lookup, child lookup, four
traversals, shape predicates, height and size queries, sparse array conversion,
and ASCII rendering.

```perl
my $tree = CodingAdventures::BinaryTree->from_level_order(
    [1, 2, 3, 4, undef, 5, undef],
);
print $tree->height;       # 2
print $tree->inorder->[0]; # 4
```

Use `undef` for absent positions in level-order arrays. Node values themselves
must be defined. `to_array` preserves missing slots as `undef` through the last
position implied by the tree height.

## Development

```bash
bash BUILD
```
