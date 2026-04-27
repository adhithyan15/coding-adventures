defmodule CodingAdventures.GraphTest do
  use ExUnit.Case, async: true
  doctest CodingAdventures.Graph

  alias CodingAdventures.Graph

  describe "construction" do
    test "new graph is empty" do
      g = Graph.new()
      assert Graph.size(g) == 0
      assert Graph.nodes(g) == []
      assert Graph.edges(g) == []
    end
  end

  describe "node operations" do
    test "add node" do
      g = Graph.new()
      g = Graph.add_node(g, "A")
      assert Graph.has_node?(g, "A")
      assert Graph.size(g) == 1
    end

    test "add multiple nodes" do
      g = Graph.new()
      g = g |> Graph.add_node("A") |> Graph.add_node("B") |> Graph.add_node("C")
      assert Graph.size(g) == 3
      assert Enum.sort(Graph.nodes(g)) == ["A", "B", "C"]
    end

    test "duplicate node is noop" do
      g = Graph.new()
      g = g |> Graph.add_node("A") |> Graph.add_node("A")
      assert Graph.size(g) == 1
    end

    test "remove node" do
      g = Graph.new()
      g = Graph.add_node(g, "A")
      {:ok, g} = Graph.remove_node(g, "A")
      assert Graph.size(g) == 0
      assert not Graph.has_node?(g, "A")
    end

    test "remove node with edges" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.add_edge(g, "A", "C")
      {:ok, g} = Graph.remove_node(g, "A")
      assert not Graph.has_node?(g, "A")
      assert Graph.has_node?(g, "B")
      assert Graph.has_node?(g, "C")
      assert length(Graph.edges(g)) == 0
    end

    test "remove nonexistent node returns error" do
      g = Graph.new()
      assert {:error, _} = Graph.remove_node(g, "X")
    end
  end

  describe "edge operations" do
    test "add edge creates nodes" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      assert Graph.has_node?(g, "A")
      assert Graph.has_node?(g, "B")
    end

    test "edge exists in both directions" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      assert Graph.has_edge?(g, "A", "B")
      assert Graph.has_edge?(g, "B", "A")
    end

    test "get edges" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.add_edge(g, "B", "C")
      edges = Graph.edges(g)
      assert length(edges) == 2
    end

    test "add weighted edge" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B", 2.5)
      {:ok, w} = Graph.edge_weight(g, "A", "B")
      assert w == 2.5
    end

    test "remove edge" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.remove_edge(g, "A", "B")
      assert not Graph.has_edge?(g, "A", "B")
    end

    test "remove nonexistent edge returns error" do
      g = Graph.new()
      assert {:error, _} = Graph.remove_edge(g, "X", "Y")
    end
  end

  describe "neighborhood" do
    test "neighbors" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.add_edge(g, "A", "C")
      {:ok, g} = Graph.add_edge(g, "A", "D")

      {:ok, neighbors} = Graph.neighbors(g, "A")
      assert Enum.sort(neighbors) == ["B", "C", "D"]
    end

    test "neighbors weighted" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B", 1.0)
      {:ok, g} = Graph.add_edge(g, "A", "C", 2.0)
      {:ok, g} = Graph.add_edge(g, "A", "D", 3.0)

      {:ok, nw} = Graph.neighbors_weighted(g, "A")
      assert nw["B"] == 1.0
      assert nw["C"] == 2.0
      assert nw["D"] == 3.0
    end

    test "degree" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.add_edge(g, "A", "C")
      {:ok, g} = Graph.add_edge(g, "A", "D")

      {:ok, degree} = Graph.degree(g, "A")
      assert degree == 3

      {:ok, degree} = Graph.degree(g, "B")
      assert degree == 1
    end

    test "neighbors of nonexistent node returns error" do
      g = Graph.new()
      assert {:error, _} = Graph.neighbors(g, "X")
    end
  end

  describe "BFS" do
    test "simple path" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.add_edge(g, "B", "C")
      {:ok, g} = Graph.add_edge(g, "C", "D")

      result = Graph.bfs(g, "A")
      assert length(result) == 4
      assert List.first(result) == "A"
    end

    test "tree with multiple branches" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.add_edge(g, "A", "C")
      {:ok, g} = Graph.add_edge(g, "B", "D")
      {:ok, g} = Graph.add_edge(g, "B", "E")

      result = Graph.bfs(g, "A")
      assert length(result) == 5
    end

    test "disconnected graph" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.add_edge(g, "C", "D")

      result = Graph.bfs(g, "A")
      assert length(result) == 2
    end
  end

  describe "DFS" do
    test "simple path" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.add_edge(g, "B", "D")
      {:ok, g} = Graph.add_edge(g, "A", "C")

      result = Graph.dfs(g, "A")
      assert length(result) == 4
      assert List.first(result) == "A"
    end
  end

  describe "shortest_path" do
    test "unweighted graph" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.add_edge(g, "B", "C")
      {:ok, g} = Graph.add_edge(g, "C", "D")

      path = Graph.shortest_path(g, "A", "D")
      assert length(path) == 4
    end

    test "same start and end" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")

      path = Graph.shortest_path(g, "A", "A")
      assert path == ["A"]
    end

    test "no path" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      g = Graph.add_node(g, "X")

      path = Graph.shortest_path(g, "A", "X")
      assert path == []
    end

    test "weighted graph (Dijkstra)" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B", 1.0)
      {:ok, g} = Graph.add_edge(g, "B", "D", 10.0)
      {:ok, g} = Graph.add_edge(g, "A", "C", 3.0)
      {:ok, g} = Graph.add_edge(g, "C", "D", 3.0)

      path = Graph.shortest_path(g, "A", "D")
      assert length(path) == 3
    end
  end

  describe "cycle_detection" do
    test "no cycle in path" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.add_edge(g, "B", "C")
      assert not Graph.has_cycle?(g)
    end

    test "cycle in triangle" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.add_edge(g, "B", "C")
      {:ok, g} = Graph.add_edge(g, "C", "A")
      assert Graph.has_cycle?(g)
    end

    test "no cycle in single node" do
      g = Graph.new()
      g = Graph.add_node(g, "A")
      assert not Graph.has_cycle?(g)
    end

    test "no cycle in two-node graph" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      assert not Graph.has_cycle?(g)
    end
  end

  describe "connected_components" do
    test "three components" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.add_edge(g, "C", "D")
      g = Graph.add_node(g, "E")

      components = Graph.connected_components(g)
      assert length(components) == 3

      sizes = components |> Enum.map(&length/1) |> Enum.sort()
      assert sizes == [1, 2, 2]
    end
  end

  describe "is_connected" do
    test "empty graph is connected" do
      g = Graph.new()
      assert Graph.is_connected?(g)
    end

    test "single node is connected" do
      g = Graph.new()
      g = Graph.add_node(g, "A")
      assert Graph.is_connected?(g)
    end

    test "two connected nodes" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      assert Graph.is_connected?(g)
    end

    test "disconnected graph" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      g = Graph.add_node(g, "C")
      assert not Graph.is_connected?(g)
    end

    test "connected after adding edge" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      g = Graph.add_node(g, "C")
      {:ok, g} = Graph.add_edge(g, "B", "C")
      assert Graph.is_connected?(g)
    end
  end

  describe "minimum_spanning_tree" do
    test "simple MST" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B", 1.0)
      {:ok, g} = Graph.add_edge(g, "B", "C", 2.0)
      {:ok, g} = Graph.add_edge(g, "C", "A", 3.0)

      mst = Graph.minimum_spanning_tree(g)
      assert mst != nil
      assert length(mst) == 2

      total_weight = mst |> Enum.map(fn {_u, _v, w} -> w end) |> Enum.sum()
      assert total_weight == 3.0
    end

    test "disconnected graph returns nil" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.add_edge(g, "C", "D")
      mst = Graph.minimum_spanning_tree(g)
      assert mst == nil
    end

    test "single node" do
      g = Graph.new()
      g = Graph.add_node(g, "A")
      mst = Graph.minimum_spanning_tree(g)
      assert mst == []
    end

    test "empty graph" do
      g = Graph.new()
      mst = Graph.minimum_spanning_tree(g)
      assert mst == []
    end
  end

  describe "edge_weight" do
    test "get weight" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B", 2.5)
      {:ok, w} = Graph.edge_weight(g, "A", "B")
      assert w == 2.5
    end

    test "nonexistent edge returns error" do
      g = Graph.new()
      assert {:error, _} = Graph.edge_weight(g, "A", "C")
    end

    test "symmetric (undirected)" do
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B", 2.5)
      {:ok, w} = Graph.edge_weight(g, "B", "A")
      assert w == 2.5
    end
  end
end
