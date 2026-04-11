defmodule CodingAdventures.Graph.NodeNotFoundError do
  defexception [:message, :node]
end

defmodule CodingAdventures.Graph.EdgeNotFoundError do
  defexception [:message, :left, :right]
end

defmodule CodingAdventures.Graph.NotConnectedError do
  defexception message: "graph is not connected"
end
