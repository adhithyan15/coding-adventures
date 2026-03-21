defmodule CodingAdventures.DirectedGraph.CycleError do
  @moduledoc """
  Raised when a topological sort encounters a cycle.

  The `cycle` field contains a list of nodes forming the cycle, starting and
  ending with the same node. For example, if the graph has edges A -> B -> C -> A,
  the cycle might be `["A", "B", "C", "A"]`.

  ## Why a dedicated error?

  Cycles are *expected* failures in dependency graphs -- they're not bugs, they're
  data errors. By using a dedicated exception struct, callers can pattern-match
  on the error type and access the cycle path for reporting.
  """

  defexception [:message, :cycle]

  @type t :: %__MODULE__{
          message: String.t(),
          cycle: [any()]
        }
end

defmodule CodingAdventures.DirectedGraph.NodeNotFoundError do
  @moduledoc """
  Raised when an operation references a node that doesn't exist in the graph.

  The `node` field carries the missing node value so callers can produce
  useful error messages.
  """

  defexception [:message, :node]

  @type t :: %__MODULE__{
          message: String.t(),
          node: any()
        }
end

defmodule CodingAdventures.DirectedGraph.EdgeNotFoundError do
  @moduledoc """
  Raised when `remove_edge` targets a nonexistent edge.

  The `from_node` and `to_node` fields identify the missing edge.
  """

  defexception [:message, :from_node, :to_node]

  @type t :: %__MODULE__{
          message: String.t(),
          from_node: any(),
          to_node: any()
        }
end
