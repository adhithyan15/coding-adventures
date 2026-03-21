defmodule CodingAdventures.DirectedGraph.LabeledGraphTest do
  @moduledoc """
  Tests for CodingAdventures.DirectedGraph.LabeledGraph

  Covers: add/remove edges with labels, multiple labels per pair, self-loops,
  label filtering, algorithm delegation, node removal cleanup, error cases.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.DirectedGraph.LabeledGraph
  alias CodingAdventures.DirectedGraph.{CycleError, EdgeNotFoundError, NodeNotFoundError}

  # ======================================================================
  # 1. Empty Graph
  # ======================================================================

  describe "empty labeled graph" do
    test "has no nodes" do
      lg = LabeledGraph.new()
      assert LabeledGraph.nodes(lg) == []
      assert LabeledGraph.size(lg) == 0
    end

    test "has no edges" do
      lg = LabeledGraph.new()
      assert LabeledGraph.edges(lg) == []
    end

    test "has_node? returns false" do
      lg = LabeledGraph.new()
      assert LabeledGraph.has_node?(lg, "A") == false
    end

    test "has_edge? returns false" do
      lg = LabeledGraph.new()
      assert LabeledGraph.has_edge?(lg, "A", "B") == false
      assert LabeledGraph.has_edge?(lg, "A", "B", "x") == false
    end

    test "labels returns empty set" do
      lg = LabeledGraph.new()
      assert LabeledGraph.labels(lg, "A", "B") == MapSet.new()
    end
  end

  # ======================================================================
  # 2. Single Labeled Edge
  # ======================================================================

  describe "single labeled edge" do
    test "add_edge creates nodes" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "friend")
      assert LabeledGraph.has_node?(lg, "A")
      assert LabeledGraph.has_node?(lg, "B")
      assert LabeledGraph.size(lg) == 2
    end

    test "add_edge creates labeled edge" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "friend")
      assert LabeledGraph.has_edge?(lg, "A", "B", "friend")
    end

    test "has_edge? without label checks any label" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "friend")
      assert LabeledGraph.has_edge?(lg, "A", "B") == true
    end

    test "has_edge? with wrong label returns false" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "friend")
      assert LabeledGraph.has_edge?(lg, "A", "B", "enemy") == false
    end

    test "has_edge? wrong direction returns false" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "friend")
      assert LabeledGraph.has_edge?(lg, "B", "A") == false
    end

    test "edges returns triple" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "friend")
      assert LabeledGraph.edges(lg) == [{"A", "B", "friend"}]
    end

    test "labels returns label set" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "friend")
      assert LabeledGraph.labels(lg, "A", "B") == MapSet.new(["friend"])
    end

    test "successors of source" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "friend")
      {:ok, succs} = LabeledGraph.successors(lg, "A")
      assert succs == ["B"]
    end

    test "predecessors of target" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "friend")
      {:ok, preds} = LabeledGraph.predecessors(lg, "B")
      assert preds == ["A"]
    end
  end

  # ======================================================================
  # 3. Multiple Labels Per Edge Pair
  # ======================================================================

  describe "multiple labels" do
    test "two labels on same edge" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "friend")
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "coworker")
      assert LabeledGraph.labels(lg, "A", "B") == MapSet.new(["friend", "coworker"])
    end

    test "multiple labels share one structural edge" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "friend")
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "coworker")
      inner = LabeledGraph.graph(lg)
      assert length(CodingAdventures.DirectedGraph.Graph.edges(inner)) == 1
    end

    test "edges returns one triple per label" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "friend")
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "coworker")
      edges = LabeledGraph.edges(lg)
      assert length(edges) == 2
      assert {"A", "B", "coworker"} in edges
      assert {"A", "B", "friend"} in edges
    end

    test "has_edge? checks specific label" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "friend")
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "coworker")
      assert LabeledGraph.has_edge?(lg, "A", "B", "friend")
      assert LabeledGraph.has_edge?(lg, "A", "B", "coworker")
      assert not LabeledGraph.has_edge?(lg, "A", "B", "enemy")
    end

    test "same label twice is idempotent" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "friend")
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "friend")
      assert LabeledGraph.labels(lg, "A", "B") == MapSet.new(["friend"])
      assert length(LabeledGraph.edges(lg)) == 1
    end

    test "three labels on same edge" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "x")
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "y")
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "z")
      assert LabeledGraph.labels(lg, "A", "B") == MapSet.new(["x", "y", "z"])
    end
  end

  # ======================================================================
  # 4. Self-Loops
  # ======================================================================

  describe "self-loops" do
    test "can be added" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "A", "reflexive")
      assert LabeledGraph.has_edge?(lg, "A", "A", "reflexive")
    end

    test "appears in edges" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "A", "loop")
      assert {"A", "A", "loop"} in LabeledGraph.edges(lg)
    end

    test "node is own successor" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "A", "loop")
      {:ok, succs} = LabeledGraph.successors(lg, "A")
      assert "A" in succs
    end

    test "node is own predecessor" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "A", "loop")
      {:ok, preds} = LabeledGraph.predecessors(lg, "A")
      assert "A" in preds
    end

    test "creates a cycle" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "A", "loop")
      assert LabeledGraph.has_cycle?(lg)
    end

    test "multiple labels on self-loop" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "A", "x")
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "A", "y")
      assert LabeledGraph.labels(lg, "A", "A") == MapSet.new(["x", "y"])
    end

    test "self-loop with normal edge" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "A", "self")
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "other")
      assert LabeledGraph.has_edge?(lg, "A", "A", "self")
      assert LabeledGraph.has_edge?(lg, "A", "B", "other")
      assert length(LabeledGraph.edges(lg)) == 2
    end
  end

  # ======================================================================
  # 5. Node Removal
  # ======================================================================

  describe "node removal" do
    test "cleans outgoing labels" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "x")
      {:ok, lg} = LabeledGraph.remove_node(lg, "A")
      assert not LabeledGraph.has_node?(lg, "A")
      assert LabeledGraph.has_node?(lg, "B")
      assert LabeledGraph.labels(lg, "A", "B") == MapSet.new()
      assert LabeledGraph.edges(lg) == []
    end

    test "cleans incoming labels" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "x")
      {:ok, lg} = LabeledGraph.remove_node(lg, "B")
      assert LabeledGraph.has_node?(lg, "A")
      assert not LabeledGraph.has_node?(lg, "B")
      assert LabeledGraph.labels(lg, "A", "B") == MapSet.new()
    end

    test "cleans hub node labels" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "x")
      {:ok, lg} = LabeledGraph.add_edge(lg, "B", "C", "y")
      {:ok, lg} = LabeledGraph.add_edge(lg, "B", "C", "z")
      {:ok, lg} = LabeledGraph.remove_node(lg, "B")
      assert LabeledGraph.edges(lg) == []
      assert LabeledGraph.has_node?(lg, "A")
      assert LabeledGraph.has_node?(lg, "C")
    end

    test "cleans self-loop labels" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "A", "loop")
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "out")
      {:ok, lg} = LabeledGraph.remove_node(lg, "A")
      assert not LabeledGraph.has_node?(lg, "A")
      assert LabeledGraph.has_node?(lg, "B")
      assert LabeledGraph.edges(lg) == []
    end

    test "nonexistent node raises" do
      lg = LabeledGraph.new()
      assert {:error, %NodeNotFoundError{}} = LabeledGraph.remove_node(lg, "X")
    end
  end

  # ======================================================================
  # 6. Edge Removal
  # ======================================================================

  describe "edge removal" do
    test "removing only label removes structural edge" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "x")
      {:ok, lg} = LabeledGraph.remove_edge(lg, "A", "B", "x")
      assert not LabeledGraph.has_edge?(lg, "A", "B")
      assert LabeledGraph.edges(lg) == []
      assert LabeledGraph.has_node?(lg, "A")
      assert LabeledGraph.has_node?(lg, "B")
    end

    test "removing one of two labels keeps the other" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "x")
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "y")
      {:ok, lg} = LabeledGraph.remove_edge(lg, "A", "B", "x")
      assert LabeledGraph.has_edge?(lg, "A", "B", "y")
      assert not LabeledGraph.has_edge?(lg, "A", "B", "x")
      assert LabeledGraph.has_edge?(lg, "A", "B")
      assert LabeledGraph.labels(lg, "A", "B") == MapSet.new(["y"])
    end

    test "removing all labels removes structural edge" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "x")
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "y")
      {:ok, lg} = LabeledGraph.remove_edge(lg, "A", "B", "x")
      {:ok, lg} = LabeledGraph.remove_edge(lg, "A", "B", "y")
      assert not LabeledGraph.has_edge?(lg, "A", "B")
    end

    test "nonexistent label raises" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "x")
      assert {:error, %EdgeNotFoundError{}} = LabeledGraph.remove_edge(lg, "A", "B", "nonexistent")
    end

    test "nonexistent edge pair raises" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_node(lg, "A")
      {:ok, lg} = LabeledGraph.add_node(lg, "B")
      assert {:error, %EdgeNotFoundError{}} = LabeledGraph.remove_edge(lg, "A", "B", "x")
    end

    test "remove self-loop label" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "A", "loop")
      {:ok, lg} = LabeledGraph.remove_edge(lg, "A", "A", "loop")
      assert not LabeledGraph.has_edge?(lg, "A", "A")
      assert LabeledGraph.has_node?(lg, "A")
    end
  end

  # ======================================================================
  # 7. Label Filtering
  # ======================================================================

  describe "label filtering" do
    test "successors unfiltered" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "x")
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "C", "y")
      {:ok, succs} = LabeledGraph.successors(lg, "A")
      assert Enum.sort(succs) == ["B", "C"]
    end

    test "successors filtered by label" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "friend")
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "C", "coworker")
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "D", "friend")
      {:ok, succs} = LabeledGraph.successors(lg, "A", "friend")
      assert Enum.sort(succs) == ["B", "D"]
    end

    test "successors filtered no match" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "friend")
      {:ok, succs} = LabeledGraph.successors(lg, "A", "enemy")
      assert succs == []
    end

    test "predecessors unfiltered" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "C", "x")
      {:ok, lg} = LabeledGraph.add_edge(lg, "B", "C", "y")
      {:ok, preds} = LabeledGraph.predecessors(lg, "C")
      assert Enum.sort(preds) == ["A", "B"]
    end

    test "predecessors filtered by label" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "C", "friend")
      {:ok, lg} = LabeledGraph.add_edge(lg, "B", "C", "coworker")
      {:ok, lg} = LabeledGraph.add_edge(lg, "D", "C", "friend")
      {:ok, preds} = LabeledGraph.predecessors(lg, "C", "friend")
      assert Enum.sort(preds) == ["A", "D"]
    end

    test "predecessors filtered no match" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "friend")
      {:ok, preds} = LabeledGraph.predecessors(lg, "B", "enemy")
      assert preds == []
    end

    test "successors nonexistent node" do
      lg = LabeledGraph.new()
      assert {:error, %NodeNotFoundError{}} = LabeledGraph.successors(lg, "X")
    end

    test "predecessors nonexistent node" do
      lg = LabeledGraph.new()
      assert {:error, %NodeNotFoundError{}} = LabeledGraph.predecessors(lg, "X")
    end

    test "successors filtered nonexistent node" do
      lg = LabeledGraph.new()
      assert {:error, %NodeNotFoundError{}} = LabeledGraph.successors(lg, "X", "friend")
    end

    test "predecessors filtered nonexistent node" do
      lg = LabeledGraph.new()
      assert {:error, %NodeNotFoundError{}} = LabeledGraph.predecessors(lg, "X", "friend")
    end

    test "self-loop in filtered successors" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "A", "self")
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "other")
      {:ok, succs} = LabeledGraph.successors(lg, "A", "self")
      assert succs == ["A"]
      {:ok, succs} = LabeledGraph.successors(lg, "A", "other")
      assert succs == ["B"]
    end

    test "self-loop in filtered predecessors" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "A", "self")
      {:ok, lg} = LabeledGraph.add_edge(lg, "B", "A", "other")
      {:ok, preds} = LabeledGraph.predecessors(lg, "A", "self")
      assert preds == ["A"]
      {:ok, preds} = LabeledGraph.predecessors(lg, "A", "other")
      assert preds == ["B"]
    end
  end

  # ======================================================================
  # 8. Algorithm Delegation
  # ======================================================================

  describe "algorithm delegation" do
    test "topological sort DAG" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "dep")
      {:ok, lg} = LabeledGraph.add_edge(lg, "B", "C", "dep")
      assert {:ok, ["A", "B", "C"]} = LabeledGraph.topological_sort(lg)
    end

    test "topological sort with multiple labels" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "compile")
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "test")
      {:ok, lg} = LabeledGraph.add_edge(lg, "B", "C", "compile")
      assert {:ok, ["A", "B", "C"]} = LabeledGraph.topological_sort(lg)
    end

    test "has_cycle? false for DAG" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "x")
      {:ok, lg} = LabeledGraph.add_edge(lg, "B", "C", "y")
      assert LabeledGraph.has_cycle?(lg) == false
    end

    test "has_cycle? true for cycle" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "x")
      {:ok, lg} = LabeledGraph.add_edge(lg, "B", "C", "y")
      {:ok, lg} = LabeledGraph.add_edge(lg, "C", "A", "z")
      assert LabeledGraph.has_cycle?(lg) == true
    end

    test "has_cycle? true for self-loop" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "A", "loop")
      assert LabeledGraph.has_cycle?(lg) == true
    end

    test "transitive closure" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "x")
      {:ok, lg} = LabeledGraph.add_edge(lg, "B", "C", "y")
      {:ok, lg} = LabeledGraph.add_edge(lg, "B", "D", "z")
      {:ok, closure} = LabeledGraph.transitive_closure(lg, "A")
      assert closure == MapSet.new(["B", "C", "D"])
    end

    test "transitive closure with self-loop" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "A", "self")
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "next")
      {:ok, closure} = LabeledGraph.transitive_closure(lg, "A")
      assert MapSet.member?(closure, "A")
      assert MapSet.member?(closure, "B")
    end

    test "transitive dependents" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "x")
      {:ok, lg} = LabeledGraph.add_edge(lg, "B", "C", "y")
      {:ok, deps} = LabeledGraph.transitive_dependents(lg, "C")
      assert deps == MapSet.new(["A", "B"])
    end

    test "graph property returns inner graph" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "x")
      g = LabeledGraph.graph(lg)
      assert CodingAdventures.DirectedGraph.Graph.has_node?(g, "A")
      assert CodingAdventures.DirectedGraph.Graph.has_edge?(g, "A", "B")
    end

    test "topological sort cycle raises" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "x")
      {:ok, lg} = LabeledGraph.add_edge(lg, "B", "A", "y")
      assert {:error, %CycleError{}} = LabeledGraph.topological_sort(lg)
    end

    test "transitive closure nonexistent node" do
      lg = LabeledGraph.new()
      assert {:error, %NodeNotFoundError{}} = LabeledGraph.transitive_closure(lg, "X")
    end

    test "transitive dependents nonexistent node" do
      lg = LabeledGraph.new()
      assert {:error, %NodeNotFoundError{}} = LabeledGraph.transitive_dependents(lg, "X")
    end
  end

  # ======================================================================
  # 9. Edge Cases
  # ======================================================================

  describe "edge cases" do
    test "add isolated node" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_node(lg, "A")
      assert LabeledGraph.has_node?(lg, "A")
      assert LabeledGraph.edges(lg) == []
    end

    test "add_node is idempotent" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_node(lg, "A")
      {:ok, lg} = LabeledGraph.add_node(lg, "A")
      assert LabeledGraph.size(lg) == 1
    end

    test "integer nodes" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, 1, 2, "next")
      assert LabeledGraph.has_edge?(lg, 1, 2, "next")
    end

    test "empty string label" do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "")
      assert LabeledGraph.has_edge?(lg, "A", "B", "")
      assert LabeledGraph.labels(lg, "A", "B") == MapSet.new([""])
    end
  end

  # ======================================================================
  # 10. Integration: Social Network
  # ======================================================================

  describe "social network integration" do
    setup do
      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "Alice", "Bob", "friend")
      {:ok, lg} = LabeledGraph.add_edge(lg, "Alice", "Bob", "coworker")
      {:ok, lg} = LabeledGraph.add_edge(lg, "Alice", "Carol", "friend")
      {:ok, lg} = LabeledGraph.add_edge(lg, "Bob", "Dave", "friend")
      {:ok, lg} = LabeledGraph.add_edge(lg, "Dave", "Alice", "follows")
      %{lg: lg}
    end

    test "node count", %{lg: lg} do
      assert LabeledGraph.size(lg) == 4
    end

    test "edge count", %{lg: lg} do
      assert length(LabeledGraph.edges(lg)) == 5
    end

    test "Alice's friends", %{lg: lg} do
      {:ok, friends} = LabeledGraph.successors(lg, "Alice", "friend")
      assert Enum.sort(friends) == ["Bob", "Carol"]
    end

    test "Alice's coworkers", %{lg: lg} do
      {:ok, coworkers} = LabeledGraph.successors(lg, "Alice", "coworker")
      assert coworkers == ["Bob"]
    end

    test "Alice-Bob labels", %{lg: lg} do
      assert LabeledGraph.labels(lg, "Alice", "Bob") == MapSet.new(["friend", "coworker"])
    end

    test "has cycle", %{lg: lg} do
      assert LabeledGraph.has_cycle?(lg)
    end

    test "transitive closure from Alice", %{lg: lg} do
      {:ok, closure} = LabeledGraph.transitive_closure(lg, "Alice")
      assert closure == MapSet.new(["Alice", "Bob", "Carol", "Dave"])
    end

    test "remove Alice cleans everything", %{lg: lg} do
      {:ok, lg} = LabeledGraph.remove_node(lg, "Alice")
      assert not LabeledGraph.has_node?(lg, "Alice")
      # Only Bob -> Dave should remain.
      assert length(LabeledGraph.edges(lg)) == 1
      assert LabeledGraph.has_edge?(lg, "Bob", "Dave", "friend")
    end
  end
end
