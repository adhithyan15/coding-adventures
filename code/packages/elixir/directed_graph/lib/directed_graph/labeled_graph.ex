defmodule CodingAdventures.DirectedGraph.LabeledGraph do
  @moduledoc """
  A directed graph where each edge carries one or more string labels.

  Architecture
  ------------

  This struct wraps an inner `Graph` (with `allow_self_loops: true`) and adds
  a label map on top:

      Inner graph: A -> B           (tracks connectivity, runs algorithms)
      Labels map:  {A, B} -> MapSet.new(["friend", "coworker"])

  The inner graph tracks the structural edges, while the labels map tracks
  which labels exist on each edge. This separation means:

  - Adding a second label to an existing edge doesn't duplicate the structural edge
  - Removing a label only removes the structural edge when no labels remain
  - Graph algorithms see the simplified structure (just nodes and edges, no labels)

  ## Example

      alias CodingAdventures.DirectedGraph.LabeledGraph

      lg = LabeledGraph.new()
      {:ok, lg} = LabeledGraph.add_edge(lg, "Alice", "Bob", "friend")
      {:ok, lg} = LabeledGraph.add_edge(lg, "Alice", "Bob", "coworker")
      LabeledGraph.labels(lg, "Alice", "Bob")  # MapSet.new(["friend", "coworker"])
  """

  alias CodingAdventures.DirectedGraph.Graph
  alias CodingAdventures.DirectedGraph.{CycleError, EdgeNotFoundError, NodeNotFoundError}

  @enforce_keys [:graph, :labels]
  defstruct [:graph, :labels]

  @type t :: %__MODULE__{
          graph: Graph.t(),
          labels: %{{any(), any()} => MapSet.t(String.t())}
        }

  # ---------------------------------------------------------------------------
  # Constructor
  # ---------------------------------------------------------------------------

  @doc """
  Create a new empty labeled directed graph.

  The inner graph always has `allow_self_loops: true` because labeled graphs
  commonly need self-referential edges (e.g., state machines).
  """
  @spec new() :: t()
  def new do
    %__MODULE__{
      graph: Graph.new(allow_self_loops: true),
      labels: %{}
    }
  end

  # ---------------------------------------------------------------------------
  # Node operations -- delegate to inner graph
  # ---------------------------------------------------------------------------

  @doc "Add a node to the graph. No-op if it already exists."
  @spec add_node(t(), any()) :: {:ok, t()}
  def add_node(%__MODULE__{} = lg, node) do
    {:ok, graph} = Graph.add_node(lg.graph, node)
    {:ok, %{lg | graph: graph}}
  end

  @doc """
  Remove a node and all its edges (including labels).

  Cleans up both the inner graph AND the label map.
  """
  @spec remove_node(t(), any()) :: {:ok, t()} | {:error, NodeNotFoundError.t()}
  def remove_node(%__MODULE__{} = lg, node) do
    if not Graph.has_node?(lg.graph, node) do
      {:error,
       %NodeNotFoundError{
         message: "Node not found: #{inspect(node)}",
         node: node
       }}
    else
      # Remove all label entries where this node is source or target.
      new_labels =
        lg.labels
        |> Enum.reject(fn {{from, to}, _} -> from == node or to == node end)
        |> Map.new()

      # Remove the node from the inner graph.
      {:ok, graph} = Graph.remove_node(lg.graph, node)
      {:ok, %{lg | graph: graph, labels: new_labels}}
    end
  end

  @doc "Return `true` if the node exists in the graph."
  @spec has_node?(t(), any()) :: boolean()
  def has_node?(%__MODULE__{} = lg, node), do: Graph.has_node?(lg.graph, node)

  @doc "Return a list of all nodes."
  @spec nodes(t()) :: [any()]
  def nodes(%__MODULE__{} = lg), do: Graph.nodes(lg.graph)

  @doc "Return the number of nodes."
  @spec size(t()) :: non_neg_integer()
  def size(%__MODULE__{} = lg), do: Graph.size(lg.graph)

  # ---------------------------------------------------------------------------
  # Labeled edge operations
  # ---------------------------------------------------------------------------

  @doc """
  Add a labeled edge from `from_node` to `to_node`.

  Both nodes are auto-created if they don't exist. If the structural edge
  doesn't exist yet, it's added to the inner graph. The label is then added
  to the label set for this edge pair.

  Multiple calls with the same `{from, to, label}` are idempotent.
  Multiple calls with the same `{from, to}` but different labels add
  multiple labels to the same structural edge.
  """
  @spec add_edge(t(), any(), any(), String.t()) :: {:ok, t()}
  def add_edge(%__MODULE__{} = lg, from_node, to_node, label) do
    # Add the structural edge if it doesn't exist yet.
    graph =
      if Graph.has_edge?(lg.graph, from_node, to_node) do
        # Ensure nodes exist (they should, since the edge exists).
        {:ok, g} = Graph.add_node(lg.graph, from_node)
        {:ok, g} = Graph.add_node(g, to_node)
        g
      else
        {:ok, g} = Graph.add_edge(lg.graph, from_node, to_node)
        g
      end

    # Add the label.
    key = {from_node, to_node}
    current_labels = Map.get(lg.labels, key, MapSet.new())
    new_labels = Map.put(lg.labels, key, MapSet.put(current_labels, label))

    {:ok, %{lg | graph: graph, labels: new_labels}}
  end

  @doc """
  Remove a specific labeled edge.

  Removes the given label. If this was the LAST label on that edge, the
  structural edge is also removed from the inner graph.

  Returns `{:error, %EdgeNotFoundError{}}` if the edge or label doesn't exist.
  """
  @spec remove_edge(t(), any(), any(), String.t()) ::
          {:ok, t()} | {:error, EdgeNotFoundError.t()}
  def remove_edge(%__MODULE__{} = lg, from_node, to_node, label) do
    key = {from_node, to_node}
    current_labels = Map.get(lg.labels, key, MapSet.new())

    if not MapSet.member?(current_labels, label) do
      {:error,
       %EdgeNotFoundError{
         message: "Edge not found: #{inspect(from_node)} -> #{inspect(to_node)} [#{inspect(label)}]",
         from_node: from_node,
         to_node: to_node
       }}
    else
      remaining = MapSet.delete(current_labels, label)

      if MapSet.size(remaining) == 0 do
        # No labels remain -- remove the structural edge too.
        {:ok, graph} = Graph.remove_edge(lg.graph, from_node, to_node)
        {:ok, %{lg | graph: graph, labels: Map.delete(lg.labels, key)}}
      else
        {:ok, %{lg | labels: Map.put(lg.labels, key, remaining)}}
      end
    end
  end

  @doc """
  Check if an edge exists, optionally with a specific label.

  - `has_edge?(lg, "A", "B")` -- true if ANY label exists between A and B
  - `has_edge?(lg, "A", "B", "x")` -- true only if label "x" exists
  """
  @spec has_edge?(t(), any(), any(), String.t() | nil) :: boolean()
  def has_edge?(%__MODULE__{} = lg, from_node, to_node, label \\ nil) do
    key = {from_node, to_node}

    case Map.fetch(lg.labels, key) do
      {:ok, label_set} ->
        if label == nil do
          MapSet.size(label_set) > 0
        else
          MapSet.member?(label_set, label)
        end

      :error ->
        false
    end
  end

  @doc """
  Return all edges as `{from_node, to_node, label}` triples.

  Each label gets its own triple. Labels are sorted within each edge pair.
  """
  @spec edges(t()) :: [{any(), any(), String.t()}]
  def edges(%__MODULE__{} = lg) do
    Enum.flat_map(lg.labels, fn {{from_node, to_node}, label_set} ->
      label_set
      |> MapSet.to_list()
      |> Enum.sort()
      |> Enum.map(fn label -> {from_node, to_node, label} end)
    end)
  end

  @doc """
  Return the set of labels on the edge from `from_node` to `to_node`.

  Returns an empty MapSet if no edge exists.
  """
  @spec labels(t(), any(), any()) :: MapSet.t(String.t())
  def labels(%__MODULE__{} = lg, from_node, to_node) do
    Map.get(lg.labels, {from_node, to_node}, MapSet.new())
  end

  # ---------------------------------------------------------------------------
  # Neighbor queries with optional label filtering
  # ---------------------------------------------------------------------------

  @doc """
  Return successors of a node, optionally filtered by label.

  - `successors(lg, "A")` -- all successors
  - `successors(lg, "A", "friend")` -- only successors connected by "friend"
  """
  @spec successors(t(), any(), String.t() | nil) ::
          {:ok, [any()]} | {:error, NodeNotFoundError.t()}
  def successors(%__MODULE__{} = lg, node, label \\ nil) do
    if not Graph.has_node?(lg.graph, node) do
      {:error,
       %NodeNotFoundError{
         message: "Node not found: #{inspect(node)}",
         node: node
       }}
    else
      {:ok, all_succs} = Graph.successors(lg.graph, node)

      if label == nil do
        {:ok, all_succs}
      else
        filtered =
          Enum.filter(all_succs, fn succ ->
            key = {node, succ}
            label_set = Map.get(lg.labels, key, MapSet.new())
            MapSet.member?(label_set, label)
          end)

        {:ok, filtered}
      end
    end
  end

  @doc """
  Return predecessors of a node, optionally filtered by label.

  - `predecessors(lg, "B")` -- all predecessors
  - `predecessors(lg, "B", "friend")` -- only predecessors connected by "friend"
  """
  @spec predecessors(t(), any(), String.t() | nil) ::
          {:ok, [any()]} | {:error, NodeNotFoundError.t()}
  def predecessors(%__MODULE__{} = lg, node, label \\ nil) do
    if not Graph.has_node?(lg.graph, node) do
      {:error,
       %NodeNotFoundError{
         message: "Node not found: #{inspect(node)}",
         node: node
       }}
    else
      {:ok, all_preds} = Graph.predecessors(lg.graph, node)

      if label == nil do
        {:ok, all_preds}
      else
        filtered =
          Enum.filter(all_preds, fn pred ->
            key = {pred, node}
            label_set = Map.get(lg.labels, key, MapSet.new())
            MapSet.member?(label_set, label)
          end)

        {:ok, filtered}
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Algorithm delegation
  # ---------------------------------------------------------------------------

  @doc "Return a topological ordering. Delegates to inner graph."
  @spec topological_sort(t()) :: {:ok, [any()]} | {:error, CycleError.t()}
  def topological_sort(%__MODULE__{} = lg), do: Graph.topological_sort(lg.graph)

  @doc "Return `true` if the graph contains a cycle. Delegates to inner graph."
  @spec has_cycle?(t()) :: boolean()
  def has_cycle?(%__MODULE__{} = lg), do: Graph.has_cycle?(lg.graph)

  @doc "Return all nodes reachable downstream. Delegates to inner graph."
  @spec transitive_closure(t(), any()) :: {:ok, MapSet.t()} | {:error, NodeNotFoundError.t()}
  def transitive_closure(%__MODULE__{} = lg, node), do: Graph.transitive_closure(lg.graph, node)

  @doc "Return all transitive dependents. Delegates to inner graph."
  @spec transitive_dependents(t(), any()) :: {:ok, MapSet.t()} | {:error, NodeNotFoundError.t()}
  def transitive_dependents(%__MODULE__{} = lg, node),
    do: Graph.transitive_dependents(lg.graph, node)

  @doc "Access the inner Graph struct."
  @spec graph(t()) :: Graph.t()
  def graph(%__MODULE__{} = lg), do: lg.graph
end
