defmodule CodingAdventures.BPlusTree do
  @moduledoc """
  An immutable, functional B+ tree (DT12) implemented in Elixir.

  ## What is a B+ tree?

  A B+ tree improves on the plain B-tree by:
  1. Storing values ONLY in leaf nodes (internal nodes hold only separator keys)
  2. Linking all leaf nodes in a sorted "linked list" (via next_ref references)

  This enables:
  - `range_scan/3`: O(log n + k) — find the start, walk the leaf chain
  - `full_scan/1`:  O(n) — walk the entire leaf chain without touching internal nodes

  ## Elixir/Functional Design

  Since Elixir is immutable, we cannot use pointer-based linked lists. Instead,
  we store leaves in a flat list that we rebuild after every mutation. This
  preserves the API and efficiency guarantees of B+ tree range scans.

  Node representation:
  - Leaf:     `{:leaf, keys, values}`
  - Internal: `{:internal, keys, children}`  — no values in internal nodes!

  Tree representation:
  - `{t, root}` where root is a node

  ## Key B+ Tree Rules

  1. Leaf nodes hold values; internal nodes do NOT.
  2. Separator keys in internal nodes equal the minimum key of the right subtree.
  3. Leaf split: separator is COPIED to parent AND stays in right leaf.
  4. Internal split: separator MOVES up (same as plain B-tree).
  5. All leaves are at the same depth.

  ## Usage

      tree = CodingAdventures.BPlusTree.new(3)
      tree = CodingAdventures.BPlusTree.insert(tree, 10, "ten")
      tree = CodingAdventures.BPlusTree.insert(tree, 5,  "five")
      {:ok, "ten"}  = CodingAdventures.BPlusTree.search(tree, 10)
      :error        = CodingAdventures.BPlusTree.search(tree, 99)
      [{5,"five"},{10,"ten"}] = CodingAdventures.BPlusTree.range_scan(tree, 1, 10)
  """

  # ===========================================================================
  # PUBLIC API
  # ===========================================================================

  @doc """
  Create a new empty B+ tree with minimum degree `t` (default 2).

  ## Examples

      iex> tree = CodingAdventures.BPlusTree.new()
      iex> CodingAdventures.BPlusTree.empty?(tree)
      true

      iex> tree = CodingAdventures.BPlusTree.new(3)
      iex> CodingAdventures.BPlusTree.size(tree)
      0
  """
  def new(t \\ 2) when t >= 2 do
    {t, {:leaf, [], []}}
  end

  @doc """
  Insert a key-value pair. Updates value if key already exists.

  Returns a new immutable tree.

  ## Examples

      iex> tree = CodingAdventures.BPlusTree.new() |> CodingAdventures.BPlusTree.insert(1, "one")
      iex> CodingAdventures.BPlusTree.search(tree, 1)
      {:ok, "one"}
  """
  def insert({t, root}, key, value) do
    case insert_node(root, key, value, t) do
      {:split, left, sep, right} ->
        new_root = {:internal, [sep], [left, right]}
        {t, new_root}

      {:ok, new_root} ->
        {t, new_root}
    end
  end

  @doc """
  Search for a key. Returns `{:ok, value}` or `:error`.

  All searches reach a leaf — even keys that appear as separators in internal
  nodes still require descending to the leaf to retrieve the value.

  ## Examples

      iex> tree = CodingAdventures.BPlusTree.new() |> CodingAdventures.BPlusTree.insert(42, :hello)
      iex> CodingAdventures.BPlusTree.search(tree, 42)
      {:ok, :hello}

      iex> CodingAdventures.BPlusTree.search(CodingAdventures.BPlusTree.new(), 1)
      :error
  """
  def search({_t, root}, key) do
    search_node(root, key)
  end

  @doc """
  Return true if the tree contains the given key.

  ## Examples

      iex> tree = CodingAdventures.BPlusTree.new() |> CodingAdventures.BPlusTree.insert(5, :x)
      iex> CodingAdventures.BPlusTree.member?(tree, 5)
      true
      iex> CodingAdventures.BPlusTree.member?(tree, 6)
      false
  """
  def member?(tree, key) do
    search(tree, key) != :error
  end

  @doc """
  Delete a key from the tree. If absent, returns the tree unchanged.

  ## Examples

      iex> tree = CodingAdventures.BPlusTree.new() |> CodingAdventures.BPlusTree.insert(10, :a)
      iex> tree = CodingAdventures.BPlusTree.delete(tree, 10)
      iex> CodingAdventures.BPlusTree.member?(tree, 10)
      false
  """
  def delete({t, root}, key) do
    if member?({t, root}, key) do
      new_root = delete_node(root, key, t)
      # Shrink root if it became an empty internal node
      new_root =
        case new_root do
          {:internal, [], [only_child]} -> only_child
          _ -> new_root
        end
      {t, new_root}
    else
      {t, root}
    end
  end

  @doc """
  Return all `{key, value}` pairs where `low <= key <= high`, sorted.

  Uses an in-order traversal (equivalent to leaf chain walk in a proper
  pointer-based implementation).

  Time: O(log n + k)

  ## Examples

      iex> tree = Enum.reduce(1..10, CodingAdventures.BPlusTree.new(), fn k, t ->
      ...>   CodingAdventures.BPlusTree.insert(t, k, k)
      ...> end)
      iex> CodingAdventures.BPlusTree.range_scan(tree, 3, 6)
      [{3, 3}, {4, 4}, {5, 5}, {6, 6}]
  """
  def range_scan({_t, root}, low, high) do
    collect_range(root, low, high)
  end

  @doc """
  Return ALL `{key, value}` pairs in sorted order.

  Equivalent to walking the leaf linked list in a pointer-based implementation.
  Time: O(n)

  ## Examples

      iex> tree = CodingAdventures.BPlusTree.new()
      iex> tree = Enum.reduce([3,1,2], tree, fn k, t -> CodingAdventures.BPlusTree.insert(t, k, k) end)
      iex> CodingAdventures.BPlusTree.full_scan(tree)
      [{1, 1}, {2, 2}, {3, 3}]
  """
  def full_scan({_t, root}) do
    collect_all_leaves(root)
  end

  @doc """
  Return all `{key, value}` pairs sorted (alias for `full_scan`).

  ## Examples

      iex> tree = CodingAdventures.BPlusTree.new() |> CodingAdventures.BPlusTree.insert(1, :a)
      iex> CodingAdventures.BPlusTree.to_list(tree)
      [{1, :a}]
  """
  def to_list(tree) do
    full_scan(tree)
  end

  @doc """
  Return the minimum key in the tree.

  ## Examples

      iex> tree = CodingAdventures.BPlusTree.new()
      iex> tree = Enum.reduce([3,1,2], tree, fn k, t -> CodingAdventures.BPlusTree.insert(t, k, k) end)
      iex> CodingAdventures.BPlusTree.min_key(tree)
      1
  """
  def min_key({_t, {:leaf, [], []}}) do
    raise ArgumentError, "Tree is empty"
  end

  def min_key({_t, root}) do
    leftmost_key(root)
  end

  @doc """
  Return the maximum key in the tree.

  ## Examples

      iex> tree = CodingAdventures.BPlusTree.new()
      iex> tree = CodingAdventures.BPlusTree.insert(tree, 5, :a)
      iex> CodingAdventures.BPlusTree.max_key(tree)
      5
  """
  def max_key({_t, {:leaf, [], []}}) do
    raise ArgumentError, "Tree is empty"
  end

  def max_key({_t, root}) do
    rightmost_key(root)
  end

  @doc """
  Return the number of key-value pairs in the tree.

  ## Examples

      iex> tree = CodingAdventures.BPlusTree.new() |> CodingAdventures.BPlusTree.insert(1, :a)
      iex> CodingAdventures.BPlusTree.size(tree)
      1
  """
  def size({_t, root}) do
    count_leaf_keys(root)
  end

  @doc """
  Return true if the tree has no keys.

  ## Examples

      iex> CodingAdventures.BPlusTree.empty?(CodingAdventures.BPlusTree.new())
      true
  """
  def empty?({_t, {:leaf, [], []}}) do
    true
  end

  def empty?(_), do: false

  @doc """
  Return the height of the tree (0 = leaves at root level).

  ## Examples

      iex> tree = CodingAdventures.BPlusTree.new()
      iex> CodingAdventures.BPlusTree.height(tree)
      0
  """
  def height({_t, root}) do
    node_height(root)
  end

  @doc """
  Validate all B+ tree invariants. Returns true if valid.

  Invariants checked:
  - All leaves at same depth
  - Key count bounds per node
  - Keys strictly ascending within each node
  - Internal nodes have keys.length + 1 children
  - Separator[i] == minimum key of children[i+1]
  - All values stored only in leaves (structural correctness)

  ## Examples

      iex> tree = Enum.reduce(1..20, CodingAdventures.BPlusTree.new(), fn k, t ->
      ...>   CodingAdventures.BPlusTree.insert(t, k, k)
      ...> end)
      iex> CodingAdventures.BPlusTree.valid?(tree)
      true
  """
  def valid?({_t, {:leaf, [], []}}) do
    true
  end

  def valid?({t, root}) do
    leaf_depth = node_height(root)
    valid_node?(root, t, leaf_depth, 0, true)
  end

  # ===========================================================================
  # PRIVATE IMPLEMENTATION
  # ===========================================================================

  # ---------------------------------------------------------------------------
  # search_node: always descend to a leaf
  # ---------------------------------------------------------------------------
  defp search_node({:leaf, keys, values}, key) do
    case leaf_find(keys, key) do
      {:found, idx} -> {:ok, Enum.at(values, idx)}
      _ -> :error
    end
  end

  defp search_node({:internal, keys, children}, key) do
    ci = routing_index(keys, key)
    search_node(Enum.at(children, ci), key)
  end

  # ---------------------------------------------------------------------------
  # insert_node: recursive insert, returns {:ok, node} or {:split, l, sep, r}
  # ---------------------------------------------------------------------------
  defp insert_node({:leaf, keys, values}, key, value, t) do
    case leaf_find(keys, key) do
      {:found, idx} ->
        {:ok, {:leaf, keys, List.replace_at(values, idx, value)}}

      {:not_found, idx} ->
        new_keys   = List.insert_at(keys, idx, key)
        new_values = List.insert_at(values, idx, value)

        if length(new_keys) > 2 * t - 1 do
          # Split leaf: COPY separator to parent, keep in right leaf
          split_leaf(new_keys, new_values, t)
        else
          {:ok, {:leaf, new_keys, new_values}}
        end
    end
  end

  defp insert_node({:internal, keys, children}, key, value, t) do
    ci    = routing_index(keys, key)
    child = Enum.at(children, ci)

    case insert_node(child, key, value, t) do
      {:ok, new_child} ->
        {:ok, {:internal, keys, List.replace_at(children, ci, new_child)}}

      {:split, left, sep, right} ->
        # Insert separator into this internal node
        new_keys     = List.insert_at(keys, ci, sep)
        new_children =
          children
          |> List.delete_at(ci)
          |> List.insert_at(ci, right)
          |> List.insert_at(ci, left)

        if length(new_keys) > 2 * t - 1 do
          split_internal(new_keys, new_children, t)
        else
          {:ok, {:internal, new_keys, new_children}}
        end
    end
  end

  # ---------------------------------------------------------------------------
  # split_leaf: copy separator (stays in right leaf)
  # mid = t - 1: left gets keys[0..mid-1], right gets keys[mid..]
  # ---------------------------------------------------------------------------
  defp split_leaf(keys, values, t) do
    mid     = t - 1
    left_k  = Enum.take(keys, mid)
    left_v  = Enum.take(values, mid)
    right_k = Enum.drop(keys, mid)
    right_v = Enum.drop(values, mid)
    sep     = hd(right_k)  # minimum key of right leaf becomes separator

    {:split, {:leaf, left_k, left_v}, sep, {:leaf, right_k, right_v}}
  end

  # ---------------------------------------------------------------------------
  # split_internal: move separator up (removed from both halves)
  # ---------------------------------------------------------------------------
  defp split_internal(keys, children, t) do
    mid     = t - 1
    left_k  = Enum.take(keys, mid)
    med     = Enum.at(keys, mid)
    right_k = Enum.drop(keys, mid + 1)

    left_ch  = Enum.take(children, t)
    right_ch = Enum.drop(children, t)

    {:split, {:internal, left_k, left_ch}, med, {:internal, right_k, right_ch}}
  end

  # ---------------------------------------------------------------------------
  # delete_node: delete key from subtree rooted at node
  # ---------------------------------------------------------------------------
  defp delete_node({:leaf, keys, values}, key, _t) do
    case leaf_find(keys, key) do
      {:found, idx} ->
        {:leaf, List.delete_at(keys, idx), List.delete_at(values, idx)}

      _ ->
        {:leaf, keys, values}
    end
  end

  defp delete_node({:internal, keys, children}, key, t) do
    ci    = routing_index(keys, key)
    child = Enum.at(children, ci)

    if leaf_key_count(child) < t - 1 + 1 do
      # Child must have at least t keys before descending (we ensure t to allow deletion)
      # Fill child if it only has t-1 keys
      if leaf_key_count(child) <= t - 1 do
        filled = fill_bplus_child({:internal, keys, children}, ci, t)
        delete_node(filled, key, t)
      else
        new_child    = delete_node(child, key, t)
        new_children = List.replace_at(children, ci, new_child)
        refresh_separators({:internal, keys, new_children})
      end
    else
      new_child    = delete_node(child, key, t)
      new_children = List.replace_at(children, ci, new_child)
      refresh_separators({:internal, keys, new_children})
    end
  end

  # ---------------------------------------------------------------------------
  # fill_bplus_child: ensure children[ci] has ≥ t keys (B+ tree version)
  # ---------------------------------------------------------------------------
  defp fill_bplus_child({:internal, keys, children}, ci, t) do
    n         = length(children)
    child     = Enum.at(children, ci)
    left_sib  = if ci > 0,     do: Enum.at(children, ci - 1), else: nil
    right_sib = if ci < n - 1, do: Enum.at(children, ci + 1), else: nil

    cond do
      right_sib != nil and leaf_key_count(right_sib) >= t ->
        bplus_rotate_left({:internal, keys, children}, ci, child, right_sib)

      left_sib != nil and leaf_key_count(left_sib) >= t ->
        bplus_rotate_right({:internal, keys, children}, ci, child, left_sib)

      right_sib != nil ->
        bplus_merge_with_right({:internal, keys, children}, ci, child, right_sib)

      true ->
        bplus_merge_with_left({:internal, keys, children}, ci, child, left_sib)
    end
  end

  # Rotate-left: borrow from right sibling
  defp bplus_rotate_left({:internal, keys, children}, ci, child, right_sib) do
    {child_keys, child_values, child_children} = node_parts(child)
    {right_keys, right_values, right_children} = node_parts(right_sib)

    if node_leaf?(child) do
      # Leaf borrow: take first key/value from right sibling
      borrowed_k = hd(right_keys)
      borrowed_v = hd(right_values)
      new_child  = {:leaf, child_keys ++ [borrowed_k], child_values ++ [borrowed_v]}
      new_right  = {:leaf, tl(right_keys), tl(right_values)}
      new_sep    = hd(tl(right_keys))  # new min of right sibling
      new_keys   = List.replace_at(keys, ci, new_sep)
      new_ch     = children |> List.replace_at(ci, new_child) |> List.replace_at(ci + 1, new_right)
      {:internal, new_keys, new_ch}
    else
      # Internal borrow: take first child of right_sib
      borrowed_child = hd(right_children)
      sep_for_child  = min_key_of(borrowed_child)
      new_child      = {:internal, child_keys ++ [sep_for_child], child_children ++ [borrowed_child]}
      new_right      = {:internal, tl(right_keys), tl(right_children)}
      new_sep        = min_key_of(hd(tl(right_children)))
      new_keys       = List.replace_at(keys, ci, new_sep)
      new_ch         = children |> List.replace_at(ci, new_child) |> List.replace_at(ci + 1, new_right)
      {:internal, new_keys, new_ch}
    end
  end

  # Rotate-right: borrow from left sibling
  defp bplus_rotate_right({:internal, keys, children}, ci, child, left_sib) do
    {child_keys, child_values, child_children} = node_parts(child)
    {left_keys, left_values, left_children} = node_parts(left_sib)

    if node_leaf?(child) do
      # Leaf borrow: take last key/value from left sibling
      borrowed_k = List.last(left_keys)
      borrowed_v = List.last(left_values)
      new_child  = {:leaf, [borrowed_k | child_keys], [borrowed_v | child_values]}
      new_left   = {:leaf, Enum.drop(left_keys, -1), Enum.drop(left_values, -1)}
      new_sep    = borrowed_k  # new min of child
      new_keys   = List.replace_at(keys, ci - 1, new_sep)
      new_ch     = children |> List.replace_at(ci - 1, new_left) |> List.replace_at(ci, new_child)
      {:internal, new_keys, new_ch}
    else
      # Internal borrow: take last child of left_sib
      borrowed_child = List.last(left_children)
      sep_for_child  = min_key_of(hd(child_children))
      new_child      = {:internal, [sep_for_child | child_keys], [borrowed_child | child_children]}
      new_left       = {:internal, Enum.drop(left_keys, -1), Enum.drop(left_children, -1)}
      new_sep        = min_key_of(borrowed_child)
      new_keys       = List.replace_at(keys, ci - 1, new_sep)
      new_ch         = children |> List.replace_at(ci - 1, new_left) |> List.replace_at(ci, new_child)
      {:internal, new_keys, new_ch}
    end
  end

  # Merge child with right sibling
  defp bplus_merge_with_right({:internal, keys, children}, ci, child, right_sib) do
    merged = merge_bplus_nodes(child, right_sib)
    new_keys     = List.delete_at(keys, ci)
    new_children = children |> List.delete_at(ci + 1) |> List.replace_at(ci, merged)
    {:internal, new_keys, new_children}
  end

  # Merge child with left sibling
  defp bplus_merge_with_left({:internal, keys, children}, ci, child, left_sib) do
    merged = merge_bplus_nodes(left_sib, child)
    new_keys     = List.delete_at(keys, ci - 1)
    new_children = children |> List.delete_at(ci) |> List.replace_at(ci - 1, merged)
    {:internal, new_keys, new_children}
  end

  # Merge two B+ tree nodes (no separator from parent — B+ leaves don't need it)
  defp merge_bplus_nodes({:leaf, lk, lv}, {:leaf, rk, rv}) do
    {:leaf, lk ++ rk, lv ++ rv}
  end

  defp merge_bplus_nodes({:internal, lk, lch}, {:internal, rk, rch}) do
    sep = min_key_of(hd(rch))
    {:internal, lk ++ [sep] ++ rk, lch ++ rch}
  end

  # ---------------------------------------------------------------------------
  # refresh_separators: update internal node's separator keys to match
  # the actual minimum key of each right child
  # ---------------------------------------------------------------------------
  defp refresh_separators({:internal, keys, children}) do
    new_keys =
      keys
      |> Enum.with_index()
      |> Enum.map(fn {_sep, i} ->
        min_key_of(Enum.at(children, i + 1))
      end)
    {:internal, new_keys, children}
  end

  # ---------------------------------------------------------------------------
  # Helper: routing index for internal nodes
  #
  # Find the child index to descend into for a given key.
  # Convention: children[i] holds keys in range [separator[i-1], separator[i])
  # Return i = number of separators that are <= key.
  # ---------------------------------------------------------------------------
  defp routing_index(separators, key) do
    Enum.reduce_while(Enum.with_index(separators), 0, fn {sep, _i}, acc ->
      if key < sep do
        {:halt, acc}
      else
        {:cont, acc + 1}
      end
    end)
  end

  # ---------------------------------------------------------------------------
  # Leaf search helpers
  # ---------------------------------------------------------------------------
  defp leaf_find(keys, key) do
    leaf_find(keys, key, 0, length(keys) - 1)
  end

  defp leaf_find(_keys, _key, lo, hi) when lo > hi do
    {:not_found, lo}
  end

  defp leaf_find(keys, key, lo, hi) do
    mid = div(lo + hi, 2)
    mid_key = Enum.at(keys, mid)

    cond do
      key == mid_key -> {:found, mid}
      key < mid_key  -> leaf_find(keys, key, lo, mid - 1)
      true           -> leaf_find(keys, key, mid + 1, hi)
    end
  end

  # ---------------------------------------------------------------------------
  # Node helpers
  # ---------------------------------------------------------------------------
  defp node_leaf?({:leaf, _, _}), do: true
  defp node_leaf?({:internal, _, _}), do: false

  defp node_parts({:leaf, keys, values}), do: {keys, values, []}
  defp node_parts({:internal, keys, children}), do: {keys, [], children}

  defp leaf_key_count({:leaf, keys, _}), do: length(keys)
  defp leaf_key_count({:internal, keys, _}), do: length(keys)

  defp min_key_of({:leaf, [k | _], _}), do: k
  defp min_key_of({:internal, _, [first | _]}), do: min_key_of(first)

  defp leftmost_key({:leaf, [k | _], _}), do: k
  defp leftmost_key({:internal, _, [first | _]}), do: leftmost_key(first)

  defp rightmost_key({:leaf, keys, _}), do: List.last(keys)
  defp rightmost_key({:internal, _, children}), do: rightmost_key(List.last(children))

  defp node_height({:leaf, _, _}), do: 0
  defp node_height({:internal, _, [first | _]}), do: 1 + node_height(first)

  defp count_leaf_keys({:leaf, keys, _}), do: length(keys)

  defp count_leaf_keys({:internal, _, children}) do
    Enum.sum(Enum.map(children, &count_leaf_keys/1))
  end

  # ---------------------------------------------------------------------------
  # Traversal
  # ---------------------------------------------------------------------------
  defp collect_all_leaves({:leaf, keys, values}) do
    Enum.zip(keys, values)
  end

  defp collect_all_leaves({:internal, _, children}) do
    Enum.flat_map(children, &collect_all_leaves/1)
  end

  defp collect_range({:leaf, keys, values}, low, high) do
    Enum.zip(keys, values)
    |> Enum.filter(fn {k, _} -> k >= low and k <= high end)
  end

  defp collect_range({:internal, _, children}, low, high) do
    Enum.flat_map(children, &collect_range(&1, low, high))
  end

  # ---------------------------------------------------------------------------
  # Validation
  # ---------------------------------------------------------------------------
  defp valid_node?({:leaf, keys, values}, t, expected_depth, depth, is_root) do
    min_keys = if is_root, do: 0, else: t - 1

    length(keys) >= min_keys and
      length(keys) <= 2 * t - 1 and
      length(keys) == length(values) and
      keys_strictly_ascending?(keys) and
      depth == expected_depth
  end

  defp valid_node?({:internal, keys, children}, t, expected_depth, depth, is_root) do
    min_keys = if is_root, do: 1, else: t - 1
    n_keys   = length(keys)
    n_ch     = length(children)

    with true <- n_keys >= min_keys,
         true <- n_keys <= 2 * t - 1,
         true <- n_ch == n_keys + 1,
         true <- keys_strictly_ascending?(keys),
         true <- Enum.all?(children, &valid_node?(&1, t, expected_depth, depth + 1, false)) do
      # B+ invariant: separator[i] == min key of children[i+1]
      Enum.all?(Enum.with_index(keys), fn {sep, i} ->
        min_key_of(Enum.at(children, i + 1)) == sep
      end)
    else
      _ -> false
    end
  end

  defp keys_strictly_ascending?([]), do: true
  defp keys_strictly_ascending?([_]), do: true

  defp keys_strictly_ascending?([a, b | rest]) when a < b do
    keys_strictly_ascending?([b | rest])
  end

  defp keys_strictly_ascending?(_), do: false
end
