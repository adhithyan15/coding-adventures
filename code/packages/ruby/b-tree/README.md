# coding_adventures_b_tree

A full-featured, in-memory **B-tree** (DT11) for Ruby.

## What is a B-tree?

A B-tree is a self-balancing search tree invented by Rudolf Bayer and Edward
McCreight (Boeing, 1970) for disk-based storage. Unlike binary trees, each
node holds many keys, so the tree stays very shallow — perfect when each node
access is expensive (disk read, cache miss, etc.).

```
Minimum degree t = 2 (a "2-3-4 tree"):

         [20]
        /    \
     [10]   [30, 40]
     /  \   /  |  \
   [5] [15][25][35][50]
```

All leaves are at the same depth — the "balanced" guarantee.

## How it fits in the stack

| Layer | Package          | Description                        |
|-------|------------------|------------------------------------|
| DT09  | `graph`          | Undirected graph                   |
| DT10  | `tree`           | Rooted tree backed by directed graph|
| **DT11** | **`b-tree`** | **B-tree (this package)**         |
| DT12  | `b-plus-tree`    | B+ tree with leaf linked list      |

## Installation

```ruby
# In your Gemfile:
gem "coding_adventures_b_tree", path: "../b-tree"
```

## Usage

```ruby
require "coding_adventures_b_tree"

tree = CodingAdventures::BTree.new(t: 3)  # minimum degree 3

# Insert
tree.insert(10, "ten")
tree.insert(20, "twenty")
tree.insert(5,  "five")
tree[30] = "thirty"                        # alias for insert

# Search
tree.search(10)           # => "ten"
tree.include?(20)         # => true
tree[30]                  # => "thirty"
tree[99]                  # => raises KeyError

# Range
tree.range_query(5, 15)   # => [[5, "five"], [10, "ten"]]
tree.inorder              # => [[5, "five"], [10, "ten"], [20, "twenty"], [30, "thirty"]]

# Min / Max
tree.min_key              # => 5
tree.max_key              # => 30

# Delete
tree.delete(10)
tree.include?(10)         # => false

# Metadata
tree.size                 # => 3
tree.empty?               # => false
tree.height               # => 1
tree.valid?               # => true  (checks all B-tree invariants)
```

## Algorithm

Follows **CLRS Chapter 18** ("B-Trees") with the proactive-split insertion
strategy: full nodes are split on the way down so deletion never needs a
second pass. Deletion handles all CLRS sub-cases:

| Case | Description                            |
|------|----------------------------------------|
| 1    | Key in leaf — simple removal           |
| 2a   | Key in internal, left child rich — use predecessor |
| 2b   | Key in internal, right child rich — use successor  |
| 2c   | Key in internal, both children sparse — merge      |
| 3    | Key not in node — fill child before descending     |
| 3a   | Fill via rotate-right (borrow from left sibling)   |
| 3b   | Fill via rotate-left (borrow from right sibling)   |
| 3c   | Fill via merge with sibling                        |

## Performance

| Operation    | Time complexity      |
|--------------|----------------------|
| search       | O(t · log_t n)       |
| insert       | O(t · log_t n)       |
| delete       | O(t · log_t n)       |
| range_query  | O(t · log_t n + k)   |
| inorder      | O(n)                 |
| min/max      | O(log_t n)           |

Where `t` is the minimum degree and `n` is the number of keys.
