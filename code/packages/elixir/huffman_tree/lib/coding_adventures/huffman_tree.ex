defmodule CodingAdventures.HuffmanTree do
  @moduledoc """
  HuffmanTree — DT27: Huffman Tree data structure.

  ## What Is a Huffman Tree?

  A Huffman tree is a full binary tree (every internal node has exactly two
  children) built from a symbol alphabet so that each symbol gets a unique
  variable-length bit code. Symbols that appear often get short codes; symbols
  that appear rarely get long codes. The total bits needed to encode a message
  is minimised — it is the theoretically optimal prefix-free code for a given
  symbol frequency distribution.

  Think of it like Morse code. In Morse, `E` is `.` (one dot) and `Z` is
  `--..` (four symbols). The designers knew `E` is the most common letter in
  English so they gave it the shortest code. Huffman's algorithm does this
  automatically and optimally for any alphabet with any frequency distribution.

  ## Algorithm: Greedy Construction via Min-Heap

  1. Create one leaf node per distinct symbol, each with its frequency as its
     weight. Push all leaves onto a min-heap keyed by weight.

  2. While the heap has more than one node:
     a. Pop the two nodes with the smallest weight.
     b. Create a new internal node whose weight = sum of the two children.
     c. Set left = the first popped node, right = the second popped node.
     d. Push the new internal node back onto the heap.

  3. The one remaining node is the root of the Huffman tree.

  ## Tie-Breaking Rules (Deterministic Output)

  Without tie-breaking, different implementations could build structurally
  different trees from the same input — producing different (but equally valid)
  code lengths. Deterministic tie-breaking ensures the canonical code table is
  identical everywhere.

    1. Lowest weight pops first.
    2. Leaf nodes have higher priority than internal nodes at equal weight
       ("leaf-before-internal" rule).
    3. Among leaves of equal weight, lower symbol value wins.
    4. Among internal nodes of equal weight, earlier-created node wins
       (insertion-order FIFO).

  ## Prefix-Free Property: Why It Works

  In a Huffman tree:
    - Symbols live ONLY at the leaves, never at internal nodes.
    - The code for a symbol is the path from root to its leaf
      (left edge = '0', right edge = '1').

  Since one leaf is never an ancestor of another leaf, no code can be a prefix
  of another code. This is the prefix-free property, and it means the bit
  stream can be decoded unambiguously without separator characters: just walk
  the tree bit by bit until you hit a leaf.

  ## Canonical Codes (DEFLATE / zlib Style)

  The standard tree-walk produces valid codes, but different tree shapes can
  produce different codes for the same symbol lengths. Canonical codes
  normalise this: given only the code *lengths*, you can reconstruct the exact
  canonical code table without transmitting the tree structure.

  Algorithm:
    1. Collect (symbol, code_length) pairs from the tree.
    2. Sort by (code_length, symbol_value).
    3. Assign codes numerically:
         code[0] = 0 (left-padded to length[0] bits)
         code[i] = (code[i-1] + 1) << (length[i] - length[i-1])

  This is exactly what DEFLATE uses: the compressed stream contains only the
  length table, not the tree, saving space.

  ## Example

      iex> tree = CodingAdventures.HuffmanTree.build([{65, 3}, {66, 2}, {67, 1}])
      iex> CodingAdventures.HuffmanTree.code_table(tree)
      %{65 => "0", 66 => "11", 67 => "10"}
      iex> CodingAdventures.HuffmanTree.weight(tree)
      6

  ## Series

      DT27 (HuffmanTree) — this module.
      CMP04 (Huffman Coding) — uses DT27 to compress/decompress data.
  """

  import Bitwise
  alias CodingAdventures.Heap.MinHeap

  # ─── Node representation ────────────────────────────────────────────────────
  #
  # We represent the Huffman tree using tagged tuples:
  #
  #   {:leaf, symbol, weight}
  #     - symbol: non-negative integer (e.g. byte value 0..255)
  #     - weight: frequency count (positive integer)
  #
  #   {:internal, weight, left, right, order}
  #     - weight: sum of children's weights
  #     - left:   left child node (reached by '0' bit)
  #     - right:  right child node (reached by '1' bit)
  #     - order:  monotonic counter for tie-breaking (FIFO among internals)
  #
  # Using tagged tuples is idiomatic Elixir — pattern matching on the first
  # element is fast and readable, and the immutable data plays well with
  # the recursive tree-walking style we use throughout.

  # ─── Priority key ──────────────────────────────────────────────────────────
  #
  # The heap compares nodes by a 4-element tuple, lower = higher priority:
  #
  #   {weight, type_flag, symbol_or_max, order_or_max}
  #
  #   type_flag: 0 = leaf (higher priority), 1 = internal
  #   symbol_or_max: leaf → symbol value; internal → max_int (unused)
  #   order_or_max:  internal → insertion order; leaf → max_int (unused)
  #
  # This single comparison key encodes all four tie-breaking rules at once.
  # By packing them into a tuple we can use Elixir's built-in tuple comparison
  # (lexicographic) without any custom comparator logic.

  @max_int :math.pow(2, 62) |> trunc()

  defp node_priority({:leaf, symbol, weight}) do
    {weight, 0, symbol, @max_int}
  end

  defp node_priority({:internal, weight, _left, _right, order}) do
    {weight, 1, @max_int, order}
  end

  defp node_weight({:leaf, _sym, weight}), do: weight
  defp node_weight({:internal, weight, _l, _r, _o}), do: weight

  # ─── build/1 ───────────────────────────────────────────────────────────────

  @doc """
  Construct a Huffman tree from a list of `{symbol, frequency}` pairs.

  The greedy algorithm uses a min-heap. At each step it pops the two
  lowest-weight nodes, combines them into a new internal node, and pushes the
  internal node back. The single remaining node is the root.

  ## Tie-breaking (for deterministic output across implementations)

    1. Lowest weight pops first.
    2. Leaves before internal nodes at equal weight.
    3. Lower symbol value wins among leaves of equal weight.
    4. Earlier-created internal node wins among internal nodes of equal
       weight (FIFO insertion order).

  ## Arguments

    - `weights` — list of `{symbol, frequency}` pairs. Each symbol must be a
      non-negative integer; each frequency must be > 0.

  ## Returns

  A `{root_node, symbol_count}` pair representing the immutable tree.

  ## Raises

    - `ArgumentError` if `weights` is empty or any frequency is ≤ 0.

  ## Examples

      iex> tree = CodingAdventures.HuffmanTree.build([{65, 3}, {66, 2}, {67, 1}])
      iex> CodingAdventures.HuffmanTree.symbol_count(tree)
      3
  """
  def build([]) do
    raise ArgumentError, "weights must not be empty"
  end

  def build(weights) when is_list(weights) do
    # Validate all frequencies are positive
    Enum.each(weights, fn {sym, freq} ->
      if freq <= 0 do
        raise ArgumentError,
              "frequency must be positive; got symbol=#{sym}, freq=#{freq}"
      end
    end)

    # Build the min-heap. Each element is {priority_tuple, node}.
    # We use a custom comparator that compares the 4-element priority tuples
    # lexicographically (Elixir's default < operator works on tuples).
    comparator = fn {pa, _na}, {pb, _nb} ->
      cond do
        pa == pb -> 0
        pa < pb -> -1
        true -> 1
      end
    end

    # Push all leaf nodes onto the heap
    heap =
      Enum.reduce(weights, MinHeap.new(comparator), fn {sym, freq}, acc ->
        leaf = {:leaf, sym, freq}
        MinHeap.push(acc, {node_priority(leaf), leaf})
      end)

    # Greedy merge loop: always combine the two lightest nodes
    {root, _order} = merge_loop(heap, 0)
    {root, length(weights)}
  end

  # Repeatedly pop two lightest nodes, merge them into an internal node,
  # push the result back. Stop when only one node remains.
  defp merge_loop(heap, order_counter) do
    if MinHeap.size(heap) == 1 do
      {_priority, root} = MinHeap.peek(heap)
      {root, order_counter}
    else
      # Pop the two lightest nodes
      {{_p1, left}, heap1} = MinHeap.pop(heap)
      {{_p2, right}, heap2} = MinHeap.pop(heap1)

      # Combine: new weight = sum of children's weights
      combined_weight = node_weight(left) + node_weight(right)
      internal = {:internal, combined_weight, left, right, order_counter}

      # Push the new internal node back
      heap3 = MinHeap.push(heap2, {node_priority(internal), internal})
      merge_loop(heap3, order_counter + 1)
    end
  end

  # ─── code_table/1 ──────────────────────────────────────────────────────────

  @doc """
  Return `%{symbol => bit_string}` for all symbols in the tree.

  Left edges are `"0"`, right edges are `"1"`. For a single-symbol tree the
  convention is `%{symbol => "0"}` (one bit per occurrence).

  Time: O(n) where n = number of distinct symbols.

  ## Examples

      iex> tree = CodingAdventures.HuffmanTree.build([{65, 3}, {66, 2}, {67, 1}])
      iex> CodingAdventures.HuffmanTree.code_table(tree)
      %{65 => "0", 66 => "11", 67 => "10"}
  """
  def code_table({root, _count}) do
    walk(root, "", %{})
  end

  # Recursively walk the tree, accumulating the bit prefix
  defp walk({:leaf, symbol, _weight}, prefix, table) do
    # Single-leaf edge case: no edges traversed; use "0" by convention
    code = if prefix == "", do: "0", else: prefix
    Map.put(table, symbol, code)
  end

  defp walk({:internal, _w, left, right, _o}, prefix, table) do
    table = walk(left, prefix <> "0", table)
    walk(right, prefix <> "1", table)
  end

  # ─── code_for/2 ────────────────────────────────────────────────────────────

  @doc """
  Return the bit string for a specific symbol, or `nil` if not in the tree.

  Walks the tree searching for the leaf with `symbol`; does NOT build the
  full code table.

  Time: O(n) worst case (full tree traversal).

  ## Examples

      iex> tree = CodingAdventures.HuffmanTree.build([{65, 3}, {66, 2}, {67, 1}])
      iex> CodingAdventures.HuffmanTree.code_for(tree, 65)
      "0"
      iex> CodingAdventures.HuffmanTree.code_for(tree, 99)
      nil
  """
  def code_for({root, _count}, symbol) do
    find_code(root, symbol, "")
  end

  defp find_code({:leaf, sym, _w}, symbol, prefix) do
    if sym == symbol do
      if prefix == "", do: "0", else: prefix
    else
      nil
    end
  end

  defp find_code({:internal, _w, left, right, _o}, symbol, prefix) do
    case find_code(left, symbol, prefix <> "0") do
      nil -> find_code(right, symbol, prefix <> "1")
      result -> result
    end
  end

  # ─── canonical_code_table/1 ────────────────────────────────────────────────

  @doc """
  Return canonical Huffman codes (DEFLATE-style).

  Sorted by `{code_length, symbol_value}`; codes assigned numerically.
  Useful when you need to transmit only code lengths, not the tree.

  The canonical code algorithm:
    1. Collect `{symbol, code_length}` pairs from the tree.
    2. Sort by `{code_length, symbol_value}`.
    3. Assign codes numerically:
         code[0] = 0 (left-padded to length[0] bits)
         code[i] = (code[i-1] + 1) << (length[i] - length[i-1])

  Time: O(n log n).

  ## Examples

      iex> tree = CodingAdventures.HuffmanTree.build([{65, 3}, {66, 2}, {67, 1}])
      iex> CodingAdventures.HuffmanTree.canonical_code_table(tree)
      %{65 => "0", 66 => "10", 67 => "11"}
  """
  def canonical_code_table({root, _count}) do
    # Step 1: collect code lengths for all leaves
    lengths = collect_lengths(root, 0, %{})

    # Single-leaf edge case: assign length 1 by convention
    if map_size(lengths) == 1 do
      [{sym, _len}] = Map.to_list(lengths)
      %{sym => "0"}
    else
      # Step 2: sort by {length, symbol}
      sorted = Enum.sort_by(Map.to_list(lengths), fn {sym, len} -> {len, sym} end)

      # Step 3: assign canonical codes numerically
      [{_first_sym, first_len} | _rest] = sorted
      {result, _code_val, _prev_len} =
        Enum.reduce(sorted, {%{}, 0, first_len}, fn {sym, len}, {acc, code_val, prev_len} ->
          # Shift left if the length increased
          shifted_code =
            if len > prev_len do
              code_val <<< (len - prev_len)
            else
              code_val
            end

          # Format as zero-padded binary string of the correct length
          bit_str = Integer.to_string(shifted_code, 2) |> String.pad_leading(len, "0")
          {Map.put(acc, sym, bit_str), shifted_code + 1, len}
        end)

      result
    end
  end

  # Recursively collect code lengths. At each leaf, d = depth = code length.
  # Single-leaf root has depth 0, but we treat it as length 1 by convention.
  defp collect_lengths({:leaf, symbol, _w}, depth, lengths) do
    length = if depth > 0, do: depth, else: 1
    Map.put(lengths, symbol, length)
  end

  defp collect_lengths({:internal, _w, left, right, _o}, depth, lengths) do
    lengths = collect_lengths(left, depth + 1, lengths)
    collect_lengths(right, depth + 1, lengths)
  end

  # ─── decode_all/3 ──────────────────────────────────────────────────────────

  @doc """
  Decode exactly `count` symbols from a bit string by walking the tree.

  ## Arguments

    - `tree` — a Huffman tree as returned by `build/1`.
    - `bits` — a string of `"0"` and `"1"` characters.
    - `count` — the exact number of symbols to decode.

  ## Returns

  A list of decoded symbols of length == `count`.

  ## Raises

    - `ArgumentError` if the bit stream is exhausted before `count` symbols
      are decoded.

  For a single-leaf tree, each `"0"` bit decodes to that symbol.

  Multi-leaf trees: after the path from root reaches a leaf, the bit index is
  already past the last consumed bit — no extra advance needed.

  Time: O(total bits consumed).

  ## Examples

      iex> tree = CodingAdventures.HuffmanTree.build([{65, 3}, {66, 2}, {67, 1}])
      iex> CodingAdventures.HuffmanTree.decode_all(tree, "010011", 4)
      [65, 65, 66, 67]
  """
  def decode_all({root, _count}, bits, count) do
    # Single-leaf trees encode each symbol as a single '0' bit.
    # Multi-leaf trees: reaching a leaf means the bit index is already past
    # the last consumed bit — no extra advance needed.
    single_leaf = match?({:leaf, _, _}, root)
    decode_loop(root, bits, count, root, 0, [], single_leaf)
  end

  # decode_loop: walk the tree consuming bits, collecting symbols.
  #
  # Arguments:
  #   node         — current position in the tree
  #   bits         — the full bit string
  #   count        — how many more symbols to collect
  #   root         — kept to reset after each symbol
  #   i            — current bit index
  #   result       — accumulated symbols (reversed for efficiency)
  #   single_leaf  — true if the tree has exactly one leaf
  defp decode_loop(_node, _bits, 0, _root, _i, result, _single_leaf) do
    Enum.reverse(result)
  end

  defp decode_loop({:leaf, sym, _w}, bits, remaining, root, i, result, single_leaf) do
    # We have landed on a leaf — emit the symbol and restart from root.
    new_result = [sym | result]

    if single_leaf do
      # Single-leaf tree: consume the '0' bit for this symbol
      new_i = if i < byte_size(bits), do: i + 1, else: i
      decode_loop(root, bits, remaining - 1, root, new_i, new_result, single_leaf)
    else
      # Multi-leaf: bit index already advanced past the consumed bit
      decode_loop(root, bits, remaining - 1, root, i, new_result, single_leaf)
    end
  end

  defp decode_loop({:internal, _w, left, right, _o}, bits, remaining, root, i, result, single_leaf) do
    # Internal node: consume the next bit to decide left or right
    if i >= byte_size(bits) do
      raise ArgumentError,
            "Bit stream exhausted after #{length(result)} symbols; expected #{length(result) + remaining}"
    end

    bit = String.at(bits, i)
    next_node = if bit == "0", do: left, else: right
    decode_loop(next_node, bits, remaining, root, i + 1, result, single_leaf)
  end

  # ─── weight/1 ──────────────────────────────────────────────────────────────

  @doc """
  Total weight of the tree = sum of all leaf frequencies = root weight.

  O(1) — stored at the root.

  ## Examples

      iex> tree = CodingAdventures.HuffmanTree.build([{65, 3}, {66, 2}, {67, 1}])
      iex> CodingAdventures.HuffmanTree.weight(tree)
      6
  """
  def weight({root, _count}), do: node_weight(root)

  # ─── depth/1 ───────────────────────────────────────────────────────────────

  @doc """
  Maximum code length = depth of the deepest leaf.

  O(n) — must traverse the tree.

  ## Examples

      iex> tree = CodingAdventures.HuffmanTree.build([{65, 3}, {66, 2}, {67, 1}])
      iex> CodingAdventures.HuffmanTree.depth(tree)
      2
  """
  def depth({root, _count}), do: max_depth(root, 0)

  defp max_depth({:leaf, _sym, _w}, d), do: d

  defp max_depth({:internal, _w, left, right, _o}, d) do
    max(max_depth(left, d + 1), max_depth(right, d + 1))
  end

  # ─── symbol_count/1 ────────────────────────────────────────────────────────

  @doc """
  Number of distinct symbols (= number of leaf nodes).

  O(1) — stored at construction time.

  ## Examples

      iex> tree = CodingAdventures.HuffmanTree.build([{65, 3}, {66, 2}, {67, 1}])
      iex> CodingAdventures.HuffmanTree.symbol_count(tree)
      3
  """
  def symbol_count({_root, count}), do: count

  # ─── leaves/1 ──────────────────────────────────────────────────────────────

  @doc """
  In-order traversal of leaves.

  Returns `[{symbol, code}, ...]`, left subtree before right subtree.
  Useful for visualisation and debugging.

  Time: O(n).

  ## Examples

      iex> tree = CodingAdventures.HuffmanTree.build([{65, 3}, {66, 2}, {67, 1}])
      iex> CodingAdventures.HuffmanTree.leaves(tree)
      [{65, "0"}, {67, "10"}, {66, "11"}]
  """
  def leaves({root, _count} = tree) do
    table = code_table(tree)
    in_order_leaves(root, table, [])
  end

  defp in_order_leaves({:leaf, sym, _w}, table, acc) do
    acc ++ [{sym, Map.fetch!(table, sym)}]
  end

  defp in_order_leaves({:internal, _w, left, right, _o}, table, acc) do
    acc
    |> then(&in_order_leaves(left, table, &1))
    |> then(&in_order_leaves(right, table, &1))
  end

  # ─── is_valid/1 ────────────────────────────────────────────────────────────

  @doc """
  Check structural invariants. For testing only.

    1. Every internal node has exactly 2 children (full binary tree).
    2. `weight(internal) == weight(left) + weight(right)`.
    3. No symbol appears in more than one leaf.

  Returns `true` if all invariants hold.

  ## Examples

      iex> tree = CodingAdventures.HuffmanTree.build([{65, 3}, {66, 2}, {67, 1}])
      iex> CodingAdventures.HuffmanTree.is_valid(tree)
      true
  """
  def is_valid({root, _count}) do
    {valid, _seen} = check_invariants(root, MapSet.new())
    valid
  end

  defp check_invariants({:leaf, sym, _w}, seen) do
    if MapSet.member?(seen, sym) do
      {false, seen}
    else
      {true, MapSet.put(seen, sym)}
    end
  end

  defp check_invariants({:internal, weight, left, right, _o}, seen) do
    # Check weight invariant: internal.weight == left.weight + right.weight
    if weight != node_weight(left) + node_weight(right) do
      {false, seen}
    else
      {valid_left, seen2} = check_invariants(left, seen)

      if valid_left do
        check_invariants(right, seen2)
      else
        {false, seen2}
      end
    end
  end
end
