# CodingAdventures::Trie

A dependency-free Perl prefix trie for string keys and arbitrary values. It
supports exact lookup, deletion with pruning, lexicographically sorted
enumeration, prefix scans, and longest-prefix matching.

```perl
use CodingAdventures::Trie;

my $trie = CodingAdventures::Trie->new;
$trie->insert('app', 1)->insert('apple', 2);

my $match = $trie->longest_prefix_match('apples');
print $match->[0]; # apple
```
