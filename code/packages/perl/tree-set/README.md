# Perl tree set

An AVL-backed mutable ordered set with duplicate suppression, rank and
selection helpers, range queries, custom comparison, and set algebra. The AVL
backend keeps insertion, deletion, and lookup logarithmic while set operations
return independent sets.

```perl
my $set = CodingAdventures::TreeSet->from_values([5, 1, 3, 3, 9]);
$set->add(7);
print $set->contains(7);                # true
print $set->kth_smallest(3);            # 5
print $set->backend->is_valid_avl;      # true
```

`add` returns the set for chaining. `delete`, `remove`, and `discard` mutate the
set and report whether a value was present. Union, intersection, difference,
and symmetric difference leave both inputs unchanged.

## Development

```bash
bash BUILD
```
