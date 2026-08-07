# Perl Fenwick tree

A dependency-free Binary Indexed Tree for `O(log n)` prefix sums, range sums,
point updates, and order-statistic lookup over numeric values.

```perl
my $tree = CodingAdventures::FenwickTree->from_list([3, 2, 1, 7, 4]);
print $tree->range_sum(2, 4); # 10
$tree->update(3, 5);
```

Positions are 1-indexed. `find_kth` assumes non-negative values so prefix sums
remain monotonic.

## Development

```bash
bash BUILD
```
