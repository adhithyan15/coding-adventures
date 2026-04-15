defmodule CodingAdventures.BTreeTest do
  @moduledoc """
  Comprehensive tests for the B-tree implementation.

  We test:
  1. Construction (new/0, new/1)
  2. Insert — sequential, reverse, random, upsert
  3. Search — existing, missing, nil values
  4. Member? — present and absent keys
  5. Delete — Case 1 (leaf), Case 2a/2b/2c (internal), Case 3 (fill)
  6. Min/max key
  7. Range query
  8. Inorder traversal
  9. Size, empty?, height
  10. valid? after every operation
  11. Stress tests with 1000+ keys
  12. t=2, t=3, t=5
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.BTree

  # =========================================================================
  # 1. Construction
  # =========================================================================

  describe "new/0 and new/1" do
    test "creates an empty tree with default t=2" do
      tree = BTree.new()
      assert BTree.empty?(tree)
      assert BTree.size(tree) == 0
      assert BTree.valid?(tree)
    end

    test "creates an empty tree with custom t" do
      tree = BTree.new(5)
      assert BTree.empty?(tree)
      assert BTree.valid?(tree)
    end

    test "height of empty tree is 0" do
      tree = BTree.new()
      assert BTree.height(tree) == 0
    end
  end

  # =========================================================================
  # 2. Insert
  # =========================================================================

  describe "insert/3" do
    test "insert single key" do
      tree = BTree.new() |> BTree.insert(42, "forty-two")
      assert BTree.size(tree) == 1
      assert BTree.search(tree, 42) == {:ok, "forty-two"}
      assert BTree.valid?(tree)
    end

    test "insert updates existing key" do
      tree =
        BTree.new()
        |> BTree.insert(10, "old")
        |> BTree.insert(10, "new")

      assert BTree.size(tree) == 1
      assert BTree.search(tree, 10) == {:ok, "new"}
      assert BTree.valid?(tree)
    end

    test "insert sequential keys t=2" do
      tree = Enum.reduce(1..20, BTree.new(), fn k, t -> BTree.insert(t, k, k * 10) end)
      assert BTree.size(tree) == 20
      assert BTree.valid?(tree)

      Enum.each(1..20, fn k ->
        assert BTree.search(tree, k) == {:ok, k * 10}
      end)
    end

    test "insert reverse keys t=2" do
      tree = Enum.reduce(20..1//-1, BTree.new(), fn k, t -> BTree.insert(t, k, k) end)
      assert BTree.size(tree) == 20
      assert BTree.valid?(tree)
    end

    test "insert random keys t=3" do
      keys = Enum.shuffle(1..50)
      tree = Enum.reduce(keys, BTree.new(3), fn k, t -> BTree.insert(t, k, "v#{k}") end)
      assert BTree.size(tree) == 50
      assert BTree.valid?(tree)
      Enum.each(keys, fn k -> assert BTree.search(tree, k) == {:ok, "v#{k}"} end)
    end

    test "insert 1000 keys t=5" do
      tree = Enum.reduce(1..1000, BTree.new(5), fn k, t -> BTree.insert(t, k, k) end)
      assert BTree.size(tree) == 1000
      assert BTree.valid?(tree)
      assert BTree.height(tree) <= 4
    end

    test "insert causes root split t=2" do
      tree = Enum.reduce(1..3, BTree.new(), fn k, t -> BTree.insert(t, k, k) end)
      assert BTree.height(tree) == 0  # still leaf root
      tree = BTree.insert(tree, 4, 4)
      assert BTree.valid?(tree)
      assert BTree.size(tree) == 4
    end

    test "inorder is sorted after inserts" do
      keys = [5, 3, 7, 1, 9, 4, 6, 2, 8]
      tree = Enum.reduce(keys, BTree.new(), fn k, t -> BTree.insert(t, k, k) end)
      result = BTree.inorder(tree) |> Enum.map(&elem(&1, 0))
      assert result == Enum.sort(keys)
    end

    test "valid after each insert" do
      Enum.reduce(1..30, BTree.new(), fn k, tree ->
        tree = BTree.insert(tree, k, k)
        assert BTree.valid?(tree), "Tree invalid after inserting #{k}"
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
        BTree.new()
        |> BTree.insert(10, "ten")
        |> BTree.insert(20, "twenty")
        |> BTree.insert(30, "thirty")

      {:ok, tree: tree}
    end

    test "returns {:ok, value} for existing key", %{tree: tree} do
      assert BTree.search(tree, 10) == {:ok, "ten"}
      assert BTree.search(tree, 30) == {:ok, "thirty"}
    end

    test "returns :error for missing key", %{tree: tree} do
      assert BTree.search(tree, 99) == :error
      assert BTree.search(tree, 0) == :error
    end

    test "search on empty tree returns :error" do
      assert BTree.search(BTree.new(), 1) == :error
    end

    test "can store nil values" do
      tree = BTree.new() |> BTree.insert(5, nil)
      assert BTree.search(tree, 5) == {:ok, nil}
    end
  end

  # =========================================================================
  # 4. Member?
  # =========================================================================

  describe "member?/2" do
    test "returns true for present key" do
      tree = BTree.new() |> BTree.insert(5, :x)
      assert BTree.member?(tree, 5)
    end

    test "returns false for absent key" do
      tree = BTree.new() |> BTree.insert(5, :x)
      refute BTree.member?(tree, 6)
    end
  end

  # =========================================================================
  # 5. Delete
  # =========================================================================

  describe "delete/2 — Case 1: key in leaf" do
    test "delete from leaf removes key" do
      tree =
        BTree.new()
        |> BTree.insert(1, :a)
        |> BTree.insert(2, :b)
        |> BTree.insert(3, :c)
        |> BTree.delete(2)

      assert BTree.size(tree) == 2
      assert BTree.search(tree, 2) == :error
      assert BTree.valid?(tree)
    end

    test "delete first key in leaf" do
      tree =
        Enum.reduce(1..4, BTree.new(), fn k, t -> BTree.insert(t, k, k) end)
        |> BTree.delete(1)

      assert BTree.valid?(tree)
      assert BTree.search(tree, 1) == :error
    end

    test "delete last key in leaf" do
      tree =
        Enum.reduce(1..4, BTree.new(), fn k, t -> BTree.insert(t, k, k) end)
        |> BTree.delete(4)

      assert BTree.valid?(tree)
      assert BTree.search(tree, 4) == :error
    end
  end

  describe "delete/2 — Case 2: key in internal node" do
    test "delete all keys from multi-level tree" do
      tree = Enum.reduce(1..15, BTree.new(), fn k, t -> BTree.insert(t, k, k) end)

      Enum.reduce(BTree.inorder(tree) |> Enum.map(&elem(&1, 0)), tree, fn k, t ->
        t = BTree.delete(t, k)
        assert BTree.valid?(t), "Tree invalid after deleting #{k}"
        t
      end)
    end

    test "delete internal node key t=2" do
      tree = Enum.reduce(1..15, BTree.new(), fn k, t -> BTree.insert(t, k, k) end)
      tree = BTree.delete(tree, 8)
      assert BTree.valid?(tree)
      assert BTree.search(tree, 8) == :error
    end
  end

  describe "delete/2 — merge and fill cases" do
    test "merge reduces height" do
      tree = Enum.reduce(1..7, BTree.new(), fn k, t -> BTree.insert(t, k, k) end)

      Enum.reduce(1..7, tree, fn k, t ->
        t = BTree.delete(t, k)
        assert BTree.valid?(t)
        t
      end)
      |> then(fn t -> assert BTree.empty?(t) end)
    end

    test "delete with t=3 triggers merge" do
      tree = Enum.reduce(1..30, BTree.new(3), fn k, t -> BTree.insert(t, k, k) end)
      deleted = Enum.take_every(1..30, 3) |> Enum.to_list()

      tree = Enum.reduce(deleted, tree, fn k, t -> BTree.delete(t, k) end)
      assert BTree.valid?(tree)

      Enum.each(deleted, fn k -> assert BTree.search(tree, k) == :error end)
      Enum.each(1..30, fn k ->
        unless k in deleted do
          assert BTree.search(tree, k) == {:ok, k}
        end
      end)
    end

    test "delete non-existent key is noop" do
      tree = Enum.reduce(1..5, BTree.new(), fn k, t -> BTree.insert(t, k, k) end)
      tree = BTree.delete(tree, 99)
      assert BTree.size(tree) == 5
      assert BTree.valid?(tree)
    end

    test "delete from empty tree" do
      tree = BTree.new() |> BTree.delete(1)
      assert BTree.empty?(tree)
      assert BTree.valid?(tree)
    end
  end

  describe "delete/2 — rotate and merge" do
    test "rotate-right borrow from left sibling" do
      tree = Enum.reduce(1..10, BTree.new(), fn k, t -> BTree.insert(t, k, k) end)
      tree = Enum.reduce(8..10, tree, fn k, t -> BTree.delete(t, k) end)
      assert BTree.valid?(tree)
      Enum.each(1..7, fn k -> assert BTree.search(tree, k) == {:ok, k} end)
    end

    test "rotate-left borrow from right sibling" do
      tree = Enum.reduce(1..10, BTree.new(), fn k, t -> BTree.insert(t, k, k) end)
      tree = Enum.reduce(1..3, tree, fn k, t -> BTree.delete(t, k) end)
      assert BTree.valid?(tree)
      Enum.each(4..10, fn k -> assert BTree.search(tree, k) == {:ok, k} end)
    end
  end

  # =========================================================================
  # 6. Min / Max
  # =========================================================================

  describe "min_key/1 and max_key/1" do
    test "min_key" do
      tree = Enum.reduce([30, 10, 50, 20, 40], BTree.new(), fn k, t -> BTree.insert(t, k, k) end)
      assert BTree.min_key(tree) == 10
    end

    test "max_key" do
      tree = Enum.reduce([30, 10, 50, 20, 40], BTree.new(), fn k, t -> BTree.insert(t, k, k) end)
      assert BTree.max_key(tree) == 50
    end

    test "min_key single element" do
      tree = BTree.new() |> BTree.insert(42, :x)
      assert BTree.min_key(tree) == 42
    end

    test "min_key raises on empty" do
      assert_raise ArgumentError, fn -> BTree.min_key(BTree.new()) end
    end

    test "max_key raises on empty" do
      assert_raise ArgumentError, fn -> BTree.max_key(BTree.new()) end
    end
  end

  # =========================================================================
  # 7. Range Query
  # =========================================================================

  describe "range_query/3" do
    setup do
      tree = Enum.reduce(1..20, BTree.new(), fn k, t -> BTree.insert(t, k, k * 10) end)
      {:ok, tree: tree}
    end

    test "full range", %{tree: tree} do
      result = BTree.range_query(tree, 1, 20)
      assert Enum.map(result, &elem(&1, 0)) == Enum.to_list(1..20)
    end

    test "middle range", %{tree: tree} do
      result = BTree.range_query(tree, 5, 10)
      assert Enum.map(result, &elem(&1, 0)) == [5, 6, 7, 8, 9, 10]
      assert Enum.map(result, &elem(&1, 1)) == [50, 60, 70, 80, 90, 100]
    end

    test "single element range", %{tree: tree} do
      assert BTree.range_query(tree, 7, 7) == [{7, 70}]
    end

    test "empty result", %{tree: tree} do
      assert BTree.range_query(tree, 100, 200) == []
    end

    test "range with t=5" do
      tree = Enum.reduce(1..100, BTree.new(5), fn k, t -> BTree.insert(t, k, k) end)
      result = BTree.range_query(tree, 45, 55)
      assert Enum.map(result, &elem(&1, 0)) == Enum.to_list(45..55)
    end
  end

  # =========================================================================
  # 8. Inorder
  # =========================================================================

  describe "inorder/1" do
    test "empty tree" do
      assert BTree.inorder(BTree.new()) == []
    end

    test "returns sorted pairs" do
      keys = [5, 3, 7, 1, 9, 4, 6, 2, 8]
      tree = Enum.reduce(keys, BTree.new(), fn k, t -> BTree.insert(t, k, k) end)
      result = BTree.inorder(tree)
      assert Enum.map(result, &elem(&1, 0)) == Enum.sort(keys)
    end
  end

  # =========================================================================
  # 9. Size, empty?, height
  # =========================================================================

  describe "size/1, empty?/1, height/1" do
    test "size grows with inserts" do
      tree = Enum.reduce(1..10, BTree.new(), fn k, t -> BTree.insert(t, k, k) end)
      assert BTree.size(tree) == 10
    end

    test "empty? is false after inserts" do
      tree = BTree.new() |> BTree.insert(1, :x)
      refute BTree.empty?(tree)
    end

    test "height is 0 for single-level tree" do
      tree = BTree.new() |> BTree.insert(1, 1) |> BTree.insert(2, 2)
      assert BTree.height(tree) == 0
    end

    test "height grows logarithmically" do
      tree = Enum.reduce(1..1000, BTree.new(3), fn k, t -> BTree.insert(t, k, k) end)
      # log_3(1000) ≈ 6.3, so height ≤ 7
      assert BTree.height(tree) <= 7
    end
  end

  # =========================================================================
  # 10. valid?
  # =========================================================================

  describe "valid?/1" do
    test "valid after every insert" do
      Enum.reduce(1..30, BTree.new(), fn k, tree ->
        tree = BTree.insert(tree, k, k)
        assert BTree.valid?(tree), "Tree invalid after inserting #{k}"
        tree
      end)
    end

    test "valid after every delete" do
      tree = Enum.reduce(1..30, BTree.new(), fn k, t -> BTree.insert(t, k, k) end)

      1..30
      |> Enum.shuffle()
      |> Enum.reduce(tree, fn k, t ->
        t = BTree.delete(t, k)
        assert BTree.valid?(t), "Tree invalid after deleting #{k}"
        t
      end)
    end

    test "valid with t=3" do
      tree = Enum.reduce(1..50, BTree.new(3), fn k, t -> BTree.insert(t, k, k) end)
      tree = Enum.reduce(1..50//2, tree, fn k, t -> BTree.delete(t, k) end)
      assert BTree.valid?(tree)
    end
  end

  # =========================================================================
  # 11. Stress Tests
  # =========================================================================

  describe "stress tests" do
    test "insert and delete 1000 keys t=2" do
      keys = Enum.shuffle(1..1000)
      tree = Enum.reduce(keys, BTree.new(), fn k, t -> BTree.insert(t, k, k) end)
      assert BTree.size(tree) == 1000
      assert BTree.valid?(tree)

      {to_delete, remaining} = Enum.split(keys, 500)
      tree = Enum.reduce(to_delete, tree, fn k, t -> BTree.delete(t, k) end)
      assert BTree.size(tree) == 500
      assert BTree.valid?(tree)

      Enum.each(to_delete, fn k -> assert BTree.search(tree, k) == :error end)
      Enum.each(remaining, fn k -> assert BTree.search(tree, k) == {:ok, k} end)
    end

    test "insert and delete all 1000 keys t=5" do
      keys = Enum.shuffle(1..1000)
      tree = Enum.reduce(keys, BTree.new(5), fn k, t -> BTree.insert(t, k, k) end)
      tree = Enum.reduce(keys, tree, fn k, t -> BTree.delete(t, k) end)
      assert BTree.empty?(tree)
      assert BTree.valid?(tree)
    end

    test "inorder always sorted" do
      keys = Enum.shuffle(1..200)
      tree = Enum.reduce(keys, BTree.new(3), fn k, t -> BTree.insert(t, k, k) end)
      result = BTree.inorder(tree) |> Enum.map(&elem(&1, 0))
      assert result == Enum.sort(result)
    end

    test "atom keys" do
      words = ~w[banana apple cherry date elderberry fig grape]a
      tree = Enum.reduce(words, BTree.new(), fn w, t -> BTree.insert(t, w, Atom.to_string(w)) end)
      assert BTree.valid?(tree)
      Enum.each(words, fn w ->
        assert BTree.search(tree, w) == {:ok, Atom.to_string(w)}
      end)
    end
  end
end
