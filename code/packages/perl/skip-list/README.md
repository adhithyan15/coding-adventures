# skip-list

Pure Perl probabilistic ordered map with expected O(log n) insertion,
deletion, lookup, rank, and selection. Forward pointers carry spans, so rank
queries navigate the tower rather than scanning the bottom level.

## Usage

```perl
use CodingAdventures::SkipList;

my $list = CodingAdventures::SkipList->new;
$list->insert(5, 'five');
$list->insert(2, 'two');
$list->insert(8, 'eight');

die unless $list->search(5) eq 'five';
die unless $list->rank(5) == 1;
die unless $list->by_rank(0) == 2;
```

The constructor accepts `max_level`, `probability`, `compare`, and `seed`
options. Defaults are 16 levels, a 0.5 promotion probability, natural
numeric-or-string ordering, and a deterministic local seed. A Park-Miller
generator keeps the topology reproducible without changing Perl's global
random state.

## API

- `insert($key, $value)` inserts or updates and reports whether the key was new.
- `delete($key)` / `remove($key)` removes a key and reports whether it existed.
- `search($key)` / `get($key)` returns the stored value; `contains($key)`
  distinguishes an undefined value from a missing key.
- `rank($key)` and `by_rank($rank)` use zero-based ranks.
- `kth_smallest($k)` uses one-based selection.
- `range_query($minimum, $maximum, $inclusive)` returns ordered key/value
  arrayrefs. Bounds are inclusive by default.
- `to_list`, `entries`, `iterator`, `min`, `max`, `size`, and `is_empty`
  expose ordered-map state.
- `is_valid_skip_list` checks ordering, height, span, and size invariants.

The package uses only core Perl modules and needs no external capabilities.

## Tests

```sh
cd code/packages/perl/skip-list
prove -l -v t/
```
