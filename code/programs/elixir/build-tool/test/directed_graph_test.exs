defmodule BuildTool.DirectedGraphTest do
  use ExUnit.Case, async: true

  alias BuildTool.DirectedGraph

  # ---------------------------------------------------------------------------
  # Construction and basic operations
  # ---------------------------------------------------------------------------

  describe "new/0" do
    test "creates an empty graph" do
      g = DirectedGraph.new()
      assert DirectedGraph.size(g) == 0
      assert DirectedGraph.nodes(g) == []
    end
  end

  describe "add_node/2" do
    test "adds a single node" do
      g = DirectedGraph.new() |> DirectedGraph.add_node("A")
      assert DirectedGraph.has_node?(g, "A")
      assert DirectedGraph.size(g) == 1
    end

    test "adding a node twice is a no-op" do
      g =
        DirectedGraph.new()
        |> DirectedGraph.add_node("A")
        |> DirectedGraph.add_node("A")

      assert DirectedGraph.size(g) == 1
    end

    test "adds multiple nodes" do
      g =
        DirectedGraph.new()
        |> DirectedGraph.add_node("A")
        |> DirectedGraph.add_node("B")
        |> DirectedGraph.add_node("C")

      assert DirectedGraph.size(g) == 3
      assert DirectedGraph.nodes(g) == ["A", "B", "C"]
    end
  end

  describe "add_edge/3" do
    test "adds an edge between two nodes" do
      g = DirectedGraph.new() |> DirectedGraph.add_edge("A", "B")
      assert DirectedGraph.has_edge?(g, "A", "B")
      refute DirectedGraph.has_edge?(g, "B", "A")
    end

    test "implicitly adds nodes" do
      g = DirectedGraph.new() |> DirectedGraph.add_edge("A", "B")
      assert DirectedGraph.has_node?(g, "A")
      assert DirectedGraph.has_node?(g, "B")
    end

    test "raises on self-loop" do
      assert_raise ArgumentError, fn ->
        DirectedGraph.new() |> DirectedGraph.add_edge("A", "A")
      end
    end
  end

  describe "predecessors/2" do
    test "returns direct predecessors" do
      g =
        DirectedGraph.new()
        |> DirectedGraph.add_edge("A", "B")
        |> DirectedGraph.add_edge("C", "B")

      assert {:ok, ["A", "C"]} = DirectedGraph.predecessors(g, "B")
    end

    test "returns empty list for node with no predecessors" do
      g = DirectedGraph.new() |> DirectedGraph.add_edge("A", "B")
      assert {:ok, []} = DirectedGraph.predecessors(g, "A")
    end

    test "returns error for unknown node" do
      g = DirectedGraph.new()
      assert {:error, {:node_not_found, "X"}} = DirectedGraph.predecessors(g, "X")
    end
  end

  describe "successors/2" do
    test "returns direct successors" do
      g =
        DirectedGraph.new()
        |> DirectedGraph.add_edge("A", "B")
        |> DirectedGraph.add_edge("A", "C")

      assert {:ok, ["B", "C"]} = DirectedGraph.successors(g, "A")
    end

    test "returns empty list for node with no successors" do
      g = DirectedGraph.new() |> DirectedGraph.add_edge("A", "B")
      assert {:ok, []} = DirectedGraph.successors(g, "B")
    end
  end

  # ---------------------------------------------------------------------------
  # Kahn's algorithm — independent_groups
  # ---------------------------------------------------------------------------

  describe "independent_groups/1" do
    test "linear chain: A -> B -> C" do
      g =
        DirectedGraph.new()
        |> DirectedGraph.add_edge("A", "B")
        |> DirectedGraph.add_edge("B", "C")

      assert {:ok, [["A"], ["B"], ["C"]]} = DirectedGraph.independent_groups(g)
    end

    test "diamond: A -> B, A -> C, B -> D, C -> D" do
      g =
        DirectedGraph.new()
        |> DirectedGraph.add_edge("A", "B")
        |> DirectedGraph.add_edge("A", "C")
        |> DirectedGraph.add_edge("B", "D")
        |> DirectedGraph.add_edge("C", "D")

      assert {:ok, [["A"], ["B", "C"], ["D"]]} = DirectedGraph.independent_groups(g)
    end

    test "isolated nodes appear in level 0" do
      g =
        DirectedGraph.new()
        |> DirectedGraph.add_node("X")
        |> DirectedGraph.add_node("Y")
        |> DirectedGraph.add_node("Z")

      assert {:ok, [level0]} = DirectedGraph.independent_groups(g)
      assert Enum.sort(level0) == ["X", "Y", "Z"]
    end

    test "mixed isolated and connected nodes" do
      g =
        DirectedGraph.new()
        |> DirectedGraph.add_node("X")
        |> DirectedGraph.add_edge("A", "B")

      assert {:ok, [level0, level1]} = DirectedGraph.independent_groups(g)
      assert "X" in level0
      assert "A" in level0
      assert level1 == ["B"]
    end

    test "empty graph returns empty levels" do
      g = DirectedGraph.new()
      assert {:ok, []} = DirectedGraph.independent_groups(g)
    end

    test "single node" do
      g = DirectedGraph.new() |> DirectedGraph.add_node("A")
      assert {:ok, [["A"]]} = DirectedGraph.independent_groups(g)
    end
  end

  # ---------------------------------------------------------------------------
  # Transitive closure
  # ---------------------------------------------------------------------------

  describe "transitive_closure/2" do
    test "follows forward edges transitively" do
      g =
        DirectedGraph.new()
        |> DirectedGraph.add_edge("A", "B")
        |> DirectedGraph.add_edge("B", "C")
        |> DirectedGraph.add_edge("C", "D")

      result = DirectedGraph.transitive_closure(g, "A")
      assert MapSet.equal?(result, MapSet.new(["B", "C", "D"]))
    end

    test "returns empty set for leaf node" do
      g = DirectedGraph.new() |> DirectedGraph.add_edge("A", "B")
      assert MapSet.equal?(DirectedGraph.transitive_closure(g, "B"), MapSet.new())
    end

    test "handles diamond graph" do
      g =
        DirectedGraph.new()
        |> DirectedGraph.add_edge("A", "B")
        |> DirectedGraph.add_edge("A", "C")
        |> DirectedGraph.add_edge("B", "D")
        |> DirectedGraph.add_edge("C", "D")

      result = DirectedGraph.transitive_closure(g, "A")
      assert MapSet.equal?(result, MapSet.new(["B", "C", "D"]))
    end
  end

  # ---------------------------------------------------------------------------
  # Affected nodes
  # ---------------------------------------------------------------------------

  describe "affected_nodes/2" do
    test "includes changed node and all transitive dependents" do
      g =
        DirectedGraph.new()
        |> DirectedGraph.add_edge("A", "B")
        |> DirectedGraph.add_edge("B", "C")

      result = DirectedGraph.affected_nodes(g, MapSet.new(["A"]))
      assert MapSet.equal?(result, MapSet.new(["A", "B", "C"]))
    end

    test "includes only relevant subgraph" do
      g =
        DirectedGraph.new()
        |> DirectedGraph.add_edge("A", "B")
        |> DirectedGraph.add_edge("C", "D")

      result = DirectedGraph.affected_nodes(g, MapSet.new(["A"]))
      assert MapSet.equal?(result, MapSet.new(["A", "B"]))
      refute MapSet.member?(result, "C")
      refute MapSet.member?(result, "D")
    end

    test "handles unknown nodes gracefully" do
      g = DirectedGraph.new() |> DirectedGraph.add_node("A")
      result = DirectedGraph.affected_nodes(g, MapSet.new(["UNKNOWN"]))
      assert MapSet.equal?(result, MapSet.new())
    end

    test "handles multiple changed nodes" do
      g =
        DirectedGraph.new()
        |> DirectedGraph.add_edge("A", "C")
        |> DirectedGraph.add_edge("B", "C")
        |> DirectedGraph.add_edge("C", "D")

      result = DirectedGraph.affected_nodes(g, MapSet.new(["A", "B"]))
      assert MapSet.equal?(result, MapSet.new(["A", "B", "C", "D"]))
    end
  end

  # ---------------------------------------------------------------------------
  # Transitive predecessors
  # ---------------------------------------------------------------------------

  describe "transitive_predecessors/2" do
    test "follows reverse edges transitively" do
      g =
        DirectedGraph.new()
        |> DirectedGraph.add_edge("A", "B")
        |> DirectedGraph.add_edge("B", "C")

      result = DirectedGraph.transitive_predecessors(g, "C")
      assert MapSet.equal?(result, MapSet.new(["A", "B"]))
    end

    test "returns empty set for root node" do
      g = DirectedGraph.new() |> DirectedGraph.add_edge("A", "B")
      assert MapSet.equal?(DirectedGraph.transitive_predecessors(g, "A"), MapSet.new())
    end
  end
end
