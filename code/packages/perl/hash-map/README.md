# Hash Map (Perl)

A pure Perl implementation of [DT18](../../../specs/DT18-hash-map.md). It
builds separate-chaining and linear-probing tables from arrays, preserves open
addressing probe chains with tombstones, and resizes automatically. Bucket
selection comes from the sibling `hash-functions` package.

```perl
use CodingAdventures::HashMap;

my $map = CodingAdventures::HashMap->new(strategy => 'open_addressing');
$map->set(language => 'Perl');
die unless $map->get('language') eq 'Perl';

my $next = $map->with_set(year => 1987);
die if $map->has('year');
die unless $next->get('year') == 1987;
```

Run the package gate from this directory with the commands in `BUILD` or
`BUILD_windows`.
