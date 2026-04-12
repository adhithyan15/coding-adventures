# coding_adventures_b_tree (Elixir)

An immutable, functional **B-tree** (DT11) implemented in idiomatic Elixir.

## What is a B-tree?

A B-tree is a self-balancing search tree invented by Rudolf Bayer and Edward
McCreight (Boeing, 1970). Unlike binary trees, each node holds many keys,
keeping the tree very shallow.

## Elixir Design

Because Elixir data structures are immutable, every operation returns a new
tree. Nodes are represented as tagged tuples for efficient pattern matching:

```elixir
{:leaf, keys, values}                      # leaf node
{:internal, keys, values, children}        # internal node
{t, root}                                  # the tree itself
```

## Usage

```elixir
tree = CodingAdventures.BTree.new(3)       # minimum degree 3

# Insert
tree = CodingAdventures.BTree.insert(tree, 10, "ten")
tree = CodingAdventures.BTree.insert(tree, 5,  "five")
tree = CodingAdventures.BTree.insert(tree, 20, "twenty")

# Search
CodingAdventures.BTree.search(tree, 10)    # => {:ok, "ten"}
CodingAdventures.BTree.search(tree, 99)    # => :error
CodingAdventures.BTree.member?(tree, 10)   # => true

# Range
CodingAdventures.BTree.range_query(tree, 5, 15)
# => [{5, "five"}, {10, "ten"}]

# Traversal
CodingAdventures.BTree.inorder(tree)
# => [{5, "five"}, {10, "ten"}, {20, "twenty"}]

# Min / Max
CodingAdventures.BTree.min_key(tree)       # => 5
CodingAdventures.BTree.max_key(tree)       # => 20

# Delete
tree = CodingAdventures.BTree.delete(tree, 10)
CodingAdventures.BTree.member?(tree, 10)   # => false

# Metadata
CodingAdventures.BTree.size(tree)          # => 2
CodingAdventures.BTree.height(tree)        # => 0 (all leaves are at height 0)
CodingAdventures.BTree.valid?(tree)        # => true
```

## Performance

| Operation    | Time complexity    |
|--------------|--------------------|
| search       | O(t · log_t n)     |
| insert       | O(t · log_t n)     |
| delete       | O(t · log_t n)     |
| range_query  | O(t · log_t n + k) |
| inorder      | O(n)               |
| min/max      | O(log_t n)         |
