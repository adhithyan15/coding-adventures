# Hash Set (Perl)

A pure Perl implementation of [DT19](../../../specs/DT19-hash-set.md). It
wraps the sibling DT18 `hash-map` package and stores elements as keys with one
sentinel value. Every add and remove returns a new set while leaving the input
unchanged.

```perl
use CodingAdventures::HashSet qw(from_list);

my $base = from_list([qw(Ada Grace Ada)]);
my $next = $base->add('Linus');

die unless $base->size == 2;
die unless $next->contains('Linus');
die unless $next->intersection($base)->equals($base);
```

The package includes union, intersection, difference, symmetric difference,
subset, superset, disjoint, and equality operations. Hash-map capacity,
collision strategy, and hash-function options are preserved across persistent
operations.

Run the package gate from this directory with the commands in `BUILD` or
`BUILD_windows`.
