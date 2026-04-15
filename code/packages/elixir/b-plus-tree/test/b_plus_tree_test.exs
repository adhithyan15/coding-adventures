defmodule CodingAdventures.BPlusTreeTest do
  @moduledoc """
  Comprehensive tests for the B+ tree implementation.

  We test:
  1. Construction
  2. Insert — sequential, reverse, random, upsert
  3. Search — leaf-always, missing, separator keys
  4. Member?
  5. Delete — borrow left/right, merge, sequential, random
  6. Range scan — correct results, empty results
  7. Full scan — all keys via leaf traversal
  8. to_list
  9. Min/max key
  10. Size, empty?, height
  11. valid? after every operation (B+ invariants)
  12. Stress tests with 1000+ keys
  13. t=2, t=3, t=5
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.BPlusTree

  # =========================================================================
  # 1. Construction
  # =========================================================================

  describe "new/0 and new/1" do
    test "creates empty tree with default t=2" do
      tree = BPlusTree.new()
      assert BPlusTree.empty?(tree)
      assert BPlusTree.size(tree) == 0
      assert BPlusTree.valid?(tree)
    end

    test "creates empty tree with custom t" do
      tree = BPlusTree.new(5)
      assert BPlusTree.empty?(tree)
    end

    test "height of empty tree is 0" do
      assert BPlusTree.height(BPlusTree.new()) == 0
    end
  end

  # =========================================================================
  # 2. Insert
  # =========================================================================

  describe "insert/3" do
    test "insert single key" do
      tree = BPlusTree.new() |> BPlusTree.insert(42, "forty-two")
      assert BPlusTree.size(tree) == 1
      assert BPlusTree.search(tree, 42) == {:ok, "forty-two"}
      assert BPlusTree.valid?(tree)
    end

    test "insert updates existing key" do
      tree =
        BPlusTree.new()
        |> BPlusTree.insert(10, "old")
        |> BPlusTree.insert(10, "new")

      assert BPlusTree.size(tree) == 1
      assert BPlusTree.search(tree, 10) == {:ok, "new"}
      assert BPlusTree.valid?(tree)
    end

    test "insert sequential keys t=2" do
      tree = Enum.reduce(1..20, BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k * 10) end)
      assert BPlusTree.size(tree) == 20
      assert BPlusTree.valid?(tree)
      Enum.each(1..20, fn k -> assert BPlusTree.search(tree, k) == {:ok, k * 10} end)
    end

    test "insert reverse keys t=2" do
      tree = Enum.reduce(20..1//-1, BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k) end)
      assert BPlusTree.size(tree) == 20
      assert BPlusTree.valid?(tree)
    end

    test "insert random keys t=3" do
      keys = Enum.shuffle(1..50)
      tree = Enum.reduce(keys, BPlusTree.new(3), fn k, t -> BPlusTree.insert(t, k, "v#{k}") end)
      assert BPlusTree.size(tree) == 50
      assert BPlusTree.valid?(tree)
      Enum.each(keys, fn k -> assert BPlusTree.search(tree, k) == {:ok, "v#{k}"} end)
    end

    test "insert 1000 keys t=5" do
      tree = Enum.reduce(1..1000, BPlusTree.new(5), fn k, t -> BPlusTree.insert(t, k, k) end)
      assert BPlusTree.size(tree) == 1000
      assert BPlusTree.valid?(tree)
      assert BPlusTree.height(tree) <= 4
    end

    test "insert causes root split t=2" do
      tree = Enum.reduce(1..3, BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k) end)
      assert BPlusTree.height(tree) == 0  # still leaf root
      tree = BPlusTree.insert(tree, 4, 4)
      assert BPlusTree.valid?(tree)
      assert BPlusTree.size(tree) == 4
    end

    test "full_scan after inserts is sorted" do
      keys = [5, 3, 7, 1, 9, 4, 6, 2, 8]
      tree = Enum.reduce(keys, BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k * 10) end)
      result = BPlusTree.full_scan(tree)
      assert Enum.map(result, &elem(&1, 0)) == Enum.sort(keys)
      assert Enum.map(result, &elem(&1, 1)) == Enum.sort(keys) |> Enum.map(&(&1 * 10))
    end

    test "valid? after each insert" do
      Enum.reduce(1..30, BPlusTree.new(), fn k, tree ->
        tree = BPlusTree.insert(tree, k, k)
        assert BPlusTree.valid?(tree), "Tree invalid after inserting #{k}"
        tree
      end)
    end
  end

  # =========================================================================
  # 3. Search
  # =========================================================================

  describe "search/2" do
    setup do
      tree =
        BPlusTree.new()
        |> BPlusTree.insert(10, "ten")
        |> BPlusTree.insert(20, "twenty")
        |> BPlusTree.insert(30, "thirty")

      {:ok, tree: tree}
    end

    test "returns {:ok, value} for existing key", %{tree: tree} do
      assert BPlusTree.search(tree, 10) == {:ok, "ten"}
      assert BPlusTree.search(tree, 30) == {:ok, "thirty"}
    end

    test "returns :error for missing key", %{tree: tree} do
      assert BPlusTree.search(tree, 99) == :error
    end

    test "search on empty tree returns :error" do
      assert BPlusTree.search(BPlusTree.new(), 1) == :error
    end

    test "can store nil values" do
      tree = BPlusTree.new() |> BPlusTree.insert(5, nil)
      assert BPlusTree.search(tree, 5) == {:ok, nil}
    end

    test "search always reaches leaf (B+ property)" do
      # Even keys that appear as separator keys in internal nodes
      # can be searched (they're also in leaves in B+ tree)
      tree = Enum.reduce(1..20, BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k * 100) end)
      # All keys should be findable regardless of internal structure
      Enum.each(1..20, fn k ->
        assert BPlusTree.search(tree, k) == {:ok, k * 100}
      end)
    end
  end

  # =========================================================================
  # 4. Member?
  # =========================================================================

  describe "member?/2" do
    test "returns true for present key" do
      tree = BPlusTree.new() |> BPlusTree.insert(5, :x)
      assert BPlusTree.member?(tree, 5)
    end

    test "returns false for absent key" do
      tree = BPlusTree.new() |> BPlusTree.insert(5, :x)
      refute BPlusTree.member?(tree, 6)
    end
  end

  # =========================================================================
  # 5. Delete
  # =========================================================================

  describe "delete/2" do
    test "delete existing key" do
      tree =
        Enum.reduce(1..10, BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k) end)
        |> BPlusTree.delete(5)

      assert BPlusTree.search(tree, 5) == :error
      assert BPlusTree.size(tree) == 9
      assert BPlusTree.valid?(tree)
    end

    test "delete missing key is noop" do
      tree = Enum.reduce(1..5, BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k) end)
      tree = BPlusTree.delete(tree, 99)
      assert BPlusTree.size(tree) == 5
      assert BPlusTree.valid?(tree)
    end

    test "delete from empty tree" do
      tree = BPlusTree.new() |> BPlusTree.delete(1)
      assert BPlusTree.empty?(tree)
    end

    test "delete all keys" do
      tree = Enum.reduce(1..15, BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k) end)
      keys = BPlusTree.full_scan(tree) |> Enum.map(&elem(&1, 0))
      tree = Enum.reduce(keys, tree, fn k, t -> BPlusTree.delete(t, k) end)
      assert BPlusTree.empty?(tree)
      assert BPlusTree.valid?(tree)
    end

    test "delete first key" do
      tree =
        Enum.reduce(1..10, BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k) end)
        |> BPlusTree.delete(1)

      assert BPlusTree.search(tree, 1) == :error
      assert BPlusTree.min_key(tree) == 2
      assert BPlusTree.valid?(tree)
    end

    test "delete last key" do
      tree =
        Enum.reduce(1..10, BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k) end)
        |> BPlusTree.delete(10)

      assert BPlusTree.search(tree, 10) == :error
      assert BPlusTree.max_key(tree) == 9
      assert BPlusTree.valid?(tree)
    end

    test "delete borrow from left sibling" do
      tree =
        Enum.reduce(1..10, BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k) end)

      tree = Enum.reduce(8..10, tree, fn k, t -> BPlusTree.delete(t, k) end)
      assert BPlusTree.valid?(tree)
      Enum.each(1..7, fn k -> assert BPlusTree.search(tree, k) == {:ok, k} end)
    end

    test "delete borrow from right sibling" do
      tree =
        Enum.reduce(1..10, BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k) end)

      tree = Enum.reduce(1..3, tree, fn k, t -> BPlusTree.delete(t, k) end)
      assert BPlusTree.valid?(tree)
      Enum.each(4..10, fn k -> assert BPlusTree.search(tree, k) == {:ok, k} end)
    end

    test "delete causes merge" do
      tree = Enum.reduce(1..7, BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k) end)
      tree = Enum.reduce(1..7, tree, fn k, t -> BPlusTree.delete(t, k) end)
      assert BPlusTree.empty?(tree)
      assert BPlusTree.valid?(tree)
    end

    test "valid? after every delete" do
      tree = Enum.reduce(1..30, BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k) end)

      1..30
      |> Enum.to_list()
      |> Enum.shuffle()
      |> Enum.reduce(tree, fn k, t ->
        t = BPlusTree.delete(t, k)
        assert BPlusTree.valid?(t), "Tree invalid after deleting #{k}"
        t
      end)
    end

    test "valid? after every delete t=3" do
      tree = Enum.reduce(1..50, BPlusTree.new(3), fn k, t -> BPlusTree.insert(t, k, k) end)

      1..50
      |> Enum.to_list()
      |> Enum.shuffle()
      |> Enum.reduce(tree, fn k, t ->
        t = BPlusTree.delete(t, k)
        assert BPlusTree.valid?(t), "Tree invalid after deleting #{k}"
        t
      end)
    end
  end

  # =========================================================================
  # 6. Range Scan
  # =========================================================================

  describe "range_scan/3" do
    setup do
      tree = Enum.reduce(1..30, BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k * 100) end)
      {:ok, tree: tree}
    end

    test "full range", %{tree: tree} do
      result = BPlusTree.range_scan(tree, 1, 30)
      assert Enum.map(result, &elem(&1, 0)) == Enum.to_list(1..30)
    end

    test "middle range", %{tree: tree} do
      result = BPlusTree.range_scan(tree, 10, 20)
      assert Enum.map(result, &elem(&1, 0)) == Enum.to_list(10..20)
      assert Enum.map(result, &elem(&1, 1)) == Enum.to_list(10..20) |> Enum.map(&(&1 * 100))
    end

    test "single element", %{tree: tree} do
      assert BPlusTree.range_scan(tree, 15, 15) == [{15, 1500}]
    end

    test "empty result", %{tree: tree} do
      assert BPlusTree.range_scan(tree, 100, 200) == []
    end

    test "range scan t=5 with 1000 keys" do
      tree = Enum.reduce(1..1000, BPlusTree.new(5), fn k, t -> BPlusTree.insert(t, k, k) end)
      result = BPlusTree.range_scan(tree, 500, 600)
      assert Enum.map(result, &elem(&1, 0)) == Enum.to_list(500..600)
      assert BPlusTree.valid?(tree)
    end
  end

  # =========================================================================
  # 7. Full Scan
  # =========================================================================

  describe "full_scan/1" do
    test "returns all keys sorted" do
      keys = Enum.shuffle(1..30)
      tree = Enum.reduce(keys, BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k) end)
      result = BPlusTree.full_scan(tree)
      assert Enum.map(result, &elem(&1, 0)) == Enum.sort(keys)
    end

    test "empty tree returns empty list" do
      assert BPlusTree.full_scan(BPlusTree.new()) == []
    end

    test "full_scan after deletes is correct" do
      tree = Enum.reduce(1..20, BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k) end)
      tree = Enum.reduce(1..20//2, tree, fn k, t -> BPlusTree.delete(t, k) end)
      expected = Enum.to_list(2..20//2)
      result = BPlusTree.full_scan(tree) |> Enum.map(&elem(&1, 0))
      assert result == expected
    end
  end

  # =========================================================================
  # 8. to_list
  # =========================================================================

  describe "to_list/1" do
    test "same as full_scan" do
      tree = Enum.reduce(1..10, BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k) end)
      assert BPlusTree.to_list(tree) == BPlusTree.full_scan(tree)
    end
  end

  # =========================================================================
  # 9. inorder (convenience wrapper via full_scan)
  # =========================================================================

  describe "inorder" do
    test "inorder helper returns sorted pairs" do
      tree = Enum.reduce([5, 3, 1, 4, 2], BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k) end)
      result = BPlusTree.full_scan(tree)
      assert Enum.map(result, &elem(&1, 0)) == [1, 2, 3, 4, 5]
    end
  end

  # =========================================================================
  # 10. Min / Max
  # =========================================================================

  describe "min_key/1 and max_key/1" do
    test "min_key" do
      tree = Enum.reduce([30, 10, 50, 20, 40], BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k) end)
      assert BPlusTree.min_key(tree) == 10
    end

    test "max_key" do
      tree = Enum.reduce([30, 10, 50, 20, 40], BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k) end)
      assert BPlusTree.max_key(tree) == 50
    end

    test "min_key raises on empty" do
      assert_raise ArgumentError, fn -> BPlusTree.min_key(BPlusTree.new()) end
    end

    test "max_key raises on empty" do
      assert_raise ArgumentError, fn -> BPlusTree.max_key(BPlusTree.new()) end
    end
  end

  # =========================================================================
  # 11. Size, empty?, height
  # =========================================================================

  describe "size/1, empty?/1, height/1" do
    test "size grows with inserts" do
      tree = Enum.reduce(1..10, BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k) end)
      assert BPlusTree.size(tree) == 10
    end

    test "empty? after all deletions" do
      tree = BPlusTree.new() |> BPlusTree.insert(1, :x) |> BPlusTree.delete(1)
      assert BPlusTree.empty?(tree)
    end

    test "height grows with inserts" do
      tree = Enum.reduce(1..1000, BPlusTree.new(3), fn k, t -> BPlusTree.insert(t, k, k) end)
      assert BPlusTree.height(tree) > 0
      assert BPlusTree.height(tree) <= 8
    end
  end

  # =========================================================================
  # 12. valid?
  # =========================================================================

  describe "valid?/1" do
    test "valid after every insert t=2" do
      Enum.reduce(1..30, BPlusTree.new(), fn k, tree ->
        tree = BPlusTree.insert(tree, k, k)
        assert BPlusTree.valid?(tree), "Tree invalid after inserting #{k}"
        tree
      end)
    end

    test "valid after every delete t=2 shuffled" do
      tree = Enum.reduce(1..30, BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k) end)

      1..30
      |> Enum.to_list()
      |> Enum.shuffle()
      |> Enum.reduce(tree, fn k, t ->
        t = BPlusTree.delete(t, k)
        assert BPlusTree.valid?(t), "Tree invalid after deleting #{k}"
        t
      end)
    end

    test "valid with t=5" do
      tree = Enum.reduce(1..100, BPlusTree.new(5), fn k, t -> BPlusTree.insert(t, k, k) end)
      tree = Enum.reduce(1..100//3, tree, fn k, t -> BPlusTree.delete(t, k) end)
      assert BPlusTree.valid?(tree)
    end
  end

  # =========================================================================
  # 13. Stress Tests
  # =========================================================================

  describe "stress tests" do
    test "insert and delete 1000 keys t=2" do
      keys = Enum.shuffle(1..1000)
      tree = Enum.reduce(keys, BPlusTree.new(), fn k, t -> BPlusTree.insert(t, k, k) end)
      assert BPlusTree.size(tree) == 1000
      assert BPlusTree.valid?(tree)

      {to_delete, remaining} = Enum.split(keys, 500)
      tree = Enum.reduce(to_delete, tree, fn k, t -> BPlusTree.delete(t, k) end)
      assert BPlusTree.size(tree) == 500
      assert BPlusTree.valid?(tree)

      expected = Enum.sort(remaining)
      actual   = BPlusTree.full_scan(tree) |> Enum.map(&elem(&1, 0))
      assert actual == expected
    end

    test "insert and delete all 1000 keys t=5" do
      keys = Enum.shuffle(1..1000)
      tree = Enum.reduce(keys, BPlusTree.new(5), fn k, t -> BPlusTree.insert(t, k, k) end)
      tree = Enum.reduce(keys, tree, fn k, t -> BPlusTree.delete(t, k) end)
      assert BPlusTree.empty?(tree)
      assert BPlusTree.valid?(tree)
    end

    test "range_scan 1000 keys" do
      tree = Enum.reduce(1..1000, BPlusTree.new(3), fn k, t -> BPlusTree.insert(t, k, k) end)
      result = BPlusTree.range_scan(tree, 200, 300)
      assert Enum.map(result, &elem(&1, 0)) == Enum.to_list(200..300)
    end

    test "full_scan walks all leaves correctly" do
      tree = Enum.reduce(Enum.shuffle(1..500), BPlusTree.new(4), fn k, t -> BPlusTree.insert(t, k, k) end)
      result = BPlusTree.full_scan(tree)
      assert Enum.map(result, &elem(&1, 0)) == Enum.to_list(1..500)
    end
  end

end
