defmodule CodingAdventures.BTree do
  @moduledoc """
  A full-featured, immutable B-tree (DT11) implemented in idiomatic Elixir.

  ## What is a B-tree?

  A B-tree is a self-balancing search tree invented by Rudolf Bayer and
  Edward McCreight (Boeing, 1970) for disk-based storage systems.

  Unlike a binary tree (1 key, 2 children), a B-tree node holds MANY keys.
  This keeps the tree very shallow — a B-tree of degree 128 with height 3
  can index over 4 million records, requiring only 3 disk reads per lookup.

  ## Elixir/Functional Design

  Because Elixir data structures are immutable, every operation returns a
  new tree. We represent a B-tree as `{t, root}` where `t` is the minimum
  degree and `root` is a node tuple.

  Node representation:
  - Leaf:     `{:leaf, keys, values}`
  - Internal: `{:internal, keys, values, children}`

  Where:
  - `keys`     — list of comparable terms, strictly ascending
  - `values`   — parallel list to keys (values[i] belongs to keys[i])
  - `children` — list of child nodes (length == length(keys) + 1)

  ## Invariants (CLRS Chapter 18)

  - Every non-root node has `t-1 ≤ |keys| ≤ 2t-1`
  - The root has `0 ≤ |keys| ≤ 2t-1` (or 0 when empty)
  - Internal nodes have `|children| == |keys| + 1`
  - All leaves are at the same depth
  - Keys within each node are strictly ascending
  - For internal node: max(children[i]) < keys[i] < min(children[i+1])

  ## Usage

      tree = CodingAdventures.BTree.new(3)        # degree-3 B-tree
      tree = CodingAdventures.BTree.insert(tree, 10, "ten")
      tree = CodingAdventures.BTree.insert(tree, 20, "twenty")
      {:ok, "ten"} = CodingAdventures.BTree.search(tree, 10)
      :error       = CodingAdventures.BTree.search(tree, 99)
      true         = CodingAdventures.BTree.member?(tree, 10)
  """

  # ---------------------------------------------------------------------------
  # Type for a B-tree node.
  # We use tagged tuples for pattern matching efficiency.
  # ---------------------------------------------------------------------------
  # @type node :: {:leaf, [key], [value]}
  #             | {:internal, [key], [value], [node]}
  # @type tree :: {pos_integer(), node}

  # ===========================================================================
  # PUBLIC API
  # ===========================================================================

  @doc """
  Create a new empty B-tree with minimum degree `t` (default 2).

  The minimum degree `t` controls node size:
  - Each non-root node has `t-1` to `2t-1` keys
  - t=2: classic "2-3-4 tree"
  - t=128: typical filesystem B-tree

  ## Examples

      iex> tree = CodingAdventures.BTree.new()
      iex> CodingAdventures.BTree.empty?(tree)
      true

      iex> tree = CodingAdventures.BTree.new(3)
      iex> CodingAdventures.BTree.size(tree)
      0
  """
  def new(t \\ 2) when t >= 2 do
    {t, {:leaf, [], []}}
  end

  @doc """
  Insert a key-value pair into the tree. If the key exists, updates the value.

  Returns a new tree (immutable).

  Time: O(t · log_t n)

  ## Examples

      iex> tree = CodingAdventures.BTree.new() |> CodingAdventures.BTree.insert(1, "one")
      iex> CodingAdventures.BTree.search(tree, 1)
      {:ok, "one"}
  """
  def insert({t, root}, key, value) do
    if node_full?(root, t) do
      # Root is full — grow tree height by 1
      # Split the old root and create a new internal root
      {left, sep_k, sep_v, right} = split_child(root, t)
      new_root_split = {:internal, [sep_k], [sep_v], [left, right]}
      {t, insert_non_full(new_root_split, key, value, t)}
    else
      {t, insert_non_full(root, key, value, t)}
    end
  end

  @doc """
  Search for a key and return `{:ok, value}` or `:error`.

  Time: O(t · log_t n)

  ## Examples

      iex> tree = CodingAdventures.BTree.new() |> CodingAdventures.BTree.insert(42, :hello)
      iex> CodingAdventures.BTree.search(tree, 42)
      {:ok, :hello}

      iex> CodingAdventures.BTree.search(CodingAdventures.BTree.new(), 1)
      :error
  """
  def search({_t, root}, key) do
    search_node(root, key)
  end

  @doc """
  Return true if the tree contains the given key.

  ## Examples

      iex> tree = CodingAdventures.BTree.new() |> CodingAdventures.BTree.insert(5, :x)
      iex> CodingAdventures.BTree.member?(tree, 5)
      true
      iex> CodingAdventures.BTree.member?(tree, 6)
      false
  """
  def member?(tree, key) do
    search(tree, key) != :error
  end

  @doc """
  Delete a key from the tree. If the key does not exist, returns the tree unchanged.

  Time: O(t · log_t n)

  ## Examples

      iex> tree = CodingAdventures.BTree.new()
      iex> tree = CodingAdventures.BTree.insert(tree, 10, :a)
      iex> tree = CodingAdventures.BTree.delete(tree, 10)
      iex> CodingAdventures.BTree.member?(tree, 10)
      false
  """
  def delete({t, root}, key) do
    if member?({t, root}, key) do
      new_root = delete_from(root, key, t)
      # If root lost its last key (after merge), its only child becomes root
      new_root =
        case new_root do
          {:internal, [], [], [only_child]} -> only_child
          _ -> new_root
        end
      {t, new_root}
    else
      {t, root}
    end
  end

  @doc """
  Return the minimum key in the tree, or `:empty` if empty.

  ## Examples

      iex> tree = CodingAdventures.BTree.new()
      iex> tree = CodingAdventures.BTree.insert(tree, 5, :a)
      iex> tree = CodingAdventures.BTree.insert(tree, 2, :b)
      iex> CodingAdventures.BTree.min_key(tree)
      2
  """
  def min_key({_t, {:leaf, [], []}}) do
    raise ArgumentError, "Tree is empty"
  end

  def min_key({_t, root}) do
    leftmost_key(root)
  end

  @doc """
  Return the maximum key in the tree, or raises if empty.

  ## Examples

      iex> tree = CodingAdventures.BTree.new()
      iex> tree = CodingAdventures.BTree.insert(tree, 5, :a)
      iex> CodingAdventures.BTree.max_key(tree)
      5
  """
  def max_key({_t, {:leaf, [], []}}) do
    raise ArgumentError, "Tree is empty"
  end

  def max_key({_t, root}) do
    rightmost_key(root)
  end

  @doc """
  Return all `{key, value}` pairs in sorted order (in-order traversal).

  Time: O(n)

  ## Examples

      iex> tree = CodingAdventures.BTree.new()
      iex> tree = Enum.reduce([3,1,2], tree, fn k, t -> CodingAdventures.BTree.insert(t, k, k*10) end)
      iex> CodingAdventures.BTree.inorder(tree)
      [{1, 10}, {2, 20}, {3, 30}]
  """
  def inorder({_t, root}) do
    inorder_node(root)
  end

  @doc """
  Return all `{key, value}` pairs where `low <= key <= high`, sorted.

  Time: O(t · log_t n + k) where k is the number of results.

  ## Examples

      iex> tree = CodingAdventures.BTree.new()
      iex> tree = Enum.reduce(1..10, tree, fn k, t -> CodingAdventures.BTree.insert(t, k, k) end)
      iex> CodingAdventures.BTree.range_query(tree, 3, 6)
      [{3, 3}, {4, 4}, {5, 5}, {6, 6}]
  """
  def range_query({_t, root}, low, high) do
    range_node(root, low, high)
  end

  @doc """
  Return the number of key-value pairs in the tree.

  ## Examples

      iex> tree = CodingAdventures.BTree.new()
      iex> tree = CodingAdventures.BTree.insert(tree, 1, :a)
      iex> CodingAdventures.BTree.size(tree)
      1
  """
  def size({_t, root}) do
    count_keys(root)
  end

  @doc """
  Return true if the tree has no keys.

  ## Examples

      iex> CodingAdventures.BTree.empty?(CodingAdventures.BTree.new())
      true
  """
  def empty?({_t, {:leaf, [], []}}) do
    true
  end

  def empty?(_) do
    false
  end

  @doc """
  Return the height of the tree (0 for a leaf-only tree).

  ## Examples

      iex> tree = CodingAdventures.BTree.new()
      iex> CodingAdventures.BTree.height(tree)
      0
  """
  def height({_t, root}) do
    node_height(root)
  end

  @doc """
  Validate all B-tree invariants. Returns true if valid.

  Checks:
  - Key counts within bounds
  - Sorted order within nodes
  - All leaves at same depth
  - Internal nodes have correct child count
  - BST property between children and separators

  ## Examples

      iex> tree = CodingAdventures.BTree.new()
      iex> tree = Enum.reduce(1..20, tree, fn k, t -> CodingAdventures.BTree.insert(t, k, k) end)
      iex> CodingAdventures.BTree.valid?(tree)
      true
  """
  def valid?({_t, {:leaf, [], []}}) do
    true
  end

  def valid?({t, root}) do
    leaf_depth = node_height(root)
    valid_node?(root, t, nil, nil, leaf_depth, 0, true)
  end

  # ===========================================================================
  # PRIVATE IMPLEMENTATION
  # ===========================================================================

  # ---------------------------------------------------------------------------
  # search_node/2: recurse down the tree looking for key
  # ---------------------------------------------------------------------------
  defp search_node({:leaf, keys, values}, key) do
    case binary_search(keys, key) do
      {:found, idx} -> {:ok, Enum.at(values, idx)}
      _ -> :error
    end
  end

  defp search_node({:internal, keys, values, children}, key) do
    case binary_search(keys, key) do
      {:found, idx} ->
        {:ok, Enum.at(values, idx)}

      {:not_found, idx} ->
        search_node(Enum.at(children, idx), key)
    end
  end

  # ---------------------------------------------------------------------------
  # insert_non_full/4: insert into a node guaranteed not full
  # ---------------------------------------------------------------------------
  defp insert_non_full({:leaf, keys, values}, key, value, _t) do
    case binary_search(keys, key) do
      {:found, idx} ->
        # Update existing key
        {:leaf, keys, List.replace_at(values, idx, value)}

      {:not_found, idx} ->
        {:leaf, List.insert_at(keys, idx, key), List.insert_at(values, idx, value)}
    end
  end

  defp insert_non_full({:internal, keys, values, children}, key, value, t) do
    case binary_search(keys, key) do
      {:found, idx} ->
        # Update existing key
        {:internal, keys, List.replace_at(values, idx, value), children}

      {:not_found, idx} ->
        child = Enum.at(children, idx)

        if node_full?(child, t) do
          # Split the child before descending
          {left, sep_k, sep_v, right} = split_child(child, t)
          new_keys     = List.insert_at(keys, idx, sep_k)
          new_values   = List.insert_at(values, idx, sep_v)
          new_children = children
                         |> List.delete_at(idx)
                         |> List.insert_at(idx, right)
                         |> List.insert_at(idx, left)

          # Recompute which child to descend into
          cmp = compare(key, sep_k)
          if cmp == 0 do
            # Key equals the promoted median — update in place
            {:internal, new_keys, List.replace_at(new_values, idx, value), new_children}
          else
            ci = if cmp > 0, do: idx + 1, else: idx
            new_child = insert_non_full(Enum.at(new_children, ci), key, value, t)
            {:internal, new_keys, new_values, List.replace_at(new_children, ci, new_child)}
          end
        else
          new_child = insert_non_full(child, key, value, t)
          {:internal, keys, values, List.replace_at(children, idx, new_child)}
        end
    end
  end

  # ---------------------------------------------------------------------------
  # split_child/2: split a full node into (left, median_key, median_val, right)
  #
  # For a full node with 2t-1 keys:
  #   left  = keys[0..t-2]  (t-1 keys)
  #   median = keys[t-1]
  #   right = keys[t..2t-2] (t-1 keys)
  # ---------------------------------------------------------------------------
  defp split_child({:leaf, keys, values}, t) do
    mid     = t - 1
    left_k  = Enum.take(keys, mid)
    left_v  = Enum.take(values, mid)
    med_k   = Enum.at(keys, mid)
    med_v   = Enum.at(values, mid)
    right_k = Enum.drop(keys, mid + 1)
    right_v = Enum.drop(values, mid + 1)

    {{:leaf, left_k, left_v}, med_k, med_v, {:leaf, right_k, right_v}}
  end

  defp split_child({:internal, keys, values, children}, t) do
    mid     = t - 1
    left_k  = Enum.take(keys, mid)
    left_v  = Enum.take(values, mid)
    med_k   = Enum.at(keys, mid)
    med_v   = Enum.at(values, mid)
    right_k = Enum.drop(keys, mid + 1)
    right_v = Enum.drop(values, mid + 1)

    left_ch  = Enum.take(children, t)
    right_ch = Enum.drop(children, t)

    {{:internal, left_k, left_v, left_ch}, med_k, med_v, {:internal, right_k, right_v, right_ch}}
  end

  # ---------------------------------------------------------------------------
  # delete_from/3: CLRS B-tree deletion algorithm
  #
  # Cases:
  #   1. Key is in a leaf → simple removal
  #   2a. Key in internal, left child has ≥ t keys → replace with predecessor
  #   2b. Key in internal, right child has ≥ t keys → replace with successor
  #   2c. Key in internal, both sparse → merge children, delete from merged
  #   3. Key not in this node → fill the child before descending
  # ---------------------------------------------------------------------------
  defp delete_from({:leaf, keys, values}, key, _t) do
    case binary_search(keys, key) do
      {:found, idx} ->
        {:leaf, List.delete_at(keys, idx), List.delete_at(values, idx)}

      _ ->
        {:leaf, keys, values}
    end
  end

  defp delete_from({:internal, keys, values, children}, key, t) do
    case binary_search(keys, key) do
      {:found, idx} ->
        # Key is in this internal node
        left  = Enum.at(children, idx)
        right = Enum.at(children, idx + 1)

        cond do
          node_key_count(left) >= t ->
            # Case 2a: predecessor from left subtree
            {pred_k, pred_v} = rightmost_key_value(left)
            new_left = delete_from(left, pred_k, t)
            new_keys     = List.replace_at(keys, idx, pred_k)
            new_values   = List.replace_at(values, idx, pred_v)
            new_children = List.replace_at(children, idx, new_left)
            {:internal, new_keys, new_values, new_children}

          node_key_count(right) >= t ->
            # Case 2b: successor from right subtree
            {succ_k, succ_v} = leftmost_key_value(right)
            new_right = delete_from(right, succ_k, t)
            new_keys     = List.replace_at(keys, idx, succ_k)
            new_values   = List.replace_at(values, idx, succ_v)
            new_children = List.replace_at(children, idx + 1, new_right)
            {:internal, new_keys, new_values, new_children}

          true ->
            # Case 2c: merge left + key + right, then delete from merged
            merged = merge_nodes(left, {Enum.at(keys, idx), Enum.at(values, idx)}, right)
            new_node = delete_from(merged, key, t)
            new_keys     = List.delete_at(keys, idx)
            new_values   = List.delete_at(values, idx)
            new_children =
              children
              |> List.delete_at(idx + 1)
              |> List.replace_at(idx, new_node)
            {:internal, new_keys, new_values, new_children}
        end

      {:not_found, idx} ->
        # Key is not in this node — descend into children[idx]
        child = Enum.at(children, idx)

        if node_key_count(child) < t do
          # Case 3: fill the child before descending
          fill_child({:internal, keys, values, children}, idx, t)
          |> delete_from(key, t)
        else
          new_child = delete_from(child, key, t)
          {:internal, keys, values, List.replace_at(children, idx, new_child)}
        end
    end
  end

  # ---------------------------------------------------------------------------
  # fill_child/3: ensure children[ci] has ≥ t keys
  #
  # Three strategies:
  #   Rotate-Right: borrow from left sibling (if it has ≥ t keys)
  #   Rotate-Left:  borrow from right sibling (if it has ≥ t keys)
  #   Merge:        merge with a sibling (both have t-1 keys)
  # ---------------------------------------------------------------------------
  defp fill_child({:internal, keys, values, children}, ci, t) do
    n    = length(children)
    left_sib  = if ci > 0,     do: Enum.at(children, ci - 1), else: nil
    right_sib = if ci < n - 1, do: Enum.at(children, ci + 1), else: nil
    child     = Enum.at(children, ci)

    cond do
      left_sib != nil and node_key_count(left_sib) >= t ->
        # Rotate-Right: borrow from left sibling
        #
        # The parent's separator at keys[ci-1] descends into child.
        # The last key of left_sib rises to replace it.
        sep_k       = Enum.at(keys, ci - 1)
        sep_v       = Enum.at(values, ci - 1)
        {left_keys, left_vals, left_ch} = node_parts(left_sib)
        borrowed_k  = List.last(left_keys)
        borrowed_v  = List.last(left_vals)

        new_child_keys   = [sep_k | node_keys(child)]
        new_child_values = [sep_v | node_values(child)]
        new_child_children = if node_leaf?(child) do
          []
        else
          [List.last(left_ch) | node_children(child)]
        end

        new_child = rebuild_node(child, new_child_keys, new_child_values, new_child_children)
        new_left  = rebuild_node(
          left_sib,
          Enum.drop(left_keys, -1),
          Enum.drop(left_vals, -1),
          if(left_ch == [], do: [], else: Enum.drop(left_ch, -1))
        )

        new_keys     = List.replace_at(keys, ci - 1, borrowed_k)
        new_values   = List.replace_at(values, ci - 1, borrowed_v)
        new_children =
          children
          |> List.replace_at(ci - 1, new_left)
          |> List.replace_at(ci, new_child)

        {:internal, new_keys, new_values, new_children}

      right_sib != nil and node_key_count(right_sib) >= t ->
        # Rotate-Left: borrow from right sibling
        sep_k       = Enum.at(keys, ci)
        sep_v       = Enum.at(values, ci)
        {right_keys, right_vals, right_ch} = node_parts(right_sib)
        borrowed_k  = hd(right_keys)
        borrowed_v  = hd(right_vals)

        new_child_keys   = node_keys(child) ++ [sep_k]
        new_child_values = node_values(child) ++ [sep_v]
        new_child_children = if node_leaf?(child) do
          []
        else
          node_children(child) ++ [hd(right_ch)]
        end

        new_child  = rebuild_node(child, new_child_keys, new_child_values, new_child_children)
        new_right  = rebuild_node(
          right_sib,
          tl(right_keys),
          tl(right_vals),
          if(right_ch == [], do: [], else: tl(right_ch))
        )

        new_keys     = List.replace_at(keys, ci, borrowed_k)
        new_values   = List.replace_at(values, ci, borrowed_v)
        new_children =
          children
          |> List.replace_at(ci, new_child)
          |> List.replace_at(ci + 1, new_right)

        {:internal, new_keys, new_values, new_children}

      left_sib != nil ->
        # Merge child into left sibling (use index ci-1)
        merged = merge_nodes(left_sib, {Enum.at(keys, ci - 1), Enum.at(values, ci - 1)}, child)
        new_keys     = List.delete_at(keys, ci - 1)
        new_values   = List.delete_at(values, ci - 1)
        new_children =
          children
          |> List.delete_at(ci)
          |> List.replace_at(ci - 1, merged)
        {:internal, new_keys, new_values, new_children}

      true ->
        # Merge with right sibling (use index ci)
        merged = merge_nodes(child, {Enum.at(keys, ci), Enum.at(values, ci)}, right_sib)
        new_keys     = List.delete_at(keys, ci)
        new_values   = List.delete_at(values, ci)
        new_children =
          children
          |> List.delete_at(ci + 1)
          |> List.replace_at(ci, merged)
        {:internal, new_keys, new_values, new_children}
    end
  end

  # ---------------------------------------------------------------------------
  # merge_nodes/3: merge left + separator + right into one node
  # ---------------------------------------------------------------------------
  defp merge_nodes({:leaf, lk, lv}, {sep_k, sep_v}, {:leaf, rk, rv}) do
    {:leaf, lk ++ [sep_k] ++ rk, lv ++ [sep_v] ++ rv}
  end

  defp merge_nodes({:internal, lk, lv, lch}, {sep_k, sep_v}, {:internal, rk, rv, rch}) do
    {:internal, lk ++ [sep_k] ++ rk, lv ++ [sep_v] ++ rv, lch ++ rch}
  end

  # ---------------------------------------------------------------------------
  # Helper: node field access
  # ---------------------------------------------------------------------------
  defp node_leaf?({:leaf, _, _}), do: true
  defp node_leaf?({:internal, _, _, _}), do: false

  defp node_full?({:leaf, keys, _}, t), do: length(keys) == 2 * t - 1
  defp node_full?({:internal, keys, _, _}, t), do: length(keys) == 2 * t - 1

  defp node_key_count({:leaf, keys, _}), do: length(keys)
  defp node_key_count({:internal, keys, _, _}), do: length(keys)

  defp node_keys({:leaf, keys, _}), do: keys
  defp node_keys({:internal, keys, _, _}), do: keys

  defp node_values({:leaf, _, values}), do: values
  defp node_values({:internal, _, values, _}), do: values

  defp node_children({:leaf, _, _}), do: []
  defp node_children({:internal, _, _, children}), do: children

  defp node_parts({:leaf, keys, values}), do: {keys, values, []}
  defp node_parts({:internal, keys, values, children}), do: {keys, values, children}

  defp rebuild_node({:leaf, _, _}, keys, values, _children) do
    {:leaf, keys, values}
  end

  defp rebuild_node({:internal, _, _, _}, keys, values, children) do
    {:internal, keys, values, children}
  end

  # ---------------------------------------------------------------------------
  # Traversal helpers
  # ---------------------------------------------------------------------------
  defp leftmost_key({:leaf, [k | _], _}), do: k
  defp leftmost_key({:internal, _, _, [first | _]}), do: leftmost_key(first)

  defp rightmost_key({:leaf, keys, _}), do: List.last(keys)
  defp rightmost_key({:internal, _, _, children}), do: rightmost_key(List.last(children))

  defp leftmost_key_value({:leaf, [k | _], [v | _]}), do: {k, v}
  defp leftmost_key_value({:internal, _, _, [first | _]}), do: leftmost_key_value(first)

  defp rightmost_key_value({:leaf, keys, values}) do
    {List.last(keys), List.last(values)}
  end

  defp rightmost_key_value({:internal, _, _, children}) do
    rightmost_key_value(List.last(children))
  end

  defp inorder_node({:leaf, keys, values}) do
    Enum.zip(keys, values)
  end

  defp inorder_node({:internal, keys, values, children}) do
    # interleave children with keys:
    # inorder(child[0]), (key[0],val[0]), inorder(child[1]), (key[1],val[1]), ...
    keys
    |> Enum.with_index()
    |> Enum.flat_map(fn {k, i} ->
      inorder_node(Enum.at(children, i)) ++ [{k, Enum.at(values, i)}]
    end)
    |> Kernel.++(inorder_node(List.last(children)))
  end

  defp range_node({:leaf, keys, values}, low, high) do
    Enum.zip(keys, values)
    |> Enum.filter(fn {k, _} -> k >= low and k <= high end)
  end

  defp range_node({:internal, keys, values, children}, low, high) do
    keys
    |> Enum.with_index()
    |> Enum.flat_map(fn {k, i} ->
      left_part = range_node(Enum.at(children, i), low, high)
      kv_part   = if k >= low and k <= high, do: [{k, Enum.at(values, i)}], else: []
      left_part ++ kv_part
    end)
    |> Kernel.++(range_node(List.last(children), low, high))
  end

  defp count_keys({:leaf, keys, _}), do: length(keys)

  defp count_keys({:internal, keys, _, children}) do
    length(keys) + Enum.sum(Enum.map(children, &count_keys/1))
  end

  defp node_height({:leaf, _, _}), do: 0
  defp node_height({:internal, _, _, [first | _]}), do: 1 + node_height(first)

  # ---------------------------------------------------------------------------
  # binary_search/2: find key in a sorted list
  # Returns {:found, idx} or {:not_found, insertion_idx}
  # ---------------------------------------------------------------------------
  defp binary_search(list, key) do
    binary_search(list, key, 0, length(list) - 1)
  end

  defp binary_search(_list, _key, lo, hi) when lo > hi do
    {:not_found, lo}
  end

  defp binary_search(list, key, lo, hi) do
    mid = div(lo + hi, 2)
    mid_key = Enum.at(list, mid)

    case compare(key, mid_key) do
      0 -> {:found, mid}
      n when n < 0 -> binary_search(list, key, lo, mid - 1)
      _ -> binary_search(list, key, mid + 1, hi)
    end
  end

  defp compare(a, b) when a < b, do: -1
  defp compare(a, b) when a > b, do: 1
  defp compare(_, _), do: 0

  # ---------------------------------------------------------------------------
  # Validation
  # ---------------------------------------------------------------------------
  defp valid_node?({:leaf, keys, _values}, t, min_k, max_k, expected_depth, depth, is_root) do
    key_count = length(keys)

    min_keys = if is_root, do: 0, else: t - 1

    with true <- key_count >= min_keys,
         true <- key_count <= 2 * t - 1,
         true <- keys_sorted?(keys, min_k, max_k),
         true <- depth == expected_depth do
      true
    else
      _ -> false
    end
  end

  defp valid_node?({:internal, keys, _values, children}, t, min_k, max_k, expected_depth, depth, is_root) do
    key_count   = length(keys)
    child_count = length(children)
    min_keys    = if is_root, do: 1, else: t - 1

    with true <- key_count >= min_keys,
         true <- key_count <= 2 * t - 1,
         true <- child_count == key_count + 1,
         true <- keys_sorted?(keys, min_k, max_k) do
      # Validate each child with updated bounds
      {all_valid, _} =
        Enum.reduce(keys, {true, {Enum.at(children, 0), min_k}}, fn sep_k, {acc, {child, cur_min}} ->
          child_valid = valid_node?(child, t, cur_min, sep_k, expected_depth, depth + 1, false)
          next_child_idx = Enum.find_index(children, fn c -> c == child end)
          next_child = Enum.at(children, (next_child_idx || -1) + 1)
          {acc and child_valid, {next_child, sep_k}}
        end)

      last_child_valid = valid_node?(List.last(children), t, List.last(keys), max_k, expected_depth, depth + 1, false)
      all_valid and last_child_valid
    else
      _ -> false
    end
  end

  defp keys_sorted?([], _min, _max), do: true
  defp keys_sorted?([h | t], min_k, max_k) do
    (min_k == nil or h > min_k) and
    (max_k == nil or h < max_k) and
    keys_sorted_asc?(t, h)
  end

  defp keys_sorted_asc?([], _prev), do: true
  defp keys_sorted_asc?([h | t], prev) when h > prev, do: keys_sorted_asc?(t, h)
  defp keys_sorted_asc?(_, _), do: false
end
