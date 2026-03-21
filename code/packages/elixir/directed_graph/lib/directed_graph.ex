defmodule CodingAdventures.DirectedGraph do
  @moduledoc """
  Directed Graph Library for Elixir
  ==================================

  A directed graph implementation with topological sort, cycle detection, transitive
  closure, and parallel execution level computation. Built for use in build systems,
  dependency resolution, and task scheduling.

  This module serves as the public API, re-exporting the two main structs:

  - `CodingAdventures.DirectedGraph.Graph` -- a basic directed graph
  - `CodingAdventures.DirectedGraph.LabeledGraph` -- a directed graph with labeled edges

  ## Quick Start

      alias CodingAdventures.DirectedGraph.Graph

      {:ok, g} = Graph.new()
                  |> Graph.add_edge("A", "B")
      {:ok, g} = Graph.add_edge(g, "B", "C")

      Graph.topological_sort(g)   # {:ok, ["A", "B", "C"]}
      Graph.has_cycle?(g)         # false

  ## Architecture

  Both Graph and LabeledGraph use **two adjacency maps** internally:

  - `forward[u]` = set of nodes that `u` points TO (successors)
  - `reverse[v]` = set of nodes that point TO `v` (predecessors)

  This dual-map design makes all neighbor queries O(1) and keeps algorithms
  efficient. The trade-off is that every edge mutation updates both maps.

  ## Immutability

  Unlike the Python version which mutates in-place, the Elixir version is
  fully immutable. Every operation returns a new graph struct (or an error
  tuple). This is idiomatic Elixir and plays well with concurrent code.
  """
end
