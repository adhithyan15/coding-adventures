defmodule CodingAdventures.DirectedGraph.GraphTest do
  @moduledoc """
  Tests for CodingAdventures.DirectedGraph.Graph

  These tests cover the fundamental data structure operations and all graph
  algorithms. They progress from simplest to most complex:

  1. Empty graph
  2. Single node operations
  3. Single edge operations
  4. Multi-node operations
  5. Error conditions
  6. Self-loops
  7. Topological sort
  8. Cycle detection
  9. Transitive closure / dependents
  10. Independent groups
  11. Affected nodes
  12. Integration test with realistic data
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.DirectedGraph.Graph
  alias CodingAdventures.DirectedGraph.{CycleError, EdgeNotFoundError, NodeNotFoundError}

  # ======================================================================
  # Helper: build common graph shapes
  # ======================================================================

  defp make_linear_chain do
    g = Graph.new()
    {:ok, g} = Graph.add_edge(g, "A", "B")
    {:ok, g} = Graph.add_edge(g, "B", "C")
    {:ok, g} = Graph.add_edge(g, "C", "D")
    g
  end

  defp make_diamond do
    g = Graph.new()
    {:ok, g} = Graph.add_edge(g, "A", "B")
    {:ok, g} = Graph.add_edge(g, "A", "C")
    {:ok, g} = Graph.add_edge(g, "B", "D")
    {:ok, g} = Graph.add_edge(g, "C", "D")
    g
  end

  defp make_cycle do
    g = Graph.new()
    {:ok, g} = Graph.add_edge(g, "A", "B")
    {:ok, g} = Graph.add_edge(g, "B", "C")
    {:ok, g} = Graph.add_edge(g, "C", "A")
    g
  end

  # ======================================================================
  # 1. Empty Graph
  # ======================================================================

  describe "empty graph" do
    test "has no nodes" do
      g = Graph.new()
      assert Graph.nodes(g) == []
      assert Graph.size(g) == 0
    end

    test "has no edges" do
      g = Graph.new()
      assert Graph.edges(g) == []
    end

    test "has_node? returns false" do
      g = Graph.new()
      assert Graph.has_node?(g, "A") == false
    end

    test "has_edge? returns false" do
      g = Graph.new()
      assert Graph.has_edge?(g, "A", "B") == false
    end
  end

  # ======================================================================
  # 2. Single Node
  # ======================================================================

  describe "single node" do
    test "add_node makes it present" do
      g = Graph.new()
      {:ok, g} = Graph.add_node(g, "A")
      assert Graph.has_node?(g, "A") == true
      assert Graph.size(g) == 1
    end

    test "remove_node makes it absent" do
      g = Graph.new()
      {:ok, g} = Graph.add_node(g, "A")
      {:ok, g} = Graph.remove_node(g, "A")
      assert Graph.has_node?(g, "A") == false
      assert Graph.size(g) == 0
    end

    test "add_node is idempotent" do
      g = Graph.new()
      {:ok, g} = Graph.add_node(g, "A")
      {:ok, g} = Graph.add_node(g, "A")
      assert Graph.size(g) == 1
    end

    test "predecessors of isolated node" do
      g = Graph.new()
      {:ok, g} = Graph.add_node(g, "A")
      {:ok, preds} = Graph.predecessors(g, "A")
      assert preds == []
    end

    test "successors of isolated node" do
      g = Graph.new()
      {:ok, g} = Graph.add_node(g, "A")
      {:ok, succs} = Graph.successors(g, "A")
      assert succs == []
    end
  end

  # ======================================================================
  # 3. Single Edge
  # ======================================================================

  describe "single edge" do
    test "add_edge creates both nodes" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      assert Graph.has_node?(g, "A")
      assert Graph.has_node?(g, "B")
      assert Graph.size(g) == 2
    end

    test "add_edge creates the edge" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      assert Graph.has_edge?(g, "A", "B") == true
      assert Graph.has_edge?(g, "B", "A") == false
    end

    test "edges returns the edge" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      assert Graph.edges(g) == [{"A", "B"}]
    end

    test "predecessors and successors" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, succs} = Graph.successors(g, "A")
      assert succs == ["B"]
      {:ok, preds} = Graph.predecessors(g, "B")
      assert preds == ["A"]
      {:ok, preds_a} = Graph.predecessors(g, "A")
      assert preds_a == []
      {:ok, succs_b} = Graph.successors(g, "B")
      assert succs_b == []
    end

    test "remove_edge deletes edge but keeps nodes" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.remove_edge(g, "A", "B")
      assert Graph.has_edge?(g, "A", "B") == false
      assert Graph.has_node?(g, "A")
      assert Graph.has_node?(g, "B")
    end

    test "duplicate edge is idempotent" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.add_edge(g, "A", "B")
      assert Graph.edges(g) == [{"A", "B"}]
    end
  end

  # ======================================================================
  # 4. Multi-Node Operations
  # ======================================================================

  describe "multi-node operations" do
    test "remove_node cleans up edges" do
      g = make_linear_chain()
      {:ok, g} = Graph.remove_node(g, "B")
      assert not Graph.has_node?(g, "B")
      assert Graph.has_node?(g, "A")
      assert Graph.has_node?(g, "C")
      assert Graph.edges(g) == [{"C", "D"}]
    end

    test "remove_node with multiple edges" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.add_edge(g, "C", "B")
      {:ok, g} = Graph.add_edge(g, "B", "D")
      {:ok, g} = Graph.remove_node(g, "B")
      assert Graph.size(g) == 3
      assert Graph.edges(g) == []
    end
  end

  # ======================================================================
  # 5. Error Conditions
  # ======================================================================

  describe "error conditions" do
    test "self-loop raises error by default" do
      g = Graph.new()
      assert {:error, msg} = Graph.add_edge(g, "A", "A")
      assert msg =~ "Self-loops are not allowed"
    end

    test "remove nonexistent node" do
      g = Graph.new()
      assert {:error, %NodeNotFoundError{node: "X"}} = Graph.remove_node(g, "X")
    end

    test "remove nonexistent edge" do
      g = Graph.new()
      {:ok, g} = Graph.add_node(g, "A")
      {:ok, g} = Graph.add_node(g, "B")
      assert {:error, %EdgeNotFoundError{}} = Graph.remove_edge(g, "A", "B")
    end

    test "predecessors of nonexistent node" do
      g = Graph.new()
      assert {:error, %NodeNotFoundError{}} = Graph.predecessors(g, "X")
    end

    test "successors of nonexistent node" do
      g = Graph.new()
      assert {:error, %NodeNotFoundError{}} = Graph.successors(g, "X")
    end
  end

  # ======================================================================
  # 6. Self-Loops
  # ======================================================================

  describe "self-loops" do
    test "allowed when flag is set" do
      g = Graph.new(allow_self_loops: true)
      {:ok, g} = Graph.add_edge(g, "A", "A")
      assert Graph.has_edge?(g, "A", "A")
    end

    test "node is own successor" do
      g = Graph.new(allow_self_loops: true)
      {:ok, g} = Graph.add_edge(g, "A", "A")
      {:ok, succs} = Graph.successors(g, "A")
      assert "A" in succs
    end

    test "node is own predecessor" do
      g = Graph.new(allow_self_loops: true)
      {:ok, g} = Graph.add_edge(g, "A", "A")
      {:ok, preds} = Graph.predecessors(g, "A")
      assert "A" in preds
    end

    test "appears in edges list" do
      g = Graph.new(allow_self_loops: true)
      {:ok, g} = Graph.add_edge(g, "A", "A")
      assert {"A", "A"} in Graph.edges(g)
    end

    test "creates a cycle" do
      g = Graph.new(allow_self_loops: true)
      {:ok, g} = Graph.add_edge(g, "A", "A")
      assert Graph.has_cycle?(g) == true
    end

    test "coexists with normal edges" do
      g = Graph.new(allow_self_loops: true)
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.add_edge(g, "B", "B")
      assert Graph.has_edge?(g, "A", "B")
      assert Graph.has_edge?(g, "B", "B")
      assert not Graph.has_edge?(g, "A", "A")
    end

    test "remove_node cleans up self-loop" do
      g = Graph.new(allow_self_loops: true)
      {:ok, g} = Graph.add_edge(g, "A", "A")
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.remove_node(g, "A")
      assert not Graph.has_node?(g, "A")
      assert Graph.has_node?(g, "B")
      assert Graph.edges(g) == []
    end

    test "remove self-loop edge" do
      g = Graph.new(allow_self_loops: true)
      {:ok, g} = Graph.add_edge(g, "A", "A")
      {:ok, g} = Graph.remove_edge(g, "A", "A")
      assert not Graph.has_edge?(g, "A", "A")
      assert Graph.has_node?(g, "A")
    end

    test "transitive closure includes self" do
      g = Graph.new(allow_self_loops: true)
      {:ok, g} = Graph.add_edge(g, "A", "A")
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, closure} = Graph.transitive_closure(g, "A")
      assert MapSet.member?(closure, "A")
      assert MapSet.member?(closure, "B")
    end

    test "topological sort raises for self-loop" do
      g = Graph.new(allow_self_loops: true)
      {:ok, g} = Graph.add_edge(g, "A", "A")
      assert {:error, %CycleError{}} = Graph.topological_sort(g)
    end
  end

  # ======================================================================
  # 7. Topological Sort
  # ======================================================================

  describe "topological sort" do
    test "empty graph" do
      g = Graph.new()
      assert {:ok, []} = Graph.topological_sort(g)
    end

    test "single node" do
      g = Graph.new()
      {:ok, g} = Graph.add_node(g, "A")
      assert {:ok, ["A"]} = Graph.topological_sort(g)
    end

    test "linear chain" do
      g = make_linear_chain()
      assert {:ok, ["A", "B", "C", "D"]} = Graph.topological_sort(g)
    end

    test "diamond" do
      g = make_diamond()
      {:ok, result} = Graph.topological_sort(g)
      assert hd(result) == "A"
      assert List.last(result) == "D"
      assert MapSet.new(Enum.slice(result, 1, 2)) == MapSet.new(["B", "C"])
    end

    test "cycle raises CycleError" do
      g = make_cycle()
      assert {:error, %CycleError{cycle: cycle}} = Graph.topological_sort(g)
      # The cycle should contain at least some of the cycle nodes.
      assert length(cycle) >= 3
    end

    test "disconnected components" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "X", "Y")
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, result} = Graph.topological_sort(g)
      assert length(result) == 4
      x_pos = Enum.find_index(result, &(&1 == "X"))
      y_pos = Enum.find_index(result, &(&1 == "Y"))
      a_pos = Enum.find_index(result, &(&1 == "A"))
      b_pos = Enum.find_index(result, &(&1 == "B"))
      assert x_pos < y_pos
      assert a_pos < b_pos
    end
  end

  # ======================================================================
  # 8. Cycle Detection
  # ======================================================================

  describe "cycle detection" do
    test "empty graph has no cycle" do
      g = Graph.new()
      assert Graph.has_cycle?(g) == false
    end

    test "linear chain has no cycle" do
      g = make_linear_chain()
      assert Graph.has_cycle?(g) == false
    end

    test "diamond has no cycle" do
      g = make_diamond()
      assert Graph.has_cycle?(g) == false
    end

    test "three-node cycle" do
      g = make_cycle()
      assert Graph.has_cycle?(g) == true
    end

    test "cycle with tail" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "X", "A")
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.add_edge(g, "B", "C")
      {:ok, g} = Graph.add_edge(g, "C", "A")
      assert Graph.has_cycle?(g) == true
    end
  end

  # ======================================================================
  # 9. Transitive Closure / Dependents
  # ======================================================================

  describe "transitive closure" do
    test "linear chain from root" do
      g = make_linear_chain()
      {:ok, closure} = Graph.transitive_closure(g, "A")
      assert closure == MapSet.new(["B", "C", "D"])
    end

    test "linear chain from middle" do
      g = make_linear_chain()
      {:ok, closure} = Graph.transitive_closure(g, "B")
      assert closure == MapSet.new(["C", "D"])
    end

    test "linear chain from leaf" do
      g = make_linear_chain()
      {:ok, closure} = Graph.transitive_closure(g, "D")
      assert closure == MapSet.new()
    end

    test "diamond from root" do
      g = make_diamond()
      {:ok, closure} = Graph.transitive_closure(g, "A")
      assert closure == MapSet.new(["B", "C", "D"])
    end

    test "nonexistent node" do
      g = Graph.new()
      assert {:error, %NodeNotFoundError{}} = Graph.transitive_closure(g, "X")
    end

    test "isolated node" do
      g = Graph.new()
      {:ok, g} = Graph.add_node(g, "A")
      {:ok, closure} = Graph.transitive_closure(g, "A")
      assert closure == MapSet.new()
    end
  end

  describe "transitive dependents" do
    test "linear chain from leaf" do
      g = make_linear_chain()
      {:ok, deps} = Graph.transitive_dependents(g, "D")
      assert deps == MapSet.new(["A", "B", "C"])
    end

    test "linear chain from root" do
      g = make_linear_chain()
      {:ok, deps} = Graph.transitive_dependents(g, "A")
      assert deps == MapSet.new()
    end

    test "diamond from D" do
      g = make_diamond()
      {:ok, deps} = Graph.transitive_dependents(g, "D")
      assert deps == MapSet.new(["A", "B", "C"])
    end

    test "diamond from B" do
      g = make_diamond()
      {:ok, deps} = Graph.transitive_dependents(g, "B")
      assert deps == MapSet.new(["A"])
    end

    test "nonexistent node" do
      g = Graph.new()
      assert {:error, %NodeNotFoundError{}} = Graph.transitive_dependents(g, "X")
    end
  end

  # ======================================================================
  # 10. Independent Groups
  # ======================================================================

  describe "independent groups" do
    test "empty graph" do
      g = Graph.new()
      assert {:ok, []} = Graph.independent_groups(g)
    end

    test "single node" do
      g = Graph.new()
      {:ok, g} = Graph.add_node(g, "A")
      assert {:ok, [["A"]]} = Graph.independent_groups(g)
    end

    test "linear chain" do
      g = make_linear_chain()
      assert {:ok, [["A"], ["B"], ["C"], ["D"]]} = Graph.independent_groups(g)
    end

    test "diamond has parallel middle" do
      g = make_diamond()
      {:ok, groups} = Graph.independent_groups(g)
      assert length(groups) == 3
      assert hd(groups) == ["A"]
      assert Enum.sort(Enum.at(groups, 1)) == ["B", "C"]
      assert List.last(groups) == ["D"]
    end

    test "two independent chains" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.add_edge(g, "X", "Y")
      {:ok, groups} = Graph.independent_groups(g)
      assert length(groups) == 2
      assert Enum.sort(hd(groups)) == ["A", "X"]
      assert Enum.sort(List.last(groups)) == ["B", "Y"]
    end

    test "cycle raises CycleError" do
      g = make_cycle()
      assert {:error, %CycleError{}} = Graph.independent_groups(g)
    end

    test "wide graph" do
      g = Graph.new()

      g =
        Enum.reduce(["A", "B", "C", "D", "E"], g, fn child, acc ->
          {:ok, acc} = Graph.add_edge(acc, "ROOT", child)
          acc
        end)

      {:ok, groups} = Graph.independent_groups(g)
      assert length(groups) == 2
      assert hd(groups) == ["ROOT"]
      assert Enum.sort(List.last(groups)) == ["A", "B", "C", "D", "E"]
    end
  end

  # ======================================================================
  # 11. Affected Nodes
  # ======================================================================

  describe "affected nodes" do
    test "change leaf affects everything" do
      g = make_linear_chain()
      result = Graph.affected_nodes(g, MapSet.new(["D"]))
      assert result == MapSet.new(["A", "B", "C", "D"])
    end

    test "change root affects only root" do
      g = make_linear_chain()
      result = Graph.affected_nodes(g, MapSet.new(["A"]))
      assert result == MapSet.new(["A"])
    end

    test "change D in diamond" do
      g = make_diamond()
      result = Graph.affected_nodes(g, MapSet.new(["D"]))
      assert result == MapSet.new(["A", "B", "C", "D"])
    end

    test "change multiple nodes" do
      g = make_diamond()
      result = Graph.affected_nodes(g, MapSet.new(["B", "C"]))
      assert result == MapSet.new(["A", "B", "C"])
    end

    test "nonexistent node is ignored" do
      g = make_diamond()
      result = Graph.affected_nodes(g, MapSet.new(["Z"]))
      assert result == MapSet.new()
    end

    test "mixed existing and nonexistent" do
      g = make_diamond()
      result = Graph.affected_nodes(g, MapSet.new(["A", "Z"]))
      assert MapSet.member?(result, "A")
      assert not MapSet.member?(result, "Z")
    end
  end

  # ======================================================================
  # 12. Integration: Real Repo Graph
  # ======================================================================

  describe "real repo graph integration" do
    setup do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "arithmetic", "logic-gates")
      {:ok, g} = Graph.add_edge(g, "lexer", "grammar-tools")
      {:ok, g} = Graph.add_edge(g, "parser", "lexer")
      {:ok, g} = Graph.add_edge(g, "parser", "grammar-tools")
      {:ok, g} = Graph.add_edge(g, "cpu-simulator", "arithmetic")
      {:ok, g} = Graph.add_edge(g, "cpu-simulator", "logic-gates")
      {:ok, g} = Graph.add_edge(g, "intel4004-simulator", "arithmetic")
      {:ok, g} = Graph.add_edge(g, "pipeline", "parser")
      {:ok, g} = Graph.add_edge(g, "pipeline", "lexer")
      {:ok, g} = Graph.add_edge(g, "assembler", "parser")
      {:ok, g} = Graph.add_edge(g, "assembler", "grammar-tools")
      {:ok, g} = Graph.add_edge(g, "virtual-machine", "cpu-simulator")
      {:ok, g} = Graph.add_edge(g, "arm-simulator", "cpu-simulator")
      {:ok, g} = Graph.add_edge(g, "arm-simulator", "assembler")
      {:ok, g} = Graph.add_edge(g, "jvm-simulator", "virtual-machine")
      {:ok, g} = Graph.add_edge(g, "clr-simulator", "virtual-machine")
      {:ok, g} = Graph.add_edge(g, "wasm-simulator", "virtual-machine")
      {:ok, g} = Graph.add_edge(g, "riscv-simulator", "cpu-simulator")
      {:ok, g} = Graph.add_edge(g, "bytecode-compiler", "jvm-simulator")
      {:ok, g} = Graph.add_edge(g, "bytecode-compiler", "clr-simulator")
      {:ok, g} = Graph.add_edge(g, "bytecode-compiler", "wasm-simulator")
      {:ok, g} = Graph.add_edge(g, "bytecode-compiler", "parser")
      {:ok, g} = Graph.add_edge(g, "html-renderer", "parser")
      {:ok, g} = Graph.add_edge(g, "html-renderer", "lexer")
      {:ok, g} = Graph.add_edge(g, "jit-compiler", "virtual-machine")
      {:ok, g} = Graph.add_edge(g, "jit-compiler", "assembler")
      {:ok, g} = Graph.add_edge(g, "ruby-lexer", "grammar-tools")
      {:ok, g} = Graph.add_edge(g, "ruby-parser", "ruby-lexer")
      {:ok, g} = Graph.add_edge(g, "ruby-parser", "grammar-tools")
      {:ok, g} = Graph.add_node(g, "directed-graph")
      %{graph: g}
    end

    test "has 21 packages", %{graph: g} do
      assert Graph.size(g) == 21
    end

    test "is acyclic", %{graph: g} do
      assert Graph.has_cycle?(g) == false
    end

    test "topological sort is valid", %{graph: g} do
      {:ok, order} = Graph.topological_sort(g)
      assert length(order) == 21

      position = Map.new(Enum.with_index(order))

      for {from, to} <- Graph.edges(g) do
        assert Map.get(position, from) < Map.get(position, to),
               "#{from} should appear before #{to}"
      end
    end

    test "independent groups partition all nodes", %{graph: g} do
      {:ok, groups} = Graph.independent_groups(g)
      all_nodes = List.flatten(groups)
      assert length(all_nodes) == 21
      assert MapSet.new(all_nodes) == MapSet.new(Graph.nodes(g))
    end

    test "transitive closure of logic-gates is empty", %{graph: g} do
      {:ok, closure} = Graph.transitive_closure(g, "logic-gates")
      assert closure == MapSet.new()
    end

    test "transitive dependents of logic-gates", %{graph: g} do
      {:ok, deps} = Graph.transitive_dependents(g, "logic-gates")
      assert MapSet.member?(deps, "arithmetic")
      assert MapSet.member?(deps, "cpu-simulator")
      assert MapSet.member?(deps, "virtual-machine")
    end

    test "affected by grammar-tools change", %{graph: g} do
      affected = Graph.affected_nodes(g, MapSet.new(["grammar-tools"]))
      assert MapSet.member?(affected, "grammar-tools")
      assert MapSet.member?(affected, "lexer")
      assert MapSet.member?(affected, "parser")
      assert MapSet.member?(affected, "assembler")
      assert MapSet.member?(affected, "pipeline")
    end

    test "affected by leaf change", %{graph: g} do
      affected = Graph.affected_nodes(g, MapSet.new(["bytecode-compiler"]))
      assert affected == MapSet.new(["bytecode-compiler"])
    end
  end

  # ======================================================================
  # 13. Edge cases
  # ======================================================================

  describe "edge cases" do
    test "integer nodes" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, 1, 2)
      {:ok, g} = Graph.add_edge(g, 2, 3)
      assert Graph.has_edge?(g, 1, 2)
      {:ok, succs} = Graph.successors(g, 1)
      assert succs == [2]
      assert Graph.size(g) == 3
    end

    test "atom nodes" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, :a, :b)
      assert Graph.has_edge?(g, :a, :b)
    end

    test "new/0 defaults" do
      g = Graph.new()
      assert g.allow_self_loops == false
    end

    test "new/1 with allow_self_loops" do
      g = Graph.new(allow_self_loops: true)
      assert g.allow_self_loops == true
    end
  end
end
