defmodule CodingAdventures.HuffmanTreeTest do
  use ExUnit.Case, async: true
  alias CodingAdventures.HuffmanTree

  # Convenience: build the canonical AAABBC tree used throughout the spec.
  # A=3, B=2, C=1 → tree depth 2, A gets code "0", B gets "10", C gets "11".
  defp abc_tree, do: HuffmanTree.build([{65, 3}, {66, 2}, {67, 1}])

  # ─── build/1 — happy path ──────────────────────────────────────────────────

  describe "build/1 — basic construction" do
    test "builds a tree from AAABBC (canonical spec example)" do
      tree = abc_tree()
      assert HuffmanTree.symbol_count(tree) == 3
      assert HuffmanTree.weight(tree) == 6
    end

    test "builds a single-symbol tree" do
      tree = HuffmanTree.build([{65, 5}])
      assert HuffmanTree.symbol_count(tree) == 1
      assert HuffmanTree.weight(tree) == 5
    end

    test "builds a two-symbol tree" do
      tree = HuffmanTree.build([{65, 1}, {66, 1}])
      assert HuffmanTree.symbol_count(tree) == 2
      assert HuffmanTree.weight(tree) == 2
    end

    test "builds a five-symbol tree" do
      tree = HuffmanTree.build([{65, 10}, {66, 4}, {67, 2}, {68, 2}, {69, 1}])
      assert HuffmanTree.symbol_count(tree) == 5
      assert HuffmanTree.weight(tree) == 19
    end

    test "order of input does not affect weight" do
      t1 = HuffmanTree.build([{65, 3}, {66, 2}, {67, 1}])
      t2 = HuffmanTree.build([{67, 1}, {65, 3}, {66, 2}])
      assert HuffmanTree.weight(t1) == HuffmanTree.weight(t2)
    end

    test "is_valid returns true for a freshly built tree" do
      assert HuffmanTree.is_valid(abc_tree())
    end
  end

  # ─── build/1 — error cases ─────────────────────────────────────────────────

  describe "build/1 — error handling" do
    test "raises ArgumentError for empty weights list" do
      assert_raise ArgumentError, ~r/empty/, fn ->
        HuffmanTree.build([])
      end
    end

    test "raises ArgumentError for zero frequency" do
      assert_raise ArgumentError, ~r/positive/, fn ->
        HuffmanTree.build([{65, 0}])
      end
    end

    test "raises ArgumentError for negative frequency" do
      assert_raise ArgumentError, ~r/positive/, fn ->
        HuffmanTree.build([{65, -1}])
      end
    end

    test "error message includes the offending symbol" do
      assert_raise ArgumentError, ~r/symbol=66/, fn ->
        HuffmanTree.build([{65, 1}, {66, 0}])
      end
    end
  end

  # ─── code_table/1 ──────────────────────────────────────────────────────────

  describe "code_table/1" do
    test "AAABBC: A=0, C=10, B=11 (C is lighter so gets left/shorter subtree path)" do
      # C(1) pops first (lightest), becomes left child of internal → C="10"
      # B(2) pops second, becomes right child of internal → B="11"
      # A(3) pops next (leaf beats internal at equal weight 3) → A="0"
      assert HuffmanTree.code_table(abc_tree()) == %{65 => "0", 66 => "11", 67 => "10"}
    end

    test "single symbol gets code '0'" do
      tree = HuffmanTree.build([{65, 5}])
      assert HuffmanTree.code_table(tree) == %{65 => "0"}
    end

    test "two equal-weight symbols: both get 1-bit codes" do
      tree = HuffmanTree.build([{65, 1}, {66, 1}])
      table = HuffmanTree.code_table(tree)
      # Both symbols should have exactly 1-bit codes
      assert String.length(Map.fetch!(table, 65)) == 1
      assert String.length(Map.fetch!(table, 66)) == 1
      # The codes must be distinct
      refute Map.fetch!(table, 65) == Map.fetch!(table, 66)
    end

    test "all symbols have distinct codes" do
      tree = HuffmanTree.build([{65, 10}, {66, 4}, {67, 2}, {68, 2}, {69, 1}])
      table = HuffmanTree.code_table(tree)
      codes = Map.values(table)
      assert length(codes) == length(Enum.uniq(codes))
    end

    test "codes are prefix-free: no code is a prefix of another" do
      tree = HuffmanTree.build([{65, 10}, {66, 4}, {67, 2}, {68, 2}, {69, 1}])
      codes = Map.values(HuffmanTree.code_table(tree))

      for c1 <- codes, c2 <- codes, c1 != c2 do
        refute String.starts_with?(c2, c1),
               "#{inspect(c1)} is a prefix of #{inspect(c2)}"
      end
    end

    test "heavier symbol gets shorter or equal code than lighter symbol" do
      tree = abc_tree()
      table = HuffmanTree.code_table(tree)
      # A (weight 3) should have shorter or equal code than B (weight 2) and C (weight 1)
      assert String.length(Map.fetch!(table, 65)) <= String.length(Map.fetch!(table, 66))
      assert String.length(Map.fetch!(table, 65)) <= String.length(Map.fetch!(table, 67))
    end
  end

  # ─── code_for/2 ────────────────────────────────────────────────────────────

  describe "code_for/2" do
    test "returns correct code for a known symbol" do
      tree = abc_tree()
      assert HuffmanTree.code_for(tree, 65) == "0"
      assert HuffmanTree.code_for(tree, 66) == "11"
      assert HuffmanTree.code_for(tree, 67) == "10"
    end

    test "returns nil for a symbol not in the tree" do
      tree = abc_tree()
      assert HuffmanTree.code_for(tree, 99) == nil
    end

    test "single-symbol tree: code_for returns '0'" do
      tree = HuffmanTree.build([{42, 7}])
      assert HuffmanTree.code_for(tree, 42) == "0"
    end

    test "code_for matches code_table for all symbols" do
      tree = HuffmanTree.build([{65, 10}, {66, 4}, {67, 2}, {68, 2}, {69, 1}])
      table = HuffmanTree.code_table(tree)

      Enum.each(table, fn {sym, code} ->
        assert HuffmanTree.code_for(tree, sym) == code
      end)
    end
  end

  # ─── canonical_code_table/1 ────────────────────────────────────────────────

  describe "canonical_code_table/1" do
    test "AAABBC: A=0, B=10, C=11 (canonical sorts by length then symbol, B<C so B=10, C=11)" do
      # code_table gives A=0(len1), C=10(len2), B=11(len2)
      # canonical sorts by (length, symbol): A(1,65), B(2,66), C(2,67)
      # assigns: A→0, B→10, C→11
      tree = abc_tree()
      canonical = HuffmanTree.canonical_code_table(tree)
      assert canonical == %{65 => "0", 66 => "10", 67 => "11"}
    end

    test "single symbol gets canonical code '0'" do
      tree = HuffmanTree.build([{99, 3}])
      assert HuffmanTree.canonical_code_table(tree) == %{99 => "0"}
    end

    test "canonical codes are sorted by (length, symbol)" do
      tree = HuffmanTree.build([{65, 10}, {66, 4}, {67, 2}, {68, 2}, {69, 1}])
      canonical = HuffmanTree.canonical_code_table(tree)
      pairs = Enum.sort_by(Map.to_list(canonical), fn {sym, code} -> {String.length(code), sym} end)
      # The first code should be all zeros (leftmost code at that bit-length)
      {_sym0, code0} = List.first(pairs)
      assert code0 == String.duplicate("0", String.length(code0))
    end

    test "canonical codes at same length are consecutive integers" do
      tree = HuffmanTree.build([{65, 3}, {66, 2}, {67, 1}])
      canonical = HuffmanTree.canonical_code_table(tree)
      # Canonical sorts by (length, symbol): B(2,66) then C(2,67)
      # B→10, C→11 (consecutive integers at length 2)
      assert canonical[66] == "10"
      assert canonical[67] == "11"
    end

    test "canonical code lengths match regular code lengths" do
      tree = HuffmanTree.build([{65, 10}, {66, 4}, {67, 2}, {68, 2}, {69, 1}])
      table = HuffmanTree.code_table(tree)
      canonical = HuffmanTree.canonical_code_table(tree)

      # The code *lengths* should be identical, even if codes differ
      Enum.each(table, fn {sym, code} ->
        assert String.length(Map.fetch!(canonical, sym)) == String.length(code),
               "Symbol #{sym}: code length mismatch between table and canonical"
      end)
    end

    test "canonical codes are prefix-free" do
      tree = HuffmanTree.build([{65, 10}, {66, 4}, {67, 2}, {68, 2}, {69, 1}])
      codes = Map.values(HuffmanTree.canonical_code_table(tree))

      for c1 <- codes, c2 <- codes, c1 != c2 do
        refute String.starts_with?(c2, c1)
      end
    end
  end

  # ─── decode_all/3 ──────────────────────────────────────────────────────────

  describe "decode_all/3" do
    test "decodes 4 symbols from the spec example" do
      tree = abc_tree()
      # Actual codes from tree: A=0, C=10, B=11
      # "010011" = 0 + 10 + 0 + 11 = A, C, A, B
      assert HuffmanTree.decode_all(tree, "010011", 4) == [65, 67, 65, 66]
    end

    test "decodes a single symbol" do
      tree = abc_tree()
      assert HuffmanTree.decode_all(tree, "0", 1) == [65]
    end

    test "single-leaf tree: each '0' decodes to the symbol" do
      tree = HuffmanTree.build([{65, 5}])
      assert HuffmanTree.decode_all(tree, "000", 3) == [65, 65, 65]
    end

    test "single-leaf tree: decode one symbol from one '0' bit" do
      tree = HuffmanTree.build([{99, 1}])
      assert HuffmanTree.decode_all(tree, "0", 1) == [99]
    end

    test "decodes 0 symbols returns empty list" do
      tree = abc_tree()
      assert HuffmanTree.decode_all(tree, "010011", 0) == []
    end

    test "encode then decode round-trip: ABCBA" do
      tree = abc_tree()
      table = HuffmanTree.code_table(tree)
      message = [65, 66, 67, 66, 65]
      bits = Enum.map_join(message, "", &Map.fetch!(table, &1))
      assert HuffmanTree.decode_all(tree, bits, 5) == message
    end

    test "encode then decode round-trip: all A" do
      tree = abc_tree()
      table = HuffmanTree.code_table(tree)
      message = List.duplicate(65, 10)
      bits = Enum.map_join(message, "", &Map.fetch!(table, &1))
      assert HuffmanTree.decode_all(tree, bits, 10) == message
    end

    test "decode raises when bit stream exhausted" do
      tree = abc_tree()
      # "0" decodes A, then we need more bits but stream is empty
      assert_raise ArgumentError, ~r/exhausted/, fn ->
        HuffmanTree.decode_all(tree, "0", 2)
      end
    end

    test "decode with two-symbol tree" do
      tree = HuffmanTree.build([{65, 3}, {66, 1}])
      table = HuffmanTree.code_table(tree)
      message = [65, 66, 65, 65]
      bits = Enum.map_join(message, "", &Map.fetch!(table, &1))
      assert HuffmanTree.decode_all(tree, bits, 4) == message
    end

    test "decode five-symbol tree round-trip" do
      symbols = [{65, 10}, {66, 4}, {67, 2}, {68, 2}, {69, 1}]
      tree = HuffmanTree.build(symbols)
      table = HuffmanTree.code_table(tree)
      message = [65, 66, 65, 67, 69, 68, 65]
      bits = Enum.map_join(message, "", &Map.fetch!(table, &1))
      assert HuffmanTree.decode_all(tree, bits, 7) == message
    end
  end

  # ─── weight/1 ──────────────────────────────────────────────────────────────

  describe "weight/1" do
    test "total weight equals sum of all frequencies (AAABBC)" do
      assert HuffmanTree.weight(abc_tree()) == 6
    end

    test "single-symbol tree weight" do
      assert HuffmanTree.weight(HuffmanTree.build([{65, 42}])) == 42
    end

    test "weight equals sum of all input frequencies" do
      weights = [{65, 10}, {66, 4}, {67, 2}, {68, 2}, {69, 1}]
      tree = HuffmanTree.build(weights)
      expected = Enum.sum(Enum.map(weights, fn {_, f} -> f end))
      assert HuffmanTree.weight(tree) == expected
    end
  end

  # ─── depth/1 ───────────────────────────────────────────────────────────────

  describe "depth/1" do
    test "AAABBC tree depth is 2" do
      assert HuffmanTree.depth(abc_tree()) == 2
    end

    test "single-symbol tree depth is 0 (root is a leaf)" do
      assert HuffmanTree.depth(HuffmanTree.build([{65, 5}])) == 0
    end

    test "two-symbol tree depth is 1" do
      assert HuffmanTree.depth(HuffmanTree.build([{65, 1}, {66, 1}])) == 1
    end

    test "depth equals max code length from code_table" do
      tree = HuffmanTree.build([{65, 10}, {66, 4}, {67, 2}, {68, 2}, {69, 1}])
      table = HuffmanTree.code_table(tree)
      max_code_len = table |> Map.values() |> Enum.map(&String.length/1) |> Enum.max()
      assert HuffmanTree.depth(tree) == max_code_len
    end
  end

  # ─── symbol_count/1 ────────────────────────────────────────────────────────

  describe "symbol_count/1" do
    test "symbol count for AAABBC is 3" do
      assert HuffmanTree.symbol_count(abc_tree()) == 3
    end

    test "single-symbol count" do
      assert HuffmanTree.symbol_count(HuffmanTree.build([{65, 1}])) == 1
    end

    test "count matches number of distinct input symbols" do
      tree = HuffmanTree.build([{65, 10}, {66, 4}, {67, 2}, {68, 2}, {69, 1}])
      assert HuffmanTree.symbol_count(tree) == 5
    end
  end

  # ─── leaves/1 ──────────────────────────────────────────────────────────────

  describe "leaves/1" do
    test "AAABBC leaves in left-to-right order" do
      result = HuffmanTree.leaves(abc_tree())
      # A="0" is left child of root; then C="10" (left of right subtree), B="11" (right of right subtree)
      assert result == [{65, "0"}, {67, "10"}, {66, "11"}]
    end

    test "single-leaf tree" do
      result = HuffmanTree.leaves(HuffmanTree.build([{99, 3}]))
      assert result == [{99, "0"}]
    end

    test "leaves count matches symbol_count" do
      tree = HuffmanTree.build([{65, 10}, {66, 4}, {67, 2}, {68, 2}, {69, 1}])
      assert length(HuffmanTree.leaves(tree)) == HuffmanTree.symbol_count(tree)
    end

    test "leaves symbols match code_table keys" do
      tree = HuffmanTree.build([{65, 10}, {66, 4}, {67, 2}, {68, 2}, {69, 1}])
      leaves = HuffmanTree.leaves(tree)
      table = HuffmanTree.code_table(tree)
      leaf_syms = Enum.map(leaves, fn {sym, _} -> sym end)
      assert Enum.sort(leaf_syms) == Enum.sort(Map.keys(table))
    end

    test "leaves codes match code_table values" do
      tree = HuffmanTree.build([{65, 10}, {66, 4}, {67, 2}, {68, 2}, {69, 1}])
      leaves = HuffmanTree.leaves(tree)
      table = HuffmanTree.code_table(tree)
      Enum.each(leaves, fn {sym, code} ->
        assert Map.fetch!(table, sym) == code
      end)
    end
  end

  # ─── is_valid/1 ────────────────────────────────────────────────────────────

  describe "is_valid/1" do
    test "freshly built tree is valid" do
      assert HuffmanTree.is_valid(abc_tree())
    end

    test "single-symbol tree is valid" do
      assert HuffmanTree.is_valid(HuffmanTree.build([{65, 1}]))
    end

    test "five-symbol tree is valid" do
      tree = HuffmanTree.build([{65, 10}, {66, 4}, {67, 2}, {68, 2}, {69, 1}])
      assert HuffmanTree.is_valid(tree)
    end

    test "weight invariant holds: root weight == sum of leaf weights" do
      tree = HuffmanTree.build([{65, 10}, {66, 4}, {67, 2}, {68, 2}, {69, 1}])
      assert HuffmanTree.is_valid(tree)
      assert HuffmanTree.weight(tree) == 19
    end
  end

  # ─── Tie-breaking rules ────────────────────────────────────────────────────

  describe "tie-breaking rules" do
    test "leaf-before-internal: equal-weight leaf is preferred over internal" do
      # A=1, B=1, C=2. After merging A+B → internal(2), we have internal(2) and C(2).
      # With leaf-before-internal, C should merge next (as left child).
      tree = HuffmanTree.build([{65, 1}, {66, 1}, {67, 2}])
      table = HuffmanTree.code_table(tree)
      # C should have the shortest code since it ties with the A+B internal at weight 2
      # and leaves have priority over internals
      assert String.length(Map.fetch!(table, 67)) <= String.length(Map.fetch!(table, 65))
      assert String.length(Map.fetch!(table, 67)) <= String.length(Map.fetch!(table, 66))
    end

    test "lower symbol value wins among equal-weight leaves" do
      # Two leaves with same weight: lower symbol gets shorter or equal code
      tree = HuffmanTree.build([{65, 5}, {66, 5}])
      table = HuffmanTree.code_table(tree)
      # Both have weight 5 — deterministic: lower symbol (65) is popped first → left child
      assert Map.fetch!(table, 65) == "0"
      assert Map.fetch!(table, 66) == "1"
    end

    test "earlier internal node wins among equal-weight internals (FIFO)" do
      # Build a 4-symbol tree where internals can tie.
      # All weight 1: A, B, C, D
      tree = HuffmanTree.build([{65, 1}, {66, 1}, {67, 1}, {68, 1}])
      table = HuffmanTree.code_table(tree)
      # All codes should be length 2 (balanced)
      Enum.each(Map.values(table), fn code ->
        assert String.length(code) == 2
      end)
    end

    test "deterministic: same input always produces same code table" do
      weights = [{65, 3}, {66, 2}, {67, 1}]
      t1 = HuffmanTree.build(weights)
      t2 = HuffmanTree.build(weights)
      assert HuffmanTree.code_table(t1) == HuffmanTree.code_table(t2)
    end

    test "deterministic: shuffled input produces same code table" do
      weights = [{65, 3}, {66, 2}, {67, 1}]
      shuffled = [{67, 1}, {65, 3}, {66, 2}]
      t1 = HuffmanTree.build(weights)
      t2 = HuffmanTree.build(shuffled)
      assert HuffmanTree.code_table(t1) == HuffmanTree.code_table(t2)
    end
  end

  # ─── Edge cases and integration ────────────────────────────────────────────

  describe "edge cases" do
    test "single symbol: weight=1" do
      tree = HuffmanTree.build([{0, 1}])
      assert HuffmanTree.symbol_count(tree) == 1
      assert HuffmanTree.weight(tree) == 1
      assert HuffmanTree.code_table(tree) == %{0 => "0"}
      assert HuffmanTree.decode_all(tree, "00", 2) == [0, 0]
    end

    test "large symbol values are handled" do
      tree = HuffmanTree.build([{1000, 3}, {2000, 1}])
      table = HuffmanTree.code_table(tree)
      assert Map.has_key?(table, 1000)
      assert Map.has_key?(table, 2000)
    end

    test "256 byte-alphabet symbols" do
      weights = Enum.map(0..255, fn b -> {b, b + 1} end)
      tree = HuffmanTree.build(weights)
      assert HuffmanTree.symbol_count(tree) == 256
      assert HuffmanTree.is_valid(tree)
    end

    test "all-equal-weight symbols produce balanced tree" do
      weights = Enum.map(0..7, fn b -> {b, 1} end)
      tree = HuffmanTree.build(weights)
      assert HuffmanTree.symbol_count(tree) == 8
      # With 8 equal-weight symbols, all codes should be length 3
      table = HuffmanTree.code_table(tree)
      Enum.each(Map.values(table), fn code ->
        assert String.length(code) == 3
      end)
    end

    test "decode_all with two-symbol balanced tree" do
      tree = HuffmanTree.build([{0, 1}, {1, 1}])
      table = HuffmanTree.code_table(tree)
      message = [0, 1, 0, 0, 1]
      bits = Enum.map_join(message, "", &Map.fetch!(table, &1))
      assert HuffmanTree.decode_all(tree, bits, 5) == message
    end

    test "canonical code table is consistent with round-trip decode" do
      # Canonical codes must still be prefix-free and decodable.
      # We cannot use canonical codes directly for decoding (they aren't stored in
      # the tree structure), but we verify they have the correct lengths.
      tree = HuffmanTree.build([{65, 3}, {66, 2}, {67, 1}])
      canonical = HuffmanTree.canonical_code_table(tree)
      table = HuffmanTree.code_table(tree)
      # Lengths must match (even if individual codes differ)
      Enum.each(table, fn {sym, code} ->
        assert String.length(Map.fetch!(canonical, sym)) == String.length(code)
      end)
    end
  end

  # ─── Spec example: AAABBC end-to-end ──────────────────────────────────────

  describe "spec example: AAABBC" do
    test "tree weight = 6 (3+2+1)" do
      assert HuffmanTree.weight(abc_tree()) == 6
    end

    test "A gets code '0' (most frequent → shortest)" do
      table = HuffmanTree.code_table(abc_tree())
      assert Map.fetch!(table, 65) == "0"
    end

    test "B gets code '11' (B is heavier than C, so B is right child of the internal node)" do
      table = HuffmanTree.code_table(abc_tree())
      assert Map.fetch!(table, 66) == "11"
    end

    test "C gets code '10' (C is lighter so pops first, becomes left child → shorter path)" do
      table = HuffmanTree.code_table(abc_tree())
      assert Map.fetch!(table, 67) == "10"
    end

    test "canonical codes match: A=0, B=10, C=11" do
      canonical = HuffmanTree.canonical_code_table(abc_tree())
      assert canonical == %{65 => "0", 66 => "10", 67 => "11"}
    end

    test "encode and decode round-trip for AABC" do
      tree = abc_tree()
      # A=0, B=11, C=10. Encode AABC: "0" + "0" + "11" + "10" = "001110"
      message = [65, 65, 66, 67]
      table = HuffmanTree.code_table(tree)
      bits = Enum.map_join(message, "", &Map.fetch!(table, &1))
      assert HuffmanTree.decode_all(tree, bits, 4) == message
    end

    test "depth is 2" do
      assert HuffmanTree.depth(abc_tree()) == 2
    end

    test "leaves in order: A, C, B (left-to-right tree traversal)" do
      # Root: left=A(0), right=internal(left=C, right=B)
      # In-order: A="0", C="10", B="11"
      assert HuffmanTree.leaves(abc_tree()) == [{65, "0"}, {67, "10"}, {66, "11"}]
    end
  end
end
