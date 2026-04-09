defmodule CodingAdventures.Graph do
  @moduledoc """
  Undirected Graph Library for Elixir
  ===================================

  An undirected graph data structure with weighted edges, supporting both
  adjacency list and adjacency matrix representations with comprehensive
  graph algorithms.

  A graph G = (V, E) is a pair of sets:
    - V (vertices/nodes): any term that can be stored in Elixir
    - E (edges): unordered pairs {u, v} with optional weights (default 1.0)

  Since edges are unordered, {u,v} == {v,u}. Think of it like a two-way street
  map: if you can travel from A to B, you can also travel from B to A.

  ## Architecture

  The graph uses an adjacency map representation:
    - `adjacency[node]` = MapSet of neighbors for that node
    - For weighted edges, we store a weights map separately

  For undirected graphs, every edge is stored in both directions.

  ## Immutability

  Like all Elixir data structures, graphs are immutable. Every operation returns
  a new graph (or error tuple), making it safe to use in concurrent code.

  ## Quick Start

      alias CodingAdventures.Graph

      {:ok, g} = Graph.new()
                  |> Graph.add_edge("A", "B")
      {:ok, g} = Graph.add_edge(g, "B", "C")

      Graph.neighbors(g, "A")   # {:ok, ["B"]}
      Graph.degree(g, "A")      # {:ok, 1}
      Graph.bfs(g, "A")         # {:ok, ["A", "B", "C"]}
  """

  @doc """
  Creates a new empty graph.

  Returns a new graph struct with no nodes or edges.
  """
  def new do
    %{adjacency: %{}, weights: %{}}
  end

  @doc """
  Adds a node to the graph.

  If the node already exists, returns the graph unchanged.
  """
  def add_node(graph, node) do
    adjacency = Map.put_new(graph.adjacency, node, MapSet.new())
    %{graph | adjacency: adjacency}
  end

  @doc """
  Removes a node and all its edges from the graph.

  Returns `{:ok, graph}` if the node existed, `{:error, reason}` otherwise.
  """
  def remove_node(graph, node) do
    if has_node?(graph, node) do
      # Get neighbors before deleting the node
      node_neighbors = Map.get(graph.adjacency, node)

      # Remove the node
      new_adjacency = Map.delete(graph.adjacency, node)

      # Remove node from all neighbors' adjacency lists
      final_adjacency =
        Enum.reduce(node_neighbors, new_adjacency, fn neighbor, acc ->
          Map.update(acc, neighbor, MapSet.new(), fn neighbors ->
            MapSet.delete(neighbors, node)
          end)
        end)

      # Also remove all weights associated with this node
      new_weights =
        graph.weights
        |> Enum.reject(fn {{u, v}, _w} ->
          u == node or v == node
        end)
        |> Map.new()

      {:ok, %{graph | adjacency: final_adjacency, weights: new_weights}}
    else
      {:error, "Node not found: #{node}"}
    end
  end

  @doc """
  Checks if a node exists in the graph.
  """
  def has_node?(graph, node) do
    Map.has_key?(graph.adjacency, node)
  end

  @doc """
  Returns all nodes in the graph, sorted.
  """
  def nodes(graph) do
    graph.adjacency
    |> Map.keys()
    |> Enum.sort()
  end

  @doc """
  Returns the number of nodes in the graph.
  """
  def size(graph) do
    map_size(graph.adjacency)
  end

  @doc """
  Adds an edge between two nodes with optional weight (default 1.0).

  Creates nodes if they don't exist. Returns `{:ok, graph}` on success,
  or error if attempting a self-loop.
  """
  def add_edge(graph, from, to, weight \\ 1.0) do
    if from == to do
      {:error, "Self-loop not allowed: #{from}"}
    else
      # Ensure both nodes exist
      graph =
        graph
        |> add_node(from)
        |> add_node(to)

      # Add edge in both directions (undirected)
      new_adjacency =
        graph.adjacency
        |> Map.update!(from, fn neighbors -> MapSet.put(neighbors, to) end)
        |> Map.update!(to, fn neighbors -> MapSet.put(neighbors, from) end)

      # Store weight (canonical ordering for undirected graph)
      {a, b} = if from <= to, do: {from, to}, else: {to, from}
      new_weights = Map.put(graph.weights, {a, b}, weight)

      {:ok, %{graph | adjacency: new_adjacency, weights: new_weights}}
    end
  end

  @doc """
  Removes an edge between two nodes.

  Returns `{:ok, graph}` if the edge existed, `{:error, reason}` otherwise.
  """
  def remove_edge(graph, from, to) do
    if has_edge?(graph, from, to) do
      new_adjacency =
        graph.adjacency
        |> Map.update!(from, fn neighbors -> MapSet.delete(neighbors, to) end)
        |> Map.update!(to, fn neighbors -> MapSet.delete(neighbors, from) end)

      # Remove weight
      {a, b} = if from <= to, do: {from, to}, else: {to, from}
      new_weights = Map.delete(graph.weights, {a, b})

      {:ok, %{graph | adjacency: new_adjacency, weights: new_weights}}
    else
      {:error, "Edge not found: #{from} -- #{to}"}
    end
  end

  @doc """
  Checks if an edge exists between two nodes.
  """
  def has_edge?(graph, from, to) do
    case Map.get(graph.adjacency, from) do
      nil -> false
      neighbors -> MapSet.member?(neighbors, to)
    end
  end

  @doc """
  Returns all edges in the graph as a list of {from, to, weight} tuples.

  Each undirected edge is returned only once.
  """
  def edges(graph) do
    graph.weights
    |> Enum.map(fn {{u, v}, w} -> {u, v, w} end)
    |> Enum.sort()
  end

  @doc """
  Returns the weight of an edge, or error if it doesn't exist.
  """
  def edge_weight(graph, u, v) do
    {a, b} = if u <= v, do: {u, v}, else: {v, u}
    case Map.get(graph.weights, {a, b}) do
      nil -> {:error, "Edge not found: #{u} -- #{v}"}
      w -> {:ok, w}
    end
  end

  @doc """
  Returns all neighbors of a node.

  Returns `{:ok, neighbors}` where neighbors is a sorted list of adjacent nodes,
  or `{:error, reason}` if the node doesn't exist.
  """
  def neighbors(graph, node) do
    if has_node?(graph, node) do
      neighbors_list =
        graph.adjacency
        |> Map.get(node)
        |> MapSet.to_list()
        |> Enum.sort()

      {:ok, neighbors_list}
    else
      {:error, "Node not found: #{node}"}
    end
  end

  @doc """
  Returns neighbors with their edge weights.

  Returns `{:ok, map}` where map is {neighbor: weight} for all neighbors,
  or `{:error, reason}` if the node doesn't exist.
  """
  def neighbors_weighted(graph, node) do
    if has_node?(graph, node) do
      {:ok, neighbors_list} = neighbors(graph, node)

      weights_map =
        Enum.reduce(neighbors_list, %{}, fn neighbor, acc ->
          {a, b} = if node <= neighbor, do: {node, neighbor}, else: {neighbor, node}
          w = Map.get(graph.weights, {a, b}, 1.0)
          Map.put(acc, neighbor, w)
        end)

      {:ok, weights_map}
    else
      {:error, "Node not found: #{node}"}
    end
  end

  @doc """
  Returns the degree (number of neighbors) of a node.

  Returns `{:ok, degree}` or `{:error, reason}` if the node doesn't exist.
  """
  def degree(graph, node) do
    if has_node?(graph, node) do
      degree_value =
        graph.adjacency
        |> Map.get(node)
        |> MapSet.size()

      {:ok, degree_value}
    else
      {:error, "Node not found: #{node}"}
    end
  end

  @doc """
  Returns true if every node can reach every other node.

  An empty graph is vacuously connected (true).
  """
  def is_connected?(graph) do
    if size(graph) == 0 do
      true
    else
      start = nodes(graph) |> List.first()
      reachable = bfs(graph, start)
      length(reachable) == size(graph)
    end
  end

  @doc """
  Returns a list of connected components, each as a list of nodes.
  """
  def connected_components(graph) do
    unvisited = graph |> nodes() |> MapSet.new()
    connected_components_helper(graph, unvisited, [])
  end

  defp connected_components_helper(_graph, unvisited, acc) do
    if MapSet.size(unvisited) == 0 do
      Enum.reverse(acc)
    else
      start = unvisited |> MapSet.to_list() |> List.first()
      component = bfs(_graph, start)
      new_unvisited = MapSet.difference(unvisited, MapSet.new(component))
      connected_components_helper(_graph, new_unvisited, [component | acc])
    end
  end

  @doc """
  BFS traversal: returns nodes reachable from start in breadth-first order.
  """
  def bfs(graph, start) do
    visited = MapSet.new()
    queue = [start]
    bfs_helper(graph, queue, visited, [])
  end

  defp bfs_helper(_graph, [], _visited, result) do
    Enum.reverse(result)
  end

  defp bfs_helper(graph, [node | rest], visited, result) do
    if MapSet.member?(visited, node) do
      bfs_helper(graph, rest, visited, result)
    else
      new_visited = MapSet.put(visited, node)
      {:ok, neighbors_list} = neighbors(graph, node)

      new_queue =
        Enum.reduce(neighbors_list, rest, fn neighbor, queue ->
          if MapSet.member?(new_visited, neighbor) do
            queue
          else
            queue ++ [neighbor]
          end
        end)

      bfs_helper(graph, new_queue, new_visited, [node | result])
    end
  end

  @doc """
  DFS traversal: returns nodes reachable from start in depth-first order.
  """
  def dfs(graph, start) do
    visited = MapSet.new()
    stack = [start]
    dfs_helper(graph, stack, visited, [])
  end

  defp dfs_helper(_graph, [], _visited, result) do
    Enum.reverse(result)
  end

  defp dfs_helper(graph, [node | rest], visited, result) do
    if MapSet.member?(visited, node) do
      dfs_helper(graph, rest, visited, result)
    else
      new_visited = MapSet.put(visited, node)
      {:ok, neighbors_list} = neighbors(graph, node)

      # Reverse sort for deterministic output
      new_stack =
        neighbors_list
        |> Enum.reverse()
        |> Enum.reduce(rest, fn neighbor, stack ->
          if MapSet.member?(new_visited, neighbor) do
            stack
          else
            [neighbor | stack]
          end
        end)

      dfs_helper(graph, new_stack, new_visited, [node | result])
    end
  end

  @doc """
  Returns the shortest (lowest-weight) path from start to end.

  Returns path as list or empty list if no path exists.
  """
  def shortest_path(graph, start, end_node) do
    cond do
      start == end_node ->
        if has_node?(graph, start), do: [start], else: []

      true ->
        # Check if all weights are 1.0
        all_unit = Enum.all?(edges(graph), fn {_u, _v, w} -> w == 1.0 end)

        if all_unit do
          shortest_path_bfs(graph, start, end_node)
        else
          shortest_path_dijkstra(graph, start, end_node)
        end
    end
  end

  defp shortest_path_bfs(graph, start, end_node) do
    parent = %{start => nil}
    queue = [start]
    result = shortest_path_bfs_helper(graph, queue, parent, end_node)
    reconstruct_path(result, end_node)
  end

  defp shortest_path_bfs_helper(_graph, [], parent, _end_node) do
    parent
  end

  defp shortest_path_bfs_helper(graph, [node | rest], parent, end_node) do
    if node == end_node do
      parent
    else
      {:ok, neighbors_list} = neighbors(graph, node)

      {new_parent, new_queue} =
        Enum.reduce(neighbors_list, {parent, rest}, fn neighbor, {p, q} ->
          if Map.has_key?(p, neighbor) do
            {p, q}
          else
            {Map.put(p, neighbor, node), q ++ [neighbor]}
          end
        end)

      shortest_path_bfs_helper(graph, new_queue, new_parent, end_node)
    end
  end

  defp shortest_path_dijkstra(graph, start, end_node) do
    all_nodes = nodes(graph)
    dist = Map.new(all_nodes, fn n -> {n, if(n == start, do: 0.0, else: :infinity)} end)
    parent = %{}
    pq = [{0.0, start}]
    visited = MapSet.new()

    result = shortest_path_dijkstra_helper(graph, pq, dist, parent, visited, end_node)
    reconstruct_path(result, end_node)
  end

  defp shortest_path_dijkstra_helper(_graph, [], _dist, parent, _visited, _end_node) do
    parent
  end

  defp shortest_path_dijkstra_helper(graph, pq, dist, parent, visited, end_node) do
    # Sort priority queue and get minimum
    sorted_pq = Enum.sort(pq)

    case sorted_pq do
      [] ->
        parent

      [{_d, node} | rest] ->
        if MapSet.member?(visited, node) do
          shortest_path_dijkstra_helper(graph, rest, dist, parent, visited, end_node)
        else
          if node == end_node do
            parent
          else
            new_visited = MapSet.put(visited, node)
            {:ok, neighbors_map} = neighbors_weighted(graph, node)
            current_dist = dist[node]

            {new_dist, new_parent, new_pq} =
              Enum.reduce(neighbors_map, {dist, parent, rest}, fn {neighbor, weight},
                                                                   {d, p, q} ->
                new_d = current_dist + weight

                if new_d < (d[neighbor] || :infinity) do
                  {Map.put(d, neighbor, new_d), Map.put(p, neighbor, node),
                   [{new_d, neighbor} | q]}
                else
                  {d, p, q}
                end
              end)

            shortest_path_dijkstra_helper(graph, new_pq, new_dist, new_parent, new_visited,
              end_node)
          end
        end
    end
  end

  defp reconstruct_path(parent, end_node) do
    if Map.has_key?(parent, end_node) do
      do_reconstruct_path(parent, end_node, [])
    else
      []
    end
  end

  defp do_reconstruct_path(_parent, nil, path), do: path

  defp do_reconstruct_path(parent, node, path) do
    p = Map.get(parent, node)
    do_reconstruct_path(parent, p, [node | path])
  end

  @doc """
  Returns true if the graph contains any cycle.
  """
  def has_cycle?(graph) do
    nodes_list = nodes(graph)
    visited = MapSet.new()

    Enum.any?(nodes_list, fn start ->
      if MapSet.member?(visited, start) do
        false
      else
        {has_cycle, new_visited} = has_cycle_from(graph, start, nil, visited)
        # Update visited for next iteration
        Process.put(:visited, new_visited)
        has_cycle
      end
    end)
  end

  defp has_cycle_from(graph, node, parent, visited) do
    new_visited = MapSet.put(visited, node)

    {:ok, neighbors_list} = neighbors(graph, node)

    Enum.reduce_while(neighbors_list, {false, new_visited}, fn neighbor,
                                                                 {_has_cycle, vis} ->
      if not MapSet.member?(vis, neighbor) do
        {has_cycle, new_vis} = has_cycle_from(graph, neighbor, node, vis)
        if has_cycle do
          {:halt, {true, new_vis}}
        else
          {:cont, {false, new_vis}}
        end
      else
        # Back edge - cycle exists if not parent
        if neighbor == parent do
          {:cont, {false, vis}}
        else
          {:halt, {true, vis}}
        end
      end
    end)
  end

  @doc """
  Returns the minimum spanning tree as a list of {u, v, weight} tuples.

  Returns nil if the graph is not connected.
  Uses Kruskal's algorithm with Union-Find.
  """
  def minimum_spanning_tree(graph) do
    nodes_list = nodes(graph)

    cond do
      length(nodes_list) == 0 ->
        []

      length(nodes_list) == 1 ->
        []

      true ->
        sorted_edges = edges(graph) |> Enum.sort_by(fn {_u, _v, w} -> w end)
        uf = union_find_new(nodes_list)
        mst_edges = []

        result = kruskal_helper(sorted_edges, uf, mst_edges, length(nodes_list))

        if length(result) < length(nodes_list) - 1 do
          nil
        else
          result
        end
    end
  end

  defp kruskal_helper([], _uf, mst, _target_count) do
    mst
  end

  defp kruskal_helper(_edges, _uf, mst, target_count) when length(mst) == target_count do
    mst
  end

  defp kruskal_helper([{u, v, w} | rest], uf, mst, target_count) do
    root_u = union_find_find(uf, u)
    root_v = union_find_find(uf, v)

    if root_u != root_v do
      new_uf = union_find_union(uf, u, v)
      kruskal_helper(rest, new_uf, [{u, v, w} | mst], target_count)
    else
      kruskal_helper(rest, uf, mst, target_count)
    end
  end

  defp union_find_new(nodes) do
    Map.new(nodes, fn n -> {n, n} end)
  end

  defp union_find_find(uf, x) do
    root = uf[x]
    if root == x do
      x
    else
      new_root = union_find_find(uf, root)
      # Path compression would mutate, so we skip it in Elixir
      new_root
    end
  end

  defp union_find_union(uf, a, b) do
    root_a = union_find_find(uf, a)
    root_b = union_find_find(uf, b)

    if root_a == root_b do
      uf
    else
      Map.put(uf, root_b, root_a)
    end
  end
end
