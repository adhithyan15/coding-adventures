# coding_adventures_b_plus_tree

A full-featured, in-memory **B+ tree** (DT12) for Ruby with leaf linked list.

## What is a B+ tree?

A B+ tree improves on the B-tree by storing values ONLY in leaf nodes and
linking all leaves in a sorted linked list. This makes range scans and full
scans extremely efficient — no backtracking needed.

```
t = 2 example:

           [4]
          /    \
       [2]    [6, 8]
      /   \   /  |  \
   [1] → [2,3] → [4,5] → [6,7] → [8,9] → nil
    └─────────── leaf linked list ────────────┘
```

All values live in the bottom row (leaves). The upper nodes contain only
separator keys for routing.

## How it fits in the stack

| Layer | Package            | Description                         |
|-------|--------------------|-------------------------------------|
| DT11  | `b-tree`           | B-tree (values in every node)       |
| **DT12** | **`b-plus-tree`** | **B+ tree (this package)**      |
| DT13  | `lsm-tree`         | LSM-tree (log-structured merge)     |

## Installation

```ruby
gem "coding_adventures_b_plus_tree", path: "../b-plus-tree"
```

## Usage

```ruby
require "coding_adventures_b_plus_tree"

tree = CodingAdventures::BPlusTree.new(t: 3)

# Insert
tree.insert(10, "ten")
tree.insert(20, "twenty")
tree.insert(5, "five")
tree[30] = "thirty"

# Point search (always reaches a leaf)
tree.search(10)           # => "ten"
tree.include?(20)         # => true
tree[30]                  # => "thirty"

# Range scan — uses leaf linked list, O(log n + k)
tree.range_scan(5, 20)    # => [[5, "five"], [10, "ten"], [20, "twenty"]]

# Full scan — walks leaf list, O(n)
tree.full_scan            # => [[5, "five"], [10, "ten"], [20, "twenty"], [30, "thirty"]]

# Enumerable
tree.map { |k, v| "#{k}=#{v}" }
tree.select { |k, _| k > 10 }

# Min / Max
tree.min_key              # => 5
tree.max_key              # => 30

# Delete
tree.delete(10)
tree.size                 # => 3
tree.valid?               # => true
```

## Key Difference from B-tree

| Aspect           | B-tree                  | B+ tree                    |
|------------------|-------------------------|----------------------------|
| Value storage    | Every node              | Leaves only                |
| Leaf split       | Median moves up         | Median copied up           |
| Range scan       | O(n) in-order           | O(log n + k) via leaf list |
| Full scan        | O(n) with backtracking  | O(n) walk leaf list        |

## Performance

| Operation    | Time complexity    |
|--------------|--------------------|
| search       | O(t · log_t n)     |
| insert       | O(t · log_t n)     |
| delete       | O(t · log_t n)     |
| range_scan   | O(t · log_t n + k) |
| full_scan    | O(n)               |
| min/max      | O(log_t n)         |
