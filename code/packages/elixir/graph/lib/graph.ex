defmodule CodingAdventures.Graph do
  @moduledoc """
  DT00 undirected weighted graph.

  The public API is split between this convenience module and the
  `%CodingAdventures.Graph.Graph{}` struct implementation.
  """

  alias CodingAdventures.Graph.Graph

  defdelegate new(opts \\ []), to: Graph
  defdelegate add_node(graph, node), to: Graph
  defdelegate remove_node(graph, node), to: Graph
  defdelegate has_node?(graph, node), to: Graph
  defdelegate nodes(graph), to: Graph
  defdelegate size(graph), to: Graph
  defdelegate add_edge(graph, left, right, weight \\ 1.0), to: Graph
  defdelegate remove_edge(graph, left, right), to: Graph
  defdelegate has_edge?(graph, left, right), to: Graph
  defdelegate edges(graph), to: Graph
  defdelegate edge_weight(graph, left, right), to: Graph
  defdelegate neighbors(graph, node), to: Graph
  defdelegate neighbors_weighted(graph, node), to: Graph
  defdelegate degree(graph, node), to: Graph
  defdelegate bfs(graph, start), to: Graph
  defdelegate dfs(graph, start), to: Graph
  defdelegate is_connected?(graph), to: Graph
  defdelegate connected_components(graph), to: Graph
  defdelegate has_cycle?(graph), to: Graph
  defdelegate shortest_path(graph, start, finish), to: Graph
  defdelegate minimum_spanning_tree(graph), to: Graph
end
