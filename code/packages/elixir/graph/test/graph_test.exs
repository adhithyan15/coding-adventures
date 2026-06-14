defmodule CodingAdventures.Graph.GraphTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Graph
  alias CodingAdventures.Graph.{EdgeNotFoundError, NodeNotFoundError, NotConnectedError}

  defp reprs, do: [:adjacency_list, :adjacency_matrix]

  defp make_graph(repr) do
    {:ok, graph} = Graph.new(repr: repr) |> Graph.add_edge("London", "Paris", 300.0)
    {:ok, graph} = Graph.add_edge(graph, "London", "Amsterdam", 520.0)
    {:ok, graph} = Graph.add_edge(graph, "Paris", "Berlin", 878.0)
    {:ok, graph} = Graph.add_edge(graph, "Amsterdam", "Berlin", 655.0)
    {:ok, graph} = Graph.add_edge(graph, "Amsterdam", "Brussels", 180.0)
    graph
  end

  defp make_path(repr) do
    {:ok, graph} = Graph.new(repr: repr) |> Graph.add_edge("A", "B")
    {:ok, graph} = Graph.add_edge(graph, "B", "C")
    graph
  end

  test "node and edge operations work in both representations" do
    for repr <- reprs() do
      {:ok, graph} = Graph.new(repr: repr) |> Graph.add_node("A")
      {:ok, graph} = Graph.add_edge(graph, "A", "B", 2.5)

      assert Graph.has_node?(graph, "A")
      assert Graph.has_edge?(graph, "A", "B")
      assert Graph.has_edge?(graph, "B", "A")
      assert Graph.nodes(graph) == ["A", "B"]
      assert Graph.edges(graph) == [{"A", "B", 2.5}]
      assert {:ok, 2.5} = Graph.edge_weight(graph, "A", "B")
      assert {:ok, 1} = Graph.degree(graph, "A")
    end
  end

  test "neighbors and weighted lookups stay sorted and weighted" do
    for repr <- reprs() do
      graph = make_graph(repr)
      assert {:ok, ["Berlin", "Brussels", "London"]} = Graph.neighbors(graph, "Amsterdam")

      assert {:ok, weights} = Graph.neighbors_weighted(graph, "Amsterdam")
      assert weights == %{"Berlin" => 655.0, "Brussels" => 180.0, "London" => 520.0}
    end
  end

  test "bfs dfs connectivity and cycle detection match the spec" do
    for repr <- reprs() do
      graph = make_graph(repr)

      assert {:ok, ["London", "Amsterdam", "Paris", "Berlin", "Brussels"]} =
               Graph.bfs(graph, "London")

      assert {:ok, ["London", "Amsterdam", "Berlin", "Paris", "Brussels"]} =
               Graph.dfs(graph, "London")

      assert Graph.is_connected?(graph)
      assert Graph.has_cycle?(graph)
      refute Graph.has_cycle?(make_path(repr))
    end
  end

  test "components shortest path and mst work in both representations" do
    for repr <- reprs() do
      graph = make_graph(repr)
      assert Graph.shortest_path(graph, "London", "Berlin") == ["London", "Amsterdam", "Berlin"]

      {:ok, mst} = Graph.minimum_spanning_tree(graph)
      assert length(mst) == Graph.size(graph) - 1
      assert Enum.sum(Enum.map(mst, fn {_l, _r, weight} -> weight end)) == 1655.0

      {:ok, other} = Graph.new(repr: repr) |> Graph.add_edge("A", "B")
      {:ok, other} = Graph.add_edge(other, "B", "C")
      {:ok, other} = Graph.add_edge(other, "D", "E")
      components = Graph.connected_components(other)
      assert ["A", "B", "C"] in components
      assert ["D", "E"] in components
    end
  end

  test "disconnected mst and missing lookups return structured errors" do
    for repr <- reprs() do
      {:ok, graph} = Graph.new(repr: repr) |> Graph.add_edge("A", "B")
      {:ok, graph} = Graph.add_node(graph, "C")

      assert {:error, %NotConnectedError{}} = Graph.minimum_spanning_tree(graph)
      assert {:error, %NodeNotFoundError{node: "missing"}} = Graph.neighbors(graph, "missing")
      assert {:error, %EdgeNotFoundError{}} = Graph.remove_edge(graph, "A", "C")
    end
  end

  test "removing edges and nodes leaves the remaining graph intact" do
    for repr <- reprs() do
      graph = make_path(repr)
      {:ok, graph} = Graph.remove_edge(graph, "A", "B")
      refute Graph.has_edge?(graph, "A", "B")
      assert Graph.has_node?(graph, "A")
      assert Graph.has_node?(graph, "B")

      {:ok, graph} = Graph.remove_node(graph, "B")
      refute Graph.has_node?(graph, "B")
      assert Graph.nodes(graph) == ["A", "C"]
    end
  end

  test "property bags track graph node and edge metadata" do
    for repr <- reprs() do
      graph = Graph.new(repr: repr)

      {:ok, graph} = Graph.set_graph_property(graph, "name", "city-map")
      {:ok, graph} = Graph.set_graph_property(graph, "version", 1)
      assert Graph.graph_properties(graph) == %{"name" => "city-map", "version" => 1}
      {:ok, graph} = Graph.remove_graph_property(graph, "version")
      assert Graph.graph_properties(graph) == %{"name" => "city-map"}

      {:ok, graph} = Graph.add_node(graph, "A", %{"kind" => "input"})
      {:ok, graph} = Graph.add_node(graph, "A", %{"trainable" => false})
      {:ok, graph} = Graph.set_node_property(graph, "A", "slot", 0)

      assert {:ok, %{"kind" => "input", "trainable" => false, "slot" => 0}} =
               Graph.node_properties(graph, "A")

      {:ok, graph} = Graph.remove_node_property(graph, "A", "slot")
      assert {:ok, %{"kind" => "input", "trainable" => false}} = Graph.node_properties(graph, "A")

      {:ok, graph} = Graph.add_edge(graph, "A", "B", 2.5, %{"role" => "distance"})

      assert {:ok, %{"role" => "distance", "weight" => 2.5}} =
               Graph.edge_properties(graph, "B", "A")

      {:ok, graph} = Graph.set_edge_property(graph, "B", "A", "weight", 7.0)
      assert {:ok, 7.0} = Graph.edge_weight(graph, "A", "B")
      {:ok, graph} = Graph.set_edge_property(graph, "A", "B", "trainable", true)
      {:ok, graph} = Graph.remove_edge_property(graph, "A", "B", "role")

      assert {:ok, %{"weight" => 7.0, "trainable" => true}} =
               Graph.edge_properties(graph, "A", "B")

      {:ok, graph} = Graph.remove_edge(graph, "A", "B")
      assert {:error, %EdgeNotFoundError{}} = Graph.edge_properties(graph, "A", "B")
    end
  end
end
