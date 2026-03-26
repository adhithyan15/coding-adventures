defmodule BuildTool.DirectedGraph do
  @moduledoc """
  A directed graph data structure with algorithms for topological sorting,
  cycle detection, and parallel execution level computation.

  ## What is a directed graph?

  A directed graph (or "digraph") is a set of nodes connected by edges,
  where each edge has a direction — it goes FROM one node TO another.
  Think of it like a one-way street map: you can travel from A to B,
  but that doesn't mean you can travel from B to A.

  In this build system, nodes are packages and edges are dependencies:
  if package B depends on package A, there's an edge from A to B
  (A must be built before B).

  ## Data structure

  The graph stores both forward edges (node -> its successors) and reverse
  edges (node -> its predecessors) for efficient lookups in both
  directions. This doubles memory usage but makes affected-node queries
  O(V+E) instead of requiring a full graph reversal.

  The graph is represented as a plain map:

      %{
        nodes: MapSet.t(),         # all node names
        forward: %{String.t() => MapSet.t()},  # node -> successors
        reverse: %{String.t() => MapSet.t()}   # node -> predecessors
      }

  We use a plain map rather than a struct to keep the implementation
  lightweight — this is an inline utility, not a standalone library.

  ## Key algorithms

    - **Kahn's algorithm** (`independent_groups/1`): partitions nodes into
      "levels" by topological depth. Nodes at the same level have no
      dependency on each other and can run in parallel.

    - **Affected nodes** (`affected_nodes/2`): given a set of changed nodes,
      finds everything that transitively depends on them. These are the
      packages that need rebuilding when something changes.

  ## Example

      iex> g = DirectedGraph.new()
      ...>   |> DirectedGraph.add_node("A")
      ...>   |> DirectedGraph.add_edge("A", "B")
      ...>   |> DirectedGraph.add_edge("A", "C")
      ...>   |> DirectedGraph.add_edge("B", "D")
      ...>   |> DirectedGraph.add_edge("C", "D")
      iex> DirectedGraph.independent_groups(g)
      {:ok, [["A"], ["B", "C"], ["D"]]}
  """

  # ---------------------------------------------------------------------------
  # Construction
  # ---------------------------------------------------------------------------

  @doc """
  Creates an empty directed graph.

  Returns a map with three keys:
    - `:nodes` — a `MapSet` of all node names
    - `:forward` — a map from each node to its set of successors
    - `:reverse` — a map from each node to its set of predecessors

  ## Example

      iex> g = BuildTool.DirectedGraph.new()
      iex> MapSet.size(g.nodes)
      0
  """
  def new do
    %{
      nodes: MapSet.new(),
      forward: %{},
      reverse: %{}
    }
  end

  # ---------------------------------------------------------------------------
  # Node and edge operations
  # ---------------------------------------------------------------------------

  @doc """
  Adds a node to the graph. No-op if the node already exists.

  Every node gets an entry in both the forward and reverse adjacency maps,
  even if it has no edges yet. This ensures that `independent_groups/1`
  includes isolated nodes (packages with no dependencies).

  ## Example

      iex> g = BuildTool.DirectedGraph.new() |> BuildTool.DirectedGraph.add_node("A")
      iex> BuildTool.DirectedGraph.has_node?(g, "A")
      true
  """
  def add_node(graph, node) do
    if MapSet.member?(graph.nodes, node) do
      graph
    else
      %{
        graph
        | nodes: MapSet.put(graph.nodes, node),
          forward: Map.put_new(graph.forward, node, MapSet.new()),
          reverse: Map.put_new(graph.reverse, node, MapSet.new())
      }
    end
  end

  @doc """
  Adds a directed edge from `from` to `to`.

  Both nodes are implicitly added if they don't exist. Self-loops
  (from == to) raise an `ArgumentError` because they would create
  a cycle of length 1, which breaks topological sorting.

  In the build system, an edge from A to B means "A must be built
  before B" (because B depends on A).

  ## Example

      iex> g = BuildTool.DirectedGraph.new() |> BuildTool.DirectedGraph.add_edge("A", "B")
      iex> BuildTool.DirectedGraph.has_edge?(g, "A", "B")
      true
  """
  def add_edge(graph, from, to) do
    if from == to do
      raise ArgumentError, "self-loop not allowed: #{inspect(from)}"
    end

    graph
    |> add_node(from)
    |> add_node(to)
    |> put_in([:forward, from], MapSet.put(graph.forward[from] || MapSet.new(), to))
    |> put_in([:reverse, to], MapSet.put(graph.reverse[to] || MapSet.new(), from))
  end

  @doc """
  Returns true if the node exists in the graph.
  """
  def has_node?(graph, node) do
    MapSet.member?(graph.nodes, node)
  end

  @doc """
  Returns true if there's an edge from `from` to `to`.
  """
  def has_edge?(graph, from, to) do
    case Map.get(graph.forward, from) do
      nil -> false
      succs -> MapSet.member?(succs, to)
    end
  end

  @doc """
  Returns all nodes in sorted order (deterministic).
  """
  def nodes(graph) do
    graph.nodes |> MapSet.to_list() |> Enum.sort()
  end

  @doc """
  Returns the direct predecessors of a node (nodes with edges TO this node).

  In the build system, predecessors of B are the packages that B depends on.

  ## Example

      iex> g = BuildTool.DirectedGraph.new()
      ...>   |> BuildTool.DirectedGraph.add_edge("A", "B")
      ...>   |> BuildTool.DirectedGraph.add_edge("C", "B")
      iex> BuildTool.DirectedGraph.predecessors(g, "B")
      {:ok, ["A", "C"]}
  """
  def predecessors(graph, node) do
    case Map.get(graph.reverse, node) do
      nil -> {:error, {:node_not_found, node}}
      preds -> {:ok, preds |> MapSet.to_list() |> Enum.sort()}
    end
  end

  @doc """
  Returns the direct successors of a node (nodes this node has edges TO).

  In the build system, successors of A are the packages that depend on A.
  """
  def successors(graph, node) do
    case Map.get(graph.forward, node) do
      nil -> {:error, {:node_not_found, node}}
      succs -> {:ok, succs |> MapSet.to_list() |> Enum.sort()}
    end
  end

  @doc """
  Returns the number of nodes in the graph.
  """
  def size(graph) do
    MapSet.size(graph.nodes)
  end

  # ---------------------------------------------------------------------------
  # Kahn's algorithm — topological levels (independent groups)
  # ---------------------------------------------------------------------------
  #
  # Kahn's algorithm partitions a DAG into "levels" where nodes at the same
  # level have no dependency on each other. The algorithm works by repeatedly
  # removing nodes with zero in-degree:
  #
  #   1. Compute in-degree for each node (number of predecessors).
  #   2. Collect all nodes with in-degree 0 into the first level.
  #   3. "Remove" these nodes: decrement in-degree for all their successors.
  #   4. Nodes that now have in-degree 0 form the next level.
  #   5. Repeat until all nodes are assigned to a level.
  #   6. If some nodes remain unassigned, the graph has a cycle.
  #
  # For the build system, each level is a set of packages that can be built
  # in parallel. Level 0 has no dependencies. Level 1 depends only on level 0.
  # And so on.
  #
  # Example for a diamond graph (A->B, A->C, B->D, C->D):
  #
  #   Level 0: [A]      -- no dependencies, build first
  #   Level 1: [B, C]   -- depend only on A, can run in parallel
  #   Level 2: [D]      -- depends on B and C, build last

  @doc """
  Partitions nodes into levels by topological depth using Kahn's algorithm.

  Returns `{:ok, levels}` where `levels` is a list of lists of node names.
  Each inner list contains nodes that can be processed in parallel.
  Returns `{:error, :cycle}` if the graph contains a cycle.

  ## Example

      iex> g = BuildTool.DirectedGraph.new()
      ...>   |> BuildTool.DirectedGraph.add_edge("A", "B")
      ...>   |> BuildTool.DirectedGraph.add_edge("A", "C")
      ...>   |> BuildTool.DirectedGraph.add_edge("B", "D")
      ...>   |> BuildTool.DirectedGraph.add_edge("C", "D")
      iex> BuildTool.DirectedGraph.independent_groups(g)
      {:ok, [["A"], ["B", "C"], ["D"]]}
  """
  def independent_groups(graph) do
    # Step 1: compute in-degree for each node.
    in_degree =
      graph.nodes
      |> Enum.reduce(%{}, fn node, acc ->
        Map.put(acc, node, MapSet.size(Map.get(graph.reverse, node, MapSet.new())))
      end)

    # Step 2: collect nodes with in-degree 0 as the initial queue.
    queue =
      in_degree
      |> Enum.filter(fn {_node, deg} -> deg == 0 end)
      |> Enum.map(fn {node, _deg} -> node end)
      |> Enum.sort()

    # Step 3-5: process levels iteratively.
    do_kahn(graph, in_degree, queue, [], 0)
  end

  # Recursive Kahn's implementation. Each call processes one level.
  defp do_kahn(_graph, _in_degree, [], levels, processed) do
    total = Enum.sum(Enum.map(levels, &length/1))

    if total == processed do
      # All nodes have been assigned to a level — this is impossible to reach
      # in the error case because we check total_nodes below.
      {:ok, Enum.reverse(levels)}
    else
      {:error, :cycle}
    end
  end

  defp do_kahn(graph, in_degree, queue, levels, _processed) do
    level = Enum.sort(queue)

    # Decrement in-degree for all successors of nodes in this level.
    new_in_degree =
      Enum.reduce(queue, in_degree, fn node, acc ->
        succs = Map.get(graph.forward, node, MapSet.new())

        Enum.reduce(succs, acc, fn succ, acc2 ->
          Map.update!(acc2, succ, &(&1 - 1))
        end)
      end)

    # Collect nodes that now have in-degree 0.
    next_queue =
      new_in_degree
      |> Enum.filter(fn {node, deg} ->
        deg == 0 and not Enum.any?(levels, fn l -> node in l end) and node not in queue
      end)
      |> Enum.map(fn {node, _deg} -> node end)
      |> Enum.sort()

    total_so_far = length(queue) + Enum.sum(Enum.map(levels, &length/1))
    total_nodes = MapSet.size(graph.nodes)

    if next_queue == [] and total_so_far < total_nodes do
      {:error, :cycle}
    else
      do_kahn(graph, new_in_degree, next_queue, [level | levels], total_nodes)
    end
  end

  # ---------------------------------------------------------------------------
  # Transitive closure and affected nodes
  # ---------------------------------------------------------------------------

  @doc """
  Returns all nodes reachable from the given node by following forward edges.

  This is a BFS traversal. The starting node is NOT included in the result.

  ## Example

      iex> g = BuildTool.DirectedGraph.new()
      ...>   |> BuildTool.DirectedGraph.add_edge("A", "B")
      ...>   |> BuildTool.DirectedGraph.add_edge("B", "C")
      iex> BuildTool.DirectedGraph.transitive_closure(g, "A")
      MapSet.new(["B", "C"])
  """
  def transitive_closure(graph, node) do
    if not has_node?(graph, node) do
      MapSet.new()
    else
      bfs_forward(graph, [node], MapSet.new())
    end
  end

  defp bfs_forward(_graph, [], visited), do: visited

  defp bfs_forward(graph, queue, visited) do
    next =
      queue
      |> Enum.flat_map(fn n ->
        Map.get(graph.forward, n, MapSet.new()) |> MapSet.to_list()
      end)
      |> Enum.reject(&MapSet.member?(visited, &1))
      |> Enum.uniq()

    new_visited = Enum.reduce(next, visited, &MapSet.put(&2, &1))
    bfs_forward(graph, next, new_visited)
  end

  @doc """
  Returns all nodes that transitively depend on any node in the changed set.

  "Affected" means: the changed nodes themselves, plus everything reachable
  by following forward edges from any changed node. This is used by the
  build tool: if you change `logic-gates`, the affected set includes
  `logic-gates` + `arithmetic` + `cpu-simulator` + ...

  ## Example

      iex> g = BuildTool.DirectedGraph.new()
      ...>   |> BuildTool.DirectedGraph.add_edge("A", "B")
      ...>   |> BuildTool.DirectedGraph.add_edge("B", "C")
      iex> BuildTool.DirectedGraph.affected_nodes(g, MapSet.new(["A"]))
      MapSet.new(["A", "B", "C"])
  """
  def affected_nodes(graph, changed) do
    changed
    |> Enum.reduce(MapSet.new(), fn node, acc ->
      if has_node?(graph, node) do
        deps = transitive_closure(graph, node)
        acc |> MapSet.put(node) |> MapSet.union(deps)
      else
        acc
      end
    end)
  end

  @doc """
  Collects all transitive predecessors of a node (everything the node
  depends on). Follows reverse edges via BFS.

  In the build system's graph convention, edge A->B means "B depends on A".
  So to find everything that `node` depends on, we follow predecessors
  (reverse edges).

  ## Example

      iex> g = BuildTool.DirectedGraph.new()
      ...>   |> BuildTool.DirectedGraph.add_edge("A", "B")
      ...>   |> BuildTool.DirectedGraph.add_edge("B", "C")
      iex> BuildTool.DirectedGraph.transitive_predecessors(g, "C")
      MapSet.new(["A", "B"])
  """
  def transitive_predecessors(graph, node) do
    if not has_node?(graph, node) do
      MapSet.new()
    else
      bfs_reverse(graph, [node], MapSet.new())
    end
  end

  defp bfs_reverse(_graph, [], visited), do: visited

  defp bfs_reverse(graph, queue, visited) do
    next =
      queue
      |> Enum.flat_map(fn n ->
        Map.get(graph.reverse, n, MapSet.new()) |> MapSet.to_list()
      end)
      |> Enum.reject(&MapSet.member?(visited, &1))
      |> Enum.uniq()

    new_visited = Enum.reduce(next, visited, &MapSet.put(&2, &1))
    bfs_reverse(graph, next, new_visited)
  end
end
