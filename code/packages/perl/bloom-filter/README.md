# CodingAdventures::BloomFilter

A pure-Perl Bloom filter for compact probabilistic set membership. Inserted
values never produce false negatives; positive lookups may be false positives
at the configured rate.

```perl
use CodingAdventures::BloomFilter;

my $filter = CodingAdventures::BloomFilter->new(
    expected_items => 1_000,
    false_positive_rate => 0.01,
);
$filter->add('hello');
die 'missing' unless $filter->contains('hello');
```

The filter automatically derives its bit count and hash count, or accepts an
explicit layout through `from_params`. It exposes the number of set bits, fill
ratio, estimated current false-positive rate, byte size, and over-capacity
state. Scalars are byte-safe, while arrays and hashes use a deterministic
encoding with stable hash-key order.

Double hashing uses FNV-1a and DJB2 from the sibling `hash-functions` package.
MurmurHash3 finalization reduces correlation between the base hashes and an odd
probe step improves bit-array coverage.
