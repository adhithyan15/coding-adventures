defmodule CodingAdventures.DirectedGraph.Graph do
  @moduledoc """
  A directed graph backed by forward and reverse adjacency maps.

  Internal Storage
  ----------------

  We maintain **two** adjacency maps:

      forward[u] = MapSet of nodes that u points TO   (successors / children)
      reverse[v] = MapSet of nodes that point TO v     (predecessors / parents)

  Every node that exists in the graph has an entry in both maps, even if its
  adjacency set is empty. This invariant lets us use `Map.has_key?(forward, node)`
  as the canonical "does this node exist?" check.

  Why two maps? Because many algorithms need to walk edges in *both* directions:

  - `topological_sort` needs nodes with zero in-degree -> check `MapSet.size(reverse[node])`
  - `transitive_dependents` walks backwards -> traverse `reverse`
  - `remove_node` cleans up both directions -> O(degree) with both maps

  The trade-off is that every `add_edge` and `remove_edge` must update both
  maps, but that's O(1) per operation, so it's a good deal.

  Self-Loops
  ----------

  By default, self-loops (A -> A) are disallowed and return an error. Pass
  `allow_self_loops: true` to `new/1` to permit them. When a self-loop exists,
  the node appears in its own forward AND reverse sets, naturally creating a cycle.

  ## Immutability

  All operations return `{:ok, new_graph}` or `{:error, reason}`. The original
  graph is never modified. This is idiomatic Elixir -- data is immutable, and
  functions transform values into new values.

  ## Example

      alias CodingAdventures.DirectedGraph.Graph

      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "compile", "link")
      {:ok, g} = Graph.add_edge(g, "link", "package")
      {:ok, order} = Graph.topological_sort(g)
      # order == ["compile", "link", "package"]
  """

  alias CodingAdventures.DirectedGraph.{CycleError, EdgeNotFoundError, NodeNotFoundError}

  # ---------------------------------------------------------------------------
  # Struct definition
  # ---------------------------------------------------------------------------
  # The struct holds the two adjacency maps and the allow_self_loops flag.
  # We use @enforce_keys to make sure the struct is always fully initialized.

  @enforce_keys [:forward, :reverse, :allow_self_loops]
  defstruct [:forward, :reverse, :allow_self_loops]

  @type t :: %__MODULE__{
          forward: %{any() => MapSet.t()},
          reverse: %{any() => MapSet.t()},
          allow_self_loops: boolean()
        }

  # ---------------------------------------------------------------------------
  # Constructor
  # ---------------------------------------------------------------------------

  @doc """
  Create a new empty directed graph.

  ## Options

  - `:allow_self_loops` - when `true`, edges from a node to itself are allowed.
    Defaults to `false`.

  ## Examples

      iex> g = CodingAdventures.DirectedGraph.Graph.new()
      iex> CodingAdventures.DirectedGraph.Graph.size(g)
      0

      iex> g = CodingAdventures.DirectedGraph.Graph.new(allow_self_loops: true)
      iex> {:ok, g} = CodingAdventures.DirectedGraph.Graph.add_edge(g, "A", "A")
      iex> CodingAdventures.DirectedGraph.Graph.has_edge?(g, "A", "A")
      true
  """
  @spec new(keyword()) :: t()
  def new(opts \\ []) do
    %__MODULE__{
      forward: %{},
      reverse: %{},
      allow_self_loops: Keyword.get(opts, :allow_self_loops, false)
    }
  end

  # ---------------------------------------------------------------------------
  # Node operations
  # ---------------------------------------------------------------------------

  @doc """
  Add a node to the graph. No-op if the node already exists.

  Returns `{:ok, graph}` always (adding a node never fails).

  ## Example

      iex> g = CodingAdventures.DirectedGraph.Graph.new()
      iex> {:ok, g} = CodingAdventures.DirectedGraph.Graph.add_node(g, "A")
      iex> CodingAdventures.DirectedGraph.Graph.has_node?(g, "A")
      true
  """
  @spec add_node(t(), any()) :: {:ok, t()}
  def add_node(%__MODULE__{} = graph, node) do
    if Map.has_key?(graph.forward, node) do
      {:ok, graph}
    else
      {:ok,
       %{
         graph
         | forward: Map.put(graph.forward, node, MapSet.new()),
           reverse: Map.put(graph.reverse, node, MapSet.new())
       }}
    end
  end

  @doc """
  Remove a node and all its incoming/outgoing edges.

  Returns `{:ok, graph}` on success, or `{:error, %NodeNotFoundError{}}` if
  the node doesn't exist.

  This is O(in-degree + out-degree) because we update the adjacency sets
  of all neighbors.
  """
  @spec remove_node(t(), any()) :: {:ok, t()} | {:error, NodeNotFoundError.t()}
  def remove_node(%__MODULE__{} = graph, node) do
    if not Map.has_key?(graph.forward, node) do
      {:error,
       %NodeNotFoundError{
         message: "Node not found: #{inspect(node)}",
         node: node
       }}
    else
      # Clean up outgoing edges: for each successor, remove `node` from
      # that successor's reverse (predecessor) set.
      successors = Map.get(graph.forward, node, MapSet.new())

      reverse =
        Enum.reduce(successors, graph.reverse, fn successor, acc ->
          Map.update!(acc, successor, &MapSet.delete(&1, node))
        end)

      # Clean up incoming edges: for each predecessor, remove `node` from
      # that predecessor's forward (successor) set.
      predecessors = Map.get(graph.reverse, node, MapSet.new())

      forward =
        Enum.reduce(predecessors, graph.forward, fn predecessor, acc ->
          Map.update!(acc, predecessor, &MapSet.delete(&1, node))
        end)

      # Remove the node itself from both maps.
      {:ok,
       %{
         graph
         | forward: Map.delete(forward, node),
           reverse: Map.delete(reverse, node)
       }}
    end
  end

  @doc """
  Return `true` if the node exists in the graph.
  """
  @spec has_node?(t(), any()) :: boolean()
  def has_node?(%__MODULE__{} = graph, node) do
    Map.has_key?(graph.forward, node)
  end

  @doc """
  Return a list of all nodes in the graph.

  The order is the map key order (not guaranteed to be insertion order,
  but deterministic for the same graph).
  """
  @spec nodes(t()) :: [any()]
  def nodes(%__MODULE__{} = graph) do
    Map.keys(graph.forward)
  end

  @doc """
  Return the number of nodes in the graph.
  """
  @spec size(t()) :: non_neg_integer()
  def size(%__MODULE__{} = graph) do
    map_size(graph.forward)
  end

  # ---------------------------------------------------------------------------
  # Edge operations
  # ---------------------------------------------------------------------------

  @doc """
  Add a directed edge from `from_node` to `to_node`.

  Both nodes are implicitly added if they don't exist yet. Duplicate edges
  are silently ignored (MapSet handles deduplication).

  Returns `{:error, reason}` if `from_node == to_node` and self-loops are
  not allowed.

  ## Example

      iex> g = CodingAdventures.DirectedGraph.Graph.new()
      iex> {:ok, g} = CodingAdventures.DirectedGraph.Graph.add_edge(g, "A", "B")
      iex> CodingAdventures.DirectedGraph.Graph.has_edge?(g, "A", "B")
      true
  """
  @spec add_edge(t(), any(), any()) :: {:ok, t()} | {:error, String.t()}
  def add_edge(%__MODULE__{} = graph, from_node, to_node) do
    if from_node == to_node and not graph.allow_self_loops do
      {:error, "Self-loops are not allowed: #{inspect(from_node)} -> #{inspect(to_node)}"}
    else
      # Ensure both nodes exist.
      {:ok, graph} = add_node(graph, from_node)
      {:ok, graph} = add_node(graph, to_node)

      # Add the edge to both adjacency maps.
      forward = Map.update!(graph.forward, from_node, &MapSet.put(&1, to_node))
      reverse = Map.update!(graph.reverse, to_node, &MapSet.put(&1, from_node))

      {:ok, %{graph | forward: forward, reverse: reverse}}
    end
  end

  @doc """
  Remove the directed edge from `from_node` to `to_node`.

  Returns `{:error, %EdgeNotFoundError{}}` if the edge doesn't exist.
  Both nodes remain in the graph after removal.
  """
  @spec remove_edge(t(), any(), any()) :: {:ok, t()} | {:error, EdgeNotFoundError.t()}
  def remove_edge(%__MODULE__{} = graph, from_node, to_node) do
    has_it =
      Map.has_key?(graph.forward, from_node) and
        MapSet.member?(Map.get(graph.forward, from_node), to_node)

    if not has_it do
      {:error,
       %EdgeNotFoundError{
         message: "Edge not found: #{inspect(from_node)} -> #{inspect(to_node)}",
         from_node: from_node,
         to_node: to_node
       }}
    else
      forward = Map.update!(graph.forward, from_node, &MapSet.delete(&1, to_node))
      reverse = Map.update!(graph.reverse, to_node, &MapSet.delete(&1, from_node))
      {:ok, %{graph | forward: forward, reverse: reverse}}
    end
  end

  @doc """
  Return `true` if the directed edge from `from_node` to `to_node` exists.
  """
  @spec has_edge?(t(), any(), any()) :: boolean()
  def has_edge?(%__MODULE__{} = graph, from_node, to_node) do
    case Map.fetch(graph.forward, from_node) do
      {:ok, successors} -> MapSet.member?(successors, to_node)
      :error -> false
    end
  end

  @doc """
  Return a list of all edges as `{from_node, to_node}` tuples.
  """
  @spec edges(t()) :: [{any(), any()}]
  def edges(%__MODULE__{} = graph) do
    Enum.flat_map(graph.forward, fn {node, successors} ->
      Enum.map(successors, fn successor -> {node, successor} end)
    end)
  end

  # ---------------------------------------------------------------------------
  # Neighbor queries
  # ---------------------------------------------------------------------------

  @doc """
  Return the direct predecessors (parents) of a node.

  These are the nodes that have an edge pointing TO this node.
  Returns `{:error, %NodeNotFoundError{}}` if the node doesn't exist.
  """
  @spec predecessors(t(), any()) :: {:ok, [any()]} | {:error, NodeNotFoundError.t()}
  def predecessors(%__MODULE__{} = graph, node) do
    case Map.fetch(graph.reverse, node) do
      {:ok, preds} ->
        {:ok, MapSet.to_list(preds)}

      :error ->
        {:error,
         %NodeNotFoundError{
           message: "Node not found: #{inspect(node)}",
           node: node
         }}
    end
  end

  @doc """
  Return the direct successors (children) of a node.

  These are the nodes that this node points TO.
  Returns `{:error, %NodeNotFoundError{}}` if the node doesn't exist.
  """
  @spec successors(t(), any()) :: {:ok, [any()]} | {:error, NodeNotFoundError.t()}
  def successors(%__MODULE__{} = graph, node) do
    case Map.fetch(graph.forward, node) do
      {:ok, succs} ->
        {:ok, MapSet.to_list(succs)}

      :error ->
        {:error,
         %NodeNotFoundError{
           message: "Node not found: #{inspect(node)}",
           node: node
         }}
    end
  end

  # ===========================================================================
  # ALGORITHMS
  # ===========================================================================

  # ---------------------------------------------------------------------------
  # Topological Sort (Kahn's Algorithm)
  # ---------------------------------------------------------------------------
  #
  # Kahn's algorithm works by repeatedly removing nodes with zero in-degree.
  # The removal order is a valid topological ordering.
  #
  # Why Kahn's instead of DFS-based?
  # 1. It naturally detects cycles (if we can't remove all nodes, there's a cycle)
  # 2. It's easy to modify for independent_groups (see below)
  #
  # Time complexity: O(V + E) where V = nodes, E = edges.
  #
  # In Elixir, we use a recursive approach instead of a while loop. The
  # `do_kahn` function processes one "round" of zero-in-degree nodes.

  @doc """
  Return a topological ordering of all nodes.

  A topological ordering is a linear sequence where for every edge u -> v,
  u appears before v. This only exists for DAGs (directed acyclic graphs).

  Returns `{:ok, list}` on success or `{:error, %CycleError{}}` if the
  graph contains a cycle.

  ## Example

      iex> g = CodingAdventures.DirectedGraph.Graph.new()
      iex> {:ok, g} = CodingAdventures.DirectedGraph.Graph.add_edge(g, "A", "B")
      iex> {:ok, g} = CodingAdventures.DirectedGraph.Graph.add_edge(g, "B", "C")
      iex> CodingAdventures.DirectedGraph.Graph.topological_sort(g)
      {:ok, ["A", "B", "C"]}
  """
  @spec topological_sort(t()) :: {:ok, [any()]} | {:error, CycleError.t()}
  def topological_sort(%__MODULE__{} = graph) do
    # Compute initial in-degrees from the reverse map.
    in_degree =
      Map.new(graph.reverse, fn {node, preds} -> {node, MapSet.size(preds)} end)

    # Start with all nodes that have zero in-degree.
    queue =
      in_degree
      |> Enum.filter(fn {_node, deg} -> deg == 0 end)
      |> Enum.map(fn {node, _deg} -> node end)
      |> Enum.sort()

    result = do_kahn(queue, in_degree, graph.forward, [])

    if length(result) != map_size(graph.forward) do
      cycle = find_cycle(graph)

      {:error,
       %CycleError{
         message: "Graph contains a cycle: #{Enum.join(Enum.map(cycle, &inspect/1), " -> ")}",
         cycle: cycle
       }}
    else
      {:ok, result}
    end
  end

  # Recursive Kahn's algorithm: process all zero-in-degree nodes, decrement
  # their successors' in-degrees, and repeat until no more zero-in-degree
  # nodes exist.
  defp do_kahn([], _in_degree, _forward, result), do: Enum.reverse(result)

  defp do_kahn([node | rest], in_degree, forward, result) do
    successors = Map.get(forward, node, MapSet.new())

    {new_queue_additions, new_in_degree} =
      Enum.reduce(successors, {[], in_degree}, fn succ, {additions, deg_map} ->
        new_deg = Map.get(deg_map, succ, 0) - 1
        deg_map = Map.put(deg_map, succ, new_deg)

        if new_deg == 0 do
          {[succ | additions], deg_map}
        else
          {additions, deg_map}
        end
      end)

    # Sort the new additions and merge with remaining queue to maintain
    # deterministic ordering.
    next_queue = Enum.sort(rest ++ new_queue_additions)
    do_kahn(next_queue, new_in_degree, forward, [node | result])
  end

  # ---------------------------------------------------------------------------
  # Cycle Detection (DFS Three-Color Algorithm)
  # ---------------------------------------------------------------------------
  #
  # The three-color algorithm uses:
  # - :white  -- not yet visited
  # - :gray   -- currently being explored (on the recursion stack)
  # - :black  -- fully explored
  #
  # If we encounter a :gray node during DFS, we've found a back edge,
  # which means there's a cycle.
  #
  # In Elixir, we pass the color map through the recursion rather than
  # using mutable state.

  @doc """
  Return `true` if the graph contains at least one cycle.

  Uses DFS with three-color marking. This is O(V + E).

  ## Example

      iex> g = CodingAdventures.DirectedGraph.Graph.new()
      iex> {:ok, g} = CodingAdventures.DirectedGraph.Graph.add_edge(g, "A", "B")
      iex> CodingAdventures.DirectedGraph.Graph.has_cycle?(g)
      false
  """
  @spec has_cycle?(t()) :: boolean()
  def has_cycle?(%__MODULE__{} = graph) do
    colors = Map.new(graph.forward, fn {node, _} -> {node, :white} end)
    do_has_cycle(Map.keys(graph.forward), colors, graph.forward)
  end

  defp do_has_cycle([], _colors, _forward), do: false

  defp do_has_cycle([node | rest], colors, forward) do
    if Map.get(colors, node) == :white do
      case dfs_cycle(node, colors, forward) do
        {:cycle_found, _colors} -> true
        {:ok, colors} -> do_has_cycle(rest, colors, forward)
      end
    else
      do_has_cycle(rest, colors, forward)
    end
  end

  defp dfs_cycle(node, colors, forward) do
    colors = Map.put(colors, node, :gray)
    successors = Map.get(forward, node, MapSet.new())

    result =
      Enum.reduce_while(MapSet.to_list(successors), {:ok, colors}, fn succ, {:ok, cols} ->
        case Map.get(cols, succ) do
          :gray ->
            {:halt, {:cycle_found, cols}}

          :white ->
            case dfs_cycle(succ, cols, forward) do
              {:cycle_found, _} = found -> {:halt, found}
              {:ok, cols} -> {:cont, {:ok, cols}}
            end

          :black ->
            {:cont, {:ok, cols}}
        end
      end)

    case result do
      {:cycle_found, _} = found -> found
      {:ok, colors} -> {:ok, Map.put(colors, node, :black)}
    end
  end

  # Find a cycle path for error reporting. Returns a list like [A, B, C, A].
  @spec find_cycle(t()) :: [any()]
  defp find_cycle(%__MODULE__{} = graph) do
    colors = Map.new(graph.forward, fn {node, _} -> {node, :white} end)
    parents = Map.new(graph.forward, fn {node, _} -> {node, nil} end)

    sorted_nodes = Enum.sort(Map.keys(graph.forward))
    do_find_cycle(sorted_nodes, colors, parents, graph.forward)
  end

  defp do_find_cycle([], _colors, _parents, _forward), do: []

  defp do_find_cycle([node | rest], colors, parents, forward) do
    if Map.get(colors, node) == :white do
      case dfs_find_cycle(node, colors, parents, forward) do
        {:found, cycle} -> cycle
        {:ok, colors, parents} -> do_find_cycle(rest, colors, parents, forward)
      end
    else
      do_find_cycle(rest, colors, parents, forward)
    end
  end

  defp dfs_find_cycle(node, colors, parents, forward) do
    colors = Map.put(colors, node, :gray)
    successors = Map.get(forward, node, MapSet.new()) |> MapSet.to_list() |> Enum.sort()

    result =
      Enum.reduce_while(successors, {:ok, colors, parents}, fn succ, {:ok, cols, pars} ->
        case Map.get(cols, succ) do
          :gray ->
            # Found the cycle! Reconstruct the path.
            cycle = reconstruct_cycle(succ, node, pars)
            {:halt, {:found, cycle}}

          :white ->
            pars = Map.put(pars, succ, node)

            case dfs_find_cycle(succ, cols, pars, forward) do
              {:found, _} = found -> {:halt, found}
              {:ok, cols, pars} -> {:cont, {:ok, cols, pars}}
            end

          :black ->
            {:cont, {:ok, cols, pars}}
        end
      end)

    case result do
      {:found, _} = found -> found
      {:ok, colors, parents} -> {:ok, Map.put(colors, node, :black), parents}
    end
  end

  defp reconstruct_cycle(target, current, parents) do
    path = collect_path(current, target, parents, [current])
    Enum.reverse([target | path])
  end

  defp collect_path(current, target, _parents, acc) when current == target, do: acc

  defp collect_path(current, target, parents, acc) do
    parent = Map.get(parents, current)

    if parent == nil do
      acc
    else
      collect_path(parent, target, parents, [parent | acc])
    end
  end

  # ---------------------------------------------------------------------------
  # Transitive Closure (BFS Forward Reachability)
  # ---------------------------------------------------------------------------
  #
  # The transitive closure of a node is the set of all nodes reachable from it
  # by following edges forward. We use BFS because it's simple and doesn't
  # risk stack overflow on deep graphs.

  @doc """
  Return all nodes reachable downstream from `node`.

  The starting node is NOT included in the result (unless reachable via a cycle).

  Returns `{:error, %NodeNotFoundError{}}` if the node doesn't exist.

  ## Example

      iex> g = CodingAdventures.DirectedGraph.Graph.new()
      iex> {:ok, g} = CodingAdventures.DirectedGraph.Graph.add_edge(g, "A", "B")
      iex> {:ok, g} = CodingAdventures.DirectedGraph.Graph.add_edge(g, "B", "C")
      iex> CodingAdventures.DirectedGraph.Graph.transitive_closure(g, "A")
      {:ok, MapSet.new(["B", "C"])}
  """
  @spec transitive_closure(t(), any()) :: {:ok, MapSet.t()} | {:error, NodeNotFoundError.t()}
  def transitive_closure(%__MODULE__{} = graph, node) do
    if not Map.has_key?(graph.forward, node) do
      {:error,
       %NodeNotFoundError{
         message: "Node not found: #{inspect(node)}",
         node: node
       }}
    else
      initial_succs = Map.get(graph.forward, node, MapSet.new())
      visited = bfs(MapSet.to_list(initial_succs), MapSet.new(initial_succs), graph.forward)
      {:ok, visited}
    end
  end

  # ---------------------------------------------------------------------------
  # Transitive Dependents (BFS Reverse Reachability)
  # ---------------------------------------------------------------------------
  #
  # Mirror of transitive_closure: walks the reverse adjacency map to find
  # everything upstream that depends on the given node.

  @doc """
  Return all nodes that transitively depend on `node`.

  This follows edges in the REVERSE direction -- it finds everything upstream
  that would be affected if `node` changed.

  The starting node is NOT included in the result.

  Returns `{:error, %NodeNotFoundError{}}` if the node doesn't exist.
  """
  @spec transitive_dependents(t(), any()) :: {:ok, MapSet.t()} | {:error, NodeNotFoundError.t()}
  def transitive_dependents(%__MODULE__{} = graph, node) do
    if not Map.has_key?(graph.reverse, node) do
      {:error,
       %NodeNotFoundError{
         message: "Node not found: #{inspect(node)}",
         node: node
       }}
    else
      initial_preds = Map.get(graph.reverse, node, MapSet.new())
      visited = bfs(MapSet.to_list(initial_preds), MapSet.new(initial_preds), graph.reverse)
      {:ok, visited}
    end
  end

  # Shared BFS implementation used by both transitive_closure and
  # transitive_dependents. The only difference is which adjacency map
  # is passed in.
  defp bfs([], visited, _adj_map), do: visited

  defp bfs([current | rest], visited, adj_map) do
    neighbors = Map.get(adj_map, current, MapSet.new())

    {new_queue, new_visited} =
      Enum.reduce(MapSet.to_list(neighbors), {rest, visited}, fn neighbor, {q, v} ->
        if MapSet.member?(v, neighbor) do
          {q, v}
        else
          {[neighbor | q], MapSet.put(v, neighbor)}
        end
      end)

    bfs(new_queue, new_visited, adj_map)
  end

  # ---------------------------------------------------------------------------
  # Independent Groups (Parallel Execution Levels)
  # ---------------------------------------------------------------------------
  #
  # A modified Kahn's algorithm. Instead of processing zero-in-degree nodes
  # one at a time, we process ALL of them as a batch -- they form one "level"
  # of independent tasks that can run in parallel.
  #
  # For a linear chain A -> B -> C:  [[A], [B], [C]]
  # For a diamond A -> {B,C} -> D:   [[A], [B, C], [D]]

  @doc """
  Partition nodes into levels by topological depth.

  Each level contains nodes that have no dependencies on each other and whose
  dependencies have all been satisfied by earlier levels. Nodes within a level
  can be executed in parallel.

  Returns `{:error, %CycleError{}}` if the graph contains a cycle.
  Returns `{:ok, []}` for an empty graph.
  """
  @spec independent_groups(t()) :: {:ok, [[any()]]} | {:error, CycleError.t()}
  def independent_groups(%__MODULE__{} = graph) do
    in_degree =
      Map.new(graph.reverse, fn {node, preds} -> {node, MapSet.size(preds)} end)

    # Collect the initial set of zero-in-degree nodes.
    current_level =
      in_degree
      |> Enum.filter(fn {_node, deg} -> deg == 0 end)
      |> Enum.map(fn {node, _deg} -> node end)
      |> Enum.sort()

    {groups, processed} = do_independent_groups(current_level, in_degree, graph.forward, [], 0)

    if processed != map_size(graph.forward) do
      cycle = find_cycle(graph)

      {:error,
       %CycleError{
         message: "Graph contains a cycle: #{Enum.join(Enum.map(cycle, &inspect/1), " -> ")}",
         cycle: cycle
       }}
    else
      {:ok, Enum.reverse(groups)}
    end
  end

  defp do_independent_groups([], _in_degree, _forward, groups, processed) do
    {groups, processed}
  end

  defp do_independent_groups(current_level, in_degree, forward, groups, processed) do
    new_processed = processed + length(current_level)

    # Decrement in-degrees for all successors of current level nodes.
    new_in_degree =
      Enum.reduce(current_level, in_degree, fn node, deg_map ->
        successors = Map.get(forward, node, MapSet.new())

        Enum.reduce(MapSet.to_list(successors), deg_map, fn succ, dm ->
          Map.update!(dm, succ, &(&1 - 1))
        end)
      end)

    # Find the next level: all nodes whose in-degree just dropped to zero.
    next_level =
      new_in_degree
      |> Enum.filter(fn {node, deg} ->
        deg == 0 and not Enum.any?(groups, &(node in &1)) and node not in current_level
      end)
      |> Enum.map(fn {node, _deg} -> node end)
      |> Enum.sort()

    do_independent_groups(
      next_level,
      new_in_degree,
      forward,
      [current_level | groups],
      new_processed
    )
  end

  # ---------------------------------------------------------------------------
  # Affected Nodes
  # ---------------------------------------------------------------------------
  #
  # Given a set of "changed" nodes, compute everything that is affected:
  # the changed nodes themselves plus all their transitive dependents.

  @doc """
  Return the changed nodes plus all their transitive dependents.

  Nodes in `changed` that don't exist in the graph are silently ignored.

  ## Example

      iex> g = CodingAdventures.DirectedGraph.Graph.new()
      iex> {:ok, g} = CodingAdventures.DirectedGraph.Graph.add_edge(g, "A", "B")
      iex> {:ok, g} = CodingAdventures.DirectedGraph.Graph.add_edge(g, "B", "C")
      iex> CodingAdventures.DirectedGraph.Graph.affected_nodes(g, MapSet.new(["C"]))
      MapSet.new(["A", "B", "C"])
  """
  @spec affected_nodes(t(), MapSet.t()) :: MapSet.t()
  def affected_nodes(%__MODULE__{} = graph, changed) do
    Enum.reduce(changed, MapSet.new(), fn node, acc ->
      if Map.has_key?(graph.forward, node) do
        {:ok, dependents} = transitive_dependents(graph, node)
        acc |> MapSet.put(node) |> MapSet.union(dependents)
      else
        acc
      end
    end)
  end
end
