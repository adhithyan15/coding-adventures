# CodingAdventures::RadixTree

A dependency-free Perl radix tree for Unicode string keys and arbitrary
values. Whole substrings are stored on edges, and deletion merges redundant
paths to preserve the compressed representation.

```perl
use CodingAdventures::RadixTree;

my $tree = CodingAdventures::RadixTree->new;
$tree->insert('search', 1)->insert('searcher', 2);

print $tree->search('search');                    # 1
print $tree->longest_prefix_match('search-path'); # search
```

The public API includes exact lookup and membership, sorted key and prefix
enumeration, longest-prefix matching, deletion, hash export, node counting,
and structural validation. Empty-string keys are supported.
