defmodule CodingAdventures.Graph.Graph do
  @moduledoc """
  Immutable undirected weighted graph with adjacency-list and adjacency-matrix
  representations.
  """

  alias CodingAdventures.Graph.{
    EdgeNotFoundError,
    NodeNotFoundError,
    NotConnectedError
  }

  @enforce_keys [:repr, :adj, :node_list, :node_index, :matrix]
  defstruct [
    :repr,
    :adj,
    :node_list,
    :node_index,
    :matrix,
    graph_properties: %{},
    node_properties: %{},
    edge_properties: %{}
  ]

  @type repr_t :: :adjacency_list | :adjacency_matrix
  @type property_value :: String.t() | number() | boolean() | nil
  @type property_bag :: %{optional(String.t()) => property_value()}
  @type t :: %__MODULE__{
          repr: repr_t(),
          adj: %{optional(String.t()) => %{optional(String.t()) => float()}},
          node_list: [String.t()],
          node_index: %{optional(String.t()) => non_neg_integer()},
          matrix: [[float() | nil]],
          graph_properties: property_bag(),
          node_properties: %{optional(String.t()) => property_bag()},
          edge_properties: %{optional({String.t(), String.t()}) => property_bag()}
        }

  @type weighted_edge :: {String.t(), String.t(), float()}

  @spec new(keyword()) :: t()
  def new(opts \\ []) do
    repr =
      opts
      |> Keyword.get(:repr, :adjacency_list)
      |> normalize_repr!()

    %__MODULE__{
      repr: repr,
      adj: %{},
      node_list: [],
      node_index: %{},
      matrix: []
    }
  end

  @spec add_node(t(), String.t(), property_bag()) :: {:ok, t()}
  def add_node(graph, node, properties \\ %{})

  def add_node(%__MODULE__{repr: :adjacency_list, adj: adj} = graph, node, properties) do
    graph = %{
      graph
      | adj: Map.put_new(adj, node, %{}),
        node_properties: merge_property_bag(graph.node_properties, node, properties)
    }

    {:ok, graph}
  end

  def add_node(
        %__MODULE__{repr: :adjacency_matrix, node_index: node_index} = graph,
        node,
        properties
      ) do
    if Map.has_key?(node_index, node) do
      {:ok,
       %{graph | node_properties: merge_property_bag(graph.node_properties, node, properties)}}
    else
      index = length(graph.node_list)
      matrix = Enum.map(graph.matrix, &(&1 ++ [nil])) ++ [List.duplicate(nil, index + 1)]

      {:ok,
       %{
         graph
         | node_list: graph.node_list ++ [node],
           node_index: Map.put(node_index, node, index),
           matrix: matrix,
           node_properties: merge_property_bag(graph.node_properties, node, properties)
       }}
    end
  end

  @spec remove_node(t(), String.t()) :: {:ok, t()} | {:error, NodeNotFoundError.t()}
  def remove_node(%__MODULE__{repr: :adjacency_list, adj: adj} = graph, node) do
    case Map.fetch(adj, node) do
      :error ->
        {:error, %NodeNotFoundError{message: "Node not found: #{inspect(node)}", node: node}}

      {:ok, neighbors} ->
        next_adj =
          neighbors
          |> Map.keys()
          |> Enum.reduce(adj, fn neighbor, acc ->
            Map.update!(acc, neighbor, &Map.delete(&1, node))
          end)
          |> Map.delete(node)

        edge_properties =
          neighbors
          |> Map.keys()
          |> Enum.reduce(graph.edge_properties, fn neighbor, acc ->
            Map.delete(acc, edge_key(node, neighbor))
          end)

        {:ok,
         %{
           graph
           | adj: next_adj,
             node_properties: Map.delete(graph.node_properties, node),
             edge_properties: edge_properties
         }}
    end
  end

  def remove_node(%__MODULE__{repr: :adjacency_matrix} = graph, node) do
    case Map.fetch(graph.node_index, node) do
      :error ->
        {:error, %NodeNotFoundError{message: "Node not found: #{inspect(node)}", node: node}}

      {:ok, index} ->
        node_list = List.delete_at(graph.node_list, index)
        matrix = graph.matrix |> List.delete_at(index) |> Enum.map(&List.delete_at(&1, index))
        node_index = rebuild_index(node_list)

        edge_properties =
          Enum.reduce(graph.node_list, graph.edge_properties, fn other, acc ->
            Map.delete(acc, edge_key(node, other))
          end)

        {:ok,
         %{
           graph
           | node_list: node_list,
             node_index: node_index,
             matrix: matrix,
             node_properties: Map.delete(graph.node_properties, node),
             edge_properties: edge_properties
         }}
    end
  end

  @spec has_node?(t(), String.t()) :: boolean()
  def has_node?(%__MODULE__{repr: :adjacency_list, adj: adj}, node), do: Map.has_key?(adj, node)

  def has_node?(%__MODULE__{repr: :adjacency_matrix, node_index: idx}, node),
    do: Map.has_key?(idx, node)

  @spec nodes(t()) :: [String.t()]
  def nodes(%__MODULE__{repr: :adjacency_list, adj: adj}), do: sort_nodes(Map.keys(adj))
  def nodes(%__MODULE__{repr: :adjacency_matrix, node_list: node_list}), do: sort_nodes(node_list)

  @spec size(t()) :: non_neg_integer()
  def size(%__MODULE__{repr: :adjacency_list, adj: adj}), do: map_size(adj)
  def size(%__MODULE__{repr: :adjacency_matrix, node_list: node_list}), do: length(node_list)

  @spec add_edge(t(), String.t(), String.t(), float(), property_bag()) :: {:ok, t()}
  def add_edge(graph, left, right, weight \\ 1.0, properties \\ %{}) do
    {:ok, graph} = add_node(graph, left)
    {:ok, graph} = add_node(graph, right)
    edge_properties = put_edge_properties(graph.edge_properties, left, right, weight, properties)

    case graph.repr do
      :adjacency_list ->
        adj =
          graph.adj
          |> put_in([left, right], weight)
          |> put_in([right, left], weight)

        {:ok, %{graph | adj: adj, edge_properties: edge_properties}}

      :adjacency_matrix ->
        left_index = Map.fetch!(graph.node_index, left)
        right_index = Map.fetch!(graph.node_index, right)

        matrix =
          graph.matrix
          |> replace_cell(left_index, right_index, weight)
          |> replace_cell(right_index, left_index, weight)

        {:ok, %{graph | matrix: matrix, edge_properties: edge_properties}}
    end
  end

  @spec remove_edge(t(), String.t(), String.t()) :: {:ok, t()} | {:error, EdgeNotFoundError.t()}
  def remove_edge(graph, left, right) do
    if has_edge?(graph, left, right) do
      case graph.repr do
        :adjacency_list ->
          adj =
            graph.adj
            |> update_in([left], &Map.delete(&1, right))
            |> update_in([right], &Map.delete(&1, left))

          {:ok,
           %{
             graph
             | adj: adj,
               edge_properties: Map.delete(graph.edge_properties, edge_key(left, right))
           }}

        :adjacency_matrix ->
          left_index = Map.fetch!(graph.node_index, left)
          right_index = Map.fetch!(graph.node_index, right)

          matrix =
            graph.matrix
            |> replace_cell(left_index, right_index, nil)
            |> replace_cell(right_index, left_index, nil)

          {:ok,
           %{
             graph
             | matrix: matrix,
               edge_properties: Map.delete(graph.edge_properties, edge_key(left, right))
           }}
      end
    else
      {:error,
       %EdgeNotFoundError{
         message: "Edge not found: #{inspect(left)} -- #{inspect(right)}",
         left: left,
         right: right
       }}
    end
  end

  @spec has_edge?(t(), String.t(), String.t()) :: boolean()
  def has_edge?(%__MODULE__{repr: :adjacency_list, adj: adj}, left, right) do
    adj |> Map.get(left, %{}) |> Map.has_key?(right)
  end

  def has_edge?(
        %__MODULE__{repr: :adjacency_matrix, node_index: idx, matrix: matrix},
        left,
        right
      ) do
    with {:ok, left_index} <- Map.fetch(idx, left),
         {:ok, right_index} <- Map.fetch(idx, right) do
      get_in(matrix, [Access.at(left_index), Access.at(right_index)]) != nil
    else
      :error -> false
    end
  end

  @spec edges(t()) :: [weighted_edge()]
  def edges(%__MODULE__{repr: :adjacency_list, adj: adj}) do
    adj
    |> Enum.reduce({MapSet.new(), []}, fn {left, neighbors}, {seen, acc} ->
      Enum.reduce(neighbors, {seen, acc}, fn {right, weight}, {inner_seen, inner_acc} ->
        {first, second} = canonical_endpoints(left, right)
        key = {first, second}

        if MapSet.member?(inner_seen, key) do
          {inner_seen, inner_acc}
        else
          {MapSet.put(inner_seen, key), [{first, second, weight} | inner_acc]}
        end
      end)
    end)
    |> elem(1)
    |> Enum.sort_by(fn {left, right, weight} -> {weight, sort_key(left), sort_key(right)} end)
  end

  def edges(%__MODULE__{repr: :adjacency_matrix, node_list: node_list, matrix: matrix}) do
    if node_list == [] do
      []
    else
      0..(length(node_list) - 1)
      |> Enum.flat_map(fn row ->
        row..(length(node_list) - 1)
        |> Enum.reduce([], fn col, acc ->
          case get_in(matrix, [Access.at(row), Access.at(col)]) do
            nil -> acc
            weight -> [{Enum.at(node_list, row), Enum.at(node_list, col), weight} | acc]
          end
        end)
      end)
      |> Enum.sort_by(fn {left, right, weight} -> {weight, sort_key(left), sort_key(right)} end)
    end
  end

  @spec edge_weight(t(), String.t(), String.t()) ::
          {:ok, float()} | {:error, EdgeNotFoundError.t()}
  def edge_weight(%__MODULE__{repr: :adjacency_list, adj: adj}, left, right) do
    case adj |> Map.get(left, %{}) |> Map.fetch(right) do
      {:ok, weight} -> {:ok, weight}
      :error -> edge_not_found(left, right)
    end
  end

  def edge_weight(
        %__MODULE__{repr: :adjacency_matrix, node_index: idx, matrix: matrix},
        left,
        right
      ) do
    with {:ok, left_index} <- Map.fetch(idx, left),
         {:ok, right_index} <- Map.fetch(idx, right),
         weight when not is_nil(weight) <-
           get_in(matrix, [Access.at(left_index), Access.at(right_index)]) do
      {:ok, weight}
    else
      _ -> edge_not_found(left, right)
    end
  end

  @spec graph_properties(t()) :: property_bag()
  def graph_properties(%__MODULE__{graph_properties: properties}), do: Map.new(properties)

  @spec set_graph_property(t(), String.t(), property_value()) :: {:ok, t()}
  def set_graph_property(%__MODULE__{} = graph, key, value) do
    {:ok, %{graph | graph_properties: Map.put(graph.graph_properties, key, value)}}
  end

  @spec remove_graph_property(t(), String.t()) :: {:ok, t()}
  def remove_graph_property(%__MODULE__{} = graph, key) do
    {:ok, %{graph | graph_properties: Map.delete(graph.graph_properties, key)}}
  end

  @spec node_properties(t(), String.t()) ::
          {:ok, property_bag()} | {:error, NodeNotFoundError.t()}
  def node_properties(graph, node) do
    if has_node?(graph, node) do
      {:ok, Map.new(Map.get(graph.node_properties, node, %{}))}
    else
      node_not_found(node)
    end
  end

  @spec set_node_property(t(), String.t(), String.t(), property_value()) ::
          {:ok, t()} | {:error, NodeNotFoundError.t()}
  def set_node_property(graph, node, key, value) do
    if has_node?(graph, node) do
      {:ok,
       %{
         graph
         | node_properties: merge_property_bag(graph.node_properties, node, %{key => value})
       }}
    else
      node_not_found(node)
    end
  end

  @spec remove_node_property(t(), String.t(), String.t()) ::
          {:ok, t()} | {:error, NodeNotFoundError.t()}
  def remove_node_property(graph, node, key) do
    if has_node?(graph, node) do
      node_properties =
        Map.update(graph.node_properties, node, %{}, fn properties ->
          Map.delete(properties, key)
        end)

      {:ok, %{graph | node_properties: node_properties}}
    else
      node_not_found(node)
    end
  end

  @spec edge_properties(t(), String.t(), String.t()) ::
          {:ok, property_bag()} | {:error, EdgeNotFoundError.t()}
  def edge_properties(graph, left, right) do
    case edge_weight(graph, left, right) do
      {:ok, weight} ->
        properties =
          graph.edge_properties
          |> Map.get(edge_key(left, right), %{})
          |> Map.put("weight", weight)

        {:ok, properties}

      error ->
        error
    end
  end

  @spec set_edge_property(t(), String.t(), String.t(), String.t(), property_value()) ::
          {:ok, t()} | {:error, EdgeNotFoundError.t()}
  def set_edge_property(graph, left, right, "weight", value) when is_number(value) do
    with {:ok, graph} <- set_edge_weight(graph, left, right, value) do
      {:ok,
       %{
         graph
         | edge_properties:
             put_edge_properties(graph.edge_properties, left, right, value, %{"weight" => value})
       }}
    end
  end

  def set_edge_property(_graph, _left, _right, "weight", _value) do
    raise ArgumentError, "edge property \"weight\" must be numeric"
  end

  def set_edge_property(graph, left, right, key, value) do
    if has_edge?(graph, left, right) do
      edge_properties =
        Map.update(
          graph.edge_properties,
          edge_key(left, right),
          %{key => value},
          &Map.put(&1, key, value)
        )

      {:ok, %{graph | edge_properties: edge_properties}}
    else
      edge_not_found(left, right)
    end
  end

  @spec remove_edge_property(t(), String.t(), String.t(), String.t()) ::
          {:ok, t()} | {:error, EdgeNotFoundError.t()}
  def remove_edge_property(graph, left, right, "weight") do
    with {:ok, graph} <- set_edge_weight(graph, left, right, 1.0) do
      {:ok,
       %{
         graph
         | edge_properties:
             put_edge_properties(graph.edge_properties, left, right, 1.0, %{"weight" => 1.0})
       }}
    end
  end

  def remove_edge_property(graph, left, right, key) do
    if has_edge?(graph, left, right) do
      edge_properties =
        Map.update(graph.edge_properties, edge_key(left, right), %{}, fn properties ->
          Map.delete(properties, key)
        end)

      {:ok, %{graph | edge_properties: edge_properties}}
    else
      edge_not_found(left, right)
    end
  end

  @spec neighbors(t(), String.t()) :: {:ok, [String.t()]} | {:error, NodeNotFoundError.t()}
  def neighbors(%__MODULE__{repr: :adjacency_list, adj: adj}, node) do
    case Map.fetch(adj, node) do
      {:ok, neighbors} -> {:ok, sort_nodes(Map.keys(neighbors))}
      :error -> node_not_found(node)
    end
  end

  def neighbors(
        %__MODULE__{
          repr: :adjacency_matrix,
          node_index: idx,
          node_list: node_list,
          matrix: matrix
        },
        node
      ) do
    case Map.fetch(idx, node) do
      {:ok, index} ->
        result =
          matrix
          |> Enum.at(index)
          |> Enum.with_index()
          |> Enum.reduce([], fn
            {nil, _col}, acc -> acc
            {_weight, col}, acc -> [Enum.at(node_list, col) | acc]
          end)
          |> sort_nodes()

        {:ok, result}

      :error ->
        node_not_found(node)
    end
  end

  @spec neighbors_weighted(t(), String.t()) :: {:ok, map()} | {:error, NodeNotFoundError.t()}
  def neighbors_weighted(%__MODULE__{repr: :adjacency_list, adj: adj}, node) do
    case Map.fetch(adj, node) do
      {:ok, neighbors} -> {:ok, neighbors}
      :error -> node_not_found(node)
    end
  end

  def neighbors_weighted(
        %__MODULE__{
          repr: :adjacency_matrix,
          node_index: idx,
          node_list: node_list,
          matrix: matrix
        },
        node
      ) do
    case Map.fetch(idx, node) do
      {:ok, index} ->
        weights =
          matrix
          |> Enum.at(index)
          |> Enum.with_index()
          |> Enum.reduce(%{}, fn
            {nil, _col}, acc -> acc
            {weight, col}, acc -> Map.put(acc, Enum.at(node_list, col), weight)
          end)

        {:ok, weights}

      :error ->
        node_not_found(node)
    end
  end

  @spec degree(t(), String.t()) :: {:ok, non_neg_integer()} | {:error, NodeNotFoundError.t()}
  def degree(graph, node) do
    case neighbors(graph, node) do
      {:ok, nodes} -> {:ok, length(nodes)}
      error -> error
    end
  end

  @spec bfs(t(), String.t()) :: {:ok, [String.t()]} | {:error, NodeNotFoundError.t()}
  def bfs(graph, start) do
    if has_node?(graph, start) do
      {:ok, do_bfs(graph, [start], MapSet.new([start]), [])}
    else
      node_not_found(start)
    end
  end

  @spec dfs(t(), String.t()) :: {:ok, [String.t()]} | {:error, NodeNotFoundError.t()}
  def dfs(graph, start) do
    if has_node?(graph, start) do
      {:ok, do_dfs(graph, [start], MapSet.new(), [])}
    else
      node_not_found(start)
    end
  end

  @spec is_connected?(t()) :: boolean()
  def is_connected?(graph) do
    case nodes(graph) do
      [] ->
        true

      [first | _] ->
        case bfs(graph, first) do
          {:ok, visited} -> length(visited) == size(graph)
          _ -> false
        end
    end
  end

  @spec connected_components(t()) :: [[String.t()]]
  def connected_components(graph) do
    graph
    |> nodes()
    |> do_components(graph, MapSet.new(), [])
    |> Enum.reverse()
  end

  @spec has_cycle?(t()) :: boolean()
  def has_cycle?(graph) do
    nodes(graph)
    |> Enum.reduce_while(MapSet.new(), fn start, visited ->
      if MapSet.member?(visited, start) do
        {:cont, visited}
      else
        if visit_cycle?(graph, start, nil, visited) do
          {:halt, :cycle}
        else
          {:cont, mark_component(graph, start, visited)}
        end
      end
    end) == :cycle
  end

  @spec shortest_path(t(), String.t(), String.t()) :: [String.t()]
  def shortest_path(graph, start, finish) do
    cond do
      not has_node?(graph, start) or not has_node?(graph, finish) ->
        []

      start == finish ->
        [start]

      Enum.all?(edges(graph), fn {_l, _r, weight} -> weight == 1.0 end) ->
        bfs_shortest_path(graph, start, finish)

      true ->
        dijkstra_shortest_path(graph, start, finish)
    end
  end

  @spec minimum_spanning_tree(t()) :: {:ok, [weighted_edge()]} | {:error, NotConnectedError.t()}
  def minimum_spanning_tree(graph) do
    cond do
      size(graph) <= 1 or edges(graph) == [] ->
        {:ok, []}

      not is_connected?(graph) ->
        {:error, %NotConnectedError{}}

      true ->
        result =
          Enum.reduce(edges(graph), {new_union_find(nodes(graph)), []}, fn {left, right, _weight} =
                                                                             edge,
                                                                           {uf, acc} ->
            if find(uf, left) == find(uf, right) do
              {uf, acc}
            else
              {union(uf, left, right), [edge | acc]}
            end
          end)
          |> elem(1)
          |> Enum.reverse()

        {:ok, result}
    end
  end

  defp do_bfs(_graph, [], _visited, acc), do: Enum.reverse(acc)

  defp do_bfs(graph, [node | rest], visited, acc) do
    {:ok, neighbors} = neighbors(graph, node)

    {next_queue, next_visited} =
      Enum.reduce(neighbors, {rest, visited}, fn neighbor, {queue_acc, visited_acc} ->
        if MapSet.member?(visited_acc, neighbor) do
          {queue_acc, visited_acc}
        else
          {queue_acc ++ [neighbor], MapSet.put(visited_acc, neighbor)}
        end
      end)

    do_bfs(graph, next_queue, next_visited, [node | acc])
  end

  defp do_dfs(_graph, [], _visited, acc), do: Enum.reverse(acc)

  defp do_dfs(graph, [node | rest], visited, acc) do
    if MapSet.member?(visited, node) do
      do_dfs(graph, rest, visited, acc)
    else
      {:ok, neighbors} = neighbors(graph, node)
      do_dfs(graph, neighbors ++ rest, MapSet.put(visited, node), [node | acc])
    end
  end

  defp do_components([], _graph, _visited, acc), do: acc

  defp do_components([node | rest], graph, visited, acc) do
    if MapSet.member?(visited, node) do
      do_components(rest, graph, visited, acc)
    else
      {:ok, component} = bfs(graph, node)
      component_set = MapSet.new(component)
      do_components(rest, graph, MapSet.union(visited, component_set), [component | acc])
    end
  end

  defp visit_cycle?(graph, node, parent, visited) do
    {:ok, next_nodes} = neighbors(graph, node)
    visited = MapSet.put(visited, node)

    Enum.any?(next_nodes, fn neighbor ->
      cond do
        not MapSet.member?(visited, neighbor) -> visit_cycle?(graph, neighbor, node, visited)
        neighbor != parent -> true
        true -> false
      end
    end)
  end

  defp mark_component(graph, start, visited) do
    {:ok, component} = bfs(graph, start)
    Enum.reduce(component, visited, &MapSet.put(&2, &1))
  end

  defp bfs_shortest_path(graph, start, finish) do
    queue = [start]
    parents = %{start => nil}
    bfs_path(queue, parents, graph, finish)
  end

  defp bfs_path([], _parents, _graph, _finish), do: []

  defp bfs_path([node | rest], parents, graph, finish) do
    if node == finish do
      reconstruct_path(parents, finish)
    else
      {:ok, next_nodes} = neighbors(graph, node)

      {queue, parents} =
        Enum.reduce(next_nodes, {rest, parents}, fn neighbor, {queue_acc, parents_acc} ->
          if Map.has_key?(parents_acc, neighbor) do
            {queue_acc, parents_acc}
          else
            {queue_acc ++ [neighbor], Map.put(parents_acc, neighbor, node)}
          end
        end)

      bfs_path(queue, parents, graph, finish)
    end
  end

  defp dijkstra_shortest_path(graph, start, finish) do
    distances = Map.new(nodes(graph), &{&1, :infinity}) |> Map.put(start, 0.0)
    queue = [{0.0, start}]
    parents = %{}
    do_dijkstra(queue, distances, parents, graph, start, finish)
  end

  defp do_dijkstra([], distances, _parents, _graph, _start, finish) do
    if Map.get(distances, finish) == :infinity, do: [], else: []
  end

  defp do_dijkstra(queue, distances, parents, graph, start, finish) do
    [{distance, node} | rest] = Enum.sort_by(queue, fn {d, n} -> {d, n} end)

    cond do
      distance > Map.get(distances, node, :infinity) ->
        do_dijkstra(rest, distances, parents, graph, start, finish)

      node == finish ->
        reconstruct_path(parents, finish, start)

      true ->
        {:ok, weighted} = neighbors_weighted(graph, node)

        {next_queue, next_distances, next_parents} =
          Enum.reduce(weighted, {rest, distances, parents}, fn {neighbor, weight},
                                                               {queue_acc, distances_acc,
                                                                parents_acc} ->
            new_distance = distance + weight
            current_distance = Map.get(distances_acc, neighbor, :infinity)

            if current_distance == :infinity or new_distance < current_distance do
              {
                [{new_distance, neighbor} | queue_acc],
                Map.put(distances_acc, neighbor, new_distance),
                Map.put(parents_acc, neighbor, node)
              }
            else
              {queue_acc, distances_acc, parents_acc}
            end
          end)

        do_dijkstra(next_queue, next_distances, next_parents, graph, start, finish)
    end
  end

  defp reconstruct_path(parents, finish), do: reconstruct_path(parents, finish, nil)

  defp reconstruct_path(parents, finish, start) do
    path =
      Stream.unfold(finish, fn
        nil -> nil
        node -> {node, Map.get(parents, node)}
      end)
      |> Enum.to_list()
      |> Enum.reverse()

    cond do
      start == nil -> path
      path == [] -> []
      hd(path) == start -> path
      true -> []
    end
  end

  defp new_union_find(nodes) do
    parents = Map.new(nodes, &{&1, &1})
    ranks = Map.new(nodes, &{&1, 0})
    %{parents: parents, ranks: ranks}
  end

  defp find(uf, node) do
    parent = Map.fetch!(uf.parents, node)
    if parent == node, do: parent, else: find(uf, parent)
  end

  defp union(uf, left, right) do
    left_root = find(uf, left)
    right_root = find(uf, right)

    if left_root == right_root do
      uf
    else
      left_rank = Map.get(uf.ranks, left_root, 0)
      right_rank = Map.get(uf.ranks, right_root, 0)

      {parent_root, child_root} =
        if left_rank < right_rank do
          {right_root, left_root}
        else
          {left_root, right_root}
        end

      ranks =
        if left_rank == right_rank do
          Map.update!(uf.ranks, parent_root, &(&1 + 1))
        else
          uf.ranks
        end

      %{uf | parents: Map.put(uf.parents, child_root, parent_root), ranks: ranks}
    end
  end

  defp replace_cell(matrix, row, col, value) do
    List.update_at(matrix, row, fn values -> List.replace_at(values, col, value) end)
  end

  defp rebuild_index(node_list) do
    node_list
    |> Enum.with_index()
    |> Map.new(fn {node, index} -> {node, index} end)
  end

  defp canonical_endpoints(left, right) do
    if sort_key(left) <= sort_key(right), do: {left, right}, else: {right, left}
  end

  defp edge_key(left, right), do: canonical_endpoints(left, right)

  defp merge_property_bag(property_map, owner, properties) do
    Map.update(property_map, owner, Map.new(properties), &Map.merge(&1, properties))
  end

  defp put_edge_properties(edge_properties, left, right, weight, properties) do
    key = edge_key(left, right)

    Map.update(
      edge_properties,
      key,
      Map.put(Map.new(properties), "weight", weight),
      fn existing -> existing |> Map.merge(properties) |> Map.put("weight", weight) end
    )
  end

  defp set_edge_weight(graph, left, right, weight) do
    if has_edge?(graph, left, right) do
      case graph.repr do
        :adjacency_list ->
          adj =
            graph.adj
            |> put_in([left, right], weight)
            |> put_in([right, left], weight)

          {:ok, %{graph | adj: adj}}

        :adjacency_matrix ->
          left_index = Map.fetch!(graph.node_index, left)
          right_index = Map.fetch!(graph.node_index, right)

          matrix =
            graph.matrix
            |> replace_cell(left_index, right_index, weight)
            |> replace_cell(right_index, left_index, weight)

          {:ok, %{graph | matrix: matrix}}
      end
    else
      edge_not_found(left, right)
    end
  end

  defp sort_nodes(nodes), do: Enum.sort_by(nodes, &sort_key/1)
  defp sort_key(node), do: "#{node}"

  defp normalize_repr!(:adjacency_list), do: :adjacency_list
  defp normalize_repr!(:adjacency_matrix), do: :adjacency_matrix
  defp normalize_repr!("adjacency_list"), do: :adjacency_list
  defp normalize_repr!("adjacency_matrix"), do: :adjacency_matrix

  defp normalize_repr!(other),
    do:
      raise(
        ArgumentError,
        "repr must be :adjacency_list or :adjacency_matrix, got: #{inspect(other)}"
      )

  defp node_not_found(node),
    do: {:error, %NodeNotFoundError{message: "Node not found: #{inspect(node)}", node: node}}

  defp edge_not_found(left, right) do
    {:error,
     %EdgeNotFoundError{
       message: "Edge not found: #{inspect(left)} -- #{inspect(right)}",
       left: left,
       right: right
     }}
  end
end
