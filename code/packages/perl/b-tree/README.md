# perl/b-tree — B-Tree (DT11)

A B-Tree implementation in pure Perl that maps string keys to arbitrary Perl values.

## What is a B-Tree?

A B-Tree is a self-balancing search tree invented at Boeing Research Labs in 1970.
It's the data structure powering virtually every database and filesystem.

## API

```perl
use CodingAdventures::BTree;

my $tree = CodingAdventures::BTree->new(t => 2);

$tree->insert("apple",  42);
$tree->insert("banana", 99);

$tree->search("apple");         # 42
$tree->search("missing");       # undef

$tree->min_key;                 # "apple"
$tree->max_key;                 # "banana"

my @all = $tree->inorder;       # (["apple", 42], ["banana", 99])
my @r   = $tree->range_query("a", "b");

$tree->delete("apple");         # 1 (found)
$tree->delete("missing");       # 0 (not found)

$tree->size;                    # 1
$tree->height;                  # 0
$tree->is_valid;                # 1
```

## Stack position

Standalone data structure (DT11). The B+ Tree lives at `perl/b-plus-tree` (DT12).
