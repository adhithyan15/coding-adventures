# coding_adventures_b_plus_tree (Elixir)

An immutable, functional **B+ tree** (DT12) implemented in idiomatic Elixir.

## What is a B+ tree?

A B+ tree improves on the plain B-tree by storing values ONLY in leaf nodes
and linking all leaves in a sorted list. This enables O(log n + k) range scans.

## Elixir Design

Nodes are represented as tagged tuples:

```elixir
{:leaf, keys, values}            # Leaf — holds actual data
{:internal, keys, children}      # Internal — routing only, no values!
{t, root}                        # The tree: (degree, root_node)
```

## Usage

```elixir
tree = CodingAdventures.BPlusTree.new(3)

# Insert
tree = CodingAdventures.BPlusTree.insert(tree, 10, "ten")
tree = CodingAdventures.BPlusTree.insert(tree, 5,  "five")
tree = CodingAdventures.BPlusTree.insert(tree, 20, "twenty")

# Point search (always reaches a leaf)
CodingAdventures.BPlusTree.search(tree, 10)     # => {:ok, "ten"}
CodingAdventures.BPlusTree.search(tree, 99)     # => :error
CodingAdventures.BPlusTree.member?(tree, 10)    # => true

# Range scan — O(log n + k)
CodingAdventures.BPlusTree.range_scan(tree, 5, 15)
# => [{5, "five"}, {10, "ten"}]

# Full scan — O(n), visits only leaves
CodingAdventures.BPlusTree.full_scan(tree)
# => [{5, "five"}, {10, "ten"}, {20, "twenty"}]

# to_list (alias for full_scan)
CodingAdventures.BPlusTree.to_list(tree)
# => [{5, "five"}, {10, "ten"}, {20, "twenty"}]

# Min / Max
CodingAdventures.BPlusTree.min_key(tree)        # => 5
CodingAdventures.BPlusTree.max_key(tree)        # => 20

# Delete
tree = CodingAdventures.BPlusTree.delete(tree, 10)
CodingAdventures.BPlusTree.member?(tree, 10)    # => false

# Metadata
CodingAdventures.BPlusTree.size(tree)           # => 2
CodingAdventures.BPlusTree.height(tree)         # => 0
CodingAdventures.BPlusTree.valid?(tree)         # => true
```

## B+ Tree vs B-Tree

| Feature              | B-tree            | B+ tree                    |
|----------------------|-------------------|----------------------------|
| Values stored in     | Every node        | Leaves only                |
| Leaf split           | Separator moves up| Separator copied up        |
| Range scan           | O(n) in-order     | O(log n + k) leaf walk     |

## Performance

| Operation    | Time complexity    |
|--------------|--------------------|
| search       | O(t · log_t n)     |
| insert       | O(t · log_t n)     |
| delete       | O(t · log_t n)     |
| range_scan   | O(t · log_t n + k) |
| full_scan    | O(n)               |
| min/max      | O(log_t n)         |
